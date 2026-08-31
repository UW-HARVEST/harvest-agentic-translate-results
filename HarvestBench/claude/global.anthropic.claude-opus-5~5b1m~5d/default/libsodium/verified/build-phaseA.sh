#!/bin/sh
# Assembles the per-area Phase-A fragments in phaseA/ into the two top-level
# artifacts ERRORS.md and CONFIGS.md. Re-run after any fragment changes.
set -eu
cd "$(dirname "$0")"

count() { grep -cE '^\| *[0-9]+\.[0-9]+[a-z]? *\|' "$1" || true; }
ticked() { grep -cE '^\| *[0-9]+\.[0-9]+[a-z]? *\|.*\[x\]' "$1" || true; }

{
  echo "# ERRORS.md — error-surface table (Phase A / Phase C)"
  echo
  echo "One row per distinct rejection branch in the C source, derived mechanically"
  echo "by grepping every \`return -1\`, \`return NULL\`, error enum, \`sodium_misuse()\`,"
  echo "\`assert\`, explicit range/null check and min/max constant in \`c_src/libsodium\`."
  echo "The \`status\` column records the Phase-C outcome for that row."
  echo
  echo "Build under test: x86-64 Linux, **no \`HAVE_*\` macros** (see c_src/CMakeLists.txt),"
  echo "so every \`#ifdef HAVE_*\` selects the portable fallback."
  echo
  echo "## Row counts"
  echo
  echo "| area | rows |"
  echo "|------|------|"
  tot=0
  for i in 1 2 3 4 5 6 7 8; do
    n=$(count "phaseA/errors_$i.md")
    tot=$((tot + n))
    echo "| $i | $n |"
  done
  echo "| **total** | **$tot** |"
  echo
  for i in 1 2 3 4 5 6 7 8; do
    echo
    cat "phaseA/errors_$i.md"
  done
} > ERRORS.md

{
  echo "# CONFIGS.md — configuration-surface table (Phase A / Phase B)"
  echo
  echo "One row per meaningful combination of runtime options and input shapes that"
  echo "the C source actually branches on, derived mechanically from the public"
  echo "headers and the \`if\`/\`switch\`/\`#ifdef\` branches in \`c_src/libsodium\`."
  echo "\`[x]\` = differentially verified across randomized inputs;"
  echo "\`[~]\` = contract-verified only (output is OS-dependent and cannot be compared byte for byte)."
  echo
  echo "Build under test: x86-64 Linux, **no \`HAVE_*\` macros**, so every"
  echo "\`#ifdef HAVE_*\` selects the portable fallback."
  echo
  echo "## Row counts"
  echo
  echo "| area | rows | checked |"
  echo "|------|------|---------|"
  tot=0
  tick=0
  for i in 1 2 3 4 5 6 7 8; do
    n=$(count "phaseA/configs_$i.md")
    t=$(ticked "phaseA/configs_$i.md")
    tot=$((tot + n))
    tick=$((tick + t))
    echo "| $i | $n | $t |"
  done
  echo "| **total** | **$tot** | **$tick** |"
  echo
  for i in 1 2 3 4 5 6 7 8; do
    echo
    cat "phaseA/configs_$i.md"
  done
} > CONFIGS.md

echo "wrote ERRORS.md ($(wc -l < ERRORS.md) lines) and CONFIGS.md ($(wc -l < CONFIGS.md) lines)"
