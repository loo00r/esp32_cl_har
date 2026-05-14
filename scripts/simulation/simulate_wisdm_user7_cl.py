#!/usr/bin/env python3
"""PC-side target-user CL simulation for fold-specific WISDM artifacts.

This mirrors the small ESP32 OnlineLayer32 + ReplayBuffer32 update loop without
touching firmware. It is intended as a gate before implementing device-side
target-user adaptation.
"""

from __future__ import annotations

import argparse
import csv
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import tensorflow as tf


PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ARTIFACT_ROOT = PROJECT_ROOT / "results" / "fold_artifacts"
OUT_DIR = PROJECT_ROOT / "results" / "tables"

CLASS_NAMES = ["Walking", "Jogging", "Upstairs", "Downstairs", "Sitting", "Standing"]
NUM_CLASSES = len(CLASS_NAMES)
FEATURE_DIM = 32
WINDOW_FLAT_LEN = 240
REPLAY_SLOTS_PER_CLASS = 16
LABELS_PER_UPDATE = 10
BATCH_SIZE = 12
LEARNING_RATE = 0.001
REPLAY_SEED = 0x1234_5678


class XorShift32:
    def __init__(self, seed: int = REPLAY_SEED) -> None:
        self.state = seed & 0xFFFFFFFF

    def next_u32(self) -> int:
        x = self.state or 0xC0DE_0420
        x ^= (x << 13) & 0xFFFFFFFF
        x ^= x >> 17
        x ^= (x << 5) & 0xFFFFFFFF
        self.state = x & 0xFFFFFFFF
        return self.state

    def bounded(self, upper: int) -> int:
        if upper <= 1:
            return 0
        return self.next_u32() % upper


class ReplayBuffer:
    def __init__(self, policy: str, seed: int = REPLAY_SEED) -> None:
        self.policy = policy
        self.features = np.zeros((NUM_CLASSES, REPLAY_SLOTS_PER_CLASS, FEATURE_DIM), dtype=np.float32)
        self.seen = np.zeros((NUM_CLASSES,), dtype=np.uint32)
        self.length = np.zeros((NUM_CLASSES,), dtype=np.uint8)
        self.fifo_next = np.zeros((NUM_CLASSES,), dtype=np.uint8)
        self.rng = XorShift32(seed)

    def push(self, label: int, features: np.ndarray) -> None:
        if self.policy == "fifo":
            self.push_fifo(label, features)
        elif self.policy == "reservoir":
            self.push_reservoir(label, features)
        else:
            raise ValueError(f"unsupported policy: {self.policy}")

    def push_fifo(self, label: int, features: np.ndarray) -> None:
        self.seen[label] += 1
        if self.length[label] < REPLAY_SLOTS_PER_CLASS:
            slot = int(self.length[label])
            self.features[label, slot] = features
            self.length[label] += 1
            self.fifo_next[label] = self.length[label] % REPLAY_SLOTS_PER_CLASS
            return
        slot = int(self.fifo_next[label])
        self.features[label, slot] = features
        self.fifo_next[label] = (slot + 1) % REPLAY_SLOTS_PER_CLASS

    def push_reservoir(self, label: int, features: np.ndarray) -> None:
        seen_after = int(self.seen[label]) + 1
        self.seen[label] = seen_after
        if self.length[label] < REPLAY_SLOTS_PER_CLASS:
            slot = int(self.length[label])
            self.features[label, slot] = features
            self.length[label] += 1
            return
        candidate = self.rng.bounded(seen_after)
        if candidate < REPLAY_SLOTS_PER_CLASS:
            self.features[label, candidate] = features

    def sample_balanced_batch(self, batch_size: int = BATCH_SIZE) -> tuple[np.ndarray, np.ndarray]:
        out_x: list[np.ndarray] = []
        out_y: list[int] = []
        empty_classes_seen = 0
        class_idx = self.rng.bounded(NUM_CLASSES)

        while len(out_x) < batch_size and empty_classes_seen < NUM_CLASSES:
            class_len = int(self.length[class_idx])
            if class_len == 0:
                empty_classes_seen += 1
            else:
                empty_classes_seen = 0
                slot = self.rng.bounded(class_len)
                out_x.append(self.features[class_idx, slot].copy())
                out_y.append(class_idx)
            class_idx = (class_idx + 1) % NUM_CLASSES

        if not out_x:
            return np.empty((0, FEATURE_DIM), dtype=np.float32), np.empty((0,), dtype=np.uint8)
        return np.stack(out_x).astype(np.float32), np.asarray(out_y, dtype=np.uint8)


