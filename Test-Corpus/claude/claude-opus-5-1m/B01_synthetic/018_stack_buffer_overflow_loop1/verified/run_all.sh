#!/usr/bin/env bash
# Full verification matrix for the C -> Rust translation of c_src/src/main.c.
#
#   * builds the C reference (CMake executable + gcc shared objects)
#   * enumerates EVERY valid Cargo feature combination from Cargo.toml
#   * cargo check + cargo test for each combination, in both dev and release
#
# Usage: ./run_all.sh
set -u -o pipefail

cd "$(dirname "$0")"
CARGO_FLAGS=(--offline)
LOG_DIR="${TMPDIR:-/tmp}/driver-verify"
mkdir -p "$LOG_DIR"
FAILED=0

hr() { printf '%s\n' "-----------------------------------------------------------------"; }
step() { printf '\n== %s\n' "$*"; }
ok()   { printf '   PASS  %s\n' "$*"; }
bad()  { printf '   FAIL  %s\n' "$*"; FAILED=1; }

run() { # run <label> <logfile> <cmd...>
  local label="$1" log="$2"; shift 2
  if timeout 600 "$@" >"$log" 2>&1; then ok "$label"; else bad "$label  (see $log)"; tail -n 25 "$log"; fi
}

# ---------------------------------------------------------------------------
step "Toolchain"
# ---------------------------------------------------------------------------
for t in cargo rustc gcc cmake nm; do
  command -v "$t" >/dev/null || { echo "missing required tool: $t" >&2; exit 2; }
done
printf '   %s\n' "$(rustc --version)" "$(gcc --version | head -1)" "$(cmake --version | head -1)"

# ---------------------------------------------------------------------------
step "C reference build (c_src is never modified; artifacts land in build dirs)"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
run "cmake configure" "$LOG_DIR/cmake-config.log" \
    cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run "cmake build (add_executable driver)" "$LOG_DIR/cmake-build.log" \
    cmake --build c_src/build

mkdir -p c_build
for opt in "" -O0 -O1 -O2 -Os; do
  tag="${opt:-default}"; tag="${tag#-}"
  run "gcc -shared ${opt:-<cmake default flags>}" "$LOG_DIR/gcc-$tag.log" \
      gcc -shared -fPIC $opt -o "c_build/libc_driver_$tag.so" c_src/src/main.c
done
printf '   C .so exports: %s\n' "$(nm -D --defined-only c_build/libc_driver_default.so | awk '{print $3}' | sort | tr '\n' ' ')"

# ---------------------------------------------------------------------------
step "Enumerating Cargo feature combinations"
# ---------------------------------------------------------------------------
# Every key under [features] other than `default`, i.e. the optional features
# that can be switched on independently.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}
printf '   optional features (%d): %s\n' "$N" "${FEATURES[*]:-<none>}"

# Powerset of the optional features, each evaluated with and without defaults.
COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  sel=()
  for ((i = 0; i < N; i++)); do (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}"); done
  joined=$(IFS=,; echo "${sel[*]}")
  COMBOS+=("default|$joined")
  COMBOS+=("no-default|$joined")
done
printf '   feature combinations to verify: %d\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "cargo check / build / test for every combination x profile"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  mode="${combo%%|*}"; feats="${combo#*|}"
  args=()
  [[ "$mode" == "no-default" ]] && args+=(--no-default-features)
  [[ -n "$feats" ]] && args+=(--features "$feats")
  label="$mode${feats:+ +$feats}"
  slug=$(echo "$label" | tr ' +,' '___')

  for profile in dev release; do
    pargs=(); [[ "$profile" == release ]] && pargs+=(--release)
    hr
    printf ' combination: %-28s profile: %s\n' "$label" "$profile"
    hr
    run "cargo check --all-targets"  "$LOG_DIR/check-$slug-$profile.log" \
        cargo check "${CARGO_FLAGS[@]}" "${args[@]}" "${pargs[@]}" --all-targets
    run "cargo build (bin + cdylib)" "$LOG_DIR/build-$slug-$profile.log" \
        cargo build "${CARGO_FLAGS[@]}" "${args[@]}" "${pargs[@]}"

    # Symbol parity, checked directly as well as inside tests/symbols.rs.
    prof_dir=$([[ "$profile" == release ]] && echo target/release || echo target/debug)
    c_syms=$(nm -D --defined-only c_build/libc_driver_default.so | awk '{print $3}' | sort)
    r_syms=$(nm -D --defined-only "$prof_dir/libdriver.so" | awk '{print $3}' | sort)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [[ -z "$missing" ]]; then
      ok "symbol parity ($(echo "$c_syms" | wc -w) C symbols all present in $prof_dir/libdriver.so)"
    else
      bad "symbol parity: missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
    fi

    # `cargo test` never refreshes the cdylib (it only needs the rlib), so the
    # test harness rebuilds it itself; tell it which feature flags to use.
    export DRIVER_LIB_BUILD_ARGS="${args[*]}"
    run "cargo test (all differential rows)" "$LOG_DIR/test-$slug-$profile.log" \
        cargo test "${CARGO_FLAGS[@]}" "${args[@]}" "${pargs[@]}" -- --test-threads=1
    unset DRIVER_LIB_BUILD_ARGS
    grep -hE "^(test result|running)" "$LOG_DIR/test-$slug-$profile.log" | sed 's/^/     /'
  done
done

hr
if ((FAILED)); then
  printf '\nRESULT: FAILURES PRESENT (logs in %s)\n\n' "$LOG_DIR"
  exit 1
fi
printf '\nRESULT: all feature combinations and profiles verified.\n\n'
