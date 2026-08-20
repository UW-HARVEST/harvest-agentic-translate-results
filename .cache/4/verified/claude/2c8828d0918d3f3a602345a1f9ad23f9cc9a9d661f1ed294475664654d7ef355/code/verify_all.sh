#!/usr/bin/env bash
# Phase D driver: build the C and Rust shared objects, diff their exported
# symbols, and run the full differential suite for EVERY cargo feature
# combination in BOTH the dev and release profiles.
#
# `Cargo.toml` has no `[features]` section, so the feature powerset is the
# single empty set; the loop below is derived from the manifest rather than
# hard-coded, so it keeps working if features are ever added.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

fail=0
note() { printf '\n=== %s ===\n' "$*"; }
check() { if [ "$1" -ne 0 ]; then echo "FAIL: $2"; fail=1; else echo "ok: $2"; fi; }

# ---------------------------------------------------------------- C library
note "Building the C shared library"
mkdir -p c_src/build
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null )
check $? "cmake build of libtranslated_rust.so"
C_SO=c_src/build/libtranslated_rust.so

# ------------------------------------------------- enumerate feature combos
# Extract feature names from the [features] section, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
n=${#FEATURES[@]}
echo "features found in Cargo.toml: ${n} (${FEATURES[*]-none})"

COMBOS=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((b = 0; b < n; b++)); do
    if ((mask & (1 << b))); then combo+="${combo:+,}${FEATURES[b]}"; fi
  done
  COMBOS+=("$combo")
done
echo "feature combinations to verify: ${#COMBOS[@]}"

# --------------------------------------------------------------- the matrix
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  FEAT_ARGS=(--no-default-features)
  [ -n "$combo" ] && FEAT_ARGS+=(--features "$combo")

  note "cargo check --no-default-features ${combo:+--features $combo}"
  timeout 600 cargo check "${FEAT_ARGS[@]}" 2>&1 | tail -3
  check "${PIPESTATUS[0]}" "cargo check [$label]"

  for profile in dev release; do
    PROF_ARGS=()
    [ "$profile" = release ] && PROF_ARGS+=(--release)

    note "build + differential tests [$label] [$profile]"
    # The library is a cdylib only, so the test binaries do not link it and
    # `cargo test` will NOT rebuild it -- build it explicitly first. (The
    # harness also asserts the .so is not stale.)
    timeout 600 cargo build "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" 2>&1 | tail -2
    check "${PIPESTATUS[0]}" "cargo build [$label] [$profile]"

    RS_SO="target/$([ "$profile" = release ] && echo release || echo debug)/libunderhanded_c_nuke_lib.so"

    # Symbol parity for this configuration.
    if diff <(nm -D --defined-only "$C_SO" | awk '{print $2, $3}' | sort) \
            <(nm -D --defined-only "$RS_SO" | awk '{print $2, $3}' | sort) >/dev/null; then
      check 0 "nm -D symbol parity [$label] [$profile]"
    else
      echo "--- symbol diff (C vs Rust) ---"
      diff <(nm -D --defined-only "$C_SO" | awk '{print $2, $3}' | sort) \
           <(nm -D --defined-only "$RS_SO" | awk '{print $2, $3}' | sort)
      check 1 "nm -D symbol parity [$label] [$profile]"
    fi

    # Any non-libc undefined symbol in the Rust .so is a completeness failure.
    missing=$(nm -D --undefined-only "$RS_SO" \
      | awk '{print $NF}' \
      | grep -vE '^(_ITM_|__cxa_|__gmon_|_Unwind_|__tls_get_addr|__errno_location|gettid|statx|memcpy|memmove|memset|bcmp|strlen|malloc|calloc|realloc|free|posix_memalign|abort|getenv|getcwd|open64|close|read|write|writev|lseek64|stat64|fstat64|readlink|realpath|mmap64|munmap|syscall|dl_iterate_phdr|pthread_|sqrt)' \
      | grep -vE '@GLIBC|@GCC|^$' )
    if [ -z "$missing" ]; then
      check 0 "no non-libc undefined symbols [$label] [$profile]"
    else
      echo "unexpected undefined symbols: $missing"
      check 1 "no non-libc undefined symbols [$label] [$profile]"
    fi

    for t in phase_b phase_c phase_d; do
      timeout 600 cargo test "${FEAT_ARGS[@]}" "${PROF_ARGS[@]}" --test "$t" 2>&1 | tail -3
      check "${PIPESTATUS[0]}" "$t [$label] [$profile]"
    done
  done
done

note "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
