#!/usr/bin/env python3
"""Summarize PC-side WISDM target-user CL simulations."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path


CLASS_ORDER = ["Walking", "Jogging", "Upstairs", "Downstairs", "Sitting", "Standing"]


def safe_float(value: str) -> float:
    if value == "":
        return 0.0
    return float(value)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="") as f:
        return list(csv.DictReader(f))


def group_rows(rows: list[dict[str, str]]) -> dict[tuple[str, int, float], list[dict[str, str]]]:
    groups: dict[tuple[str, int, float], list[dict[str, str]]] = {}
    for row in rows:
        key = (row["mode"], int(row["budget_per_class"]), safe_float(row["lr"]))
        groups.setdefault(key, []).append(row)
    return groups


def macro_recall(rows: list[dict[str, str]]) -> float:
    recalls = [
        safe_float(row["recall"])
        for row in rows
        if int(row["support"]) > 0
    ]
    return sum(recalls) / len(recalls) if recalls else 0.0


def class_recalls(rows: list[dict[str, str]]) -> dict[str, float]:
    return {row["class"]: safe_float(row["recall"]) for row in rows}


def row_accuracy(rows: list[dict[str, str]]) -> float:
    return safe_float(rows[0]["accuracy"]) if rows else 0.0


def row_total(rows: list[dict[str, str]]) -> int:
    return int(rows[0]["total"]) if rows else 0


def row_updates(rows: list[dict[str, str]]) -> int:
    return int(rows[0]["train_updates"]) if rows else 0


def row_labels(rows: list[dict[str, str]]) -> int:
    return int(rows[0]["adapted_labels"]) if rows else 0


def summarize_one(path: Path, target_user: int) -> list[dict[str, object]]:
    rows = read_rows(path)
    groups = group_rows(rows)
    out: list[dict[str, object]] = []

    for (mode, budget, lr), policy_rows in sorted(groups.items()):
        if mode not in {"fifo", "reservoir"}:
            continue
        baseline_rows = groups.get(("no_adapt_eval_split", budget, 0.0))
        if not baseline_rows:
            continue

        policy_recalls = class_recalls(policy_rows)
        baseline_recalls = class_recalls(baseline_rows)
        class_gains = {
            cls: policy_recalls.get(cls, 0.0) - baseline_recalls.get(cls, 0.0)
            for cls in CLASS_ORDER
        }
        best_class = max(class_gains, key=class_gains.get)

        out.append(
            {
                "target_user": target_user,
                "source_csv": str(path),
                "mode": mode,
                "budget_per_class": budget,
                "lr": lr,
                "eval_total": row_total(policy_rows),
                "adapted_labels": row_labels(policy_rows),
                "train_updates": row_updates(policy_rows),
                "baseline_accuracy": row_accuracy(baseline_rows),
                "adapted_accuracy": row_accuracy(policy_rows),
                "accuracy_delta_pp": (row_accuracy(policy_rows) - row_accuracy(baseline_rows)) * 100.0,
                "baseline_macro_recall": macro_recall(baseline_rows),
                "adapted_macro_recall": macro_recall(policy_rows),
                "macro_recall_delta_pp": (macro_recall(policy_rows) - macro_recall(baseline_rows)) * 100.0,
                "best_class_gain": best_class,
                "best_class_gain_pp": class_gains[best_class] * 100.0,
                **{
                    f"{cls.lower()}_recall_delta_pp": class_gains.get(cls, 0.0) * 100.0
                    for cls in CLASS_ORDER
                },
            }
        )
    return out


def parse_user_from_name(path: Path) -> int:
    stem = path.stem
    prefix = "wisdm_user"
    if not stem.startswith(prefix):
        raise ValueError(f"Cannot infer target user from {path}")
    return int(stem[len(prefix) :].split("_", 1)[0])


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("csv", nargs="+", type=Path)
    parser.add_argument("--out-csv", type=Path, default=Path("results/tables/wisdm_target_user_cl_screening_summary.csv"))
    args = parser.parse_args()

    rows: list[dict[str, object]] = []
    for path in args.csv:
        rows.extend(summarize_one(path, parse_user_from_name(path)))

    args.out_csv.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = [
        "target_user",
        "source_csv",
        "mode",
        "budget_per_class",
        "lr",
        "eval_total",
        "adapted_labels",
        "train_updates",
        "baseline_accuracy",
        "adapted_accuracy",
        "accuracy_delta_pp",
        "baseline_macro_recall",
        "adapted_macro_recall",
        "macro_recall_delta_pp",
        "best_class_gain",
        "best_class_gain_pp",
        "walking_recall_delta_pp",
        "jogging_recall_delta_pp",
        "upstairs_recall_delta_pp",
        "downstairs_recall_delta_pp",
        "sitting_recall_delta_pp",
        "standing_recall_delta_pp",
    ]
    with args.out_csv.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    ranked = sorted(rows, key=lambda row: (float(row["accuracy_delta_pp"]), float(row["best_class_gain_pp"])), reverse=True)
    print(f"wrote {args.out_csv}")
    for row in ranked[:10]:
        print(
            "user={target_user} mode={mode} budget={budget_per_class} lr={lr} "
            "acc_delta_pp={accuracy_delta_pp:.2f} macro_delta_pp={macro_recall_delta_pp:.2f} "
            "best_class={best_class_gain} best_class_delta_pp={best_class_gain_pp:.2f}".format(**row)
        )


if __name__ == "__main__":
    main()
