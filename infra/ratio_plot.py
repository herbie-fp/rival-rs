import argparse
import json
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt


def tool_speeds(data, iter_col):
    sorted_data = data.sort_values(by=[iter_col])
    x = sorted_data[iter_col].astype(int)
    y = sorted_data['number_of_points'] / sorted_data['time'] * 1000
    return x, y


def configure_speed_axis(ax):
    ax.set_yscale("log", base=2)
    lo, hi = ax.get_ylim()
    lo_exp = int(np.floor(np.log2(lo)))
    hi_exp = int(np.ceil(np.log2(hi)))
    exponents = np.arange(lo_exp, hi_exp + 1)
    ticks = 2.0 ** exponents
    labels = ["$2^{" + str(exp) + "}$" if i % 2 == 0 else "" for i, exp in enumerate(exponents)]

    ax.set_ylim(ticks[0], ticks[-1])
    ax.set_yticks(ticks)
    ax.set_yticklabels(labels)
    ax.yaxis.grid(True, linestyle='-', which='major', color='grey', alpha=0.3)


def plot_speed_graph_rival_iter(outcomes, args):
    fig, ax = plt.subplots(figsize=(4, 2.5))
    
    outcomes = outcomes.drop(['baseline_iter', 'number_of_ops'], axis=1)
    outcomes = outcomes.groupby(['rival_iter', 'tool_name'], as_index=False).sum()
    
    baseline_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-baseline") & (outcomes['rival_iter'] > 0)]
    ziv_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-ziv") & (outcomes['rival_iter'] > 0)]
    rival_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-rival") & (outcomes['rival_iter'] > 0)]
    sollya_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-sollya") & (outcomes['rival_iter'] > 0)]
    
    x, y = tool_speeds(rival_cmp, 'rival_iter')
    ax.plot(x, y, '.-', linewidth=2.0, color='r', label='reval')
    x, y = tool_speeds(ziv_cmp, 'rival_iter')
    ax.plot(x, y, '-', marker='s', linewidth=2.5, color='dimgrey', label='ziv')
    x, y = tool_speeds(baseline_cmp, 'rival_iter')
    ax.plot(x, y, '-', linewidth=2.0, color='g', label='ziv+')
    x, y = tool_speeds(sollya_cmp, 'rival_iter')
    ax.plot(x, y, ':', marker='>', linewidth=2.0, color='b', label='sollya')
    
    ax.legend()
    ax.set_xlabel("Difficulty")
    ax.set_ylabel("Speed")
    configure_speed_axis(ax)
    plt.tight_layout()
    plt.savefig(args.path + "/ratio_plot_iter.pdf", format="pdf")
    
    ax.set_title("Speed plot per iteration")
    plt.tight_layout()
    plt.savefig(args.path + "/ratio_plot_iter.png", format="png")


