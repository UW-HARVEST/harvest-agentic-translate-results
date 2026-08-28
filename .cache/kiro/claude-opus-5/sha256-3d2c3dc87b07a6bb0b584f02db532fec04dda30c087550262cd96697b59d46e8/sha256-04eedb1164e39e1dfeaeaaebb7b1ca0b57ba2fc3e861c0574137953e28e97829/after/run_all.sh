#!/usr/bin/env bash
# Run the FFI parity suite for every feature combination, then compare the
# `driver` executables (Rust vs the CMake build) for the canonical ones.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"

fail=0
run_suite() {
  local feats="$1" label="${1:-<none>}"
  local out
  if [[ -z "$feats" ]]; then
    out=$(timeout 600 cargo test --no-default-features --test ffi_parity 2>&1)
  else
    out=$(timeout 600 cargo test --no-default-features --features "$feats" --test ffi_parity 2>&1)
  fi
  if [[ $? -ne 0 ]]; then
    echo "### FFI FAIL [$label]"
    echo "$out" | grep -vE '^\s*$' | tail -25
    fail=1
  else
    echo "ok  ffi  [$label]  $(echo "$out" | grep -o 'config: .*')"
  fi
}

ALL_FEATS="add,sub,mul,0,1,2,3,4,5,6,7,op_add,op_sub,op_mul,repeat_0,repeat_1,repeat_2,repeat_3,repeat_4,repeat_5,repeat_6,repeat_7"

case "${1:-all}" in
  canonical)
    for op in add sub mul; do for r in 0 1 2 3 4 5 6 7; do run_suite "$op,$r"; done; done ;;
  alias)
    for op in op_add op_sub op_mul; do
      for r in repeat_0 repeat_1 repeat_2 repeat_3 repeat_4 repeat_5 repeat_6 repeat_7; do
        run_suite "$op,$r"
      done
    done ;;
  edge)
    run_suite ""
    for op in add sub mul op_add op_sub op_mul; do run_suite "$op"; done
    for r in 0 1 2 3 4 5 6 7; do run_suite "$r"; done
    run_suite "$ALL_FEATS"
    run_suite "mul,sub,add,7,3"
    run_suite "op_mul,3,repeat_5" ;;
  cli)
    # Executable-level parity: same stdout/stderr/exit status as the C driver.
    INPUTS=("7 3" "0 0" "-5 9" "3 -4" "1 1" "2147483647 2" "-2147483648 -1" \
            "  -12abc +9" "99999999999999999999 3" "12x 7" "+0 -0" "0x10 5" \
            "   42    -7" "abc def" "-99999999999999999999 -3" "2147483648 1")
    for op in add sub mul; do for r in 0 1 2 3 4 5 6 7; do
      timeout 600 cargo build --release --no-default-features --features "$op,$r" >/dev/null 2>&1 || {
        echo "### BUILD FAIL [$op,$r]"; fail=1; continue; }
      cref="/tmp/cref/${op}_${r}/driver"; rbin="target/release/driver"
      for pair in "${INPUTS[@]}"; do
        cout=$("$cref" $pair 2>/dev/null); cst=$?
        rout=$("$rbin" $pair 2>/dev/null); rst=$?
        if [[ "$cout" != "$rout" || "$cst" != "$rst" ]]; then
          echo "### CLI MISMATCH $op/$r args[$pair] (exit c=$cst r=$rst)"
          diff <(printf '%s\n' "$cout") <(printf '%s\n' "$rout") | sed 's/^/    /'
          fail=1
        fi
      done
      cerr=$("$cref" 2>&1 >/dev/null); cst=$?
      rerr=$("$rbin" 2>&1 >/dev/null); rst=$?
      cerr=${cerr/$cref/PROG}; rerr=${rerr/$rbin/PROG}
      [[ "$cerr" == "$rerr" && "$cst" == "$rst" ]] || {
        echo "### CLI USAGE MISMATCH $op/$r c=[$cerr:$cst] r=[$rerr:$rst]"; fail=1; }
      cerr=$("$cref" 5 2>&1 >/dev/null); cst=$?
      rerr=$("$rbin" 5 2>&1 >/dev/null); rst=$?
      cerr=${cerr/$cref/PROG}; rerr=${rerr/$rbin/PROG}
      [[ "$cerr" == "$rerr" && "$cst" == "$rst" ]] || {
        echo "### CLI 1-ARG MISMATCH $op/$r c=[$cerr:$cst] r=[$rerr:$rst]"; fail=1; }
      echo "ok  cli  [$op,$r]"
    done; done ;;
  all)
    bash "$ROOT/run_all.sh" canonical; s1=$?
    bash "$ROOT/run_all.sh" alias;     s2=$?
    bash "$ROOT/run_all.sh" edge;      s3=$?
    bash "$ROOT/run_all.sh" cli;       s4=$?
    fail=$(( s1 | s2 | s3 | s4 )) ;;
esac

[[ $fail -eq 0 ]] && echo "=== PASS (${1:-all}) ==="
exit $fail
