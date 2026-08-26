#!/bin/bash
# Full verification driver: enumerates every build-time configuration, checks
# and tests each one, and diffs the exported symbol tables of the two `.so`s.
#
# Usage: ./verify.sh
set -uo pipefail
cd "$(dirname "$0")" || exit 1

WORK="${TMPDIR:-/tmp}/driver_verify.$$"
mkdir -p "$WORK" || { echo "cannot create work dir $WORK"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

CARGO_FLAGS=(--offline)
fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml (powerset of [features],
#    excluding the implicit "default" key).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)
note "features declared in Cargo.toml: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((b = 0; b < n; b++)); do
    if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FEATURES[$b]}"; fi
  done
  COMBOS+=("$combo")   # first entry ("") is the no-feature build
done
echo "feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library.
# ---------------------------------------------------------------------------
note "building C reference .so"
(mkdir -p c_src/build && cd c_src/build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
C_SO=c_src/build/libdriver.so
echo "ok: $C_SO"

# ---------------------------------------------------------------------------
# 3. cargo check + cargo test for every combination and both profiles.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  featargs=(--no-default-features)
  [[ -n "$combo" ]] && featargs+=(--features "$combo")

  note "cargo check --no-default-features ${combo:+--features $combo}"
  if ! timeout 600 cargo check "${CARGO_FLAGS[@]}" "${featargs[@]}" --all-targets 2>&1 | tail -3; then
    echo "CHECK FAILED for $label"; fail=1; continue
  fi

  for profile in dev release; do
    profargs=(); [[ $profile == release ]] && profargs+=(--release)
    note "cargo test [$profile] $label"
    if ! timeout 600 cargo test "${CARGO_FLAGS[@]}" "${featargs[@]}" "${profargs[@]}" 2>&1 |
         tee $WORK/vt | grep -E "test result:|^error"; then :; fi
    if grep -q "test result: FAILED" $WORK/vt || grep -qE "^error" $WORK/vt; then
      echo "TESTS FAILED for $label [$profile]"; fail=1
    fi
    rm -f $WORK/vt

    # ------------------------------------------------------------------
    # 4. Symbol parity for the .so produced by THIS configuration.
    # ------------------------------------------------------------------
    dir=target/debug; [[ $profile == release ]] && dir=target/release
    timeout 300 cargo build "${CARGO_FLAGS[@]}" "${featargs[@]}" "${profargs[@]}" --lib >/dev/null 2>&1
    R_SO=$dir/libdriver.so
    nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > $WORK/csyms
    nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > $WORK/rsyms
    missing=$(comm -23 $WORK/csyms $WORK/rsyms)
    extra=$(comm -13 $WORK/csyms $WORK/rsyms)
    if [[ -n "$missing" ]]; then
      echo "MISSING from Rust .so ($label/$profile):"; echo "$missing"; fail=1
    else
      echo "symbol parity ok ($label/$profile): $(wc -l < $WORK/csyms) C symbol(s), 0 missing"
    fi
    [[ -n "$extra" ]] && { echo "note: extra Rust-only symbols:"; echo "$extra"; }
    # Undefined non-libc symbols in the Rust .so must be empty.
    und=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' |
          grep -vE '@GLIBC|@GCC|^_ITM_|^__cxa_finalize$|^__gmon_start__$|^_Unwind|^__tls_get_addr$' || true)
    if [[ -n "$und" ]]; then
      echo "UNDEFINED non-libc symbols in Rust .so ($label/$profile):"; echo "$und"; fail=1
    else
      echo "undefined non-libc symbols ($label/$profile): none"
    fi
    rm -f $WORK/csyms $WORK/rsyms
  done
done

note "RESULT"
if [[ $fail -eq 0 ]]; then echo "ALL CONFIGURATIONS VERIFIED"; else echo "FAILURES PRESENT"; fi
exit $fail
