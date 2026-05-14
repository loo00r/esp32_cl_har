#!/usr/bin/env python3

import json
from pathlib import Path

import numpy as np
import pandas as pd


PROJECT_ROOT = Path(__file__).resolve().parents[2]
DATASET_PATH = PROJECT_ROOT / "data" / "WISDM_ar_v1.1" / "WISDM_ar_v1.1_raw.txt"
METADATA_PATH = Path("/tmp/esp32_cl_har_artifacts/microflow_fullconv_classifier_int8_metadata.json")

WINDOW_SIZE = 80
FEATURE_COLUMNS = ["x", "y", "z"]
ACTIVITY_ORDER = ["Walking", "Jogging", "Upstairs", "Downstairs", "Sitting", "Standing"]

MPU6050_LSB_PER_G = 16_384.0
STANDARD_GRAVITY_MPS2 = 9.80665


def load_wisdm_dataframe(path: Path) -> pd.DataFrame:
    df = pd.read_csv(
        path,
        header=None,
        names=["user_id", "activity", "timestamp", "x", "y", "z_raw"],
        engine="python",
        on_bad_lines="skip",
    )
    df["z"] = pd.to_numeric(df["z_raw"].astype(str).str.rstrip(";"), errors="coerce")
    df = df.drop(columns=["z_raw"])
    df["user_id"] = pd.to_numeric(df["user_id"], errors="coerce")
    df["timestamp"] = pd.to_numeric(df["timestamp"], errors="coerce")
    df[FEATURE_COLUMNS] = df[FEATURE_COLUMNS].apply(pd.to_numeric, errors="coerce")
    df = df.dropna(subset=["user_id", "timestamp", *FEATURE_COLUMNS]).copy()
    df = df[df["activity"].isin(ACTIVITY_ORDER)].copy()
    df = df.sort_values(["user_id", "timestamp"]).reset_index(drop=True)
    return df


def find_first_contiguous_window(df: pd.DataFrame, window_size: int = WINDOW_SIZE) -> pd.DataFrame:
    for (_, activity), group in df.groupby(["user_id", "activity"], sort=False):
        if len(group) < window_size:
            continue
        return group.iloc[:window_size].copy()
    raise RuntimeError("No contiguous WISDM window with 80 samples found")


def load_metadata(path: Path) -> dict:
    return json.loads(path.read_text())


def quantize_scalar(value: float, scale: float, zero_point: int) -> np.int8:
    centered = int(round(value / scale)) + int(zero_point)
    centered = max(-128, min(127, centered))
    return np.int8(centered)


def rust_equivalent_quantized(window_mps2: np.ndarray, metadata: dict) -> np.ndarray:
    means = np.array([metadata["zscore_means"][axis] for axis in FEATURE_COLUMNS], dtype=np.float32)
    stds = np.array([metadata["zscore_stds"][axis] for axis in FEATURE_COLUMNS], dtype=np.float32)
    scale = float(metadata["input_scale"])
    zero_point = int(metadata["input_zero_point"])

    # Simulate what firmware receives from MPU6050: raw counts, then convert back to m/s².
    raw_counts = np.rint(window_mps2 / STANDARD_GRAVITY_MPS2 * MPU6050_LSB_PER_G).astype(np.int16)
    reconstructed_mps2 = (raw_counts.astype(np.float32) / MPU6050_LSB_PER_G) * STANDARD_GRAVITY_MPS2
    normalized = (reconstructed_mps2 - means) / stds

    flat = np.empty(WINDOW_SIZE * len(FEATURE_COLUMNS), dtype=np.int8)
    for sample_index in range(WINDOW_SIZE):
        base = sample_index * len(FEATURE_COLUMNS)
        for axis in range(len(FEATURE_COLUMNS)):
            flat[base + axis] = quantize_scalar(normalized[sample_index, axis], scale, zero_point)
    return flat


def python_reference_quantized(window_mps2: np.ndarray, metadata: dict) -> np.ndarray:
    means = np.array([metadata["zscore_means"][axis] for axis in FEATURE_COLUMNS], dtype=np.float32)
    stds = np.array([metadata["zscore_stds"][axis] for axis in FEATURE_COLUMNS], dtype=np.float32)
    scale = float(metadata["input_scale"])
    zero_point = int(metadata["input_zero_point"])

    normalized = (window_mps2.astype(np.float32) - means) / stds

    flat = np.empty(WINDOW_SIZE * len(FEATURE_COLUMNS), dtype=np.int8)
    for sample_index in range(WINDOW_SIZE):
        base = sample_index * len(FEATURE_COLUMNS)
        for axis in range(len(FEATURE_COLUMNS)):
            flat[base + axis] = quantize_scalar(normalized[sample_index, axis], scale, zero_point)
    return flat


def main() -> None:
    df = load_wisdm_dataframe(DATASET_PATH)
    metadata = load_metadata(METADATA_PATH)
    window_df = find_first_contiguous_window(df)
    window_mps2 = window_df[FEATURE_COLUMNS].to_numpy(dtype=np.float32)

    rust_q = rust_equivalent_quantized(window_mps2, metadata)
    python_q = python_reference_quantized(window_mps2, metadata)

    diff = rust_q.astype(np.int16) - python_q.astype(np.int16)
    max_abs_diff = int(np.max(np.abs(diff)))
    mismatches = int(np.count_nonzero(diff))

    print("Quantization sanity check")
    print(f"dataset: {DATASET_PATH}")
    print(f"metadata: {METADATA_PATH}")
    print(
        "window:",
        f"user={int(window_df['user_id'].iloc[0])}",
        f"activity={window_df['activity'].iloc[0]}",
        f"timestamp_start={int(window_df['timestamp'].iloc[0])}",
    )
    print(f"input shape: {metadata['input_shape']}")
    print(f"flat tensor length: {len(rust_q)}")
    print(f"max_abs_diff: {max_abs_diff}")
    print(f"mismatch_count: {mismatches}")
    print("first 12 python_q:", python_q[:12].tolist())
    print("first 12 rust_q:  ", rust_q[:12].tolist())

    if max_abs_diff > 1:
        raise SystemExit("Quantization mismatch is too large; inspect scale/layout before firmware integration.")


if __name__ == "__main__":
    main()
