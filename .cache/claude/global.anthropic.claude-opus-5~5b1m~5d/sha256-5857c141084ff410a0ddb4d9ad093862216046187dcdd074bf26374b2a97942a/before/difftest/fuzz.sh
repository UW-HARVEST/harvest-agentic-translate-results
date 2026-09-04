#!/bin/bash
R=$HARVEST_WORKDIR/translation/target/release/driver
C=$HARVEST_WORKDIR/difftest/cref
fail=0
for i in $(seq 1 500); do
  head -c $((RANDOM % 22)) /dev/urandom | tr -dc ' \n\t+-0123456789abz' > f.txt
  co=$("$C" < f.txt 2>/dev/null); cr=$?
  ro=$("$R" < f.txt 2>/dev/null); rr=$?
  if [ "$co" != "$ro" ] || [ "$cr" != "$rr" ]; then echo "DIFF: [$(od -c f.txt | head -2)] C(rc=$cr)[$co] RS(rc=$rr)[$ro]"; fail=$((fail+1)); fi
done
echo "diffs=$fail"
