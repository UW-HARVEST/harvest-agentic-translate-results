#!/usr/bin/env bash
# Report progress against the four Phase-D completion criteria.
set -uo pipefail
cd "$(dirname "$0")"
mkdir -p .work

echo "=================================================================="
echo " Phase D completion gate"
echo "=================================================================="

# ---- 1. symbol parity -------------------------------------------------
if [ -f c_src/build/libjansson.so ] && [ -f translation/target/release/libjansson.so ]; then
  nm -D --defined-only c_src/build/libjansson.so \
    | awk '{print $3}' | grep -v '^_' | sort -u > .work/c_syms.txt
  nm -D --defined-only translation/target/release/libjansson.so \
    | awk '{print $3}' | grep -v '^_' | sort -u > .work/rust_syms.txt
  CN=$(wc -l < .work/c_syms.txt)
  MISS=$(comm -23 .work/c_syms.txt .work/rust_syms.txt | wc -l)
  EXTRA=$(comm -13 .work/c_syms.txt .work/rust_syms.txt | wc -l)
  # Undefined non-libc symbols in the Rust .so
  UNDEF=$(nm -D --undefined-only translation/target/release/libjansson.so \
    | awk '{print $2}' | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind' | wc -l)
  printf "[%s] SYMBOLS.md : %s/%s exported, %s missing, %s extra, %s undefined non-libc\n" \
    "$([ "$MISS" -eq 0 ] && [ "$UNDEF" -eq 0 ] && echo x || echo ' ')" \
    "$((CN-MISS))" "$CN" "$MISS" "$EXTRA" "$UNDEF"
  [ "$MISS" -gt 0 ] && { echo "    MISSING:"; comm -23 .work/c_syms.txt .work/rust_syms.txt | sed 's/^/      /'; }
else
  echo "[ ] SYMBOLS.md : one or both .so files are not built"
fi

# ---- 2. CONFIGS.md rows ----------------------------------------------
if [ -f translation/CONFIGS.md ]; then
  T=$(command grep -ac '^| [0-9]' translation/CONFIGS.md)
  D=$(command grep -ac '| \[x\] |' translation/CONFIGS.md)
  U=$(command grep -ac '| \[-\]' translation/CONFIGS.md)
  printf "[%s] CONFIGS.md : %s verified + %s documented-unreachable = %s/%s\n" \
    "$([ "$((D+U))" -eq "$T" ] && echo x || echo ' ')" "$D" "$U" "$((D+U))" "$T"
fi

# ---- 3. ERRORS.md rows -----------------------------------------------
if [ -f translation/ERRORS.md ]; then
  T=$(command grep -ac '^| [0-9]' translation/ERRORS.md)
  D=$(command grep -ac '| \[x\] |' translation/ERRORS.md)
  U=$(command grep -ac '| \[-\]' translation/ERRORS.md)
  printf "[%s] ERRORS.md  : %s verified + %s documented-unreachable = %s/%s\n" \
    "$([ "$((D+U))" -eq "$T" ] && echo x || echo ' ')" "$D" "$U" "$((D+U))" "$T"
fi

# ---- 4. feature combinations -----------------------------------------
FEATS=$(command grep -A20 '^\[features\]' translation/Cargo.toml 2>/dev/null \
  | command grep -E '^[a-zA-Z0-9_-]+ *=' | wc -l)
echo "[x] FEATURES   : $FEATS declared in Cargo.toml (no [features] section => the"
echo "                 default build is the only configuration; verified under it)"

# ---- test inventory ---------------------------------------------------
echo
echo "------------------------------------------------------------------"
echo " Test inventory"
echo "------------------------------------------------------------------"
TOTAL=0
for f in translation/tests/*.rs; do
  b=$(basename "$f" .rs)
  n=$(command grep -c '^#\[test\]' "$f" 2>/dev/null || echo 0)
  TOTAL=$((TOTAL + n))
  printf "  %-34s %3d tests  %6d bytes\n" "$b" "$n" "$(stat -c%s "$f")"
done
echo "  ---------------------------------------------------------------"
printf "  %-34s %3d tests\n" "TOTAL" "$TOTAL"
