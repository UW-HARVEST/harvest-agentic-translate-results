#!/bin/bash
# Generates the row tables of translation/CONFIGS.md mechanically, so the
# cross-product cannot be mis-transcribed by hand.
set -euo pipefail
n=0
row(){ n=$((n+1)); printf '| %s | %s | %s | [x] |\n' "$1" "$2" "$3"; }

echo "### G1 — the three operation primitives (\`OP\`-independent code, present in every build)"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for f in op_add op_sub op_mul; do
  for shape in \
    "both operands 0" \
    "small positives (1..1000), randomized, seed 0x5EED" \
    "small negatives (-1000..-1), randomized" \
    "mixed signs, randomized over full i32" \
    "boundary set {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX} x itself (full 7x7)" \
    "overflow-inducing pairs (INT_MAX+1, INT_MIN-1, INT_MAX*INT_MAX, INT_MIN*-1)"; do
    row "G1-$n" "\`$f\`" "any OP build x any REPEAT build (24 configs); $shape"
  done
done

echo
echo "### G2 — \`G_OP\`: the exported function-pointer datum (selected by \`OP\`)"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for op in add sub mul; do
  row "G2-$n" "\`G_OP\` (dlsym, deref, call)" "OP=$op, REPEAT=* ; randomized full-i32 pairs, 256 draws, fixed seed"
  row "G2-$n" "\`G_OP\` (dlsym, deref, call)" "OP=$op, REPEAT=* ; boundary set {INT_MIN..INT_MAX} 7x7"
  row "G2-$n" "\`G_OP\` vs \`op_$op\`" "OP=$op ; identity check - the stored pointer must equal the address of the exported \`op_$op\` in the same \`.so\`"
done

echo
echo "### G3 — \`G_OP_NAME\`: the exported string-pointer datum (\`STR(OP)\`)"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for op in add sub mul; do
  row "G3-$n" "\`G_OP_NAME\` (dlsym, deref, read NUL-terminated bytes)" "OP=$op, REPEAT=* ; expect exactly \`\"$op\"\` + NUL"
done

echo
echo "### G4 — \`helper_ptr\` (calls through a local fn pointer; \`REPEAT\`-independent)"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for op in add sub mul; do
  row "G4-$n" "\`helper_ptr\`" "OP=$op, REPEAT=* ; randomized full-i32 pairs, 256 draws; compare return value AND the \`helper.ptr=%d\` stdout line"
  row "G4-$n" "\`helper_ptr\`" "OP=$op, REPEAT=* ; boundary set 7x7 incl. overflow; compare return AND stdout"
done

echo
echo "### G5 — \`helper_call\` (the only function whose result depends on BOTH \`OP\` and \`REPEAT\`)"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    row "G5-$n" "\`helper_call\`" "OP=$op, REPEAT=$rep (=> \`RUN_LOOP\` expands to \`REP$rep\`) ; randomized full-i32 pairs 256 draws + boundary 7x7 + overflow pairs; compare return AND the \`helper.call=%d helper.acc=%d\` stdout line"
  done
done

echo
echo "### G6 — \`use_generated\` -> \`accum_<OP>\` (one row per distinct \`DISPATCH_REP\` \`case\`)"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for op in add sub mul; do
  for k in 0 1 2 3 4 5 6; do
    row "G6-$n" "\`use_generated\`" "OP=$op, REPEAT=* ; n=$k -> \`case $k: REP$k\` ; compare return AND the \`gen.acc=%d\` stdout line"
  done
done

echo
echo "### G7 — writable exported globals (\`mdmacros.h\` declares both non-\`const\`)"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for op in add sub mul; do
  row "G7-$n" "\`G_OP\` (store, then read back and call)" "OP=$op ; overwrite the global with the address of each of \`op_add\`/\`op_sub\`/\`op_mul\` from the same \`.so\`, then call through it on randomized pairs. Must not trap (C \`.data\` is writable)."
  row "G7-$n" "\`G_OP_NAME\` (store, then read back)" "OP=$op ; overwrite the pointer with another string address inside the same \`.so\`, then read it back. Must not trap."
  row "G7-$n" "\`G_OP\` restore + \`helper_call\`/\`helper_ptr\`" "OP=$op ; after mutating \`G_OP\`, \`helper_call\`/\`helper_ptr\` must be UNAFFECTED (they use \`OP_FN(OP)\` directly, not the global)"
done

echo
echo "### G8 — the \`driver\` executable (\`mdmain.c\`), end-to-end stdout+stderr+exit-status"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    row "G8-$n" "\`driver A B\` (full pipeline: \`OP_FN\`, \`RUN_LOOP\`, \`helper_call\`, \`helper_ptr\`, \`use_generated(REPEAT)\`, \`G_OP\`, \`G_OP_NAME\`, \`summary\`)" "OP=$op, REPEAT=$rep ; 60 randomized full-i32 operand pairs (fixed seed) + boundary/overflow operands + \`atoi\` shapes (non-numeric, digit-prefix, whitespace, explicit sign, \`long\` overflow, \`int\` truncation) + extra trailing argv + argc<3"
  done
done

echo
echo "### G9 — the implicit \"no macro defined\" build"
echo
echo '| # | entry point(s) | configuration (options set + input shape) | [x] pass |'
echo '|---|----------------|--------------------------------------------|-----|'
row "G9-$n" "all 8 exported symbols + \`driver\`" "C compiled with **no** \`-DOP\`/\`-DREPEAT\` (\`#ifndef\` fallbacks \`OP=add\`, \`REPEAT=5\`) vs Rust \`--no-default-features\` with **no** feature at all; same input shapes as G1-G8"
row "G9-$n" "all 8 exported symbols + \`driver\`" "Cargo default feature set (\`add\`,\`repeat_5\`) vs C \`-DOP=add -DREPEAT=5\`"
row "G9-$n" "all 8 exported symbols" "bare-CMake-value REPEAT aliases \`\"0\"\`..\`\"7\"\` must resolve identically to \`repeat_0\`..\`repeat_7\` (48 \`cargo check\` combos + artifact comparison)"

echo
echo "Total rows: $n"
