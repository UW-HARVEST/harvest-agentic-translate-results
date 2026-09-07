#!/usr/bin/env bash
# Feature-resolution differential tests.
#
# The CMake cache variables OP/REPEAT always hold exactly one value, but Cargo
# features are a *set*: a build may select none, or several conflicting ones.
# `mdmacros.rs` resolves those to a single (OP, REPEAT) pair:
#
#   OP     = mul if `mul` else sub if `sub` else add          (CMake default: add)
#   REPEAT = the highest enabled of 0..7, else 5              (CMake default: 5)
#
# This script builds the Rust side with each such feature *set* and diffs it
# against the C build for the (OP, REPEAT) the set is supposed to resolve to.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(dirname "$HERE")"
ROOT="$(dirname "$CRATE")"
CSRC="$ROOT/c_src/src"
OUT="$ROOT/cbuild"
RUSTOUT="$ROOT/rbuild"
mkdir -p "$OUT" "$RUSTOUT"

# "<cargo feature set>|<expected OP>|<expected REPEAT>"
CASES=(
  "|add|5"                             # no features at all -> CMake defaults
  "add|add|5"                          # OP only
  "sub|sub|5"
  "mul|mul|5"
  "5|add|5"                            # REPEAT only
  "0|add|0"
  "7|add|7"
  "sub,mul|mul|5"                      # conflicting OP: mul wins
  "add,sub|sub|5"
  "add,mul|mul|5"
  "add,sub,mul|mul|5"
  "0,7|add|7"                          # conflicting REPEAT: highest wins
  "1,3|add|3"
  "0,1,2|add|2"
  "6,7|add|7"
  "add,sub,mul,0,1,2,3,4,5,6,7|mul|7"  # everything on
  "sub,0,6|sub|6"
  "mul,2,4|mul|4"
)

pass=0; fail=0; failed=()

for case in "${CASES[@]}"; do
  IFS='|' read -r feats op rep <<<"$case"
  label="${feats:-<none>}"
  echo "==================== features=[$label] -> OP=$op REPEAT=$rep ===================="

  cfg="${op}_${rep}"
  gcc -O2 -shared -fPIC -DOP="$op" -DREPEAT="$rep" -o "$OUT/libc_$cfg.so" "$CSRC/mdcore.c" || { fail=$((fail+1)); failed+=("$label"); continue; }
  gcc -O2 -DOP="$op" -DREPEAT="$rep" -o "$OUT/cdriver_$cfg" "$CSRC/mdcore.c" "$CSRC/mdmain.c" || { fail=$((fail+1)); failed+=("$label"); continue; }

  if [[ -z "$feats" ]]; then FEATARGS=(); else FEATARGS=(--features "$feats"); fi
  tag="res_$(echo "${feats:-none}" | tr ',' '-')"

  ( cd "$CRATE" && cargo build --offline --release --no-default-features "${FEATARGS[@]}" ) \
      > "$OUT/rustbuild_$tag.log" 2>&1 \
      || { echo "Rust build FAILED"; tail -20 "$OUT/rustbuild_$tag.log"; fail=$((fail+1)); failed+=("$label"); continue; }
  cp "$CRATE/target/release/libdriver.so" "$RUSTOUT/librust_$tag.so"
  cp "$CRATE/target/release/driver"       "$RUSTOUT/rustdriver_$tag"

  ( cd "$CRATE" && \
    DIFF_C_SO="$OUT/libc_$cfg.so" \
    DIFF_RUST_SO="$RUSTOUT/librust_$tag.so" \
    DIFF_C_BIN="$OUT/cdriver_$cfg" \
    DIFF_RUST_BIN="$RUSTOUT/rustdriver_$tag" \
    timeout 600 cargo test --offline --release --no-default-features "${FEATARGS[@]}" 2>&1 ) \
    | tee "$OUT/test_$tag.log" | grep -E "^test result|panicked|FAILED"
  st=${PIPESTATUS[0]}
  grep -qE "FAILED|error\[|^error" "$OUT/test_$tag.log" && st=1
  if [[ $st -eq 0 ]]; then echo "RESULT [$label]: PASS"; pass=$((pass+1));
  else echo "RESULT [$label]: FAIL ($OUT/test_$tag.log)"; fail=$((fail+1)); failed+=("$label"); fi
done

echo
echo "############ RESOLUTION SUMMARY: $pass passed, $fail failed ############"
[[ $fail -gt 0 ]] && { printf 'failed: %s\n' "${failed[*]}"; exit 1; }
exit 0
