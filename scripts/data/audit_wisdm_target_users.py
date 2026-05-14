#!/usr/bin/env python3
"""Audit WISDM target-user candidates for staged CL evaluation.

This is a PC-only planning helper. It does not train a model and does not touch
firmware. The goal is to identify users with enough windows for a held-out
target-user adaptation experiment.
"""

from __future__ import annotations

import csv
from pathlib import Path

import numpy as np

from export_wisdm_device_eval_artifact import ACTIVITY_ORDER, build_final_corpus


PROJECT_ROOT = Path(__file__).resolve().parents[2]
OUT_PATH = PROJECT_ROOT / "results" / "tables" / "wisdm_target_user_audit.csv"


def main() -> int:
    _windows, labels, subjects, _metadata = build_final_corpus()
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)

    rows: list[dict[str, int | float]] = []
    for user_id in sorted(np.unique(subjects).astype(int).tolist()):
        mask = subjects == user_id
        user_labels = labels[mask]
        class_counts = {
            f"{name.lower()}_windows": int(np.count_nonzero(user_labels == class_id))
            for class_id, name in enumerate(ACTIVITY_ORDER)
        }
        nonzero_counts = [count for count in class_counts.values() if count > 0]
        rows.append(
            {
                "user_id": user_id,
                "total_windows": int(mask.sum()),
                "class_coverage": len(nonzero_counts),
                "min_nonzero_class_windows": int(min(nonzero_counts)) if nonzero_counts else 0,
                **class_counts,
            }
        )

    fieldnames = [
        "user_id",
        "total_windows",
        "class_coverage",
        "min_nonzero_class_windows",
        *[f"{name.lower()}_windows" for name in ACTIVITY_ORDER],
    ]
    with OUT_PATH.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    ranked = sorted(
        rows,
        key=lambda row: (
            int(row["class_coverage"]),
            int(row["min_nonzero_class_windows"]),
            int(row["total_windows"]),
        ),
        reverse=True,
    )

    print(f"rows={len(rows)} output={OUT_PATH}")
    print("Top target-user candidates:")
    for row in ranked[:10]:
        counts = " ".join(
            f"{name}={row[f'{name.lower()}_windows']}" for name in ACTIVITY_ORDER
        )
        print(
            f"user={row['user_id']} total={row['total_windows']} "
            f"coverage={row['class_coverage']} min_nonzero={row['min_nonzero_class_windows']} "
            f"{counts}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
