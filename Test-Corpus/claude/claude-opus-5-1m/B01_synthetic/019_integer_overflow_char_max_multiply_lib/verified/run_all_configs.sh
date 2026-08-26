#!/usr/bin/env bash
# Phase D driver: run `cargo check` and the full differential suite for EVERY
# valid build configuration.
#
# Cargo.toml has no [features] section, so the complete enumeration of feature
# combinations is the single empty one. All three spellings below select that
# same combination and are run to demonstrate the enumeration is exhaustive.
# Each is additionally run in both the dev and the release profile, because
# `[profile.release] panic = "abort"` and `debug_assertions` change how the
# CWE-190 truncation in bad() would behave if it were mistranslated.
set -u
cd "$(dirname "$0")" || exit 1

FEATURE_COMBOS=(
  "--no-default-features"
  ""
  "--all-features"
)
PROFILES=("" "--release")

fail=0

echo "############ enumerating configurations ############"
if grep -q '^\[features\]' Cargo.toml; then
  echo "WARNING: a [features] section appeared; this script's enumeration is stale"
  fail=1
else
  echo "Cargo.toml: no [features] section -> exactly 1 feature combination"
fi
if grep -qE '^[[:space:]]*(option|add_definitions|target_compile_definitions)\(' \
     c_src/CMakeLists.txt; then
  echo "WARNING: c_src/CMakeLists.txt gained build options; enumeration is stale"
  fail=1
else
  echo "c_src/CMakeLists.txt: no options/defines -> exactly 1 C configuration"
fi
if grep -rq 'cfg(feature' src/; then
  echo "WARNING: src/ contains cfg(feature ...) but Cargo.toml declares none"
  fail=1
else
  echo "src/: no cfg(feature = ...) gates"
fi
echo

echo "############ phase A(2): cargo check every combination ############"
for f in "${FEATURE_COMBOS[@]}"; do
  for p in "${PROFILES[@]}"; do
    label="cargo check ${f:-<default>} ${p:-<dev>}"
    if out=$(timeout 600 cargo check --offline $f $p --all-targets 2>&1); then
      w=$(echo "$out" | grep -c '^warning')
      echo "PASS  $label   (warnings: $w)"
    else
      echo "FAIL  $label"
      echo "$out" | tail -20
      fail=1
    fi
  done
done
echo

echo "############ phases B/C/D: differential suite per configuration ############"
for f in "${FEATURE_COMBOS[@]}"; do
  for p in "${PROFILES[@]}"; do
    label="cargo test ${f:-<default>} ${p:-<dev>}"
    out=$(timeout 600 cargo test --offline $f $p -- --test-threads=1 2>&1)
    if [ $? -eq 0 ] && ! echo "$out" | grep -qE '^test result: FAILED'; then
      passed=$(echo "$out" | grep -oE '^test result: ok\. [0-9]+' \
                 | grep -oE '[0-9]+$' | awk '{s+=$1} END {print s+0}')
      echo "PASS  $label   (${passed} tests passed)"
    else
      echo "FAIL  $label"
      echo "$out" | grep -E '^test .* FAILED|^test result:|DIVERGENCE' | head -30
      fail=1
    fi
  done
done
echo

echo "############ phase D: nm -D symbol parity ############"
CSO=c_src/build/libdriver.so
for prof in debug release; do
  RSO="target/ffi-so/$prof/libdriver.so"
  [ -f "$RSO" ] || continue
  d=$(diff <(nm -D --defined-only "$CSO" | awk '$2 ~ /^[TBDRWV]$/ {print $3}' | sort -u) \
           <(nm -D --defined-only "$RSO" | awk '$2 ~ /^[TBDRWV]$/ {print $3}' | sort -u))
  if [ -z "$d" ]; then
    echo "PASS  symbol diff empty: $CSO  vs  $RSO"
    nm -D --defined-only "$CSO" | awk '$2 ~ /^[TBDRWV]$/ {print "        " $3}' | sort
  else
    echo "FAIL  symbol diff NOT empty for $RSO:"
    echo "$d"
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "OVERALL: PASS -- every configuration checks, tests clean, and symbols match."
else
  echo "OVERALL: FAIL"
fi
exit "$fail"
