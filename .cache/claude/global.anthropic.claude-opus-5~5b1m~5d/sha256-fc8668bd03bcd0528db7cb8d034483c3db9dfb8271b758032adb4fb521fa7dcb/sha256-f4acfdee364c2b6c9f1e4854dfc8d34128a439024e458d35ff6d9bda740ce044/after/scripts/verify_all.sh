#!/bin/bash
# Run run_cfg.sh for all 48 configurations, PAR at a time.
set -u
W="$(cd "$(dirname "$0")/.." && pwd)"
PAR="${PAR:-8}"
mkdir -p "$W/work"
: > "$W/work/verify_all.summary"

pids=()
run() {
  "$W/scripts/run_cfg.sh" "$@" >> "$W/work/verify_all.summary" 2>&1
}

n=0
while IFS=, read -r b t s; do
  run "$b" "$s" "$t" "$@" &
  pids+=($!)
  n=$((n+1))
  if [ "${#pids[@]}" -ge "$PAR" ]; then
    wait "${pids[0]}"
    pids=("${pids[@]:1}")
  fi
done < <("$W/scripts/all_combos.sh")
wait

sort "$W/work/verify_all.summary"
okc=$(grep -c '^ok' "$W/work/verify_all.summary")
failc=$(grep -c '^FAIL' "$W/work/verify_all.summary")
echo "=== $okc/$n configurations OK, $failc failed ==="
[ "$failc" -eq 0 ]
