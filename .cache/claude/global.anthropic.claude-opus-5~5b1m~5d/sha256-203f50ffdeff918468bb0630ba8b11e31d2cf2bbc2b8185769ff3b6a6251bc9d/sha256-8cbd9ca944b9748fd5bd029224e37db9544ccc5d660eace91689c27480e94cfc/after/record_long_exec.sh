#!/bin/bash
# Records every expensive `long_exec` run.
#
# One `long_exec` per PROCESS: glibc `srand`/`rand` is a single process-global,
# so two concurrent `long_exec` calls (even in different .so's - they share the
# one libc) would interleave their draws. Separate processes have separate PRNG
# state, so the recorders are safe to run in parallel as processes.
set -u
W="$(cd "$(dirname "$0")" && pwd)"
cd "$W/translation" || exit 1
LOG="$W/reclogs"; mkdir -p "$LOG"

TAGS=(seed_0 seed_1 seed_42 seed_42_poisoned seed_7 seed_7_junk_abi seed_int_max seed_0x80000000 seed_uint_max)

cargo build --offline --release >/dev/null 2>&1 || { echo "build failed"; exit 1; }
cargo test --offline --release --test long_exec_full --no-run >/dev/null 2>&1

pids=()
for t in "${TAGS[@]}"; do
  ( timeout 580 cargo test --offline --release --test long_exec_full -- \
      --ignored --test-threads=1 --nocapture --exact "rec_c_$t" \
      > "$LOG/c_$t.log" 2>&1 ; echo "$? c_$t" >> "$LOG/exits" ) &
  pids+=($!)
  ( timeout 580 cargo test --offline --release --test long_exec_full -- \
      --ignored --test-threads=1 --nocapture --exact "rec_rust_$t" \
      > "$LOG/rust_$t.log" 2>&1 ; echo "$? rust_$t" >> "$LOG/exits" ) &
  pids+=($!)
done
echo "launched ${#pids[@]} recorder processes"
wait "${pids[@]}"
echo "all recorders finished"
grep -h "^[1-9]" "$LOG/exits" 2>/dev/null && echo "SOME RECORDERS FAILED" || echo "all recorder exits are 0"
