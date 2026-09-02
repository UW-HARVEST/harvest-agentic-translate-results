#!/bin/bash
# Launch full-runtime C-vs-Rust comparisons for every "valid seed" input class,
# all in parallel.  Results land in scratch/results/<tag>.{c,rs}.{out,err,rc}
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
C="$ROOT/c_src/build/driver"
R="$ROOT/translation/target/release/driver"
OUT="$ROOT/scratch/results"
mkdir -p "$OUT"

run_one() {
  local tag="$1"; shift
  local bin="$1"; shift
  local which="$1"; shift
  # remaining args are argv[1..]
  "$bin" "$@" > "$OUT/$tag.$which.out" 2> "$OUT/$tag.$which.err"
  echo $? > "$OUT/$tag.$which.rc"
}

# tag <TAB> single argv[1] value (may be empty / contain spaces)
declare -a TAGS=(
  "empty"
  "zero"
  "one"
  "two"
  "fortytwo"
  "int32max"
  "int32max_p1"
  "uintmax"
  "uintmax_m1"
  "neg_zero"
  "plus_zeros_42"
  "lead_space_7"
  "many_zeros_42"
  "lead_tab_9"
)
declare -a ARGS=(
  ""
  "0"
  "1"
  "2"
  "42"
  "2147483647"
  "2147483648"
  "4294967295"
  "4294967294"
  "-0"
  "+000000042"
  "   7"
  "0000000000000000000000000042"
  $'\t\n 9'
)

for i in "${!TAGS[@]}"; do
  t="${TAGS[$i]}"
  a="${ARGS[$i]}"
  run_one "$t" "$C" c "$a" &
  run_one "$t" "$R" rs "$a" &
done
wait
echo "ALL DONE" > "$OUT/.done"
