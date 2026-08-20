#!/usr/bin/env bash
# Full differential verification driver.
#
# `Cargo.toml` has no [features] section, so the only valid feature combination
# is the empty one. It is nevertheless verified under BOTH cargo profiles,
# because the `dev` profile turns on integer-overflow checks that the C code
# does not have — a wrapping arithmetic mistake in the translation shows up as a
# panic there even when the release build silently matches.
set -uo pipefail
cd "$(dirname "$0")"

TD=${TMPDIR:-/tmp}/harvest_verify.$$
mkdir -p "$TD"
trap 'rm -rf "$TD"' EXIT
FAIL=0
step() { printf '\n\033[1m=== %s\033[0m\n' "$*"; }

step "C shared library"
( cd c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$PWD/c_src/build/libtranslated_rust.so
ls -l "$C_SO"

# ---------------------------------------------------------------------------
# Phase A / D: feature combinations
# ---------------------------------------------------------------------------
step "feature combinations (cargo check)"
COMBOS=$(python3 - <<'PY'
import re
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            names.append(line.split('=')[0].strip())
names = [n for n in names if n != 'default']
import itertools
combos = []
for k in range(len(names) + 1):
    for c in itertools.combinations(names, k):
        combos.append(','.join(c))
print('\n'.join(combos) if combos else '')
PY
)
echo "feature names found: [$(echo "$COMBOS" | tr '\n' '|')]"
# always at least the empty combination
if [ -z "$COMBOS" ]; then COMBOS=""; fi
while IFS= read -r combo; do
  label=${combo:-"<none>"}
  echo "--- cargo check --no-default-features --features '$combo'"
  if [ -z "$combo" ]; then
    cargo check --no-default-features --all-targets 2>&1 | tail -3
    rc=${PIPESTATUS[0]}
  else
    cargo check --no-default-features --features "$combo" --all-targets 2>&1 | tail -3
    rc=${PIPESTATUS[0]}
  fi
  [ "$rc" = "0" ] || { echo "cargo check FAILED for [$label]"; FAIL=1; }
done <<< "$COMBOS"

# ---------------------------------------------------------------------------
# Phases B, C, D under every profile
# ---------------------------------------------------------------------------
for prof in release debug; do
  step "test suite (profile=$prof, features=<none>)"
  if [ "$prof" = "release" ]; then
    cargo build --release --no-default-features >/dev/null 2>&1
    RUST_SO=$PWD/target/release/libsh_geti_lib.so
    ARGS=(--release)
  else
    cargo build --no-default-features >/dev/null 2>&1
    RUST_SO=$PWD/target/debug/libsh_geti_lib.so
    ARGS=()
  fi
  ls -l "$RUST_SO" || { echo "missing $RUST_SO"; FAIL=1; continue; }

  step "symbol diff (profile=$prof)"
  nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort > "$TD"/c_syms.txt
  nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | sort > "$TD"/r_syms.txt
  echo "C exports:    $(wc -l < "$TD"/c_syms.txt)"
  echo "Rust exports: $(wc -l < "$TD"/r_syms.txt)"
  MISSING=$(comm -23 "$TD"/c_syms.txt "$TD"/r_syms.txt)
  if [ -n "$MISSING" ]; then
    echo "MISSING FROM RUST .so:"; echo "$MISSING"; FAIL=1
  else
    echo "symbol diff is EMPTY (0 missing)"
  fi

  DIFF_C_SO="$C_SO" DIFF_RUST_SO="$RUST_SO" \
    timeout 900 cargo test "${ARGS[@]}" --no-default-features 2>&1 \
    | grep -E "^(test result|running|error|---- )|FAILED|panicked" | sed "s/^/[$prof] /"
  rc=${PIPESTATUS[0]}
  [ "$rc" = "0" ] || { echo "test suite FAILED for profile=$prof"; FAIL=1; }
done

step "RESULT"
if [ "$FAIL" = "0" ]; then echo "ALL VERIFICATION STEPS PASSED"; else echo "VERIFICATION FAILED"; fi
exit $FAIL
