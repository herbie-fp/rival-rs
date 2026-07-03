import numpy as np
import requests
from matplotlib import pyplot as plt, ticker
import matplotlib
import pandas as pd
import json
import argparse

def load_outcomes(path):
    outcomes = json.load(open(path, "r"))["density"]
    if len(outcomes) > 0 and len(outcomes[0]) == 2:
        outcomes = [["rival", precision, count] for precision, count in outcomes]
    outcomes = pd.DataFrame(outcomes, columns=['tool', 'precision', 'count'])
    return outcomes

def bucket_density(outcomes):
    outcomes = outcomes.copy()
    outcomes['precision'] = np.clip(np.array(outcomes['precision'], dtype=float), 0.0, 1.0)
    outcomes['precision'] = np.minimum(np.floor(outcomes['precision'] / 0.05) * 0.05, 0.95)
    return outcomes.groupby(by=['tool', 'precision'], as_index=False, sort=True).sum()

def plot_density(rival, args):
    fig, ax = plt.subplots(figsize=(4, 3))

    ax.bar(rival['precision']+0.025, rival["count"], color="red", alpha=0.7, width=0.05, label='reval')

    ax.set_ylabel("Number of operations")
    ax.set_xlabel("Precision (normalized)")
    ax.set_xlim(0.0, 1.0)
    ax.set_xticks(np.linspace(0.0, 1.0, 6))
    ax.xaxis.set_major_formatter(ticker.FormatStrFormatter('%.1f'))
    ax.yaxis.grid(True, linestyle='-', which='major', color='grey', alpha=0.3)

    plt.legend()
    plt.tight_layout()
    plt.savefig(args.path + "/density_plot.pdf", format="pdf")

    ax.set_title("Density plot")
    plt.tight_layout()
    plt.savefig(args.path + "/density_plot.png", format="png")
    plt.close(fig)

def plot_density_cdf(outcomes, args):
    fig, ax = plt.subplots(figsize=(4, 3))

    styles = {
        "optimal": ("orange", "-", "optimal"),
        "rival": ("red", "-.", "rival"),
        "baseline": ("green", "--", "baseline"),
    }
    for tool in ["optimal", "rival", "baseline"]:
        tool_outcomes = outcomes[outcomes["tool"] == tool].copy()
        if tool_outcomes.empty:
            continue
        total = tool_outcomes["count"].sum()
        tool_outcomes["cdf"] = tool_outcomes["count"].cumsum() / total
        x = np.concatenate(([0.0], np.array(tool_outcomes['precision']+0.05, dtype=float)))
        y = np.concatenate(([0.0], np.array(tool_outcomes["cdf"], dtype=float)))

        color, linestyle, label = styles[tool]
        ax.step(x, y, where='post', linestyle=linestyle, color=color, linewidth=2, label=label)

    ax.set_ylim(0, 1.0)
    ax.set_xlim(0.0, 1.0)
    ax.set_xticks(np.linspace(0.0, 1.0, 6))
    ax.xaxis.set_major_formatter(ticker.FormatStrFormatter('%.1f'))
    ax.yaxis.set_major_formatter(ticker.PercentFormatter(xmax=1.0))
    ax.set_ylabel("Fraction of operations")
    ax.set_xlabel("Precision (normalized)")
    ax.yaxis.grid(True, linestyle='-', which='major', color='grey', alpha=0.3)

    plt.legend(loc="best")
    plt.tight_layout()
    plt.savefig(args.path + "/density_cdf_plot.pdf", format="pdf")

    ax.set_title("Density CDF")
    plt.tight_layout()
    plt.savefig(args.path + "/density_cdf_plot.png", format="png")
    plt.close(fig)

def plot_density_plots(args):
    outcomes = load_outcomes(args.timeline)
    outcomes = bucket_density(outcomes)
    rival = outcomes[outcomes["tool"] == "rival"]

    print("\\newcommand{\\DensityPercentageOfLowerPrecision}{" + str(round(rival["count"][:4].sum() / rival["count"].sum() * 100, 2)) + "}")

    plot_density(rival, args)
    plot_density_cdf(outcomes, args)

parser = argparse.ArgumentParser(prog='histograms.py', description='Script outputs mixed precision histograms for a Herbie run')
parser.add_argument('-t', '--timeline', dest='timeline', default="report/timeline.json")
parser.add_argument('-o', '--output-path', dest='path', default="report")

args = parser.parse_args()
matplotlib.rcParams.update({'font.size': 12})
plot_density_plots(args)