@dataclass
class OnlineLayer:
    weights: np.ndarray
    bias: np.ndarray

    def copy(self) -> "OnlineLayer":
        return OnlineLayer(self.weights.copy(), self.bias.copy())

    def logits(self, features: np.ndarray) -> np.ndarray:
        return features @ self.weights + self.bias

    def probs(self, features: np.ndarray) -> np.ndarray:
        logits = self.logits(features)
        logits = logits - np.max(logits, axis=1, keepdims=True)
        exp_values = np.exp(logits)
        return exp_values / np.sum(exp_values, axis=1, keepdims=True)

    def predict(self, features: np.ndarray) -> np.ndarray:
        return np.argmax(self.probs(features), axis=1).astype(np.uint8)

    def backward_batch(self, batch: np.ndarray, labels: np.ndarray, lr: float = LEARNING_RATE) -> None:
        if len(batch) == 0 or len(batch) != len(labels) or lr <= 0:
            return
        probs = self.probs(batch)
        delta = probs.copy()
        for row, label in enumerate(labels.astype(int)):
            if 0 <= label < NUM_CLASSES:
                delta[row, label] -= 1.0
        inv_batch = 1.0 / len(batch)
        grad_w = batch.T @ delta
        grad_b = np.sum(delta, axis=0)
        self.weights -= lr * grad_w * inv_batch
        self.bias -= lr * grad_b * inv_batch


def load_head(path: Path) -> OnlineLayer:
    payload = json.loads(path.read_text(encoding="utf-8"))
    return OnlineLayer(
        weights=np.asarray(payload["weights"], dtype=np.float32),
        bias=np.asarray(payload["bias"], dtype=np.float32),
    )


def default_artifact_dir(target_user: int) -> Path:
    return DEFAULT_ARTIFACT_ROOT / f"wisdm_user{target_user}_microflow32"


def load_target_data(artifact_dir: Path, target_user: int) -> tuple[np.ndarray, np.ndarray]:
    windows = np.fromfile(artifact_dir / f"wisdm_user{target_user}_target_windows_i8.bin", dtype=np.int8)
    labels = np.fromfile(artifact_dir / f"wisdm_user{target_user}_target_labels_u8.bin", dtype=np.uint8)
    if len(windows) != len(labels) * WINDOW_FLAT_LEN:
        raise RuntimeError("target windows/labels size mismatch")
    return windows.reshape((len(labels), WINDOW_FLAT_LEN)), labels


def extract_features(artifact_dir: Path, windows: np.ndarray) -> np.ndarray:
    feature_paths = sorted(artifact_dir.glob("microflow32_user*_feature_extractor_int8.tflite"))
    metadata_paths = sorted(artifact_dir.glob("microflow32_user*_feature_extractor_int8_metadata.json"))
    if len(feature_paths) != 1 or len(metadata_paths) != 1:
        raise RuntimeError(f"Expected one feature extractor artifact in {artifact_dir}")
    model_path = feature_paths[0]
    metadata = json.loads(metadata_paths[0].read_text())
    output_scale = float(metadata["output_scale"])
    output_zero_point = int(metadata["output_zero_point"])

    interpreter = tf.lite.Interpreter(model_path=str(model_path))
    interpreter.allocate_tensors()
    input_details = interpreter.get_input_details()[0]
    output_details = interpreter.get_output_details()[0]

    features = np.empty((len(windows), FEATURE_DIM), dtype=np.float32)
    for idx, flat in enumerate(windows):
        interpreter.set_tensor(input_details["index"], flat.reshape(input_details["shape"]).astype(np.int8))
        interpreter.invoke()
        output = interpreter.get_tensor(output_details["index"]).reshape((FEATURE_DIM,)).astype(np.int16)
        features[idx] = (output.astype(np.float32) - output_zero_point) * output_scale
    return features


def select_adaptation_indices(labels: np.ndarray, per_class: int) -> np.ndarray:
    selected: list[int] = []
    for class_id in range(NUM_CLASSES):
        class_indices = np.where(labels == class_id)[0]
        count = min(per_class, max(0, len(class_indices) - 1))
        selected.extend(class_indices[:count].tolist())
    return np.asarray(sorted(selected), dtype=np.int64)


def confusion(y_true: np.ndarray, y_pred: np.ndarray) -> np.ndarray:
    matrix = np.zeros((NUM_CLASSES, NUM_CLASSES), dtype=np.int32)
    for truth, pred in zip(y_true.astype(int), y_pred.astype(int), strict=True):
        matrix[truth, pred] += 1
    return matrix


