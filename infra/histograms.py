import numpy as np
import requests
from matplotlib import pyplot as plt, ticker
import matplotlib
import pandas as pd
import json
import argparse

def load_mixsample(path, tool, valid):
    timeline_json = json.load(open(path, "r"))
    outcomes = timeline_json["mixsample-" + str(tool) + "-valid" if valid else "mixsample-" + str(tool) + "-all"]
    outcomes = pd.DataFrame(outcomes, columns=['time', 'op', 'precision'])
    return outcomes

def plot_histogram_valid(args):
    baseline = load_mixsample(args.timeline, "baseline", True)
    rival = load_mixsample(args.timeline, "rival", True)
    ziv = load_mixsample(args.timeline, "ziv", True)

    adjust_time_baseline = round(baseline[baseline["op"] == 'adjust']['time'].sum()/1000, 2)
    baseline = baseline[baseline["op"] != 'adjust']

    adjust_time_rival = round(rival[rival["op"] == 'adjust']['time'].sum()/1000, 2)
    rival = rival[rival["op"] != 'adjust']

    adjust_time_ziv = round(ziv[ziv["op"] == 'adjust']['time'].sum()/1000, 2)
    ziv = ziv[ziv["op"] != 'adjust']

    print("\\newcommand{\\TuningTime}{" + str(adjust_time_rival) + "\\xspace}")
    print("\\newcommand{\\TuningTimePercentage}{" + str(round(adjust_time_rival/rival['time'].sum()*1000 * 100, 1)) + "}")
    print("\\newcommand{\\RivalSpeedupHistograms}{" + str(round((baseline['time'].sum()-rival['time'].sum())/baseline['time'].sum() * 100, 2)) + "}")

    fig, ax = plt.subplots(figsize=(6.5, 2.75))

    bins = 2 ** np.arange(5, 17, 1)

    buckets_base = bucket_precisions_by_bins(baseline, bins)
    buckets_rival = bucket_precisions_by_bins(rival, bins)
    buckets_ziv = bucket_precisions_by_bins(ziv, bins)

    ax.yaxis.grid(True, linestyle='-', which='major', color='grey', alpha=0.3)
    
    ax.bar(np.arange(len(bins)) + 0.3, buckets_ziv, color="darkgrey", alpha=1, width=0.45, label='ziv', hatch='\\')
    ax.bar(np.arange(len(bins)) + 0.5, buckets_base, color="green", alpha=1, width=0.45, label='baseline', hatch='/')
    ax.bar(np.arange(len(bins)) + 0.7, buckets_rival, color="red", alpha=1, width=0.45, label='reval')
    
    # ax.bar(np.arange(len(bins)) + 0.3, buckets_base, color="green", alpha=1, width=0.25, label='baseline', hatch='/')
    # ax.bar(np.arange(len(bins)) + 0.55, buckets_rival, color="red", alpha=0.7, width=0.25, label='reval')
    # ax.bar(np.arange(len(bins)) + 0.8, buckets_ziv, color="purple", alpha=0.7, width=0.25, label='ziv')

    tuning_time_ziv = np.zeros_like(buckets_ziv)
    tuning_time_ziv[-1] = adjust_time_ziv
    ax.bar(np.arange(len(bins)) - 0.2, tuning_time_ziv, color="darkgrey", alpha=1, width=0.45, hatch='\\')
    
    # Baseline tuning time
    tuning_time_baseline = np.zeros_like(buckets_base)
    tuning_time_baseline[-1] = adjust_time_baseline
    ax.bar(np.arange(len(bins)), tuning_time_baseline, color="green", alpha=1, width=0.45, hatch='/')

    # Reval tuning time
    tuning_time_rival = np.zeros_like(buckets_rival)
    tuning_time_rival[-1] = adjust_time_rival
    ax.bar(np.arange(len(bins)) + 0.2, tuning_time_rival, color="red", alpha=1, width=0.45)

    ax.set_xticks(np.arange(len(bins)), bins)
    ax.set_xticklabels(["$2^{" + str(i+5) + "}$" if i != 11 else "tuning" for i, x in enumerate(bins)])

    ax.margins(x=0.02)
    ax.set_ylabel("Seconds spent")
    ax.set_xlabel("Precision (number of bits)")
    
    plt.legend()
    plt.tight_layout()
    plt.savefig(args.path + "/histogram_valid.pdf", format="pdf")

    ax.set_title("Histogram for valid points")
    plt.tight_layout()
    plt.savefig(args.path + "/histogram_valid.png", format="png")
    
