import argparse
import json

import matplotlib
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np
import pandas as pd


def load_optimality(path):
    rows = json.load(open(path, "r")).get("optimality", [])
    return pd.DataFrame(
        rows,
        columns=[
            "iteration",
            "optimal_precision",
            "rival_precision",
            "baseline_precision",
            "ziv_precision",
        ],
    )


def plot_empty(args):
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.text(0.5, 0.5, "No optimality data", ha="center", va="center")
    ax.set_axis_off()
    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.pdf", format="pdf")
    plt.savefig(args.path + "/optimality_plot.png", format="png")


def plot_optimality(outcomes, args):

    for col in ["iteration", "optimal_precision", "rival_precision", "baseline_precision", "ziv_precision"]:
        outcomes[col] = pd.to_numeric(outcomes[col], errors="coerce")
    outcomes = outcomes.dropna(
        subset=["iteration", "optimal_precision", "rival_precision", "baseline_precision", "ziv_precision"]
    )
    outcomes["iteration"] = outcomes["iteration"].astype(int)

    outcomes = outcomes.loc[outcomes["iteration"] > 0]

    outcomes = outcomes.sort_values(by=["iteration"])
    iterations = outcomes["iteration"].to_list()
    x = np.arange(len(iterations))

    fig, ax = plt.subplots(figsize=(4, 3))
    fig.tight_layout(pad=2.0)

    # ax.bar(x + 0.925, outcomes["baseline_precision"], color="green", alpha=1, width=0.5, label="baseline", hatch="/")
    # ax.bar(x + 1.075, outcomes["rival_precision"], color="red", alpha=0.7, width=0.5, label="reval")
    # ax.bar(x + 1.2, outcomes["ziv_precision"], color="purple", alpha=0.7, width=0.5, label="ziv")
    
    ax.bar(x + 0.825, outcomes["ziv_precision"], color="darkgrey", alpha=1, width=0.4, label='ziv', hatch='\\')
    ax.bar(x + 1.0, outcomes["baseline_precision"], color="green", alpha=1, width=0.4, label='baseline', hatch='/')
    ax.bar(x + 1.175, outcomes["rival_precision"], color="red", alpha=1, width=0.4, label='reval')
    
    ax.plot(x + 1, outcomes["optimal_precision"], ".-", linewidth=2.0, color="orange", label="optimal")

    ax.legend()
    ax.set_xlabel("True uniform precision")
    ax.set_ylabel("Avg. precision")
    ax.set_yscale("symlog", base=2, linthresh=1)
    ax.yaxis.set_major_locator(ticker.LogLocator(base=2.0))
    ax.set_xticks(x + 1)
    ax.set_xticklabels([
        "$2^{" + str(pos + 7) + "}$" if (pos + 7) % 2 == 1 else " "
        for pos in range(len(iterations))
    ])
    ax.yaxis.grid(True, linestyle="-", which="major", color="grey", alpha=0.3)

    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.pdf", format="pdf")

    ax.set_title("Precision compared to optimal")
    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.png", format="png")


parser = argparse.ArgumentParser(prog="optimality_plot.py", description="Script outputs optimality plots")
parser.add_argument("-t", "--timeline", dest="timeline", default="report/timeline.json")
parser.add_argument("-o", "--output-path", dest="path", default="report")
args = parser.parse_args()

matplotlib.rcParams.update({"font.size": 12})
plot_optimality(load_optimality(args.timeline), args)
