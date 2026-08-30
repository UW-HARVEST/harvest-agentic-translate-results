#!/usr/bin/env bash
# Full verification driver: builds both libraries, diffs the exported symbols,
# and runs every differential test under every feature combination and profile.
#
#   ./verify.sh
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
C_DIR="$(cd .. && pwd)/c_src"
fail=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  [ok] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "libStaticAlias.so (C)" || bad "C build"
C_SO="$C_DIR/build/libStaticAlias.so"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations from Cargo.toml"
# All subsets of the declared features, derived mechanically -- plus the
# no-default-features and default builds.
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml)
echo "  declared features: ${FEATURES[*]:-<none>}"

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("--no-default-features" "")   # identical here, but checked anyway
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
  COMBOS+=("")   # the default feature set
fi
printf '  combination: %s\n' "${COMBOS[@]/#/cargo test }"

# ---------------------------------------------------------------------------
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="profile='${profile:-dev}' features='${combo:-<default>}'"
    step "$label"

    # shellcheck disable=SC2086
    if cargo build $profile $combo >/dev/null 2>&1; then
      ok "cargo build"
    else
      bad "cargo build ($label)"; continue
    fi

    if [ -n "$profile" ]; then R_SO="$CRATE_DIR/target/release/libStaticAlias.so"
    else                       R_SO="$CRATE_DIR/target/debug/libStaticAlias.so"; fi

    # --- symbol parity: every symbol the C .so exports must be exported by Rust
    diff_out=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u))
    if [ -z "$diff_out" ]; then
      ok "symbol parity (C-exported symbols missing from Rust: none)"
    else
      bad "symbols missing from the Rust .so: $(echo "$diff_out" | tr '\n' ' ')"
    fi

    # --- no unresolved non-libc symbols in the Rust .so
    if ldd -r "$R_SO" 2>&1 | grep -qi 'undefined symbol\|not found'; then
      bad "unresolved symbols in $R_SO"
    else
      ok "no unresolved symbols"
    fi

    # --- the differential tests (Phase B + Phase C)
    log="$(mktemp)"
    # shellcheck disable=SC2086
    timeout 600 cargo test $profile $combo >"$log" 2>&1
    rc=$?
    n_ok=$(grep -c '^test result: ok' "$log")
    n_bad=$(grep -c '^test result: FAILED' "$log")
    n_tests=$(grep -c '^test .* \.\.\. ok$' "$log")
    if [ "$rc" -eq 0 ] && [ "$n_bad" -eq 0 ] && [ "$n_ok" -eq 4 ] && [ "$n_tests" -ge 45 ]; then
      ok "differential tests: $n_tests test fns green across $n_ok test binaries"
    else
      bad "differential tests ($label): rc=$rc ok-binaries=$n_ok failed-binaries=$n_bad tests=$n_tests"
      tail -40 "$log"
    fi
    rm -f "$log"
  done
done

step "Summary"
if [ "$fail" -eq 0 ]; then
  echo "  ALL CHECKS PASSED"
else
  echo "  THERE WERE FAILURES"
fi
exit "$fail"
