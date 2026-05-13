#!/usr/bin/env python3
"""Train/export a fold-specific MicroFlow-32 artifact for one held-out WISDM user.

This is a PC-side staged step for a future target-user CL experiment. It does
not modify firmware artifacts under src/model_artifacts and does not run ESP32.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import tensorflow as tf
from tensorflow import keras
from tensorflow.keras import layers
from tensorflow.keras.utils import to_categorical

from export_wisdm_device_eval_artifact import (
    ACTIVITY_ORDER,
    FEATURE_COLUMNS,
    FLAT_LEN,
    STEP_SIZE,
    WINDOW_SIZE,
    add_contiguous_segments,
    apply_zscore_stats,
    create_windows_from_segments,
    fit_zscore_stats,
    load_raw_wisdm,
    prepare_dataframe,
)


PROJECT_ROOT = Path(__file__).resolve().parents[1]
DATASET_PATH = PROJECT_ROOT / "data" / "WISDM_ar_v1.1" / "WISDM_ar_v1.1_raw.txt"
DEFAULT_OUT_DIR = PROJECT_ROOT / "results" / "fold_artifacts" / "wisdm_user7_microflow32"
SUMMARY_PATH = PROJECT_ROOT / "results" / "tables" / "wisdm_fold_user7_microflow32_summary.csv"

MICROFLOW_FEATURE_DIM = 32
NUM_CLASSES = len(ACTIVITY_ORDER)
REPRESENTATIVE_SAMPLES_PER_CLASS = 32


def set_reproducibility(seed: int) -> None:
    os.environ.setdefault("PYTHONHASHSEED", str(seed))
    np.random.seed(seed)
    tf.random.set_seed(seed)


def build_microflow_fullconv_models(input_shape: tuple[int, int, int]) -> tuple[keras.Model, keras.Model]:
    inputs = keras.Input(shape=input_shape, name="imu_window")
    x = layers.Conv2D(
        filters=32,
        kernel_size=(5, 3),
        activation="relu",
        padding="valid",
        name="conv2d_1",
    )(inputs)
    x = layers.Conv2D(
        filters=MICROFLOW_FEATURE_DIM,
        kernel_size=(3, 1),
        activation="relu",
        padding="valid",
        name="conv2d_2",
    )(x)
    features = layers.AveragePooling2D(pool_size=(74, 1), name="avg_pool_features")(x)
    logits = layers.Conv2D(
        filters=NUM_CLASSES,
        kernel_size=(1, 1),
        padding="valid",
        name="classifier_head",
    )(features)
    outputs = layers.Softmax(axis=-1, name="classifier_softmax")(logits)

    classifier = keras.Model(inputs=inputs, outputs=outputs, name="microflow32_fold_classifier")
    feature_extractor = keras.Model(
        inputs=inputs,
        outputs=features,
        name="microflow32_fold_feature_extractor",
    )
    classifier.compile(
        optimizer=keras.optimizers.Adam(learning_rate=1e-3),
        loss="categorical_crossentropy",
        metrics=["accuracy"],
    )
    return classifier, feature_extractor


def class_distribution(labels: np.ndarray) -> dict[str, int]:
    counts = Counter(int(label) for label in labels)
    return {name: int(counts[idx]) for idx, name in enumerate(ACTIVITY_ORDER)}


def confusion_matrix(y_true: np.ndarray, y_pred: np.ndarray) -> np.ndarray:
    matrix = np.zeros((NUM_CLASSES, NUM_CLASSES), dtype=np.int32)
    for truth, pred in zip(y_true.astype(int), y_pred.astype(int), strict=True):
        if 0 <= truth < NUM_CLASSES and 0 <= pred < NUM_CLASSES:
            matrix[truth, pred] += 1
    return matrix


def per_class_recall(matrix: np.ndarray) -> dict[str, float]:
    recalls: dict[str, float] = {}
    for class_id, name in enumerate(ACTIVITY_ORDER):
        support = int(matrix[class_id].sum())
        recalls[name] = float(matrix[class_id, class_id] / support) if support else 0.0
    return recalls


def representative_subset(x_train: np.ndarray, y_train: np.ndarray, seed: int) -> np.ndarray:
    rng = np.random.default_rng(seed)
    selected: list[int] = []
    for class_id in range(NUM_CLASSES):
        class_indices = np.where(y_train == class_id)[0]
        if len(class_indices) == 0:
            continue
        sample_count = min(REPRESENTATIVE_SAMPLES_PER_CLASS, len(class_indices))
        selected.extend(rng.choice(class_indices, size=sample_count, replace=False).tolist())
    selected_indices = np.asarray(selected, dtype=np.int32)
    return x_train[selected_indices].astype(np.float32)


def make_representative_dataset(samples: np.ndarray):
    def representative_dataset():
        for idx in range(len(samples)):
            yield [samples[idx : idx + 1]]

    return representative_dataset


def convert_to_int8_tflite(model: keras.Model, representative_samples: np.ndarray) -> bytes:
    converter = tf.lite.TFLiteConverter.from_keras_model(model)
    converter.optimizations = [tf.lite.Optimize.DEFAULT]
    converter.representative_dataset = make_representative_dataset(representative_samples)
    converter.target_spec.supported_ops = [tf.lite.OpsSet.TFLITE_BUILTINS_INT8]
    converter.inference_input_type = tf.int8
    converter.inference_output_type = tf.int8
    return converter.convert()


def tflite_io_metadata(tflite_model: bytes) -> tuple[dict, dict]:
    interpreter = tf.lite.Interpreter(model_content=tflite_model)
    interpreter.allocate_tensors()
    return interpreter.get_input_details()[0], interpreter.get_output_details()[0]


def write_metadata(
    path: Path,
    model_name: str,
    tflite_model: bytes,
    target_user: int,
    train_means,
    train_stds,
    representative_count: int,
    train_distribution: dict[str, int],
    target_distribution: dict[str, int],
    extra: dict,
) -> dict:
    input_details, output_details = tflite_io_metadata(tflite_model)
    input_scale, input_zero_point = input_details["quantization"]
    output_scale, output_zero_point = output_details["quantization"]
    metadata = {
        "model_name": model_name,
        "target_user": int(target_user),
        "held_out_protocol": "train users != target user",
        "window_size": WINDOW_SIZE,
        "step_size": STEP_SIZE,
        "feature_columns": FEATURE_COLUMNS,
        "activity_order": ACTIVITY_ORDER,
        "input_shape": [int(x) for x in input_details["shape"]],
        "output_shape": [int(x) for x in output_details["shape"]],
        "input_dtype": str(input_details["dtype"]),
        "output_dtype": str(output_details["dtype"]),
        "input_scale": float(input_scale),
        "input_zero_point": int(input_zero_point),
        "output_scale": float(output_scale),
        "output_zero_point": int(output_zero_point),
        "zscore_means": {str(k): float(v) for k, v in train_means.to_dict().items()},
        "zscore_stds": {str(k): float(v) for k, v in train_stds.to_dict().items()},
        "representative_samples": int(representative_count),
        "microflow_feature_dim": MICROFLOW_FEATURE_DIM,
        "train_distribution": train_distribution,
        "target_distribution": target_distribution,
        "creation_timestamp": datetime.now(timezone.utc).isoformat(),
        **extra,
    }
    path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return metadata


def evaluate_keras(model: keras.Model, x_test: np.ndarray, y_test: np.ndarray) -> tuple[float, np.ndarray, np.ndarray]:
    probs = model.predict(x_test, verbose=0)
    preds = np.argmax(probs.reshape((len(x_test), NUM_CLASSES)), axis=1).astype(np.uint8)
    acc = float(np.mean(preds == y_test))
    return acc, preds, confusion_matrix(y_test, preds)


def quantize_windows(windows_4d: np.ndarray, input_scale: float, input_zero_point: int) -> np.ndarray:
    windows = windows_4d.reshape((len(windows_4d), WINDOW_SIZE, len(FEATURE_COLUMNS)))
    quantized = np.rint(windows / input_scale).astype(np.int32) + int(input_zero_point)
    quantized = np.clip(quantized, -128, 127).astype(np.int8)
    return quantized.reshape((len(windows), FLAT_LEN))


def evaluate_tflite_classifier(
    tflite_model: bytes,
    quantized_windows: np.ndarray,
    y_test: np.ndarray,
) -> tuple[float, np.ndarray, np.ndarray]:
    interpreter = tf.lite.Interpreter(model_content=tflite_model)
    interpreter.allocate_tensors()
    input_details = interpreter.get_input_details()[0]
    output_details = interpreter.get_output_details()[0]

    preds = np.empty((len(y_test),), dtype=np.uint8)
    for idx, flat in enumerate(quantized_windows):
        input_tensor = flat.reshape(input_details["shape"]).astype(np.int8)
        interpreter.set_tensor(input_details["index"], input_tensor)
        interpreter.invoke()
        output = interpreter.get_tensor(output_details["index"])
        preds[idx] = int(np.argmax(output.reshape(-1)))

    acc = float(np.mean(preds == y_test))
    return acc, preds, confusion_matrix(y_test, preds)


def write_matrix_csv(path: Path, matrix: np.ndarray) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["true", *[f"pred_{name}" for name in ACTIVITY_ORDER]])
        for class_id, row in enumerate(matrix):
            writer.writerow([ACTIVITY_ORDER[class_id], *row.astype(int).tolist()])


def write_per_class_csv(path: Path, matrix: np.ndarray) -> None:
    recalls = per_class_recall(matrix)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["class", "support", "correct", "recall"])
        writer.writeheader()
        for class_id, name in enumerate(ACTIVITY_ORDER):
            writer.writerow(
                {
                    "class": name,
                    "support": int(matrix[class_id].sum()),
                    "correct": int(matrix[class_id, class_id]),
                    "recall": recalls[name],
                }
            )


def write_predictions_csv(path: Path, y_true: np.ndarray, keras_preds: np.ndarray, tflite_preds: np.ndarray) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["idx", "true", "keras_pred", "tflite_pred"])
        writer.writeheader()
        for idx, (truth, keras_pred, tflite_pred) in enumerate(zip(y_true, keras_preds, tflite_preds, strict=True)):
            writer.writerow(
                {
                    "idx": idx,
                    "true": ACTIVITY_ORDER[int(truth)],
                    "keras_pred": ACTIVITY_ORDER[int(keras_pred)],
                    "tflite_pred": ACTIVITY_ORDER[int(tflite_pred)],
                }
            )


def write_head_json(path: Path, model: keras.Model) -> None:
    kernel, bias = model.get_layer("classifier_head").get_weights()
    weights = kernel.reshape((MICROFLOW_FEATURE_DIM, NUM_CLASSES))
    payload = {
        "layout": "[feature][class]",
        "feature_dim": MICROFLOW_FEATURE_DIM,
        "num_classes": NUM_CLASSES,
        "class_names": ACTIVITY_ORDER,
        "weights": weights.astype(float).tolist(),
        "bias": bias.astype(float).tolist(),
    }
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def build_fold_data(target_user: int):
    raw = load_raw_wisdm(DATASET_PATH)
    df_model = add_contiguous_segments(prepare_dataframe(raw))
    train_df = df_model[df_model["user_id"] != target_user].copy()
    target_df = df_model[df_model["user_id"] == target_user].copy()
    if target_df.empty:
        raise RuntimeError(f"target_user={target_user} has no rows")

    train_means, train_stds = fit_zscore_stats(train_df)
    train_scaled = apply_zscore_stats(train_df, train_means, train_stds)
    target_scaled = apply_zscore_stats(target_df, train_means, train_stds)

    x_train, y_train, train_subjects = create_windows_from_segments(train_scaled)
    x_target, y_target, target_subjects = create_windows_from_segments(target_scaled)
    return x_train, y_train, train_subjects, x_target, y_target, target_subjects, train_means, train_stds


def main() -> int:
    parser = argparse.ArgumentParser(description="Train fold-specific MicroFlow-32 for one WISDM target user.")
    parser.add_argument("--target-user", type=int, default=7)
    parser.add_argument("--epochs", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=64)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    args = parser.parse_args()

    set_reproducibility(args.seed)
    out_dir = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    SUMMARY_PATH.parent.mkdir(parents=True, exist_ok=True)

    x_train, y_train, _train_subjects, x_target, y_target, target_subjects, train_means, train_stds = build_fold_data(
        args.target_user
    )
    x_train_4d = x_train[..., np.newaxis].astype(np.float32)
    x_target_4d = x_target[..., np.newaxis].astype(np.float32)
    y_train_onehot = to_categorical(y_train, num_classes=NUM_CLASSES)
    y_train_onehot_4d = y_train_onehot[:, np.newaxis, np.newaxis, :].astype(np.float32)

    model, feature_extractor = build_microflow_fullconv_models(x_train_4d.shape[1:])
    history = model.fit(
        x_train_4d,
        y_train_onehot_4d,
        validation_split=0.1,
        epochs=args.epochs,
        batch_size=args.batch_size,
        verbose=1,
    )

    keras_acc, keras_preds, keras_matrix = evaluate_keras(model, x_target_4d, y_target)
    representative = representative_subset(x_train_4d, y_train, args.seed)

    classifier_tflite = convert_to_int8_tflite(model, representative)
    feature_tflite = convert_to_int8_tflite(feature_extractor, representative)

    classifier_path = out_dir / f"microflow32_user{args.target_user}_classifier_int8.tflite"
    feature_path = out_dir / f"microflow32_user{args.target_user}_feature_extractor_int8.tflite"
    classifier_meta_path = out_dir / f"microflow32_user{args.target_user}_classifier_int8_metadata.json"
    feature_meta_path = out_dir / f"microflow32_user{args.target_user}_feature_extractor_int8_metadata.json"
    classifier_path.write_bytes(classifier_tflite)
    feature_path.write_bytes(feature_tflite)

    train_distribution = class_distribution(y_train)
    target_distribution = class_distribution(y_target)
    classifier_metadata = write_metadata(
        classifier_meta_path,
        f"microflow32_user{args.target_user}_classifier_int8",
        classifier_tflite,
        args.target_user,
        train_means,
        train_stds,
        len(representative),
        train_distribution,
        target_distribution,
        {"artifact_role": "fold_specific_classifier"},
    )
    write_metadata(
        feature_meta_path,
        f"microflow32_user{args.target_user}_feature_extractor_int8",
        feature_tflite,
        args.target_user,
        train_means,
        train_stds,
        len(representative),
        train_distribution,
        target_distribution,
        {"artifact_role": "fold_specific_feature_extractor"},
    )

    quantized_target = quantize_windows(
        x_target_4d,
        float(classifier_metadata["input_scale"]),
        int(classifier_metadata["input_zero_point"]),
    )
    target_windows_path = out_dir / f"wisdm_user{args.target_user}_target_windows_i8.bin"
    target_labels_path = out_dir / f"wisdm_user{args.target_user}_target_labels_u8.bin"
    target_metadata_path = out_dir / f"wisdm_user{args.target_user}_target_metadata.json"
    quantized_target.astype(np.int8).tofile(target_windows_path)
    y_target.astype(np.uint8).tofile(target_labels_path)
    target_metadata_path.write_text(
        json.dumps(
            {
                "target_user": int(args.target_user),
                "num_windows": int(len(y_target)),
                "target_subjects_unique": sorted(np.unique(target_subjects).astype(int).tolist()),
                "window_size": WINDOW_SIZE,
                "axes": len(FEATURE_COLUMNS),
                "flat_len": FLAT_LEN,
                "class_distribution": target_distribution,
                "input_scale": float(classifier_metadata["input_scale"]),
                "input_zero_point": int(classifier_metadata["input_zero_point"]),
                "source": "held-out target user windows normalized with train-user z-score stats",
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    tflite_acc, tflite_preds, tflite_matrix = evaluate_tflite_classifier(classifier_tflite, quantized_target, y_target)
    write_matrix_csv(out_dir / f"wisdm_user{args.target_user}_target_tflite_confusion.csv", tflite_matrix)
    write_per_class_csv(out_dir / f"wisdm_user{args.target_user}_target_tflite_per_class.csv", tflite_matrix)
    write_predictions_csv(out_dir / f"wisdm_user{args.target_user}_target_predictions.csv", y_target, keras_preds, tflite_preds)
    write_head_json(out_dir / f"microflow32_user{args.target_user}_head_float.json", model)

    final_train_acc = float(history.history["accuracy"][-1])
    final_val_acc = float(history.history["val_accuracy"][-1])
    summary = {
        "target_user": int(args.target_user),
        "train_windows": int(len(y_train)),
        "target_windows": int(len(y_target)),
        "representative_samples": int(len(representative)),
        "epochs": int(args.epochs),
        "batch_size": int(args.batch_size),
        "final_train_accuracy": final_train_acc,
        "final_val_accuracy": final_val_acc,
        "target_keras_accuracy": keras_acc,
        "target_tflite_accuracy": tflite_acc,
        "classifier_tflite_bytes": classifier_path.stat().st_size,
        "feature_tflite_bytes": feature_path.stat().st_size,
        "target_windows_bytes": target_windows_path.stat().st_size,
        "target_labels_bytes": target_labels_path.stat().st_size,
    }

    with SUMMARY_PATH.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(summary.keys()))
        writer.writeheader()
        writer.writerow(summary)

    print("FOLD_EXPORT_SUMMARY")
    for key, value in summary.items():
        print(f"{key}={value}")
    print(f"out_dir={out_dir}")
    print(f"train_distribution={train_distribution}")
    print(f"target_distribution={target_distribution}")
    print(f"target_tflite_per_class_recall={per_class_recall(tflite_matrix)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