def plot_histogram_all(args):
    baseline = load_mixsample(args.timeline, "baseline", False)
    rival = load_mixsample(args.timeline, "rival", False)
    ziv = load_mixsample(args.timeline, "ziv", False)

    adjust_time_baseline = round(baseline[baseline["op"] == 'adjust']['time'].sum()/1000, 2)
    baseline = baseline[baseline["op"] != 'adjust']

    adjust_time_rival = round(rival[rival["op"] == 'adjust']['time'].sum()/1000, 2)
    rival = rival[rival["op"] != 'adjust']

    adjust_time_ziv = round(ziv[ziv["op"] == 'adjust']['time'].sum()/1000, 2)
    ziv = ziv[ziv["op"] != 'adjust']

    fig, ax = plt.subplots(figsize=(6.5, 2.0))

    bins = 2 ** np.arange(5, 17, 1)

    buckets_base = bucket_precisions_by_bins(baseline, bins)
    buckets_rival = bucket_precisions_by_bins(rival, bins)
    buckets_ziv = bucket_precisions_by_bins(ziv, bins)
    
    ax.yaxis.grid(True, linestyle='-', which='major', color='grey', alpha=0.3)
    
    ax.bar(np.arange(len(bins)) + 0.3, buckets_ziv, color="darkgrey", alpha=1, width=0.45, label='ziv', hatch='\\')
    ax.bar(np.arange(len(bins)) + 0.5, buckets_base, color="green", alpha=1, width=0.45, label='baseline', hatch='/')
    ax.bar(np.arange(len(bins)) + 0.7, buckets_rival, color="red", alpha=1, width=0.45, label='reval')
    
    # ax.bar(np.arange(len(bins)) + 0.3, buckets_base, color="green", alpha=1, width=0.25, label='baseline', hatch='/')
    # ax.bar(np.arange(len(bins)) + 0.55, buckets_rival, color="red", alpha=0.7, width=0.25, label='reval')
    # ax.bar(np.arange(len(bins)) + 0.8, buckets_ziv, color="purple", alpha=0.7, width=0.25, label='ziv')

    tuning_time_ziv = np.zeros_like(buckets_ziv)
    tuning_time_ziv[-1] = adjust_time_ziv
    ax.bar(np.arange(len(bins)) - 0.2, tuning_time_ziv, color="darkgrey", alpha=1, width=0.45, hatch='\\')
    
    # Baseline tuning time
    tuning_time_baseline = np.zeros_like(buckets_base)
    tuning_time_baseline[-1] = adjust_time_baseline
    ax.bar(np.arange(len(bins)), tuning_time_baseline, color="green", alpha=1, width=0.45, hatch='/')

    # Reval tuning time
    tuning_time_rival = np.zeros_like(buckets_rival)
    tuning_time_rival[-1] = adjust_time_rival
    ax.bar(np.arange(len(bins)) + 0.2, tuning_time_rival, color="red", alpha=1, width=0.45)
    
    

    ax.set_xticks(np.arange(len(bins)), bins)
    ax.set_xticklabels(["$2^{" + str(i+5) + "}$" if i != 11 else "tuning" for i, x in enumerate(bins)])

    ax.margins(x=0.02)
    ax.set_ylabel("Seconds spent")
    ax.set_xlabel("Precision (number of bits)")
    
    plt.legend()
    plt.tight_layout()
    plt.savefig(args.path + "/histogram_all.pdf", format="pdf")

    ax.set_title("Histogram for all the points")
    plt.tight_layout()
    plt.savefig(args.path + "/histogram_all.png", format="png")
    
def bucket_precisions_by_bins(data, bins):
    x = [0] * len(bins)
    for i in range(len(bins) - 1):
        time_per_bucket = data.loc[(data["precision"] >= bins[i]) & (data["precision"] < bins[i + 1]), "time"].sum()
        time_fraction_per_bucket = time_per_bucket/1000
        x[i] = time_fraction_per_bucket
    return np.array(x)

parser = argparse.ArgumentParser(prog='histograms.py', description='Script outputs mixed precision histograms for a Herbie run')
parser.add_argument('-t', '--timeline', dest='timeline', default="report/timeline.json")
parser.add_argument('-o', '--output-path-valid', dest='path', default="report")

args = parser.parse_args()
matplotlib.rcParams.update({'font.size': 11})
plot_histogram_all(args)
plot_histogram_valid(args)
