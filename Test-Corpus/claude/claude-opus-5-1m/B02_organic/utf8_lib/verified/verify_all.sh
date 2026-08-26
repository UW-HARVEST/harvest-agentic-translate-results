#!/bin/bash
# Phase D driver: enumerate EVERY cargo feature combination, `cargo check` each
# one, verify C-vs-Rust symbol parity, and run the full differential suite
# (Phases B and C) for each combination in both the dev and release profiles.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_OFFLINE=${CARGO_OFFLINE:---offline}
fail=0
note() { printf '%s\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=$((fail+1)); }

# ---------------------------------------------------------------------------
# 0. build the C reference shared library
# ---------------------------------------------------------------------------
note "=== 0. C reference library ==="
if [ ! -f c_src/build/libdriver.so ]; then
  ( mkdir -p c_src/build && cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
fi
ok "c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# 1. enumerate every feature combination declared in Cargo.toml
# ---------------------------------------------------------------------------
note "=== 1. feature combinations ==="
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys

txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
# every subset of the non-default features (the empty set == no features)
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print(','.join(c))
PY
)
note "  declared optional features: $( [ "${#COMBOS[@]}" -eq 1 ] && echo '(none)' || echo yes )"
note "  ${#COMBOS[@]} combination(s) to verify"

# ---------------------------------------------------------------------------
# 2. cargo check every combination
# ---------------------------------------------------------------------------
note "=== 2. cargo check per combination ==="
for combo in "${COMBOS[@]}"; do
  label="--no-default-features --features '${combo}'"
  if cargo check $CARGO_OFFLINE --all-targets --no-default-features --features "$combo" \
       > "${TMPDIR:-/tmp}/check.log" 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"
    tail -30 "${TMPDIR:-/tmp}/check.log"
  fi
done

# ---------------------------------------------------------------------------
# 3. symbol parity, per combination and per profile
# ---------------------------------------------------------------------------
note "=== 3. symbol parity (nm -D) ==="
C_SYMS=$(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort -u)
for combo in "${COMBOS[@]}"; do
  for prof in debug release; do
    args=(--no-default-features --features "$combo")
    [ "$prof" = release ] && args+=(--release)
    if ! cargo build $CARGO_OFFLINE --lib "${args[@]}" > "${TMPDIR:-/tmp}/build.log" 2>&1; then
      bad "cargo build ($prof, '$combo')"; tail -30 "${TMPDIR:-/tmp}/build.log"; continue
    fi
    so="target/$prof/libdriver.so"
    R_SYMS=$(nm -D --defined-only "$so" | awk '{print $NF}' | sort -u)
    missing=$(comm -23 <(printf '%s\n' "$C_SYMS") <(printf '%s\n' "$R_SYMS"))
    if [ -z "$missing" ]; then
      ok "$prof '$combo': all $(printf '%s\n' "$C_SYMS" | wc -l) C symbols exported by Rust"
    else
      bad "$prof '$combo': missing from Rust .so: $(printf '%s' "$missing" | tr '\n' ' ')"
    fi
    # every unresolved symbol must be libc / libgcc, i.e. resolvable
    if ldd -r "$so" 2>&1 | grep -q "undefined symbol"; then
      bad "$prof '$combo': unresolved non-libc symbols"
      ldd -r "$so" 2>&1 | grep "undefined symbol" | head
    else
      ok "$prof '$combo': no unresolved symbols"
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. differential suite (Phase B + Phase C) per combination and profile
# ---------------------------------------------------------------------------
note "=== 4. differential tests (Phase B + C) ==="
for combo in "${COMBOS[@]}"; do
  for prof in debug release; do
    args=(--no-default-features --features "$combo")
    [ "$prof" = release ] && args+=(--release)
    if timeout 600 cargo test $CARGO_OFFLINE "${args[@]}" \
         > "${TMPDIR:-/tmp}/test-$prof.log" 2>&1; then
      ok "$prof '$combo': $(grep -hoE '[0-9]+ passed' "${TMPDIR:-/tmp}/test-$prof.log" | paste -sd'+' | sed 's/ passed//g') tests passed"
    else
      bad "$prof '$combo': differential tests failed"
      grep -E "FAILED|panicked at|assertion|error" "${TMPDIR:-/tmp}/test-$prof.log" | head -20
    fi
  done
done

note ""
if [ "$fail" -eq 0 ]; then
  note "=== ALL CHECKS PASSED ==="
else
  note "=== $fail CHECK(S) FAILED ==="
fi
exit "$fail"
