import argparse
import json

import matplotlib
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd


def load_optimality(path):
    rows = json.load(open(path, "r")).get("optimality", [])
    outcomes = pd.DataFrame(rows, columns=["tool", "iteration", "diff", "relative_diff"])
    return outcomes.replace({False: np.nan})

def plot_optimality(outcomes, args):
    outcomes["iteration"] = pd.to_numeric(outcomes["iteration"], errors="coerce")
    outcomes["diff"] = pd.to_numeric(outcomes["diff"], errors="coerce")
    outcomes = outcomes.dropna(subset=["tool", "iteration", "diff"])
    outcomes["iteration"] = outcomes["iteration"].astype(int)

    outcomes = outcomes.loc[outcomes["iteration"] > 0]
    if outcomes.empty:
        plot_empty(args)
        return

    iterations = sorted(outcomes["iteration"].unique())
    x = np.arange(len(iterations))

    def tool_values(tool):
        by_iter = outcomes.loc[outcomes["tool"] == tool].set_index("iteration")["diff"]
        return np.array([by_iter.get(iteration, np.nan) for iteration in iterations], dtype=float)

    fig, ax = plt.subplots(figsize=(4, 3))
    fig.tight_layout(pad=2.0)

    width = 0.5
    ax.bar(x - 0.075, tool_values("baseline"), color="green", alpha=1, width=width, label="baseline", hatch="/")
    ax.bar(x + 0.075, tool_values("rival"), color="red", alpha=0.7, width=width, label="reval")

    ax.legend()
    ax.set_xlabel("True uniform precision")
    ax.set_ylabel("Avg. precision overhead")
    ax.set_yscale("symlog", linthresh=1)

    ax.set_xticks(x)
    ax.set_xticklabels([
        "$2^{" + str(pos + 7) + "}$" if (pos + 7) % 2 == 1 else " "
        for pos in range(len(iterations))
    ])
    ax.yaxis.grid(True, linestyle="-", which="major", color="grey", alpha=0.3)

    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.pdf", format="pdf")

    ax.set_title("Precision overhead over optimal")
    plt.tight_layout()
    plt.savefig(args.path + "/optimality_plot.png", format="png")


def plot_relative_optimality(outcomes, args):
    outcomes["iteration"] = pd.to_numeric(outcomes["iteration"], errors="coerce")
    outcomes["relative_diff"] = pd.to_numeric(outcomes["relative_diff"], errors="coerce")
    outcomes = outcomes.dropna(subset=["tool", "iteration", "relative_diff"])
    outcomes["iteration"] = outcomes["iteration"].astype(int)

    outcomes = outcomes.loc[outcomes["iteration"] > 0]
    if outcomes.empty:
        plot_empty_relative(args)
        return

    iterations = sorted(outcomes["iteration"].unique())
    x = np.arange(len(iterations))

    def tool_values(tool):
        by_iter = outcomes.loc[outcomes["tool"] == tool].set_index("iteration")["relative_diff"]
        return np.array([by_iter.get(iteration, np.nan) for iteration in iterations], dtype=float)

    fig, ax = plt.subplots(figsize=(4, 3))
    fig.tight_layout(pad=2.0)

    width = 0.5
    ax.bar(x - 0.075, tool_values("baseline"), color="green", alpha=1, width=width, label="baseline", hatch="/")
    ax.bar(x + 0.075, tool_values("rival"), color="red", alpha=0.7, width=width, label="reval")

    ax.legend()
    ax.set_xlabel("Iteration")
    ax.set_ylabel("Avg. relative precision overhead")
    ax.set_yscale("symlog", linthresh=1)
    ax.set_xticks(x)
    ax.set_xticklabels([
        f"{iteration}" if iteration % 2 == 0 else " "
        for iteration in iterations
    ])
    ax.yaxis.grid(True, linestyle="-", which="major", color="grey", alpha=0.3)

    plt.tight_layout()
    plt.savefig(args.path + "/optimality_relative_plot.pdf", format="pdf")

    ax.set_title("Relative precision overhead over optimal")
    plt.tight_layout()
    plt.savefig(args.path + "/optimality_relative_plot.png", format="png")


parser = argparse.ArgumentParser(prog="optimality_plot.py", description="Script outputs optimality plots")
parser.add_argument("-t", "--timeline", dest="timeline", default="report/timeline.json")
parser.add_argument("-o", "--output-path", dest="path", default="report")
args = parser.parse_args()

matplotlib.rcParams.update({"font.size": 12})
optimality = load_optimality(args.timeline)
plot_optimality(optimality, args)
plot_relative_optimality(optimality, args)
