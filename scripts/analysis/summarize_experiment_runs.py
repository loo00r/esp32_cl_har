#!/usr/bin/env python3
"""Build a compact comparison table from parsed ESP32 CL-HAR summaries."""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path
from typing import Any


FIELDS = [
    "run",
    "mode",
    "pred_rows",
    "label_rows",
    "train_rows",
    "replay_ram_est",
    "feature_dim",
    "slots_per_class",
    "batch_size",
    "pred_infer_us_mean",
    "pred_infer_us_min",
    "pred_infer_us_max",
    "pred_head_us_mean",
    "label_push_us_mean",
    "train_sample_us_mean",
    "train_update_us_mean",
    "cl_update_vs_infer_pct",
    "persistence",
]

MODE_ORDER = {
    "no_adapt": 0,
    "fifo": 1,
    "reservoir": 2,
}


def number(value: Any) -> int | float | str:
    if isinstance(value, (int, float)):
        return round(value, 3)
    return ""


def metric(summary: dict[str, Any], key: str, field: str = "mean") -> int | float | str:
    data = summary.get(key)
    if not isinstance(data, dict):
        return ""
    return number(data.get(field))


def first_resource(summary: dict[str, Any]) -> dict[str, Any]:
    resources = summary.get("resources")
    if isinstance(resources, list) and resources and isinstance(resources[0], dict):
        return resources[0]
    return {}


def infer_mode(summary: dict[str, Any], resource: dict[str, Any]) -> str:
    mode = resource.get("mode")
    if isinstance(mode, str):
        return mode

    modes = summary.get("modes")
    if isinstance(modes, dict) and modes:
        return sorted(modes)[0]
    return ""


def row_from_summary(path: Path) -> dict[str, Any]:
    summary = json.loads(path.read_text(encoding="utf-8"))
    resource = first_resource(summary)
    rows = summary.get("rows", {})
    if not isinstance(rows, dict):
        rows = {}

    pred_infer_mean = metric(summary, "pred_infer_us")
    train_update_mean = metric(summary, "train_update_us")
    overhead_pct: int | float | str = ""
    if isinstance(pred_infer_mean, (int, float)) and isinstance(train_update_mean, (int, float)):
        overhead_pct = round((train_update_mean / pred_infer_mean) * 100.0, 3)

    return {
        "run": path.stem.removesuffix("_summary"),
        "mode": infer_mode(summary, resource),
        "pred_rows": rows.get("PRED", 0),
        "label_rows": rows.get("LABEL", 0),
        "train_rows": rows.get("TRAIN", 0),
        "replay_ram_est": resource.get("replay_ram_est", ""),
        "feature_dim": resource.get("feature_dim", ""),
        "slots_per_class": resource.get("slots_per_class", ""),
        "batch_size": resource.get("batch_size", ""),
        "pred_infer_us_mean": pred_infer_mean,
        "pred_infer_us_min": metric(summary, "pred_infer_us", "min"),
        "pred_infer_us_max": metric(summary, "pred_infer_us", "max"),
        "pred_head_us_mean": metric(summary, "pred_head_us"),
        "label_push_us_mean": metric(summary, "label_push_us"),
        "train_sample_us_mean": metric(summary, "train_sample_us"),
        "train_update_us_mean": train_update_mean,
        "cl_update_vs_infer_pct": overhead_pct,
        "persistence": resource.get("persistence", ""),
    }


def write_rows(rows: list[dict[str, Any]], out_path: Path | None) -> None:
    handle = out_path.open("w", newline="", encoding="utf-8") if out_path else sys.stdout
    try:
        writer = csv.DictWriter(handle, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)
    finally:
        if out_path:
            handle.close()


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Summarize parsed ESP32 CL-HAR experiment summary JSON files.",
    )
    parser.add_argument("summaries", nargs="+", type=Path, help="*_summary.json files")
    parser.add_argument("--out-csv", type=Path, help="Optional output CSV path")
    args = parser.parse_args()

    rows = [row_from_summary(path) for path in args.summaries]
    rows.sort(key=lambda row: (MODE_ORDER.get(str(row["mode"]), 99), str(row["mode"])))

    if args.out_csv:
        args.out_csv.parent.mkdir(parents=True, exist_ok=True)
    write_rows(rows, args.out_csv)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
