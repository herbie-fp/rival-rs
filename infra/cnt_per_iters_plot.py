import argparse
import json
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib
import requests

def plot_cnt_per_iters(outcomes, args):
    # Create figure
    fig, ax = plt.subplots(figsize=(4, 3))
    fig.tight_layout(pad=2.0)
    
    # Select tools
    baseline = outcomes.loc[(outcomes['tool_name'] == "valid-baseline") & (outcomes['baseline_iter'] > 0)]
    baseline = baseline.drop(['rival_iter', 'ziv_iter', 'number_of_ops'], axis=1)
    baseline = baseline.groupby(['baseline_iter'], as_index=False, sort=True).sum()

    ziv = outcomes.loc[(outcomes['tool_name'] == "valid-ziv") & (outcomes['ziv_iter'] > 0)]
    ziv = ziv.drop(['rival_iter', 'baseline_iter', 'number_of_ops'], axis=1)
    ziv = ziv.groupby(['ziv_iter'], as_index=False, sort=True).sum()
    
    rival = outcomes.loc[(outcomes['tool_name'] == "valid-rival") & (outcomes['rival_iter'] > 0)]
    rival = rival.drop(['baseline_iter', 'ziv_iter', 'number_of_ops'], axis=1)
    rival = rival.groupby(['rival_iter'], as_index=False, sort=True).sum()
    
    ax.yaxis.grid(True, linestyle='-', which='major', color='grey', alpha=0.3)
    ax.bar(np.arange(len(ziv)) + 0.825, ziv['number_of_points'], color="darkgrey", alpha=1, width=0.4, label='ziv', hatch='\\')
    ax.bar(np.arange(len(baseline)) + 1.0, baseline['number_of_points'], color="green", alpha=1, width=0.4, label='ziv+', hatch='/')
    ax.bar(np.arange(len(rival)) + 1.175, rival['number_of_points'], color="red", alpha=1, width=0.4, label='reval')
   
    def convergence_at(data, n):
        return round(float(data['number_of_points'].head(n).sum()) / data['number_of_points'].sum() * 100, 2)

    print("\\newcommand{\\RivalFirstIterConvergence}{" + str(convergence_at(rival, 1)) + "}")
    print("\\newcommand{\\BaselineFirstIterConvergence}{" + str(convergence_at(baseline, 1)) + "}")
    print("\\newcommand{\\ZivFirstIterConvergence}{" + str(convergence_at(ziv, 1)) + "}")

    print("\\newcommand{\\RivalSecondIterConvergence}{" + str(convergence_at(rival, 2)) + "}")
    print("\\newcommand{\\BaselineSecondIterConvergence}{" + str(convergence_at(baseline, 2)) + "}")
    print("\\newcommand{\\ZivSecondIterConvergence}{" + str(convergence_at(ziv, 2)) + "}")
    
    ax.legend()
    ax.set_xlabel("Iteration")
    ax.set_ylabel("# of converged points")
    iterations = sorted(set(baseline['baseline_iter']).union(set(ziv['ziv_iter'])).union(set(rival['rival_iter'])))
    ax.set_xticks(np.arange(len(iterations)) + 1)
    ax.set_xticklabels([str(iteration) if iteration % 2 == 0 else " " for iteration in iterations])
    plt.ticklabel_format(axis='y', style='sci', scilimits=(4,4))
    
    plt.tight_layout()
    plt.savefig(args.path + "/cnt_per_iters_plot.pdf", format="pdf")
    
    ax.set_title("Convergence distribution")
    plt.tight_layout()
    plt.savefig(args.path + "/cnt_per_iters_plot.png", format="png")
   

def load_outcomes(path):
    outcomes = json.load(open(path, "r"))["outcomes"]
    outcomes = pd.DataFrame(outcomes, columns=['time', 'rival_iter', 'baseline_iter', 'ziv_iter', 'number_of_ops', 'tool_name', 'number_of_points'])
    return outcomes

parser = argparse.ArgumentParser(prog='ratio_plot.py', description='Script outputs ratio plots')
parser.add_argument('-t', '--timeline', dest='timeline', default="report/timeline.json")
parser.add_argument('-o', '--output-path', dest='path', default="report")
args = parser.parse_args()

outcomes = load_outcomes(args.timeline)
matplotlib.rcParams.update({'font.size': 12})
plot_cnt_per_iters(outcomes, args)
