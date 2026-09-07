#!/usr/bin/env bash
# Full verification driver: builds both objects, checks symbol parity, then runs
# Phases B and C across every feature combination and every build profile.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cd "$here"

fail=0
step() { printf '\n=========== %s ===========\n' "$*"; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
( cd "$root/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "FAIL: C build"; exit 1; }
find "$root/c_src/build" -maxdepth 1 -name '*.so' -printf '  %p\n'

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# Every declared feature name (the keys of the [features] table).
features=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    sub(/[[:space:]]*=.*/, ""); print
  }' Cargo.toml | grep -v '^default$' | sort -u)

combos=()
if [[ -z "$features" ]]; then
  echo "  No [features] table in Cargo.toml -> the only configuration is the"
  echo "  default one. --no-default-features is byte-identical to the default."
  combos=("__default__" "__nodefault__")
else
  echo "  features: $features"
  # Full power set of the declared features, plus the plain default build.
  fa=($features)
  n=${#fa[@]}
  combos=("__default__")
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel+="${fa[$i]},"; fi
    done
    combos+=("${sel%,}")
  done
fi
printf '  combination: %s\n' "${combos[@]}"

# ---------------------------------------------------------------------------
# For each combination: build the cdylib in both profiles, check symbols
# against that exact object, then run both test files against it.
for combo in "${combos[@]}"; do
  case "$combo" in
    __default__)   flags=() ; label="default" ;;
    __nodefault__) flags=(--no-default-features) ; label="no-default-features" ;;
    "")            flags=(--no-default-features) ; label="no-default-features" ;;
    *)             flags=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac

  for profile in release debug; do
    pflags=()
    [[ $profile == release ]] && pflags=(--release)

    step "[$label / $profile] build cdylib"
    if ! timeout 600 cargo build "${pflags[@]}" "${flags[@]}"; then
      echo "FAIL: build $label/$profile"; fail=1; continue
    fi
    so="$here/target/$profile/libhsl_to_rgb_lib.so"
    if [[ ! -f "$so" ]]; then echo "FAIL: missing $so"; fail=1; continue; fi

    step "[$label / $profile] symbol parity"
    if ! RUST_SO="$so" ./check_symbols.sh | grep -E '^(PASS|FAIL|=== Missing|=== Extra)' -A1; then
      echo "FAIL: symbol parity $label/$profile"; fail=1
    fi

    step "[$label / $profile] Phase B + Phase C differential tests"
    if ! RUST_SO="$so" timeout 600 cargo test "${pflags[@]}" "${flags[@]}" 2>&1 \
           | grep -E '^(     Running|test result|test .* FAILED|error)'; then
      echo "FAIL: tests $label/$profile"; fail=1
    fi
  done
done

# ---------------------------------------------------------------------------
step "Binary / driver executables"
c_bins=$(find "$root/c_src/build" -maxdepth 1 -type f -executable ! -name '*.so' 2>/dev/null)
r_bins=$(find "$here/target/release" -maxdepth 1 -type f -executable ! -name '*.so' 2>/dev/null)
if [[ -z "$c_bins" && -z "$r_bins" ]]; then
  echo "  Neither project builds an executable driver"
  echo "  (c_src/CMakeLists.txt has only add_library(... SHARED ...);"
  echo "   translation/Cargo.toml has only [lib] crate-type = [\"cdylib\"])."
  echo "  -> the stdout-comparison requirement is vacuous."
else
  echo "  C binaries:    ${c_bins:-<none>}"
  echo "  Rust binaries: ${r_bins:-<none>}"
  echo "  FAIL: a driver exists but is not being compared." ; fail=1
fi

# ---------------------------------------------------------------------------
step "RESULT"
if [[ $fail -eq 0 ]]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit $fail