def plot_speed_graph_baseline_precision(outcomes, args):
    fig, ax = plt.subplots(figsize=(4, 2.5))
    
    outcomes = outcomes.drop(['rival_iter', 'baseline_iter', 'number_of_ops'], axis=1)
    outcomes = outcomes.groupby(['ziv_iter', 'tool_name'], as_index=False).sum()
    
    baseline_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-baseline") & (outcomes['ziv_iter'] > 0)]
    ziv_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-ziv") & (outcomes['ziv_iter'] > 0)]
    rival_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-rival") & (outcomes['ziv_iter'] > 0)]
    sollya_cmp = outcomes.loc[(outcomes['tool_name'] == "valid-sollya") & (outcomes['ziv_iter'] > 0)]

    rival_initial = float(outcomes.loc[(outcomes['tool_name'] == "valid-rival") & (outcomes['ziv_iter'] == 0)]['time'].iloc[0])
    baseline_initial = float(outcomes.loc[(outcomes['tool_name'] == "valid-baseline") & (outcomes['ziv_iter'] == 0)]['time'].iloc[0])
    ziv_initial = float(outcomes.loc[(outcomes['tool_name'] == "valid-ziv") & (outcomes['ziv_iter'] == 0)]['time'].iloc[0])
    sollya_initial = float(outcomes.loc[(outcomes['tool_name'] == "valid-sollya") & (outcomes['ziv_iter'] == 0)]['time'].iloc[0])
    
    rival_points = outcomes.loc[outcomes['tool_name'] == "valid-rival", 'number_of_points'].sum()
    rival_tuned_points = rival_cmp['number_of_points'].sum()
    print("\\newcommand{\\NumTunedPoints}{" + str(rival_tuned_points) + "\\xspace}")
    print("\\newcommand{\\NumUntunedPoints}{" + str(rival_points-rival_tuned_points) + "\\xspace}")
    print("\\newcommand{\\RivalInitialSpeedupOverSollya}{" + str(round(sollya_initial/rival_initial, 2)) + "\\xspace}")
    print("\\newcommand{\\RivalInitialSpeedupOverBaseline}{" + str(round(baseline_initial/rival_initial, 2)) + "\\xspace}")
    print("\\newcommand{\\RivalInitialSpeedupOverZiv}{" + str(round(ziv_initial/rival_initial, 2)) + "\\xspace}")
    
    x, y = tool_speeds(ziv_cmp, 'ziv_iter')
    xticks = x
    ax.plot(x - 1, y, '-', marker='s', linewidth=2.5, color='dimgrey', label='ziv')
    x, y = tool_speeds(baseline_cmp, 'ziv_iter')
    ax.plot(x - 1, y, '-', linewidth=2.0, color='g', label='ziv+')
    x, y = tool_speeds(sollya_cmp, 'ziv_iter')
    ax.plot(x - 1, y, ':', marker='>', linewidth=2.0, color='b', label='sollya')
    x, y = tool_speeds(rival_cmp, 'ziv_iter')
    ax.plot(x - 1, y, '.-', linewidth=2.0, color='r', label='reval')
    
    ax.legend()
    ax.set_xlabel("True uniform precision")
    ax.set_ylabel("Speed")
    
    configure_speed_axis(ax)
    ax.set_xticks(np.arange(len(xticks)), xticks)
    ax.set_xticklabels(["$2^{" + str(i + 7) + "}$" for i, _ in enumerate(xticks)])
    
    plt.tight_layout()
    plt.savefig(args.path + "/ratio_plot_precision.pdf", format="pdf")
    
    ax.set_title("Speed plot per precision")
    plt.tight_layout()
    plt.savefig(args.path + "/ratio_plot_precision.png", format="png")
    
    # Latex stuff  
    average_over_sollya = round(sollya_cmp['time'].sum() / rival_cmp['time'].sum(), 2)
    average_over_baseline = round(baseline_cmp['time'].sum() / rival_cmp['time'].sum(), 2)
    average_over_ziv = round(ziv_cmp['time'].sum() / rival_cmp['time'].sum(), 2)
    print("\\newcommand{\\RivalAvgSpeedupOverSollya}{" + str(average_over_sollya) + "\\xspace}")
    print("\\newcommand{\\RivalAvgSpeedupOverBaseline}{" + str(average_over_baseline) + "\\xspace}")
    print("\\newcommand{\\RivalAvgSpeedupOverZiv}{" + str(average_over_ziv) + "\\xspace}")
    
    _, rival_speed = tool_speeds(rival_cmp, 'ziv_iter')
    _, sollya_speed = tool_speeds(sollya_cmp, 'ziv_iter')
    _, baseline_speed = tool_speeds(baseline_cmp, 'ziv_iter')
    _, ziv_speed = tool_speeds(ziv_cmp, 'ziv_iter')
    max_over_sollya = max([round(i/j, 2) for i, j in zip(rival_speed, sollya_speed)])
    max_over_baseline = max([round(i/j, 2) for i, j in zip(rival_speed, baseline_speed)])
    max_over_ziv = max([round(i/j, 2) for i, j in zip(rival_speed, ziv_speed)])
    print("\\newcommand{\\RivalMaxSpeedupOverSollya}{" + str(max_over_sollya) + "\\xspace}")
    print("\\newcommand{\\RivalMaxSpeedupOverBaseline}{" + str(max_over_baseline) + "\\xspace}")
    print("\\newcommand{\\RivalMaxSpeedupOverZiv}{" + str(max_over_ziv) + "\\xspace}")
        
def load_outcomes(path):
    outcomes = json.load(open(path, "r"))["outcomes"]
    outcomes = pd.DataFrame(outcomes, columns=['time', 'rival_iter', 'baseline_iter', 'ziv_iter', 'number_of_ops', 'tool_name', 'number_of_points'])
    return outcomes

parser = argparse.ArgumentParser(prog='ratio_plot.py', description='Script outputs ratio plots')
parser.add_argument('-t', '--timeline', dest='timeline', default="report/timeline.json")
parser.add_argument('-o', '--output-path', dest='path', default="report")
args = parser.parse_args()

outcomes = load_outcomes(args.timeline)
plot_speed_graph_rival_iter(outcomes, args)
plot_speed_graph_baseline_precision(outcomes, args)
