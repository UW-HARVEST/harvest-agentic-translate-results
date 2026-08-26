#!/bin/bash
# Emit every canonical feature combination (OP x REPEAT) one per line.
for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do
    echo "$op,$r"
  done
done
