#!/usr/bin/env python3
"""Evaluate parsed PRED rows over manually annotated attempt segments."""

from __future__ import annotations

import argparse
import csv
from collections import Counter
from pathlib import Path
from statistics import mean
from typing import Any


FIELDS = [
    "run",
    "mode",
    "segment",
    "attempt_start",
    "attempt_end",
    "accepted_labels",
    "rows",
    "accepted_rows",
    "accepted_rate",
    "mean_conf",
    "pred_counts",
]


def parse_segment(raw: str) -> dict[str, Any]:
    parts = raw.split(":", maxsplit=3)
    if len(parts) != 4:
        raise argparse.ArgumentTypeError(
            "segment must use name:start:end:accepted_label|accepted_label",
        )

    name, start_raw, end_raw, accepted_raw = parts
    try:
        start = int(start_raw)
        end = int(end_raw)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("segment start/end must be integers") from exc

    if start > end:
        raise argparse.ArgumentTypeError("segment start must be <= end")

    accepted = [label for label in accepted_raw.split("|") if label]
    if not name or not accepted:
        raise argparse.ArgumentTypeError("segment name and accepted labels are required")

    return {
        "name": name,
        "start": start,
        "end": end,
        "accepted": accepted,
    }


def load_pred_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def infer_mode(rows: list[dict[str, str]], fallback: str) -> str:
    for row in rows:
        mode = row.get("mode")
        if mode:
            return mode
    return fallback


def row_attempt(row: dict[str, str]) -> int | None:
    try:
        return int(row["attempt"])
    except (KeyError, ValueError):
        return None


def row_conf(row: dict[str, str]) -> float | None:
    try:
        return float(row["conf"])
    except (KeyError, ValueError):
        return None


def summarize_segment(
    run: str,
    mode: str,
    rows: list[dict[str, str]],
    segment: dict[str, Any],
) -> dict[str, Any]:
    selected: list[dict[str, str]] = []
    for row in rows:
        attempt = row_attempt(row)
        if attempt is not None and segment["start"] <= attempt <= segment["end"]:
            selected.append(row)

    counts = Counter(row.get("label", "") for row in selected)
    accepted = set(segment["accepted"])
    accepted_rows = sum(count for label, count in counts.items() if label in accepted)
    confs = [conf for row in selected if (conf := row_conf(row)) is not None]
    rate = accepted_rows / len(selected) if selected else 0.0

    return {
        "run": run,
        "mode": mode,
        "segment": segment["name"],
        "attempt_start": segment["start"],
        "attempt_end": segment["end"],
        "accepted_labels": "|".join(segment["accepted"]),
        "rows": len(selected),
        "accepted_rows": accepted_rows,
        "accepted_rate": round(rate, 4),
        "mean_conf": round(mean(confs), 6) if confs else "",
        "pred_counts": ";".join(f"{label}={count}" for label, count in sorted(counts.items())),
    }


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Evaluate parsed ESP32 CL-HAR PRED rows over attempt segments.",
    )
    parser.add_argument("pred_csv", type=Path, help="Parsed *_pred.csv file")
    parser.add_argument(
        "--segment",
        action="append",
        required=True,
        type=parse_segment,
        help="Manual segment as name:start:end:accepted_label|accepted_label",
    )
    parser.add_argument("--run", help="Run name override")
    parser.add_argument("--out-csv", type=Path, help="Output segment summary CSV")
    args = parser.parse_args()

    rows = load_pred_rows(args.pred_csv)
    run = args.run or args.pred_csv.stem.removesuffix("_pred")
    mode = infer_mode(rows, fallback=run)
    summaries = [summarize_segment(run, mode, rows, segment) for segment in args.segment]

    if args.out_csv:
        write_csv(args.out_csv, summaries)
    else:
        writer = csv.DictWriter(__import__("sys").stdout, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(summaries)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
