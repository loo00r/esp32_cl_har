#!/usr/bin/env python3
"""Parse ESP32 CL-HAR experiment logs into CSV tables.

Input is plain serial output captured from espflash/monitor. The parser looks
for stable firmware tags:

    EXPERIMENT ...
    RESOURCE ...
    PRED ...
    LABEL ...
    TRAIN ...

It ignores bootloader text, ANSI color codes, and older human-readable logs.
Only Python stdlib is used so the script can run in the project environment
without extra dependencies.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
from collections import Counter
from pathlib import Path
from statistics import mean
from typing import Iterable


TAGS = ("EXPERIMENT", "RESOURCE", "PRED", "LABEL", "TRAIN")
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
TAG_RE = re.compile(r"\b(EXPERIMENT|RESOURCE|PRED|LABEL|TRAIN)\b\s*(.*)$")
TOKEN_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)=([^=\s]+)")


def strip_ansi(line: str) -> str:
    return ANSI_RE.sub("", line).strip()


def parse_value(raw: str) -> object:
    if raw in {"true", "false"}:
        return raw == "true"

    try:
        if raw.startswith("0") and raw != "0" and not raw.startswith("0."):
            return raw
        return int(raw)
    except ValueError:
        pass

    try:
        return float(raw)
    except ValueError:
        return raw


def parse_line(line: str) -> tuple[str, dict[str, object]] | None:
    clean = strip_ansi(line)
    match = TAG_RE.search(clean)
    if not match:
        return None

    tag, rest = match.groups()
    row: dict[str, object] = {"tag": tag}
    tokens = TOKEN_RE.findall(rest)
    if not tokens:
        return None
    for key, value in tokens:
        row[key] = parse_value(value)
    if any(isinstance(value, str) and value.startswith("...") for value in row.values()):
        return None
    return tag, row


def parse_lines(lines: Iterable[str]) -> dict[str, list[dict[str, object]]]:
    tables: dict[str, list[dict[str, object]]] = {tag: [] for tag in TAGS}
    for line in lines:
        parsed = parse_line(line)
        if parsed is None:
            continue
        tag, row = parsed
        tables[tag].append(row)
    return tables


def fieldnames(rows: list[dict[str, object]]) -> list[str]:
    preferred = [
        "tag",
        "mode",
        "policy",
        "attempt",
        "step",
        "class",
        "label",
        "name",
        "conf",
        "infer_us",
        "head_us",
        "batch_len",
        "sample_us",
        "update_us",
        "push_us",
        "buffer_len",
        "class_len",
        "total_seen",
        "replay_ram_est",
        "feature_dim",
        "slots_per_class",
        "batch_size",
        "labels_per_update",
        "persistence",
    ]
    keys = set()
    for row in rows:
        keys.update(row)
    ordered = [key for key in preferred if key in keys]
    ordered.extend(sorted(keys - set(ordered)))
    return ordered


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    names = fieldnames(rows)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=names)
        writer.writeheader()
        writer.writerows(rows)


def numeric_values(rows: list[dict[str, object]], key: str) -> list[float]:
    values: list[float] = []
    for row in rows:
        value = row.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, (int, float)):
            values.append(float(value))
    return values


def summarize(tables: dict[str, list[dict[str, object]]]) -> dict[str, object]:
    mode_counts = Counter()
    for tag in TAGS:
        for row in tables[tag]:
            mode = row.get("mode")
            if isinstance(mode, str):
                mode_counts[mode] += 1

    pred_rows = tables["PRED"]
    train_rows = tables["TRAIN"]
    label_rows = tables["LABEL"]
    resource_rows = tables["RESOURCE"]

    summary: dict[str, object] = {
        "rows": {tag: len(tables[tag]) for tag in TAGS},
        "modes": dict(sorted(mode_counts.items())),
    }

    for summary_key, source_key, rows in [
        ("pred_infer_us", "infer_us", pred_rows),
        ("pred_head_us", "head_us", pred_rows),
        ("train_sample_us", "sample_us", train_rows),
        ("train_update_us", "update_us", train_rows),
        ("label_push_us", "push_us", label_rows),
    ]:
        values = numeric_values(rows, source_key)
        if values:
            summary[summary_key] = {
                "count": len(values),
                "min": min(values),
                "mean": mean(values),
                "max": max(values),
            }

    if resource_rows:
        summary["resources"] = resource_rows

    return summary


def parse_log(input_path: Path, out_dir: Path) -> dict[str, object]:
    tables = parse_lines(input_path.read_text(encoding="utf-8", errors="replace").splitlines())
    out_dir.mkdir(parents=True, exist_ok=True)

    stem = input_path.stem
    for tag in TAGS:
        rows = tables[tag]
        if rows:
            write_csv(out_dir / f"{stem}_{tag.lower()}.csv", rows)

    summary = summarize(tables)
    (out_dir / f"{stem}_summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Parse ESP32 CL-HAR experiment logs into CSV tables.",
    )
    parser.add_argument("input", type=Path, help="Raw serial log file")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("parsed_logs"),
        help="Output directory for CSV/JSON files",
    )
    args = parser.parse_args()

    summary = parse_log(args.input, args.out_dir)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
