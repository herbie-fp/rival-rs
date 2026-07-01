import argparse
import json

import matplotlib
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd


def load_optimality(path):
    rows = json.load(open(path, "r")).get("optimality", [])
    return pd.DataFrame(rows, columns=["tool", "iteration", "diff"])


def plot_empty(args):
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.text(0.5, 0.5, "No optimality data", ha="center", va="center")
    ax.set_axis_off()
    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.pdf", format="pdf")
    plt.savefig(args.path + "/optimality_plot.png", format="png")


def plot_optimality(outcomes, args):
    if outcomes.empty:
        plot_empty(args)
        return

    outcomes["iteration"] = pd.to_numeric(outcomes["iteration"], errors="coerce")
    outcomes["diff"] = pd.to_numeric(outcomes["diff"], errors="coerce")
    outcomes = outcomes.dropna(subset=["tool", "iteration", "diff"])
    outcomes["iteration"] = outcomes["iteration"].astype(int)

    outcomes = outcomes.loc[outcomes["iteration"] > 0]
    if outcomes.empty:
        plot_empty(args)
        return

    averages = outcomes.groupby(["tool", "iteration"], as_index=False, sort=True)["diff"].mean()
    iterations = sorted(averages["iteration"].unique())
    x = np.arange(len(iterations))

    def tool_values(tool):
        by_iter = averages.loc[averages["tool"] == tool].set_index("iteration")["diff"]
        return np.array([by_iter.get(iteration, np.nan) for iteration in iterations], dtype=float)

    fig, ax = plt.subplots(figsize=(4, 3))
    fig.tight_layout(pad=2.0)

    width = 0.5
    ax.bar(x - 0.075, tool_values("baseline"), color="green", alpha=1, width=width, label="baseline", hatch="/")
    ax.bar(x + 0.075, tool_values("rival"), color="red", alpha=0.7, width=width, label="reval")

    ax.legend()
    ax.set_xlabel("True uniform precision")
    ax.set_ylabel("Avg. precision overhead")
    ax.set_xticks(x)
    ax.set_xticklabels(["$2^{" + str(iteration + 6) + "}$" for iteration in iterations])
    ax.yaxis.grid(True, linestyle="-", which="major", color="grey", alpha=0.3)

    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.pdf", format="pdf")

    ax.set_title("Precision overhead over optimal")
    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.png", format="png")


parser = argparse.ArgumentParser(prog="optimality_plot.py", description="Script outputs optimality plots")
parser.add_argument("-t", "--timeline", dest="timeline", default="report/timeline.json")
parser.add_argument("-o", "--output-path", dest="path", default="report")
args = parser.parse_args()

matplotlib.rcParams.update({"font.size": 12})
plot_optimality(load_optimality(args.timeline), args)
