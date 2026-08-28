#!/usr/bin/env bash
# Generalized differential runner: takes explicit `FEATURES|OP|REPEAT` triples,
# where FEATURES is the cargo feature list to build the Rust cdylib with (may be
# empty) and OP/REPEAT are the C defines the resulting library is expected to
# behave like.
#
# This covers the feature *spellings* that `run_all.sh` does not:
#   * omitted features (the CMake defaults: no OP feature -> add, no REPEAT -> 5)
#   * conflicting features, whose documented priority is mul > sub > add and
#     "highest REPEAT wins"
#
# usage: run_combos.sh 'FEATS|OP|REP' ...
#        run_combos.sh --spellings     (the built-in list described above)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/translation"
mkdir -p "$ROOT/cbuild/rs" "$ROOT/cbuild/logs"

if [ "${1:-}" = "--spellings" ]; then
  TRIPLES=(
    '|add|5'                              # no features at all -> CMake defaults
    'add|add|5' 'sub|sub|5' 'mul|mul|5'   # OP only -> REPEAT defaults to 5
    '0|add|0' '1|add|1' '2|add|2' '3|add|3'
    '4|add|4' '5|add|5' '6|add|6' '7|add|7'   # REPEAT only -> OP defaults to add
    'add,sub|sub|5'                       # conflicting: mul > sub > add
    'add,mul|mul|5'
    'sub,mul|mul|5'
    'add,sub,mul|mul|5'
    '0,1,2,3,4,5,6,7|add|7'               # conflicting: highest REPEAT wins
    'add,sub,mul,0,1,2,3,4,5,6,7|mul|7'
  )
else
  TRIPLES=("$@")
fi

pass=0; fail=0; failed=()
for t in "${TRIPLES[@]}"; do
  feats="${t%%|*}"; rest="${t#*|}"; op="${rest%%|*}"; rep="${rest##*|}"
  tag=$(echo "${feats:-none}" | tr ',' '-')
  log="$ROOT/cbuild/logs/combo_${tag}.log"

  c_so=$("$ROOT/scripts/build_c_so.sh" "$op" "$rep") || { echo "C BUILD FAIL $t"; fail=$((fail+1)); failed+=("$t"); continue; }
  c_exe=$("$ROOT/scripts/build_c_exe.sh" "$op" "$rep") || { echo "C EXE FAIL $t"; fail=$((fail+1)); failed+=("$t"); continue; }

  if [ -z "$feats" ]; then
    build=(cargo build --release --offline --no-default-features)
    test=(cargo test --release --offline --no-default-features)
  else
    build=(cargo build --release --offline --no-default-features --features "$feats")
    test=(cargo test --release --offline --no-default-features --features "$feats")
  fi

  if ! timeout 300 "${build[@]}" >"$log" 2>&1; then
    echo "RUST BUILD FAIL [$feats]"; tail -n 15 "$log"; fail=$((fail+1)); failed+=("$t"); continue
  fi
  rs_so="$ROOT/cbuild/rs/libmacrodepth_combo_${tag}.so"
  cp "$ROOT/translation/target/release/libmacrodepth_add_5.so" "$rs_so"

  if MD_OP="$op" MD_REPEAT="$rep" MD_C_SO="$c_so" MD_RUST_SO="$rs_so" MD_C_EXE="$c_exe" \
     timeout 600 "${test[@]}" -- --test-threads=1 >>"$log" 2>&1; then
    n=$(grep -c '^test .* ok$' "$log")
    echo "PASS features=[${feats:-<none>}] behaves like OP=$op REPEAT=$rep  ($n tests)"
    pass=$((pass+1))
  else
    echo "FAIL features=[${feats:-<none>}] expected OP=$op REPEAT=$rep (see $log)"
    grep -E "^(test .*FAILED|failures:|thread .* panicked)" -A6 "$log" | head -n 30
    fail=$((fail+1)); failed+=("$t")
  fi
done

echo "=================================================="
echo "combinations passed: $pass   failed: $fail"
[ "$fail" -eq 0 ] || { printf 'failing: %s\n' "${failed[*]}"; exit 1; }
