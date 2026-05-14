#!/usr/bin/env python3
"""PC-only CL gate on the balanced_600 WISDM device-eval artifact.

The split is stratified inside the existing balanced_600 artifact:
adaptation windows are excluded from evaluation, so this is not train-on-test.
No firmware is changed or executed.
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
EVAL_ARTIFACT_DIR = PROJECT_ROOT / "src" / "eval_artifacts"
MODEL_ARTIFACT_DIR = PROJECT_ROOT / "src" / "model_artifacts"
OUT_DIR = PROJECT_ROOT / "results" / "tables"

CLASS_NAMES = ["Walking", "Jogging", "Upstairs", "Downstairs", "Sitting", "Standing"]
NUM_CLASSES = len(CLASS_NAMES)
FEATURE_DIM = 32
WINDOW_FLAT_LEN = 240
REPLAY_SLOTS_PER_CLASS = 16
BATCH_SIZE = 12
LABELS_PER_UPDATE = 10
LEARNING_RATE = 0.001
REPLAY_SEED = 0x1234_5678


class XorShift32:
    def __init__(self, seed: int = REPLAY_SEED) -> None:
        self.state = seed & 0xFFFFFFFF

    def bounded(self, upper: int) -> int:
        if upper <= 1:
            return 0
        return self.next_u32() % upper

    def next_u32(self) -> int:
        x = self.state or 0xC0DE_0420
        x ^= (x << 13) & 0xFFFFFFFF
        x ^= x >> 17
        x ^= (x << 5) & 0xFFFFFFFF
        self.state = x & 0xFFFFFFFF
        return self.state


class ReplayBuffer:
    def __init__(self, policy: str) -> None:
        self.policy = policy
        self.features = np.zeros((NUM_CLASSES, REPLAY_SLOTS_PER_CLASS, FEATURE_DIM), dtype=np.float32)
        self.seen = np.zeros((NUM_CLASSES,), dtype=np.uint32)
        self.length = np.zeros((NUM_CLASSES,), dtype=np.uint8)
        self.fifo_next = np.zeros((NUM_CLASSES,), dtype=np.uint8)
        self.rng = XorShift32()

    def push(self, label: int, features: np.ndarray) -> None:
        if self.policy == "fifo":
            self.push_fifo(label, features)
        elif self.policy == "reservoir":
            self.push_reservoir(label, features)
        else:
            raise ValueError(self.policy)

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

    def sample_balanced_batch(self) -> tuple[np.ndarray, np.ndarray]:
        batch_x: list[np.ndarray] = []
        batch_y: list[int] = []
        empty_classes_seen = 0
        class_idx = self.rng.bounded(NUM_CLASSES)
        while len(batch_x) < BATCH_SIZE and empty_classes_seen < NUM_CLASSES:
            class_len = int(self.length[class_idx])
            if class_len == 0:
                empty_classes_seen += 1
            else:
                empty_classes_seen = 0
                slot = self.rng.bounded(class_len)
                batch_x.append(self.features[class_idx, slot].copy())
                batch_y.append(class_idx)
            class_idx = (class_idx + 1) % NUM_CLASSES
        if not batch_x:
            return np.empty((0, FEATURE_DIM), dtype=np.float32), np.empty((0,), dtype=np.uint8)
        return np.stack(batch_x).astype(np.float32), np.asarray(batch_y, dtype=np.uint8)


@dataclass
class OnlineLayer:
    weights: np.ndarray
    bias: np.ndarray

    def copy(self) -> "OnlineLayer":
        return OnlineLayer(self.weights.copy(), self.bias.copy())

    def probs(self, features: np.ndarray) -> np.ndarray:
        logits = features @ self.weights + self.bias
        logits -= np.max(logits, axis=1, keepdims=True)
        exp_values = np.exp(logits)
        return exp_values / np.sum(exp_values, axis=1, keepdims=True)

    def predict(self, features: np.ndarray) -> np.ndarray:
        return np.argmax(self.probs(features), axis=1).astype(np.uint8)

    def backward_batch(self, batch: np.ndarray, labels: np.ndarray) -> None:
        if len(batch) == 0:
            return
        probs = self.probs(batch)
        delta = probs.copy()
        for row, label in enumerate(labels.astype(int)):
            delta[row, label] -= 1.0
        inv_batch = 1.0 / len(batch)
        self.weights -= LEARNING_RATE * (batch.T @ delta) * inv_batch
        self.bias -= LEARNING_RATE * np.sum(delta, axis=0) * inv_batch


def load_online_layer() -> OnlineLayer:
    # Values match src/online_layer.rs and the final-corpus MicroFlow-32 head.
    import ast

    text = (PROJECT_ROOT / "src" / "online_layer.rs").read_text(encoding="utf-8")
    start = text.index("pub const MICROFLOW32_HEAD_WEIGHTS")
    bias_start = text.index("pub const MICROFLOW32_HEAD_BIAS")
    weights_block = text[start:bias_start]
    bias_block = text[bias_start:text.index("impl OnlineLayer", bias_start)]

    def parse_array(block: str):
        equals = block.index("=")
        array_start = block.index("[", equals)
        array_end = block.index("];", array_start)
        return ast.literal_eval(block[array_start : array_end + 1])

    weights_values = parse_array(weights_block)
    bias_values = parse_array(bias_block)
    weights = np.asarray(weights_values, dtype=np.float32)
    bias = np.asarray(bias_values, dtype=np.float32)
    if weights.shape != (FEATURE_DIM, NUM_CLASSES) or bias.shape != (NUM_CLASSES,):
        raise RuntimeError("Failed to parse final MicroFlow-32 head from src/online_layer.rs")
    return OnlineLayer(weights=weights, bias=bias)


def load_balanced600() -> tuple[np.ndarray, np.ndarray]:
    windows = np.fromfile(EVAL_ARTIFACT_DIR / "wisdm_eval_windows_i8_balanced_600.bin", dtype=np.int8)
    labels = np.fromfile(EVAL_ARTIFACT_DIR / "wisdm_eval_labels_u8_balanced_600.bin", dtype=np.uint8)
    if len(windows) != len(labels) * WINDOW_FLAT_LEN:
        raise RuntimeError("balanced_600 artifact size mismatch")
    return windows.reshape((len(labels), WINDOW_FLAT_LEN)), labels


def extract_features(windows: np.ndarray) -> np.ndarray:
    model_path = MODEL_ARTIFACT_DIR / "microflow_fullconv32_feature_extractor_int8.tflite"
    metadata = json.loads((MODEL_ARTIFACT_DIR / "microflow_fullconv32_feature_extractor_int8_metadata.json").read_text())
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


def split_indices(labels: np.ndarray, adaptation_per_class: int) -> tuple[np.ndarray, np.ndarray]:
    adapt: list[int] = []
    eval_idx: list[int] = []
    for class_id in range(NUM_CLASSES):
        class_indices = np.where(labels == class_id)[0]
        adapt.extend(class_indices[:adaptation_per_class].tolist())
        eval_idx.extend(class_indices[adaptation_per_class:].tolist())
    return np.asarray(sorted(adapt), dtype=np.int64), np.asarray(sorted(eval_idx), dtype=np.int64)


def adapt_layer(base: OnlineLayer, features: np.ndarray, labels: np.ndarray, adapt_idx: np.ndarray, policy: str) -> tuple[OnlineLayer, int]:
    layer = base.copy()
    replay = ReplayBuffer(policy)
    labels_since_update = 0
    updates = 0
    for idx in adapt_idx:
        replay.push(int(labels[idx]), features[idx])
        labels_since_update += 1
        if labels_since_update >= LABELS_PER_UPDATE:
            batch_x, batch_y = replay.sample_balanced_batch()
            layer.backward_batch(batch_x, batch_y)
            updates += 1
            labels_since_update = 0
    return layer, updates


def summarize(mode: str, budget: int, y_true: np.ndarray, y_pred: np.ndarray, adapted_labels: int, updates: int) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    total = int(len(y_true))
    total_correct = int(np.count_nonzero(y_true == y_pred))
    for class_id, name in enumerate(CLASS_NAMES):
        mask = y_true == class_id
        support = int(np.count_nonzero(mask))
        correct = int(np.count_nonzero(y_pred[mask] == class_id))
        rows.append(
            {
                "mode": mode,
                "adaptation_per_class": budget,
                "class": name,
                "support": support,
                "correct": correct,
                "recall": correct / support if support else 0.0,
                "total": total,
                "total_correct": total_correct,
                "accuracy": total_correct / total if total else 0.0,
                "adapted_labels": adapted_labels,
                "train_updates": updates,
            }
        )
    return rows


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    parser = argparse.ArgumentParser(description="Simulate CL on balanced_600 adaptation/eval split.")
    parser.add_argument("--adaptation-per-class", type=int, default=20)
    args = parser.parse_args()

    windows, labels = load_balanced600()
    features = extract_features(windows)
    base_layer = load_online_layer()
    adapt_idx, eval_idx = split_indices(labels, args.adaptation_per_class)

    rows: list[dict[str, object]] = []
    y_eval = labels[eval_idx]
    no_adapt_pred = base_layer.predict(features[eval_idx])
    rows.extend(summarize("no_adapt", args.adaptation_per_class, y_eval, no_adapt_pred, 0, 0))

    for policy in ["fifo", "reservoir"]:
        layer, updates = adapt_layer(base_layer, features, labels, adapt_idx, policy)
        pred = layer.predict(features[eval_idx])
        rows.extend(summarize(policy, args.adaptation_per_class, y_eval, pred, len(adapt_idx), updates))

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out_path = OUT_DIR / f"wisdm_balanced600_pc_cl_split_{args.adaptation_per_class}perclass.csv"
    write_csv(out_path, rows)
    print(f"output={out_path}")
    for row in rows:
        if row["class"] in {"Upstairs", "Downstairs"}:
            print(
                f"mode={row['mode']} class={row['class']} acc={row['accuracy']:.4f} "
                f"recall={row['recall']:.4f} updates={row['train_updates']}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
