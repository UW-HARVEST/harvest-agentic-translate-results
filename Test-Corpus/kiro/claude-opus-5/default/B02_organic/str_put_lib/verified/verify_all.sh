#!/usr/bin/env bash
# Verify the Rust translation against the C reference across every build
# configuration that exists for this project.
#
#   * Cargo.toml declares no [features], so the only feature combination is the
#     default one (also checked with --no-default-features / --all-features).
#   * CMakeLists.txt exposes no options, so the C side varies only by optimiser
#     level, which is swept here to make sure the comparison does not depend on
#     a particular codegen choice.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { echo "FAILED: $*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "cargo check: every feature combination"
# ---------------------------------------------------------------------------
FEATURES=$(cd "$CRATE" && python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip()
        if k and k != 'default':
            names.append(k)
print(' '.join(names))
PY
)
echo "declared features: [${FEATURES:-none}]"

combos=("")
for f in $FEATURES; do
  new=()
  for c in "${combos[@]}"; do
    new+=("$c")
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  combos=("${new[@]}")
done

for c in "${combos[@]}"; do
  echo "--- cargo check --no-default-features --features '$c'"
  (cd "$CRATE" && timeout 600 cargo check --no-default-features --features "$c" 2>&1 | tail -3) \
    || fail "cargo check --no-default-features --features '$c'"
done
echo "--- cargo check (default features)"
(cd "$CRATE" && timeout 600 cargo check 2>&1 | tail -3) || fail "cargo check (default)"
echo "--- cargo check --all-features"
(cd "$CRATE" && timeout 600 cargo check --all-features 2>&1 | tail -3) || fail "cargo check --all-features"

# ---------------------------------------------------------------------------
step "build Rust cdylib: debug and release"
# ---------------------------------------------------------------------------
(cd "$CRATE" && timeout 600 cargo build 2>&1 | tail -2) || fail "cargo build (debug)"
(cd "$CRATE" && timeout 600 cargo build --release 2>&1 | tail -2) || fail "cargo build --release"

# ---------------------------------------------------------------------------
step "build C shared library at several optimiser levels"
# ---------------------------------------------------------------------------
declare -A C_SO
# The in-tree default build, exactly as documented.
(cd "$ROOT/c_src" && mkdir -p build && cd build \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null) || fail "default C build"
C_SO[default]="$(ls "$ROOT"/c_src/build/*.so | head -1)"

for opt in O0 O2 O3; do
  d="/tmp/c_build_$opt"
  rm -rf "$d"
  (timeout 600 cmake -S "$ROOT/c_src" -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      -DCMAKE_C_FLAGS="-$opt" >/dev/null \
    && timeout 600 cmake --build "$d" >/dev/null) || fail "C build -$opt"
  C_SO[$opt]="$(ls "$d"/*.so | head -1)"
done

# ---------------------------------------------------------------------------
step "symbol parity"
# ---------------------------------------------------------------------------
for key in "${!C_SO[@]}"; do
  nm -D --defined-only "${C_SO[$key]}" | awk '{print $NF}' | sort -u > /tmp/csyms.txt
  for prof in debug release; do
    nm -D --defined-only "$CRATE/target/$prof/libstr_put_lib.so" | awk '{print $NF}' | sort -u > /tmp/rsyms.txt
    missing=$(comm -23 /tmp/csyms.txt /tmp/rsyms.txt | grep -v '^_' | grep -v '^__' || true)
    if [ -n "$missing" ]; then
      fail "C[$key] symbols missing from Rust[$prof]: $missing"
    else
      echo "ok: C[$key] -> Rust[$prof]"
    fi
  done
done

# ---------------------------------------------------------------------------
step "cross-comparison test matrix"
# ---------------------------------------------------------------------------
for ckey in default O0 O2 O3; do
  for prof in debug release; do
    echo "--- C=$ckey  Rust=$prof"
    (cd "$CRATE" && C_SO="${C_SO[$ckey]}" \
        RUST_SO="$CRATE/target/$prof/libstr_put_lib.so" \
        timeout 600 cargo test --release 2>&1 | grep -E "^(test result|error|thread|failures:)" | sort -u) \
      || fail "tests with C=$ckey Rust=$prof"
    (cd "$CRATE" && C_SO="${C_SO[$ckey]}" \
        RUST_SO="$CRATE/target/$prof/libstr_put_lib.so" \
        timeout 600 cargo test --release 2>&1 | grep -qE "FAILED|panicked|error:") \
      && fail "tests with C=$ckey Rust=$prof reported failures"
  done
done

printf '\n'
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS MATCH"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAIL"
