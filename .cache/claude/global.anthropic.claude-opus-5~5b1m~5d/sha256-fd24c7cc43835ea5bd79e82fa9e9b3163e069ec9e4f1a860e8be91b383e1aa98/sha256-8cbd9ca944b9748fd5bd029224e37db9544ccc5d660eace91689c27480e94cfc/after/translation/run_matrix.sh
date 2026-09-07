#!/usr/bin/env bash
# Runs the full differential test matrix.
#
# `translation/Cargo.toml` declares no [features], so the feature axis has a
# single point; the axis that *does* matter for the shipped artifact is the
# cargo profile, because [profile.release] sets panic = "abort". Both profiles
# are therefore exercised, each against the .so built with that same profile.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
ROOT="$(cd .. && pwd)"

fail=0
log() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library.
# ---------------------------------------------------------------------------
log "building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so)
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate cargo feature combinations (there are none; assert that).
# ---------------------------------------------------------------------------
log "feature combinations"
FEATURES=$(python3 - <<'PY'
import re
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
if not m:
    print('')  # no [features] table -> only the default configuration
else:
    names = re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)
    print(' '.join(n for n in names if n != 'default'))
PY
)
if [ -z "$FEATURES" ]; then
  echo "no [features] table -> single configuration (default == no-default-features)"
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "features: $FEATURES"
  COMBOS=("" "--no-default-features" "--all-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
fi

# ---------------------------------------------------------------------------
# 3. cargo check every combination, then build + test in both profiles.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  log "cargo check $combo"
  # shellcheck disable=SC2086
  cargo check $combo 2>&1 | tail -3 || fail=1
done

for profile in dev release; do
  if [ "$profile" = release ]; then PF=--release; else PF=; fi
  for combo in "${COMBOS[@]}"; do
    log "profile=$profile $combo : build cdylib"
    # shellcheck disable=SC2086
    cargo build $PF $combo 2>&1 | tail -2 || { fail=1; continue; }
    log "profile=$profile $combo : symbol parity"
    R_SO="$CRATE_DIR/target/$([ "$profile" = release ] && echo release || echo debug)/libcomplexmode_lib.so"
    missing=$(comm -23 \
      <(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sort -u) \
      <(nm -D --defined-only --format=posix "$R_SO" | awk '{print $1}' | sort -u))
    if [ -n "$missing" ]; then
      echo "MISSING SYMBOLS in $R_SO:"; echo "$missing"; fail=1
    else
      echo "symbol diff empty (all C exports present in the Rust .so)"
    fi
    log "profile=$profile $combo : cargo test"
    # shellcheck disable=SC2086
    cargo test $PF $combo 2>&1 | grep -E "^(test result|error|warning: unused|thread .* panicked)" || fail=1
  done
done

log "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
