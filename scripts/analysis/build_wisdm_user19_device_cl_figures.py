#!/usr/bin/env python3
"""Build paper-ready tables and figures for the user=19 device-side CL run."""

from __future__ import annotations

from pathlib import Path

import pandas as pd
import matplotlib.pyplot as plt


ROOT = Path(__file__).resolve().parents[2]
TABLE_DIR = ROOT / "results" / "tables"
FIG_DIR = ROOT / "results" / "figures"

SUMMARY_CSV = TABLE_DIR / "wisdm_user19_device_cl_summary.csv"
CLASS_CSV = TABLE_DIR / "wisdm_user19_device_cl_per_class.csv"
TRAIN_CSV = TABLE_DIR / "wisdm_user19_device_cl_train.csv"


def save_current_figure(name: str) -> None:
    FIG_DIR.mkdir(parents=True, exist_ok=True)
    plt.tight_layout()
    plt.savefig(FIG_DIR / f"{name}.png", dpi=200)
    plt.savefig(FIG_DIR / f"{name}.pdf")


def mode_from_log(path: str) -> str:
    if "fifo" in path:
        return "fifo"
    if "reservoir" in path:
        return "reservoir"
    return "unknown"


def main() -> int:
    TABLE_DIR.mkdir(parents=True, exist_ok=True)
    FIG_DIR.mkdir(parents=True, exist_ok=True)

    summary = pd.read_csv(SUMMARY_CSV)
    per_class = pd.read_csv(CLASS_CSV)
    train = pd.read_csv(TRAIN_CSV)

    summary["policy"] = summary["policy"].astype(str)
    summary["pre_accuracy_pct"] = summary["pre_held_out_accuracy"] * 100.0
    summary["post_accuracy_pct"] = summary["post_held_out_accuracy"] * 100.0
    summary["downstairs_pre_recall_pct"] = summary["downstairs_pre_recall"] * 100.0
    summary["downstairs_post_recall_pct"] = summary["downstairs_post_recall"] * 100.0
    summary["inference_ms"] = summary["mean_infer_us"] / 1000.0

    train_summary = (
        train.groupby("policy", as_index=False)
        .agg(
            train_updates=("step", "count"),
            update_us_mean=("update_us", "mean"),
            update_us_min=("update_us", "min"),
            update_us_max=("update_us", "max"),
            sample_us_mean=("sample_us", "mean"),
        )
    )
    summary_table = summary.merge(train_summary, on="policy", how="left")
    summary_table["cl_update_vs_infer_pct"] = (
        summary_table["update_us_mean"] / summary_table["mean_infer_us"] * 100.0
    )
    summary_table = summary_table[
        [
            "policy",
            "target_user",
            "split_selected_adaptation",
            "split_held_out",
            "budget_per_class",
            "lr",
            "train_updates",
            "pre_accuracy_pct",
            "post_accuracy_pct",
            "accuracy_delta_pp",
            "downstairs_pre_recall_pct",
            "downstairs_post_recall_pct",
            "downstairs_delta_pp",
            "inference_ms",
            "update_us_mean",
            "cl_update_vs_infer_pct",
        ]
    ]
    summary_table.to_csv(TABLE_DIR / "table_wisdm_user19_device_cl_summary.csv", index=False)
    summary_table.to_markdown(TABLE_DIR / "table_wisdm_user19_device_cl_summary.md", index=False)

    per_class = per_class.copy()
    per_class["policy"] = per_class["source_log"].map(mode_from_log)
    heldout = per_class[per_class["split"].eq("held_out")].copy()
    heldout["recall_pct"] = heldout["recall"] * 100.0
    heldout_table = heldout[
        ["policy", "phase", "class", "name", "support", "correct", "recall_pct"]
    ].sort_values(["policy", "class", "phase"])
    heldout_table.to_csv(TABLE_DIR / "table_wisdm_user19_device_cl_per_class_heldout.csv", index=False)
    heldout_table.to_markdown(TABLE_DIR / "table_wisdm_user19_device_cl_per_class_heldout.md", index=False)

    # Plot 1: held-out accuracy before/after adaptation.
    fig, ax = plt.subplots(figsize=(6.5, 4))
    x = range(len(summary))
    width = 0.36
    ax.bar([i - width / 2 for i in x], summary["pre_accuracy_pct"], width=width, label="Pre-adaptation")
    ax.bar([i + width / 2 for i in x], summary["post_accuracy_pct"], width=width, label="Post-adaptation")
    ax.set_xticks(list(x))
    ax.set_xticklabels(summary["policy"])
    ax.set_ylim(0, 100)
    ax.set_ylabel("Held-out accuracy, %")
    ax.set_title("Device-side target-user CL on WISDM user 19")
    ax.legend()
    for i, row in summary.iterrows():
        ax.text(i + width / 2, row["post_accuracy_pct"] + 2, f"+{row['accuracy_delta_pp']:.2f} pp", ha="center", fontsize=9)
    save_current_figure("fig_wisdm_user19_accuracy_pre_post")
    plt.close(fig)

    # Plot 2: Downstairs recall, the weak class recovered by adaptation.
    fig, ax = plt.subplots(figsize=(6.5, 4))
    ax.bar([i - width / 2 for i in x], summary["downstairs_pre_recall_pct"], width=width, label="Pre-adaptation")
    ax.bar([i + width / 2 for i in x], summary["downstairs_post_recall_pct"], width=width, label="Post-adaptation")
    ax.set_xticks(list(x))
    ax.set_xticklabels(summary["policy"])
    ax.set_ylim(0, 100)
    ax.set_ylabel("Downstairs recall, %")
    ax.set_title("Weak-class recall recovery on held-out user 19")
    ax.legend()
    save_current_figure("fig_wisdm_user19_downstairs_recall_pre_post")
    plt.close(fig)

    # Plot 3: per-class held-out recall for one policy. FIFO and reservoir match in this gate.
    reservoir_heldout = heldout[heldout["policy"].eq("reservoir")].copy()
    phases = ["pre", "post"]
    class_names = list(reservoir_heldout[reservoir_heldout["phase"].eq("pre")]["name"])
    fig, ax = plt.subplots(figsize=(8, 4.5))
    indices = list(range(len(class_names)))
    width = 0.36
    for offset, phase in [(-width / 2, "pre"), (width / 2, "post")]:
        phase_rows = reservoir_heldout[reservoir_heldout["phase"].eq(phase)].sort_values("class")
        label = "Pre-adaptation" if phase == "pre" else "Post-adaptation"
        ax.bar([i + offset for i in indices], phase_rows["recall_pct"], width=width, label=label)
    ax.set_xticks(indices)
    ax.set_xticklabels(class_names, rotation=25, ha="right")
    ax.set_ylim(0, 100)
    ax.set_ylabel("Recall, %")
    ax.set_title("Per-class held-out recall on WISDM user 19")
    ax.legend()
    save_current_figure("fig_wisdm_user19_per_class_recall")
    plt.close(fig)

    # Plot 4: OnlineLayer update cost.
    fig, ax = plt.subplots(figsize=(6, 4))
    ax.bar(train_summary["policy"], train_summary["update_us_mean"])
    ax.set_ylabel("OnlineLayer update time, us")
    ax.set_title("Device-side RAM-only CL update cost")
    for i, row in train_summary.iterrows():
        ax.text(i, row["update_us_mean"] + 12, f"{row['update_us_mean']:.1f} us", ha="center", fontsize=9)
    save_current_figure("fig_wisdm_user19_update_cost")
    plt.close(fig)

    print(f"wrote {TABLE_DIR / 'table_wisdm_user19_device_cl_summary.csv'}")
    print(f"wrote {TABLE_DIR / 'table_wisdm_user19_device_cl_per_class_heldout.csv'}")
    print("wrote user19 WISDM CL figures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
