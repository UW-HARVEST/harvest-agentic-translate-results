#!/usr/bin/env bash
# Phase D driver: build both objects, then run the whole differential suite
# against EVERY configuration (cargo profile x feature combination).
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a feature automatically widens the matrix.
set -uo pipefail

cd "$(dirname "$0")"
CRATE="$PWD"
WORK="$(cd .. && pwd)"

pass=0
fail=0
declare -a FAILED

record() { # record <label> <exit-status>
  if [ "$2" -eq 0 ]; then
    printf 'PASS: %s\n' "$1"; pass=$((pass + 1))
  else
    printf 'FAIL: %s\n' "$1"; fail=$((fail + 1)); FAILED+=("$1")
  fi
}

run_fn() { # run_fn <label> <shell-function>  (timeout cannot exec a function)
  local label="$1"; shift
  printf '\n=== %s ===\n' "$label"
  "$@"
  record "$label" "$?"
}

run() { # run <label> <external-cmd...>
  local label="$1"; shift
  printf '\n=== %s ===\n' "$label"
  if timeout 600 "$@"; then
    printf 'PASS: %s\n' "$label"
    pass=$((pass + 1))
  else
    printf 'FAIL: %s\n' "$label"
    fail=$((fail + 1))
    FAILED+=("$label")
  fi
}

# ---------------------------------------------------------------------------
# 1. Build the C shared library (never modified; only built).
# ---------------------------------------------------------------------------
echo "### building C .so"
mkdir -p "$WORK/c_src/build"
( cd "$WORK/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$WORK"/c_src/build/lib*.so | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1b. Additional C builds at other -O levels, for the robustness study in
#     tests/olevels.rs. Built OUT OF TREE (into target/) so c_src is untouched.
# ---------------------------------------------------------------------------
OLEV="$CRATE/target/olevels"
mkdir -p "$OLEV"
for O in "" -O0 -O1 -O2 -O3 -Os -Ofast; do
  n="$(echo "${O:-default}" | tr -d '-')"
  # shellcheck disable=SC2086
  gcc $O -shared -fPIC -o "$OLEV/lib$n.so" \
      -I"$WORK/c_src/include" "$WORK/c_src/src/lib.c" -lm 2>/dev/null \
    || echo "  (could not build C at ${O:-default}; skipping)"
done
ALT_SOS="$(ls "$OLEV"/*.so 2>/dev/null | tr '\n' ':')"
echo "alt C builds: $ALT_SOS"
export TFM_ALT_C_SOS="$ALT_SOS"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:］ ]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

# Combination list: always the default build and the no-default-features build;
# then every single feature, then the all-features build.
declare -a COMBOS=("DEFAULT" "NO_DEFAULT")
for f in "${FEATURES[@]:-}"; do
  [ -n "$f" ] && COMBOS+=("FEAT:$f")
done
if [ "${#FEATURES[@]:-0}" -gt 1 ]; then
  COMBOS+=("ALL")
fi

echo "### feature combinations: ${COMBOS[*]}"
if [ "${#FEATURES[@]:-0}" -eq 0 ]; then
  echo "    (Cargo.toml declares no [features]; DEFAULT and NO_DEFAULT are the"
  echo "     complete set and compile identical code — src/lib.rs has no #[cfg])"
fi

combo_args() {
  case "$1" in
    DEFAULT)    echo "" ;;
    NO_DEFAULT) echo "--no-default-features" ;;
    ALL)        echo "--all-features" ;;
    FEAT:*)     echo "--no-default-features --features ${1#FEAT:}" ;;
  esac
}

