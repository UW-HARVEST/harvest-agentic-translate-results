#!/bin/bash
# Sweep "chain of k cities + one overflowing back-edge into city j" and diff both
# programs.  These are the inputs that make the `previous` chain cyclic, so the
# path reconstruction overruns `node_t *path[MAX_NODES]`.
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
C=$ROOT/c_src/build/driver
R=$ROOT/translation/target/release/driver
W="${1:-2147483647}"
fails=0
for k in 3 4 5 6 7 8 10; do
  # the back-edge has to sit on a node that is visited *before* the end, i.e. on
  # C(k-1) at the latest, otherwise the loop breaks before exploring it
  for j in $(seq 1 $((k - 2))); do
    {
      for i in $(seq 1 $k); do echo 1; echo "C$i"; done
      for i in $(seq 1 $((k - 1))); do echo 2; echo "C$i"; echo "C$((i + 1))"; echo 1; done
      echo 2; echo "C$((k - 1))"; echo "C$j"; echo "$W"
      echo 5; echo C1; echo "C$k"
      echo 8
    } > "$TMPDIR/sw.in"
    "$C" < "$TMPDIR/sw.in" > "$TMPDIR/sc.o" 2> "$TMPDIR/sc.e"; cs=$?
    "$R" < "$TMPDIR/sw.in" > "$TMPDIR/sr.o" 2> "$TMPDIR/sr.e"; rs=$?
    cn=$(grep -cE '^  [0-9]+\. ' "$TMPDIR/sc.o")
    rn=$(grep -cE '^  [0-9]+\. ' "$TMPDIR/sr.o")
    if cmp -s "$TMPDIR/sc.o" "$TMPDIR/sr.o" && cmp -s "$TMPDIR/sc.e" "$TMPDIR/sr.e" && [ "$cs" = "$rs" ]; then
      echo "k=$k j=$j  MATCH   (status=$cs entries=$cn stdout=$(wc -c < "$TMPDIR/sc.o"))"
    else
      echo "k=$k j=$j  DIFF    C:status=$cs entries=$cn stdout=$(wc -c < "$TMPDIR/sc.o") | R:status=$rs entries=$rn stdout=$(wc -c < "$TMPDIR/sr.o")"
      fails=$((fails + 1))
    fi
  done
done
echo "== differing shapes: $fails =="
