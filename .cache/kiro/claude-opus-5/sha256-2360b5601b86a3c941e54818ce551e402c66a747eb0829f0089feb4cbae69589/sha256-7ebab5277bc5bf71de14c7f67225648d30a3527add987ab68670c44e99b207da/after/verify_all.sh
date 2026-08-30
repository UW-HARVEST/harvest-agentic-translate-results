#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration: each cargo feature combination, in both debug and release.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
RS="$ROOT/translation"
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations declared in Cargo.toml.
#    CMakeLists.txt defines no compile-time options (no add_definitions /
#    target_compile_definitions / option()), so the C side has a single
#    configuration and every Rust combination is compared against it.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - "$RS/Cargo.toml" <<'PY'
import re, sys
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if '=' in line:
            n = line.split('=', 1)[0].strip().strip('"')
            if n != 'default':
                names.append(n)
print('\n'.join(names))
PY
)
# Drop empty entries produced when there are no features at all.
CLEAN=()
for f in "${FEATURES[@]+"${FEATURES[@]}"}"; do [[ -n "$f" ]] && CLEAN+=("$f"); done
FEATURES=("${CLEAN[@]+"${CLEAN[@]}"}")

COMBOS=("")   # always test the empty (--no-default-features) combination
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[i]}"
    done
    COMBOS+=("$combo")
  done
fi

step "feature combinations"
if (( n == 0 )); then
  echo "Cargo.toml declares no [features]; single configuration only."
else
  printf '  %s\n' "${FEATURES[@]}"
fi
for c in "${COMBOS[@]}"; do echo "  combo: '${c:-<none>}'"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared library.
# ---------------------------------------------------------------------------
step "building C shared library"
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
echo "ok: $C_SO"

c_syms() { nm -D --defined-only "$1" | awk '$2 ~ /^[TtDdBbWwRr]$/ {print $3}' | sort -u; }

# ---------------------------------------------------------------------------
# 3. For every combination: check, build, symbol-compare, test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    label="features='${combo:-<none>}' profile=$profile"
    step "$label"

    relflag=""; [[ $profile == release ]] && relflag="--release"
    featflag=(--no-default-features)
    [[ -n "$combo" ]] && featflag+=(--features "$combo")

    if ! ( cd "$RS" && timeout 600 cargo check "${featflag[@]}" $relflag 2>&1 | tail -3 ); then
      echo "CHECK FAILED: $label"; FAILED=1; continue
    fi
    if ! ( cd "$RS" && timeout 600 cargo build "${featflag[@]}" $relflag 2>&1 | tail -3 ); then
      echo "BUILD FAILED: $label"; FAILED=1; continue
    fi

    RS_SO="$RS/target/$profile/libdriver.so"
    echo "-- symbol parity (nm -D) --"
    missing="$(comm -23 <(c_syms "$C_SO") <(c_syms "$RS_SO"))"
    if [[ -n "$missing" ]]; then
      echo "MISSING EXPORTS in Rust .so:"; echo "$missing"; FAILED=1
    else
      echo "ok: every C export is present in the Rust .so"
      c_syms "$C_SO" | sed 's/^/   /'
    fi

    echo "-- differential tests --"
    if ! ( cd "$RS" && timeout 600 cargo test "${featflag[@]}" $relflag 2>&1 \
             | grep -E '^(test |running |test result|error)' ); then
      echo "TESTS FAILED: $label"; FAILED=1
    fi
  done
done

step "summary"
if (( FAILED )); then echo "FAILURES PRESENT"; exit 1; fi
echo "ALL CONFIGURATIONS MATCH THE C GROUND TRUTH"
