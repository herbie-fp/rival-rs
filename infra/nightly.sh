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
racket -y infra/time.rkt --dir "$REPORTDIR" --profile profile.json infra/points.json
python3 infra/ratio_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python3 infra/point_graph.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python3 infra/histograms.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python3 infra/cnt_per_iters_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python3 infra/repeats_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python3 infra/density_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
python3 infra/optimality_plot.py -t "$REPORTDIR"/timeline.json -o "$REPORTDIR"
cp profile.json "$REPORTDIR"/profile.json
cp infra/profile.js "$REPORTDIR"/profile.js
