#!/usr/bin/env bash
# Compare the Rust driver against the C reference build for every OP x REPEAT
# configuration. Reference binaries live in /tmp/cref/<op>_<repeat>/driver.
set -u
cd "$(dirname "$0")"

INPUTS=("7 3" "0 0" "-5 9" "3 -4" "1 1" "2147483647 2" "-2147483648 -1" \
        "  -12abc +9" "99999999999999999999 3" "12x 7")

fail=0
for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    cargo build --release --no-default-features --features "$OP,$R" >/dev/null 2>&1 || {
      echo "RUST BUILD FAIL $OP $R"; fail=1; continue; }
    cref="/tmp/cref/${OP}_${R}/driver"
    rbin="target/release/driver"
    for pair in "${INPUTS[@]}"; do
      # shellcheck disable=SC2086
      cout=$("$cref" $pair 2>/dev/null); cst=$?
      # shellcheck disable=SC2086
      rout=$("$rbin" $pair 2>/dev/null); rst=$?
      if [[ "$cout" != "$rout" || "$cst" != "$rst" ]]; then
        echo "MISMATCH $OP/$R args[$pair]"
        diff <(printf '%s\n' "$cout") <(printf '%s\n' "$rout") | sed 's/^/    /'
        echo "    exit: c=$cst rust=$rst"
        fail=1
      fi
    done
    # usage path (argc < 3): compare with argv[0] normalised away
    cerr=$("$cref" 2>&1 >/dev/null); cst=$?
    rerr=$("$rbin" 2>&1 >/dev/null); rst=$?
    cerr=${cerr/$cref/PROG}; rerr=${rerr/$rbin/PROG}
    if [[ "$cerr" != "$rerr" || "$cst" != "$rst" ]]; then
      echo "MISMATCH $OP/$R usage: c=[$cerr:$cst] rust=[$rerr:$rst]"; fail=1
    fi
  done
done

# Alias features and the header's #ifndef fallbacks (OP=add, REPEAT=5).
for spec in "op_sub,repeat_3:sub_3" "op_mul,repeat_6:mul_6" "op_add,repeat_0:add_0"; do
  feats=${spec%%:*}; ref=${spec##*:}
  cargo build --release --no-default-features --features "$feats" >/dev/null 2>&1 || {
    echo "RUST BUILD FAIL $feats"; fail=1; continue; }
  if ! diff <("/tmp/cref/${ref}/driver" 7 3) <(target/release/driver 7 3) >/dev/null; then
    echo "MISMATCH alias $feats vs $ref"; fail=1
  fi
done
cargo build --release >/dev/null 2>&1 || { echo "RUST BUILD FAIL default"; fail=1; }
if ! diff <(/tmp/cref/add_5/driver 7 3) <(target/release/driver 7 3) >/dev/null; then
  echo "MISMATCH default features vs cmake defaults (add/5)"; fail=1
fi

# Every feature enabled at once must still compile (documented precedence).
allf="add,sub,mul,0,1,2,3,4,5,6,7,op_add,op_sub,op_mul,repeat_0,repeat_1,repeat_2,repeat_3,repeat_4,repeat_5,repeat_6,repeat_7"
cargo build --release --features "$allf" >/dev/null 2>&1 || { echo "RUST BUILD FAIL all-features"; fail=1; }
cargo build --release --all-features >/dev/null 2>&1 || { echo "RUST BUILD FAIL --all-features"; fail=1; }

[[ $fail -eq 0 ]] && echo "ALL CONFIGURATIONS MATCH"
exit $fail
