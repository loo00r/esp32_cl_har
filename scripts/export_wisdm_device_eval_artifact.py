#!/usr/bin/env python3
"""Export read-only WISDM device-evaluation artifacts for ESP32.

This script reproduces the MicroFlow-32 final-corpus preprocessing used by the
training notebook, then writes compact int8 windows and u8 labels for isolated
device-side inference evaluation. It does not stream data to the device and it
does not modify firmware.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import pandas as pd


PROJECT_ROOT = Path(__file__).resolve().parents[1]
DATASET_PATH = PROJECT_ROOT / "data" / "WISDM_ar_v1.1" / "WISDM_ar_v1.1_raw.txt"
METADATA_PATH = (
    PROJECT_ROOT
    / "src"
    / "model_artifacts"
    / "microflow_fullconv32_feature_extractor_int8_metadata.json"
)
OUT_DIR = PROJECT_ROOT / "src" / "eval_artifacts"

ACTIVITY_ORDER = ["Walking", "Jogging", "Upstairs", "Downstairs", "Sitting", "Standing"]
FEATURE_COLUMNS = ["x", "y", "z"]
WINDOW_SIZE = 80
STEP_SIZE = 40
TIMESTAMP_GAP_FACTOR = 3.0
FLAT_LEN = WINDOW_SIZE * len(FEATURE_COLUMNS)

BALANCED_TAGS = {
    "smoke_120": 20,
    "balanced_600": 100,
    "balanced_1200": 200,
}


def load_raw_wisdm(path: Path) -> pd.DataFrame:
    df = pd.read_csv(
        path,
        header=None,
        names=["user_id", "activity", "timestamp", "x", "y", "z_raw"],
        engine="python",
        on_bad_lines="skip",
    )
    df["z"] = pd.to_numeric(df["z_raw"].astype(str).str.rstrip(";"), errors="coerce")
    df = df.drop(columns=["z_raw"])
    return df


def prepare_dataframe(dataframe: pd.DataFrame) -> pd.DataFrame:
    df_model = dataframe.copy()
    df_model = df_model[df_model["activity"].isin(ACTIVITY_ORDER)].copy()
    df_model["user_id"] = pd.to_numeric(df_model["user_id"], errors="coerce")
    df_model["timestamp"] = pd.to_numeric(df_model["timestamp"], errors="coerce")
    df_model[FEATURE_COLUMNS] = df_model[FEATURE_COLUMNS].apply(pd.to_numeric, errors="coerce")
    df_model = df_model.dropna(subset=["user_id", "timestamp", *FEATURE_COLUMNS]).copy()
    df_model["user_id"] = df_model["user_id"].astype(np.int32)
    df_model["timestamp"] = df_model["timestamp"].astype(np.int64)
    df_model = df_model.sort_values(["user_id", "activity", "timestamp"]).reset_index(drop=True)
    activity_to_label = {activity: idx for idx, activity in enumerate(ACTIVITY_ORDER)}
    df_model["label"] = df_model["activity"].map(activity_to_label).astype(np.int32)
    return df_model


def add_contiguous_segments(dataframe: pd.DataFrame) -> pd.DataFrame:
    segmented_groups: list[pd.DataFrame] = []
    for (_, _), group in dataframe.groupby(["user_id", "activity"], sort=False):
        group = group.sort_values("timestamp").copy()
        deltas = group["timestamp"].diff()
        positive_deltas = deltas[deltas > 0]
        if positive_deltas.empty:
            gap_threshold = np.inf
        else:
            gap_threshold = positive_deltas.median() * TIMESTAMP_GAP_FACTOR
        new_segment = deltas.isna() | (deltas <= 0) | (deltas > gap_threshold)
        group["segment_id"] = new_segment.cumsum().astype(np.int32)
        segmented_groups.append(group)
    return pd.concat(segmented_groups, ignore_index=True)


def fit_zscore_stats(dataframe: pd.DataFrame) -> tuple[pd.Series, pd.Series]:
    means = dataframe[FEATURE_COLUMNS].mean()
    stds = dataframe[FEATURE_COLUMNS].std().replace(0, 1.0)
    return means, stds


def apply_zscore_stats(dataframe: pd.DataFrame, means: pd.Series, stds: pd.Series) -> pd.DataFrame:
    scaled = dataframe.copy()
    scaled[FEATURE_COLUMNS] = (scaled[FEATURE_COLUMNS] - means) / stds
    return scaled


def create_windows_from_segments(dataframe: pd.DataFrame) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    windows: list[np.ndarray] = []
    labels: list[int] = []
    subjects: list[int] = []
    grouped = dataframe.groupby(["user_id", "activity", "segment_id"], sort=False)

    for (user_id, _activity, _segment_id), group in grouped:
        values = group[FEATURE_COLUMNS].to_numpy(dtype=np.float32)
        label = int(group["label"].iloc[0])
        if len(values) < WINDOW_SIZE:
            continue
        for start in range(0, len(values) - WINDOW_SIZE + 1, STEP_SIZE):
            windows.append(values[start : start + WINDOW_SIZE])
            labels.append(label)
            subjects.append(int(user_id))

    if not windows:
        raise RuntimeError("No windows were produced from WISDM preprocessing.")

    return (
        np.stack(windows).astype(np.float32),
        np.asarray(labels, dtype=np.uint8),
        np.asarray(subjects, dtype=np.int32),
    )


def quantize_windows(windows: np.ndarray, metadata: dict) -> np.ndarray:
    scale = float(metadata["input_scale"])
    zero_point = int(metadata["input_zero_point"])
    quantized = np.rint(windows / scale).astype(np.int32) + zero_point
    quantized = np.clip(quantized, -128, 127).astype(np.int8)
    return quantized.reshape((len(windows), FLAT_LEN))


def select_balanced_indices(labels: np.ndarray, windows_per_class: int) -> np.ndarray:
    selected: list[np.ndarray] = []
    for class_id in range(len(ACTIVITY_ORDER)):
        class_indices = np.where(labels == class_id)[0]
        if len(class_indices) < windows_per_class:
            raise RuntimeError(
                f"Class {class_id} ({ACTIVITY_ORDER[class_id]}) has only "
                f"{len(class_indices)} windows; need {windows_per_class}."
            )
        positions = np.linspace(0, len(class_indices) - 1, windows_per_class, dtype=np.int64)
        selected.append(class_indices[positions])
    return np.concatenate(selected).astype(np.int64)


def class_distribution(labels: np.ndarray) -> dict[str, int]:
    counts = Counter(int(label) for label in labels)
    return {ACTIVITY_ORDER[class_id]: int(counts[class_id]) for class_id in range(len(ACTIVITY_ORDER))}


def write_artifact(
    tag: str,
    quantized_windows: np.ndarray,
    labels: np.ndarray,
    metadata: dict,
    windows_per_class: int | None,
    source_indices: np.ndarray,
    out_dir: Path,
) -> dict:
    out_dir.mkdir(parents=True, exist_ok=True)
    windows_path = out_dir / f"wisdm_eval_windows_i8_{tag}.bin"
    labels_path = out_dir / f"wisdm_eval_labels_u8_{tag}.bin"
    metadata_path = out_dir / f"wisdm_eval_metadata_{tag}.json"

    quantized_windows.astype(np.int8).tofile(windows_path)
    labels.astype(np.uint8).tofile(labels_path)

    artifact_metadata = {
        "tag": tag,
        "num_windows": int(len(labels)),
        "windows_per_class": windows_per_class,
        "window_len": WINDOW_SIZE,
        "axes": len(FEATURE_COLUMNS),
        "flat_len": FLAT_LEN,
        "num_classes": len(ACTIVITY_ORDER),
        "class_names": ACTIVITY_ORDER,
        "source_dataset": str(DATASET_PATH.relative_to(PROJECT_ROOT)),
        "preprocessing_note": (
            "WISDM rows cleaned and sorted by user_id/activity/timestamp; contiguous "
            "segments split by timestamp gaps; full-corpus z-score normalization; "
            "80x3 windows with stride 40; MicroFlow-32 int8 quantization."
        ),
        "input_scale": float(metadata["input_scale"]),
        "input_zero_point": int(metadata["input_zero_point"]),
        "class_distribution": class_distribution(labels),
        "creation_timestamp": datetime.now(timezone.utc).isoformat(),
        "selection": "full_corpus" if windows_per_class is None else "deterministic_evenly_spaced_per_class",
        "source_index_min": int(source_indices.min()) if len(source_indices) else None,
        "source_index_max": int(source_indices.max()) if len(source_indices) else None,
    }
    metadata_path.write_text(json.dumps(artifact_metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    return {
        "tag": tag,
        "num_windows": int(len(labels)),
        "windows_bytes": windows_path.stat().st_size,
        "labels_bytes": labels_path.stat().st_size,
        "metadata": artifact_metadata,
    }


def build_final_corpus() -> tuple[np.ndarray, np.ndarray, np.ndarray, dict]:
    raw = load_raw_wisdm(DATASET_PATH)
    df_model = add_contiguous_segments(prepare_dataframe(raw))
    final_means, final_stds = fit_zscore_stats(df_model)
    df_model_final = apply_zscore_stats(df_model, final_means, final_stds)
    windows, labels, subjects = create_windows_from_segments(df_model_final)
    metadata = json.loads(METADATA_PATH.read_text(encoding="utf-8"))
    return windows, labels, subjects, metadata


def export_requested(tags: list[str]) -> list[dict]:
    windows, labels, _subjects, metadata = build_final_corpus()
    quantized = quantize_windows(windows, metadata)
    results: list[dict] = []

    for tag in tags:
        if tag in BALANCED_TAGS:
            windows_per_class = BALANCED_TAGS[tag]
            try:
                indices = select_balanced_indices(labels, windows_per_class)
            except RuntimeError as exc:
                print(f"SKIP {tag}: {exc}")
                continue
            result = write_artifact(
                tag,
                quantized[indices],
                labels[indices],
                metadata,
                windows_per_class,
                indices,
                OUT_DIR,
            )
            results.append(result)
            continue

        if tag == "full_9154":
            if len(labels) != 9154:
                print(
                    f"SKIP full_9154: reproduced corpus has {len(labels)} windows, "
                    "not the expected 9154.",
                )
                continue
            indices = np.arange(len(labels), dtype=np.int64)
            result = write_artifact(tag, quantized, labels, metadata, None, indices, OUT_DIR)
            results.append(result)
            continue

        raise ValueError(f"Unknown artifact tag: {tag}")

    return results


def main() -> int:
    parser = argparse.ArgumentParser(description="Export WISDM ESP32 eval artifacts.")
    parser.add_argument(
        "--tags",
        nargs="+",
        default=["smoke_120", "balanced_600", "balanced_1200", "full_9154"],
        help="Artifact tags to export.",
    )
    args = parser.parse_args()

    results = export_requested(args.tags)
    for result in results:
        print(
            "ARTIFACT",
            f"tag={result['tag']}",
            f"num_windows={result['num_windows']}",
            f"windows_bytes={result['windows_bytes']}",
            f"labels_bytes={result['labels_bytes']}",
            f"class_distribution={result['metadata']['class_distribution']}",
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