def summarize_predictions(
    mode: str,
    budget: int,
    lr: float,
    eval_split: str,
    y_true: np.ndarray,
    y_pred: np.ndarray,
    train_updates: int,
    adapted_labels: int,
) -> list[dict[str, object]]:
    matrix = confusion(y_true, y_pred)
    rows: list[dict[str, object]] = []
    total = int(len(y_true))
    correct = int(np.trace(matrix))
    accuracy = correct / total if total else 0.0
    for class_id, name in enumerate(CLASS_NAMES):
        support = int(matrix[class_id].sum())
        class_correct = int(matrix[class_id, class_id])
        rows.append(
            {
                "mode": mode,
                "budget_per_class": budget,
                "lr": lr,
                "eval_split": eval_split,
                "class": name,
                "support": support,
                "correct": class_correct,
                "recall": class_correct / support if support else 0.0,
                "total": total,
                "total_correct": correct,
                "accuracy": accuracy,
                "adapted_labels": adapted_labels,
                "train_updates": train_updates,
            }
        )
    return rows


def run_adaptation(
    base_layer: OnlineLayer,
    features: np.ndarray,
    labels: np.ndarray,
    adaptation_indices: np.ndarray,
    policy: str,
    lr: float,
) -> tuple[OnlineLayer, int]:
    layer = base_layer.copy()
    replay = ReplayBuffer(policy)
    labels_since_update = 0
    train_updates = 0
    for idx in adaptation_indices:
        label = int(labels[idx])
        replay.push(label, features[idx])
        labels_since_update += 1
        if labels_since_update >= LABELS_PER_UPDATE:
            batch_x, batch_y = replay.sample_balanced_batch()
            layer.backward_batch(batch_x, batch_y, lr=lr)
            train_updates += 1
            labels_since_update = 0
    return layer, train_updates


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    if not rows:
        return
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    parser = argparse.ArgumentParser(description="Simulate target-user CL on fold-specific WISDM features.")
    parser.add_argument("--target-user", type=int, default=7)
    parser.add_argument("--artifact-dir", type=Path, default=None)
    parser.add_argument("--budgets", nargs="+", type=int, default=[3, 5, 10, 20])
    parser.add_argument("--lrs", nargs="+", type=float, default=[0.001])
    args = parser.parse_args()

    artifact_dir = args.artifact_dir or default_artifact_dir(args.target_user)
    windows, labels = load_target_data(artifact_dir, args.target_user)
    features = extract_features(artifact_dir, windows)
    base_layer = load_head(artifact_dir / f"microflow32_user{args.target_user}_head_float.json")

    rows: list[dict[str, object]] = []
    base_pred = base_layer.predict(features)
    rows.extend(summarize_predictions("no_adapt", 0, 0.0, "all", labels, base_pred, 0, 0))

    for budget in args.budgets:
        adaptation_indices = select_adaptation_indices(labels, budget)
        eval_mask = np.ones((len(labels),), dtype=bool)
        eval_mask[adaptation_indices] = False
        eval_indices = np.where(eval_mask)[0]

        no_adapt_eval_pred = base_layer.predict(features[eval_indices])
        rows.extend(
            summarize_predictions(
                "no_adapt_eval_split",
                budget,
                0.0,
                "held_out_after_adapt_selection",
                labels[eval_indices],
                no_adapt_eval_pred,
                0,
                int(len(adaptation_indices)),
            )
        )

        for lr in args.lrs:
            for policy in ["fifo", "reservoir"]:
                adapted_layer, train_updates = run_adaptation(base_layer, features, labels, adaptation_indices, policy, lr)
                pred = adapted_layer.predict(features[eval_indices])
                rows.extend(
                    summarize_predictions(
                        policy,
                        budget,
                        lr,
                        "held_out_after_adapt_selection",
                        labels[eval_indices],
                        pred,
                        train_updates,
                        int(len(adaptation_indices)),
                    )
                )

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out_path = OUT_DIR / f"wisdm_user{args.target_user}_pc_cl_simulation.csv"
    write_csv(out_path, rows)

    print(f"output={out_path}")
    for row in rows:
        if row["class"] == "Downstairs":
            print(
                f"mode={row['mode']} budget={row['budget_per_class']} "
                f"lr={row['lr']} "
                f"eval={row['eval_split']} acc={row['accuracy']:.4f} "
                f"downstairs_recall={row['recall']:.4f} updates={row['train_updates']} "
                f"labels={row['adapted_labels']}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
