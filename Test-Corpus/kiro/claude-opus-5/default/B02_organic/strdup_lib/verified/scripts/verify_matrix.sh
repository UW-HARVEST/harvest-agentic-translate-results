#!/usr/bin/env bash
# Phase D driver: symbol parity + the full feature/profile matrix.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# this keeps working if features are ever added.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

C_SO=../c_src/build/libdriver.so
rc=0

echo "=== Building the C shared library ==="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }

# --- feature combinations ------------------------------------------------
# Every declared feature name, plus the two canonical endpoints.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/[ \t]*=.*/,""); gsub(/[ \t"]/,""); if ($0 != "") print}' Cargo.toml
)

declare -a COMBOS=("default" "none")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  # Powerset of declared features (guarded: only feasible for small sets).
  n=${#FEATURES[@]}
  if [ "$n" -le 10 ]; then
    for ((mask=1; mask < (1<<n); mask++)); do
      combo=""
      for ((b=0; b<n; b++)); do
        if (( mask & (1<<b) )); then combo+="${FEATURES[$b]},"; fi
      done
      COMBOS+=("${combo%,}")
    done
  else
    echo "NOTE: $n features declared; powerset skipped, testing each singly."
    for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
  fi
else
  echo "NOTE: Cargo.toml declares no [features]; the matrix is {default, --no-default-features}."
fi

for profile in dev release; do
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      default) featflags=() ;;
      none)    featflags=(--no-default-features) ;;
      *)       featflags=(--no-default-features --features "$combo") ;;
    esac
    profflags=(); [ "$profile" = release ] && profflags=(--release)

    label="profile=$profile features=$combo"
    echo
    echo "=== $label ==="

    if ! timeout 600 cargo build -q "${profflags[@]}" "${featflags[@]}" 2>&1 | tail -5; then
      echo "BUILD FAILED: $label"; rc=1; continue
    fi

    rust_so="target/$([ "$profile" = release ] && echo release || echo debug)/libdriver.so"
    if [ ! -f "$rust_so" ]; then
      echo "MISSING cdylib: $rust_so"; rc=1; continue
    fi

    # --- symbol parity: the C .so's defined symbols must all be in the Rust .so
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo "SYMBOL PARITY FAILED ($label) — missing from Rust .so:"
      echo "$missing" | sed 's/^/  /'
      rc=1
    else
      echo "symbol parity: OK (0 missing)"
    fi

    # --- undefined symbols must resolve only against libc / the loader
    undef=$(nm -D --undefined-only "$rust_so" | awk '{print $NF}' | sort -u \
      | grep -vE '^(malloc|memcpy|strlen|free|__libc_start_main|__gmon_start__|_ITM_|__cxa_|__tls_get_addr|__stack_chk_fail|_Unwind_|abort|memset|memmove|bcmp|memcmp)' )
    if [ -n "$undef" ]; then
      echo "NOTE: additional undefined symbols (verify they are libc):"
      echo "$undef" | sed 's/^/  /'
    fi

    # --- differential suite, pointed at this exact .so
    if DRIVER_RUST_SO="$PWD/$rust_so" timeout 600 cargo test -q "${profflags[@]}" "${featflags[@]}" 2>&1 | tail -20; then
      echo "tests: PASS ($label)"
    else
      echo "TESTS FAILED: $label"; rc=1
    fi
  done
done

echo
if [ "$rc" -eq 0 ]; then
  echo "PHASE D: all combinations passed (symbol parity + differential suite)."
else
  echo "PHASE D: failures above."
fi
exit "$rc"
