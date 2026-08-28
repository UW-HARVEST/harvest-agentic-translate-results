#!/bin/bash
W=$HARVEST_WORKDIR
C=$W/c_src/build/driver
R=$W/translation/target/release/driver
S=$W/scratch
IN="$1"
printf '%s' "$IN" > $S/in.txt
"$C" < $S/in.txt > $S/c.out 2> $S/c.err; cs=$?
"$R" < $S/in.txt > $S/r.out 2> $S/r.err; rs=$?
ok=1
cmp -s $S/c.out $S/r.out || { ok=0; echo "STDOUT DIFF"; }
cmp -s $S/c.err $S/r.err || { ok=0; echo "STDERR DIFF"; }
[ "$cs" = "$rs" ] || { ok=0; echo "STATUS DIFF c=$cs r=$rs"; }
if [ $ok = 0 ]; then
  echo "--- C out:"; head -c 400 $S/c.out; echo "--- R out:"; head -c 400 $S/r.out
  echo "--- C err:"; head -c 400 $S/c.err; echo "--- R err:"; head -c 400 $S/r.err
  echo "=== c status $cs / r status $rs"
else
  echo "OK (status $cs) out=$(head -c 120 $S/c.out | tr '\n' '|') err=$(head -c 120 $S/c.err | tr '\n' '|')"
fi
