#!/usr/bin/env bash
# Phase D driver: derive every feature combination from Cargo.toml mechanically,
# then run cargo check + the full differential test suite under each one, and
# finish with the C-vs-Rust `nm -D` symbol diff.
set -uo pipefail

cd "$(dirname "$0")"
WORK="$(cd .. && pwd)"
FAIL=0

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml (no guessing).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys
src = open("Cargo.toml").read()
# Grab the [features] table, if any.
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', src, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
for f in feats:
    print(f)
PY
)

echo "== declared non-default features: ${#FEATURES[@]} =="
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "   (none — Cargo.toml has no [features] table, so the only build"
  echo "    configuration is the default one)"
fi

# Combination list: default, then --no-default-features, then the powerset of
# any declared features.
COMBOS=("DEFAULT" "NODEFAULT")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FEATURES[b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "== combinations to verify: ${#COMBOS[@]} =="
printf '   %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. Confirm neither build produces a binary/driver executable.
# ---------------------------------------------------------------------------
echo
echo "== binary/driver check =="
if grep -qE '^\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ] || [ -d src/bin ]; then
  echo "   FAIL: Rust crate declares a binary target"
  FAIL=1
else
  echo "   Rust: no [[bin]], no src/main.rs, no src/bin/ -> library only"
fi
if grep -qE 'add_executable' "$WORK/c_src/CMakeLists.txt"; then
  echo "   FAIL: CMakeLists.txt declares an executable"
  FAIL=1
else
  echo "   C: CMakeLists.txt has only add_library(... SHARED ...) -> library only"
fi
echo "   => no stdout comparison applies to this project"

# ---------------------------------------------------------------------------
# 3. cargo check + full test suite under every combination.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) args=() ;;
    NODEFAULT) args=(--no-default-features) ;;
    *) args=(--no-default-features --features "$combo") ;;
  esac
  echo
  echo "== combo: $combo =="
  if ! timeout 300 cargo check --release "${args[@]}" >/tmp/check.$$ 2>&1; then
    echo "   cargo check FAILED"; tail -20 /tmp/check.$$; FAIL=1; continue
  fi
  echo "   cargo check ok"
  # Rebuild the cdylib under this combo so the tests load the right .so.
  if ! timeout 300 cargo build --release "${args[@]}" >/tmp/build.$$ 2>&1; then
    echo "   cargo build FAILED"; tail -20 /tmp/build.$$; FAIL=1; continue
  fi
  if ! timeout 600 cargo test --release "${args[@]}" >/tmp/test.$$ 2>&1; then
    echo "   cargo test FAILED"; grep -E "^(test |---- |thread )" /tmp/test.$$ | head -40; FAIL=1; continue
  fi
  grep -E "^test result:" /tmp/test.$$ | sed 's/^/   /'

  # Symbol diff under this combo.
  C_SO=$(ls "$WORK"/c_src/build/lib*.so | head -1)
  R_SO=target/release/libinreftree_lib.so
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u >/tmp/c.syms.$$
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u >/tmp/r.syms.$$
  missing=$(comm -23 /tmp/c.syms.$$ /tmp/r.syms.$$)
  if [ -n "$missing" ]; then
    echo "   SYMBOL DIFF FAILED — missing from Rust .so:"; echo "$missing" | sed 's/^/     /'; FAIL=1
  else
    echo "   symbol diff: empty ($(wc -l </tmp/c.syms.$$) C symbols, all exported by Rust)"
  fi
  # Undefined non-libc symbols in the Rust .so.
  undef=$(nm -D -u "$R_SO" | awk '{print $2}' | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_|__tls_get_addr|__errno_location)' \
    | grep -vE '@GLIBC|@GCC|^$' | sort -u)
  if [ -n "$undef" ]; then
    echo "   UNDEFINED non-libc symbols in Rust .so:"; echo "$undef" | sed 's/^/     /'; FAIL=1
  else
    echo "   undefined non-libc symbols: 0"
  fi
done

rm -f /tmp/check.$$ /tmp/build.$$ /tmp/test.$$ /tmp/c.syms.$$ /tmp/r.syms.$$
echo
if [ "$FAIL" -eq 0 ]; then echo "PHASE D: PASS"; else echo "PHASE D: FAIL"; fi
exit "$FAIL"
