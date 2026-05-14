#!/usr/bin/env python3
"""Parse WISDM device-side eval logs into paper-ready CSV tables."""

from __future__ import annotations

import argparse
import csv
import re
from pathlib import Path
from statistics import mean


OUT_DIR = Path("results/tables")
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
TOKEN_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)=([^=\s]+)")


def strip_ansi(line: str) -> str:
    return ANSI_RE.sub("", line).strip()


def parse_value(raw: str) -> object:
    try:
        return int(raw)
    except ValueError:
        pass
    try:
        return float(raw)
    except ValueError:
        return raw


def parse_tokens(line: str) -> dict[str, object]:
    return {key: parse_value(value) for key, value in TOKEN_RE.findall(line)}


def write_csv(path: Path, rows: list[dict[str, object]], fieldnames: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def parse_log(path: Path) -> tuple[dict[str, object] | None, list[dict[str, object]], list[dict[str, object]]]:
    summary: dict[str, object] | None = None
    per_class: list[dict[str, object]] = []
    confusion: list[dict[str, object]] = []

    for raw_line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = strip_ansi(raw_line)
        if "WISDM_EVAL_SUMMARY" in line:
            summary = parse_tokens(line)
            summary["notes"] = path.name
        elif "WISDM_EVAL_CLASS" in line:
            per_class.append(parse_tokens(line))
        elif "WISDM_EVAL_CONF" in line:
            confusion.append(parse_tokens(line))

    if summary and per_class:
        recalls = [float(row["recall"]) for row in per_class if "recall" in row]
        if recalls:
            summary["macro_recall"] = mean(recalls)
    return summary, per_class, confusion


def tag_from_summary(summary: dict[str, object], fallback: Path) -> str:
    tag = summary.get("tag")
    if isinstance(tag, str):
        return tag
    return fallback.stem


def main() -> int:
    parser = argparse.ArgumentParser(description="Parse WISDM device eval logs.")
    parser.add_argument("inputs", nargs="+", type=Path, help="Raw serial log files")
    parser.add_argument("--out-dir", type=Path, default=OUT_DIR, help="CSV output directory")
    args = parser.parse_args()

    summaries: list[dict[str, object]] = []
    for input_path in args.inputs:
        summary, per_class, confusion = parse_log(input_path)
        if summary is None:
            print(f"WARN no WISDM_EVAL_SUMMARY found in {input_path}")
            continue

        tag = tag_from_summary(summary, input_path)
        summaries.append(summary)

        write_csv(
            args.out_dir / f"wisdm_device_eval_per_class_{tag}.csv",
            per_class,
            ["class", "name", "support", "correct", "recall"],
        )
        write_csv(
            args.out_dir / f"wisdm_device_eval_confusion_{tag}.csv",
            confusion,
            ["true", "pred0", "pred1", "pred2", "pred3", "pred4", "pred5"],
        )

    if summaries:
        write_csv(
            args.out_dir / "wisdm_device_eval_summary.csv",
            summaries,
            [
                "tag",
                "total",
                "correct",
                "accuracy",
                "mean_infer_us",
                "min_infer_us",
                "max_infer_us",
                "macro_recall",
                "notes",
            ],
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
