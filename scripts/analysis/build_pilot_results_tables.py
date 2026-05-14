#!/usr/bin/env python3
"""Build paper-ready Markdown tables for the Phase 5 pilot results."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
from typing import Iterable


RESOURCE_COLUMNS = [
    "Mode",
    "PRED rows",
    "Labels",
    "Train updates",
    "Replay RAM",
    "Inference mean",
    "Head mean",
    "Update mean",
    "Update / inference",
]

SEGMENT_COLUMNS = [
    "Mode",
    "Segment",
    "Attempts",
    "Accepted labels",
    "Rows",
    "Accepted rate",
    "Pred counts",
]


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def fmt_float(raw: str, digits: int = 2) -> str:
    if raw == "":
        return "-"
    return f"{float(raw):.{digits}f}"


def fmt_ms_from_us(raw: str) -> str:
    if raw == "":
        return "-"
    return f"{float(raw) / 1000.0:.2f} ms"


def fmt_us(raw: str) -> str:
    if raw == "":
        return "-"
    return f"{float(raw):.1f} us"


def fmt_kib(raw: str) -> str:
    if raw == "":
        return "-"
    value = float(raw)
    if value == 0:
        return "0"
    return f"{value / 1024.0:.1f} KiB"


def fmt_rate(raw: str) -> str:
    if raw == "":
        return "-"
    return f"{float(raw) * 100.0:.1f}%"


def fmt_pct(raw: str) -> str:
    if raw == "":
        return "-"
    return f"{float(raw):.3f}%"


def markdown_table(headers: list[str], rows: Iterable[list[str]]) -> str:
    def cell(value: str) -> str:
        return value.replace("|", "\\|")

    lines = [
        "| " + " | ".join(cell(header) for header in headers) + " |",
        "| " + " | ".join("---" for _ in headers) + " |",
    ]
    for row in rows:
        lines.append("| " + " | ".join(cell(value) for value in row) + " |")
    return "\n".join(lines)


def resource_rows(rows: list[dict[str, str]]) -> list[list[str]]:
    table_rows: list[list[str]] = []
    for row in rows:
        table_rows.append(
            [
                row["mode"],
                row["pred_rows"],
                row["label_rows"],
                row["train_rows"],
                fmt_kib(row["replay_ram_est"]),
                fmt_ms_from_us(row["pred_infer_us_mean"]),
                fmt_us(row["pred_head_us_mean"]),
                fmt_us(row["train_update_us_mean"]),
                fmt_pct(row["cl_update_vs_infer_pct"]),
            ],
        )
    return table_rows


def segment_rows(rows: list[dict[str, str]]) -> list[list[str]]:
    table_rows: list[list[str]] = []
    for row in rows:
        attempts = f"{row['attempt_start']}-{row['attempt_end']}"
        table_rows.append(
            [
                row["mode"],
                row["segment"],
                attempts,
                row["accepted_labels"],
                row["rows"],
                fmt_rate(row["accepted_rate"]),
                row["pred_counts"],
            ],
        )
    return table_rows


def build_markdown(comparison_path: Path, segment_path: Path) -> str:
    comparison = read_csv(comparison_path)
    segments = read_csv(segment_path)

    return "\n\n".join(
        [
            "# Phase 5 Pilot Results Tables",
            (
                "Scope: real-device pilot sanity check on ESP32 + MPU6050. "
                "The second segment was standing-like small movement, not full Walking."
            ),
            "## Resource And CL Overhead",
            markdown_table(RESOURCE_COLUMNS, resource_rows(comparison)),
            "## Segment-Level Pilot Summary",
            markdown_table(SEGMENT_COLUMNS, segment_rows(segments)),
            "## Interpretation Notes",
            "\n".join(
                [
                    "- These tables are pilot evidence, not final 6-class HAR accuracy.",
                    "- Persistence is off; CL state is RAM-only.",
                    "- FIFO and reservoir use the same replay budget: 16 slots/class, feature_dim=32.",
                    "- CL update overhead stays below 1% of MicroFlow-32 inference time in this pilot.",
                    "- Reservoir shows the strongest prediction distribution shift on the standing-like segment.",
                ],
            ),
            "",
        ],
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build paper-ready Markdown tables for Phase 5 pilot results.",
    )
    parser.add_argument("--comparison-csv", required=True, type=Path)
    parser.add_argument("--segment-csv", required=True, type=Path)
    parser.add_argument("--out-md", required=True, type=Path)
    args = parser.parse_args()

    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.write_text(
        build_markdown(args.comparison_csv, args.segment_csv),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
