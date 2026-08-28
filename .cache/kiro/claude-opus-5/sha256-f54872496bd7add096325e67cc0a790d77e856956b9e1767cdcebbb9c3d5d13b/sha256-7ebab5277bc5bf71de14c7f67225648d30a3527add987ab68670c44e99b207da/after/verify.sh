#!/usr/bin/env bash
# Differential verification of translation/ against c_src/.
#
#   1. enumerate every feature combination declared in translation/Cargo.toml
#   2. cargo check each combination
#   3. build the C reference .so (default configuration, plus -O0/-O2/-O3)
#   4. build the Rust cdylib and run the libloading-based comparison tests
#   5. diff the exported dynamic symbol sets
#
# Nothing under c_src/ is modified; the extra optimisation levels are built
# out-of-tree under /tmp.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"
TIMEOUT=600
FAILED=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 1. Feature combinations
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
mapfile -t COMBOS < <(python3 - "$CRATE/Cargo.toml" <<'PY'
import itertools, sys, re
text = open(sys.argv[1]).read()
# Crude [features] section scrape - no external deps needed.
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
if not feats:
    print('')  # single, featureless configuration
else:
    for n in range(len(feats) + 1):
        for c in itertools.combinations(feats, n):
            print(','.join(c))
PY
)
if [ "${#COMBOS[@]}" -eq 1 ] && [ -z "${COMBOS[0]}" ]; then
  echo "translation/Cargo.toml declares no [features]; one configuration to verify."
else
  printf 'combination: %s\n' "${COMBOS[@]}"
fi

feature_args() {
  if [ -z "$1" ]; then
    echo "--no-default-features"
  else
    echo "--no-default-features --features $1"
  fi
}

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
step "cargo check (all combinations)"
for combo in "${COMBOS[@]}"; do
  args=$(feature_args "$combo")
  label="${combo:-<none>}"
  if timeout $TIMEOUT cargo check --manifest-path "$CRATE/Cargo.toml" $args \
        > /tmp/check_"${combo//,/_}".log 2>&1; then
    echo "ok    cargo check $label"
  else
    fail "cargo check $label"; tail -20 /tmp/check_"${combo//,/_}".log
  fi
done

# ---------------------------------------------------------------------------
# 3. C reference builds
# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$CBUILD"
if cmake -S "$ROOT/c_src" -B "$CBUILD" -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /tmp/cmake.log 2>&1 \
   && cmake --build "$CBUILD" >> /tmp/cmake.log 2>&1; then
  C_DEFAULT="$(ls "$CBUILD"/*.so | head -1)"
  echo "ok    default -> $C_DEFAULT"
else
  fail "C default build"; tail -20 /tmp/cmake.log; exit 1
fi

C_SOS=("$C_DEFAULT")
for opt in "-O0" "-O2" "-O3"; do
  d="/tmp/c_opt${opt}"
  rm -rf "$d"
  if cmake -S "$ROOT/c_src" -B "$d" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$opt" > /tmp/cmake"$opt".log 2>&1 \
     && cmake --build "$d" >> /tmp/cmake"$opt".log 2>&1; then
    so="$(ls "$d"/*.so | head -1)"
    C_SOS+=("$so")
    echo "ok    $opt -> $so"
  else
    fail "C build $opt"
  fi
done

# ---------------------------------------------------------------------------
# 4/5. Build + test + symbol diff, for every combination and both profiles
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  args=$(feature_args "$combo")
  label="${combo:-<none>}"
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"

    step "Build cdylib [features=$label, $profile]"
    if timeout $TIMEOUT cargo build --manifest-path "$CRATE/Cargo.toml" $args $relflag \
          > /tmp/build.log 2>&1; then
      echo "ok    cargo build"
    else
      fail "cargo build features=$label $profile"; tail -20 /tmp/build.log; continue
    fi
    RUST_SO="$CRATE/target/$profile/libhelxo_lib.so"

    step "Symbol diff [features=$label, $profile]"
    nm -D --defined-only "$C_DEFAULT" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
    nm -D --defined-only "$RUST_SO"   | awk '{print $3}' | sort -u > /tmp/r_syms.txt
    missing="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
    if [ -z "$missing" ]; then
      echo "ok    Rust .so exports all $(wc -l < /tmp/c_syms.txt) C symbols"
    else
      fail "missing exports (features=$label $profile):"; echo "$missing"
    fi

    for cso in "${C_SOS[@]}"; do
      step "cargo test [features=$label, $profile] vs $(basename "$(dirname "$cso")")"
      if C_SO_PATH="$cso" timeout $TIMEOUT cargo test --manifest-path "$CRATE/Cargo.toml" \
            $args $relflag -- --test-threads=1 > /tmp/test.log 2>&1; then
        grep -E "^test result" /tmp/test.log | sed 's/^/  /'
      else
        fail "cargo test features=$label $profile C=$cso"
        grep -E "MISMATCH|panicked|^test .* FAILED|test result" /tmp/test.log | head -30
      fi
    done
  done
done

step "Summary"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit $FAILED
