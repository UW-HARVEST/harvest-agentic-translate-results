#!/bin/sh
# Long-running randomized sweep: re-runs the whole differential suite with a
# fresh (but reproducible) PRNG corpus for each DIFF_SEED value.
#
#   ./stress.sh 40          # 40 independent corpora
set -e
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
n=${1:-20}
log="$here/target/stress.log"
i=0
while [ "$i" -lt "$n" ]; do
    printf 'DIFF_SEED=%s ... ' "$i"
    if DIFF_SEED=$i cargo test --release --offline -q -- --test-threads=8 --skip ub_crash_matrix >"$log" 2>&1; then
        echo ok
    else
        echo FAILED
        tail -80 "$log"
        exit 1
    fi
    i=$((i + 1))
done
echo "all $n corpora agree"
