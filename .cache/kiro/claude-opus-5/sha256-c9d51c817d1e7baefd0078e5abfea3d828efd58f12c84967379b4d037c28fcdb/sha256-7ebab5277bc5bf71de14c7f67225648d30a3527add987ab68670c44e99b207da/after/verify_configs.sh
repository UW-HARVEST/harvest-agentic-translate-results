#!/usr/bin/env bash
# Enumerate every valid feature combination declared in translation/Cargo.toml,
# `cargo check` each one, and verify the Rust cdylib exports every dynamic
# symbol the C shared library exports.
set -uo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
cd "$root/translation" || exit 1

# ---- 1. enumerate features -------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml
)

echo "=== features declared: ${#FEATURES[@]} ==="
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "(none -- Cargo.toml has no [features] table; single configuration)"
fi
printf '%s\n' "${FEATURES[@]+"${FEATURES[@]}"}"

# Build the combination list: the empty set plus every subset of FEATURES.
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo
echo "=== cargo check over ${#COMBOS[@]} combination(s) ==="
fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  if timeout 600 cargo check --no-default-features ${combo:+--features "$combo"} \
      --all-targets >"/tmp/check-${combo//,/_}.log" 2>&1; then
    echo "  OK    $label"
  else
    echo "  FAIL  $label  (see /tmp/check-${combo//,/_}.log)"
    tail -20 "/tmp/check-${combo//,/_}.log"
    fail=1
  fi
done

# ---- 2. exported-symbol comparison ----------------------------------------
c_so=$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)
if [ -z "$c_so" ]; then
  echo "!! C shared library not built" >&2
  exit 1
fi

echo
echo "=== exported dynamic symbols ==="
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  timeout 600 cargo build --release --lib --no-default-features \
    ${combo:+--features "$combo"} >"/tmp/build-${combo//,/_}.log" 2>&1 || {
      echo "  FAIL  build $label"; tail -20 "/tmp/build-${combo//,/_}.log"; fail=1; continue; }
  rust_so="target/release/libunfilter_lib.so"

  nm -D --defined-only "$c_so"   | awk '{print $NF}' | sort -u >/tmp/c_syms.txt
  nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u >/tmp/rust_syms.txt

  missing=$(comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt)
  if [ -n "$missing" ]; then
    echo "  FAIL  $label -- Rust .so is missing:"
    printf '        %s\n' $missing
    fail=1
  else
    echo "  OK    $label -- all $(wc -l </tmp/c_syms.txt) C symbols present"
  fi

  # informational: symbols the Rust .so adds (allowed)
  extra=$(comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt)
  if [ -n "$extra" ]; then
    echo "        (extra Rust-only symbols: $(echo $extra | tr '\n' ' '))"
  fi

  # type/binding must match too (T vs D vs B)
  while read -r sym; do
    ct=$(nm -D --defined-only "$c_so"    | awk -v s="$sym" '$NF==s {print $(NF-1)}' | head -1)
    rt=$(nm -D --defined-only "$rust_so" | awk -v s="$sym" '$NF==s {print $(NF-1)}' | head -1)
    if [ "$ct" != "$rt" ]; then
      echo "  WARN  $label -- $sym: C type '$ct' vs Rust type '$rt'"
    fi
  done </tmp/c_syms.txt
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS OK"; else echo "FAILURES PRESENT"; fi
exit "$fail"
