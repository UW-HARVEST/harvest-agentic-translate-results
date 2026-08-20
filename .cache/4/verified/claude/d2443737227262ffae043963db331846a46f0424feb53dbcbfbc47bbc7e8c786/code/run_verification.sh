#!/bin/bash
# Full verification sweep: every feature combination x {debug, release},
# plus the C-vs-Rust symbol diff. Usage: ./run_verification.sh
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"
FAILED=0
LOGDIR="target/verification-logs"
mkdir -p "$LOGDIR"

# --- Phase A: enumerate the feature powerset from Cargo.toml -----------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, itertools, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name != 'default':
                feats.append(name)
combos = []
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        combos.append(','.join(c))
for c in combos:
    print(c)
PY
)

echo "==> optional features: ${#FEATURES[@]} combination(s) of non-default features"
echo "==> combinations (each also run with and without the 'default' feature)"

run() { # run <label> <cargo-subcommand> <extra flags...>
  local label="$1"; shift
  echo "--- $label"
  if timeout 600 cargo "$@" $CARGO_FLAGS > "$LOGDIR/verif.log" 2>&1; then
    grep -E "^test result:" "$LOGDIR/verif.log" | sed 's/^/    /'
    echo "    OK: $label"
  else
    echo "    FAIL: $label"
    tail -40 "$LOGDIR/verif.log" | sed 's/^/    /'
    FAILED=1
  fi
}

for combo in "${FEATURES[@]}"; do
  for defaults in "--no-default-features" ""; do
    if [ -n "$combo" ]; then FEAT_ARGS=(--features "$combo"); else FEAT_ARGS=(); fi
    LBL="features='${combo:-<none>}' ${defaults:---default-features}"
    # Phase A step 2: it must compile in every combination
    run "cargo check   [$LBL]"        check   $defaults "${FEAT_ARGS[@]}"
    # Phases B, C, D (debug)
    run "cargo test    [$LBL] debug"  test    $defaults "${FEAT_ARGS[@]}"
    # Phases B, C, D (release: profile has panic = "abort")
    run "cargo test    [$LBL] release" test --release $defaults "${FEAT_ARGS[@]}"
  done
done

# --- explicit `--features default` spelling ---------------------------------
run "cargo test    [--features default]" test --features default

# --- Phase D: raw symbol diff (independent of the in-test check) -------------
echo "--- nm -D symbol diff (C .so vs Rust .so)"
C_SO=c_src/build/libtranslated_rust.so
for R_SO in target/debug/libdiv_euclid_lib.so target/release/libdiv_euclid_lib.so; do
  [ -f "$R_SO" ] || continue
  MISSING=$(comm -23 \
    <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$R_SO"  | awk '{print $NF}' | sort -u))
  if [ -z "$MISSING" ]; then
    echo "    OK: every C-defined symbol is exported by $R_SO"
  else
    echo "    FAIL: missing from $R_SO:"; echo "$MISSING" | sed 's/^/      /'
    FAILED=1
  fi
done

echo
if [ "$FAILED" -eq 0 ]; then echo "ALL VERIFICATION PASSED"; else echo "VERIFICATION FAILED"; fi
exit $FAILED
