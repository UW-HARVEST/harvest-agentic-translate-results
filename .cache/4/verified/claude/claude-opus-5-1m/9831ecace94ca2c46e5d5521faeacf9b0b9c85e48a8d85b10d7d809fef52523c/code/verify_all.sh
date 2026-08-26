#!/usr/bin/env bash
# Full verification driver: builds the C .so, then for EVERY feature combination
# (and for both the dev and release profiles) builds the Rust cdylib and runs the
# whole differential suite against it, finishing with the symbol-parity check.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

rc=0
step() { echo; echo "############ $* ############"; }

# ---------------------------------------------------------------- C library ---
step "build the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l c_src/build/libdriver.so

# --------------------------------------------- enumerate feature combinations --
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}
COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    (((mask >> i) & 1)) && combo="${combo:+$combo,}${FEATURES[$i]}"
  done
  COMBOS+=("$combo")
done
step "feature space: ${#COMBOS[@]} combination(s) from ${N} non-default feature(s) [${FEATURES[*]:-none}]"

# ------------------------------------------------------------- run the matrix --
run_config() {
  local desc="$1"; shift
  step "cargo check  ($desc)"
  timeout 600 cargo check --offline "$@" || { echo "check FAILED: $desc"; rc=1; return; }
  step "cargo build  ($desc)"
  timeout 600 cargo build --offline "$@" || { echo "build FAILED: $desc"; rc=1; return; }
  step "cargo test   ($desc)"
  timeout 600 cargo test --offline "$@" || { echo "TESTS FAILED: $desc"; rc=1; return; }
  echo ">>> OK: $desc"
}

for combo in "${COMBOS[@]}"; do
  label="features='${combo:-<empty>}'"
  run_config "dev, $label"     --no-default-features --features "$combo"
  run_config "release, $label" --release --no-default-features --features "$combo"
done
run_config "dev, default features"
run_config "release, default features" --release

# --------------------------------------------------------------- Phase D gate --
step "restore the dev-profile cdylib and check symbol parity"
timeout 600 cargo build --offline >/dev/null 2>&1
./symbol_parity.sh || rc=1

step "SUMMARY"
if [ "$rc" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "AT LEAST ONE CONFIGURATION FAILED"
fi
exit "$rc"
