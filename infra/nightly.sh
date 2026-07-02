#!/bin/bash
set -e -x

REPORTDIR="report"
export PATH=~/.cargo/bin:$PATH

function clean {
  if [ -d "$REPORTDIR" ]; then
    rm -r "$REPORTDIR"
  fi
  mkdir -p "$REPORTDIR"
}

clean
rustup update
make install
xz -d -k -f infra/points.json.xz
xz -d -k -f infra/optimal_precisions.json.xz
racket -y infra/time.rkt --dir "$REPORTDIR" --profile profile.json --optimal-precisions infra/optimal_precisions.json infra/points.json
python infra/ratio_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python infra/point_graph.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python infra/histograms.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python infra/cnt_per_iters_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python infra/repeats_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python infra/density_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python infra/optimality_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
cp profile.json "$REPORTDIR"/profile.json
cp infra/profile.js "$REPORTDIR"/profile.js
