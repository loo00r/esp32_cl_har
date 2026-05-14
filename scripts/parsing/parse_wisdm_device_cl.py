#!/usr/bin/env python3
"""Parse isolated ESP32 WISDM target-user CL logs."""

from __future__ import annotations

import argparse
import csv
import re
from pathlib import Path


ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
KV_RE = re.compile(r"([A-Za-z0-9_]+)=([^ ]+)")


def clean_line(line: str) -> str:
    return ANSI_RE.sub("", line).strip()


def parse_kv(line: str) -> dict[str, str]:
    return {key: value for key, value in KV_RE.findall(line)}


def write_csv(path: Path, rows: list[dict[str, str]]) -> None:
    if not rows:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    keys: list[str] = []
    for row in rows:
        for key in row:
            if key not in keys:
                keys.append(key)
    with path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=keys)
        writer.writeheader()
        writer.writerows(rows)


def parse_log(path: Path) -> tuple[list[dict[str, str]], list[dict[str, str]], list[dict[str, str]]]:
    eval_rows: list[dict[str, str]] = []
    class_rows: list[dict[str, str]] = []
    train_rows: list[dict[str, str]] = []
    summary: dict[str, str] = {"source_log": str(path)}

    for raw_line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = clean_line(raw_line)
        if "WISDM_CL_START" in line:
            summary.update(parse_kv(line))
        elif "WISDM_CL_SPLIT" in line:
            summary.update({f"split_{k}": v for k, v in parse_kv(line).items()})
        elif "WISDM_CL_EVAL" in line:
            row = {"source_log": str(path), **parse_kv(line)}
            eval_rows.append(row)
        elif "WISDM_CL_CLASS" in line:
            row = {"source_log": str(path), **parse_kv(line)}
            class_rows.append(row)
        elif "WISDM_CL_TRAIN" in line:
            row = {"source_log": str(path), **parse_kv(line)}
            train_rows.append(row)
        elif "WISDM_CL_SUMMARY" in line:
            summary.update(parse_kv(line))

    if eval_rows:
        pre = next((row for row in eval_rows if row.get("phase") == "pre" and row.get("split") == "held_out"), None)
        post = next((row for row in eval_rows if row.get("phase") == "post" and row.get("split") == "held_out"), None)
        if pre and post:
            pre_acc = float(pre["accuracy"])
            post_acc = float(post["accuracy"])
            summary["pre_held_out_accuracy"] = f"{pre_acc:.8f}"
            summary["post_held_out_accuracy"] = f"{post_acc:.8f}"
            summary["accuracy_delta_pp"] = f"{(post_acc - pre_acc) * 100.0:.4f}"
            summary["pre_held_out_total"] = pre["total"]
            summary["post_held_out_total"] = post["total"]
            summary["mean_infer_us"] = post["mean_infer_us"]

    downstream_pre = next(
        (
            row
            for row in class_rows
            if row.get("phase") == "pre" and row.get("split") == "held_out" and row.get("name") == "Downstairs"
        ),
        None,
    )
    downstream_post = next(
        (
            row
            for row in class_rows
            if row.get("phase") == "post" and row.get("split") == "held_out" and row.get("name") == "Downstairs"
        ),
        None,
    )
    if downstream_pre and downstream_post:
        pre_recall = float(downstream_pre["recall"])
        post_recall = float(downstream_post["recall"])
        summary["downstairs_pre_recall"] = f"{pre_recall:.8f}"
        summary["downstairs_post_recall"] = f"{post_recall:.8f}"
        summary["downstairs_delta_pp"] = f"{(post_recall - pre_recall) * 100.0:.4f}"

    return [summary], class_rows, train_rows


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("logs", nargs="+", type=Path)
    parser.add_argument("--out-dir", type=Path, default=Path("results/tables"))
    args = parser.parse_args()

    summaries: list[dict[str, str]] = []
    classes: list[dict[str, str]] = []
    trains: list[dict[str, str]] = []
    for log_path in args.logs:
        summary_rows, class_rows, train_rows = parse_log(log_path)
        summaries.extend(summary_rows)
        classes.extend(class_rows)
        trains.extend(train_rows)

    write_csv(args.out_dir / "wisdm_user19_device_cl_summary.csv", summaries)
    write_csv(args.out_dir / "wisdm_user19_device_cl_per_class.csv", classes)
    write_csv(args.out_dir / "wisdm_user19_device_cl_train.csv", trains)
    print(f"parsed_logs={len(args.logs)} out_dir={args.out_dir}")
    for row in summaries:
        print(
            "policy={policy} pre={pre_held_out_accuracy} post={post_held_out_accuracy} "
            "delta_pp={accuracy_delta_pp} downstairs_delta_pp={downstairs_delta_pp}".format(**row)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
