#!/usr/bin/env bash
# Enumerate every build-time configuration and run check + differential tests
# for each one.
#
#  * Cargo features: parsed out of translation/Cargo.toml. This crate declares
#    no [features] table, so the only valid combination is the empty set
#    (--no-default-features). The loop below is written generically so it keeps
#    working if features are added later.
#  * CMake options: c_src/CMakeLists.txt declares no option()/
#    target_compile_definitions, so the C side has a single configuration too.
#  * Rust cargo profiles: the release cdylib (the shipped artifact, built with
#    panic="abort") and the debug cdylib are both loaded and compared against C.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

log() { printf '\n=== %s ===\n' "$*"; }

log "Building C shared library"
mkdir -p c_src/build
(cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build .) > /tmp/cbuild.log 2>&1 || { tail -30 /tmp/cbuild.log; exit 1; }
c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "C library: $c_so"

# --- enumerate cargo feature combinations -----------------------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' translation/Cargo.toml
)

combos=("")
n=${#features[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${features[$i]}"
      fi
    done
    combos+=("$combo")
  done
fi

log "Feature combinations to verify: ${#combos[@]}"
for c in "${combos[@]}"; do echo "  - '${c:-<none>}'"; done

# --- check + test every combination -----------------------------------------
cd translation
status=0
for combo in "${combos[@]}"; do
  label="${combo:-<no features>}"

  log "cargo check --no-default-features --features '$combo' ($label)"
  if ! timeout 600 cargo check --all-targets --no-default-features \
        ${combo:+--features "$combo"} 2>&1 | tail -5; then
    echo "CHECK FAILED for $label"; status=1; continue
  fi

  log "cargo build --release ($label)"
  timeout 600 cargo build --release --no-default-features \
    ${combo:+--features "$combo"} 2>&1 | tail -3

  # Symbol parity against the C library for this configuration.
  log "symbol parity ($label)"
  nm -D --defined-only "$c_so" | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort > /tmp/c_syms.txt
  nm -D --defined-only target/release/libbuffapp_lib.so \
    | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort > /tmp/rs_syms.txt
  if missing="$(comm -23 /tmp/c_syms.txt /tmp/rs_syms.txt)" && [[ -n "$missing" ]]; then
    echo "MISSING EXPORTS for $label:"; echo "$missing"; status=1
  else
    echo "all $(wc -l < /tmp/c_syms.txt) C exports present in the Rust .so"
  fi

  # Differential tests against both Rust cdylib profiles.
  for profile in release debug; do
    if [[ "$profile" == debug ]]; then
      timeout 600 cargo build --no-default-features \
        ${combo:+--features "$combo"} 2>&1 | tail -2
    fi
    so="$root/translation/target/$profile/libbuffapp_lib.so"
    log "cargo test ($label, Rust .so profile=$profile)"
    if ! RUST_SO_PATH="$so" C_SO_PATH="$c_so" \
        timeout 600 cargo test --no-default-features \
        ${combo:+--features "$combo"} 2>&1 | grep -E "^(test result|error|failures:)" ; then
      echo "TESTS FAILED for $label/$profile"; status=1
    fi
  done
done

log "overall status: $([[ $status -eq 0 ]] && echo PASS || echo FAIL)"
exit $status
