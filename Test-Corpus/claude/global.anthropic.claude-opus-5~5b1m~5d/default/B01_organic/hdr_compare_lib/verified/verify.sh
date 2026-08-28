#!/usr/bin/env bash
# Phase D driver: build the C .so, then run the whole differential suite for every
# cargo feature combination x profile, and diff `nm -D` between the two shared objects.
#
#   ./verify.sh            # every combo, debug + release
#   ./verify.sh --quick    # debug only
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
cd "$HERE"

fail=0
note() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
bad()  { printf '\033[31mFAIL: %s\033[0m\n' "$*"; fail=1; }

# ---------------------------------------------------------------- C shared object
note "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "cmake build"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C  .so: $C_SO"

# ---------------------------------------------------------------- feature combos
# Enumerate the power set of the [features] declared in Cargo.toml. This crate declares
# none, so the set collapses to {default, --no-default-features}; the loop is written
# generically so it keeps working if features are ever added.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblock=1; next }
    /^\[/           { inblock=0 }
    inblock && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
COMBOS+=("--offline")                                # default features
COMBOS+=("--offline --no-default-features")          # nothing enabled
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--offline --no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

echo "Declared features: ${FEATURES[*]:-<none>}"
echo "Combinations to verify: ${#COMBOS[@]}"

PROFILES=("")
[[ "${1:-}" == "--quick" ]] || PROFILES=("" "--release")

# ---------------------------------------------------------------- run
for prof in "${PROFILES[@]}"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test ${prof:-<debug>} ${combo}"
    note "$label"

    # shellcheck disable=SC2086
    cargo build $combo $prof --lib >/dev/null 2>&1 || { bad "build: $label"; continue; }

    pdir="target/$([[ -n $prof ]] && echo release || echo debug)"
    R_SO="$pdir/libhdr_compare_lib.so"
    [[ -f "$R_SO" ]] || { bad "missing $R_SO"; continue; }

    # nm -D parity: every defined, non-weak symbol of the C .so must exist in the Rust .so
    c_syms=$(nm -D --defined-only "$C_SO" | awk '$2!="w"{print $NF}' | sort -u)
    r_syms=$(nm -D --defined-only "$R_SO" | awk '$2!="w"{print $NF}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [[ -n "$missing" ]]; then
      bad "symbols missing from the Rust .so ($label): $(echo $missing)"
    else
      echo "symbol parity OK: $(echo $c_syms | tr '\n' ' ')"
    fi

    log="target/verify-$(echo "${prof:-debug}$combo" | tr -c 'A-Za-z0-9' '_').log"
    # shellcheck disable=SC2086
    timeout 600 cargo test $combo $prof >"$log" 2>&1
    rc=$?
    grep -E '^test result:|^running ' "$log" | sed 's/^/    /'
    if (( rc != 0 )); then
      bad "tests: $label (rc=$rc, see $HERE/$log)"
      tail -30 "$log" | sed 's/^/    /'
    fi
  done
done

note "Summary"
if (( fail )); then
  echo "VERIFICATION FAILED"
  exit 1
fi
echo "ALL COMBINATIONS PASSED"