# ---------------------------------------------------------------------------
# 3. For each (combo, profile): build the cdylib, then run the suite against it.
#    The Rust .so under test is selected explicitly via TFM_RUST_SO, because
#    `cargo test` does NOT rebuild a cdylib that the tests only dlopen.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  ARGS=$(combo_args "$combo")

  for profile in debug release; do
    if [ "$profile" = release ]; then
      # shellcheck disable=SC2086
      cargo build --release $ARGS >/dev/null 2>&1 \
        || { echo "FAIL: build $combo/$profile"; fail=$((fail+1)); FAILED+=("build $combo/$profile"); continue; }
      RS_SO="$CRATE/target/release/libtfm_lib.so"
    else
      # shellcheck disable=SC2086
      cargo build $ARGS >/dev/null 2>&1 \
        || { echo "FAIL: build $combo/$profile"; fail=$((fail+1)); FAILED+=("build $combo/$profile"); continue; }
      RS_SO="$CRATE/target/debug/libtfm_lib.so"
    fi

    export TFM_C_SO="$C_SO"
    export TFM_RUST_SO="$RS_SO"

    # shellcheck disable=SC2086
    run "$combo / $profile .so / clippy-free check" cargo check --tests $ARGS
    # shellcheck disable=SC2086
    run "$combo / $profile .so / symbols (Phase A+D)" cargo test --test symbols $ARGS
    # shellcheck disable=SC2086
    run "$combo / $profile .so / recon (broad fuzz)" cargo test --test recon $ARGS
    # shellcheck disable=SC2086
    run "$combo / $profile .so / Phase B (CONFIGS.md)" cargo test --test phase_b $ARGS
    # shellcheck disable=SC2086
    run "$combo / $profile .so / Phase C (ERRORS.md)" cargo test --test phase_c $ARGS
    # shellcheck disable=SC2086
    run "$combo / $profile .so / -O level robustness" cargo test --test olevels $ARGS
  done
done

# ---------------------------------------------------------------------------
# 3b. Independent ABI cross-check: a plain C program LINKED DIRECTLY against
#     each .so (real dynamic linking, not dlsym), outputs diffed.
# ---------------------------------------------------------------------------
abi_probe() {
  local PD="$CRATE/target/probe"
  mkdir -p "$PD"
  gcc -O0 -o "$PD/p_c" "$CRATE/probe/abi_probe.c" "$C_SO" \
      -Wl,-rpath,"$(dirname "$C_SO")" || return 1
  "$PD/p_c" > "$PD/out_c.txt" || return 1
  local rc=0
  for prof in debug release; do
    [ -f "$CRATE/target/$prof/libtfm_lib.so" ] || continue
    gcc -O0 -o "$PD/p_$prof" "$CRATE/probe/abi_probe.c" \
        "$CRATE/target/$prof/libtfm_lib.so" \
        -Wl,-rpath,"$CRATE/target/$prof" || { rc=1; continue; }
    "$PD/p_$prof" > "$PD/out_$prof.txt" || { rc=1; continue; }
    if diff -q "$PD/out_c.txt" "$PD/out_$prof.txt" >/dev/null; then
      echo "  native C caller: C .so == Rust $prof .so  ($(wc -l < "$PD/out_c.txt") lines identical)"
    else
      echo "  native C caller: C .so != Rust $prof .so"
      diff "$PD/out_c.txt" "$PD/out_$prof.txt" | head -20
      rc=1
    fi
  done
  return $rc
}
run_fn "ABI probe (native C caller, direct linking)" abi_probe

# ---------------------------------------------------------------------------
# 4. Symbol diff, printed for the record.
# ---------------------------------------------------------------------------
printf '\n=== nm -D --defined-only diff (C vs Rust) ===\n'
diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
     <(nm -D --defined-only "$CRATE/target/release/libtfm_lib.so" | awk '{print $NF}' | sort) \
  && echo "symbol sets are IDENTICAL"

printf '\n===================== SUMMARY =====================\n'
printf 'passed: %d\nfailed: %d\n' "$pass" "$fail"
if [ "$fail" -ne 0 ]; then
  printf 'failing steps:\n'
  for f in "${FAILED[@]}"; do printf '  - %s\n' "$f"; done
  exit 1
fi
echo "ALL CONFIGURATIONS PASS"
