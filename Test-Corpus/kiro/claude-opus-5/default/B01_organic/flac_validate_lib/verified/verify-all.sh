#!/usr/bin/env bash
# verify-all.sh — Phase D driver.
#
# Rebuilds the C .so and the Rust .so, then runs the whole differential suite
# across every build configuration:
#   * feature combos  : enumerated from Cargo.toml's [features] (plus the
#                       default and --no-default-features baselines)
#   * cdylib profiles : debug (overflow checks, panic=unwind) and
#                       release (optimised, panic=abort)
#
# The Rust .so under test is selected with RUST_SO so the tests always dlopen
# the exact artifact for the configuration being verified.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
TIMEOUT=${TIMEOUT:-600}
fails=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; fails=$((fails+1)); }

# ---------------------------------------------------------------- C build ----
step "building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | tail -1)"
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
ok "C .so = $C_SO"

# ------------------------------------------------- feature-combo discovery ---
# Read the [features] table out of Cargo.toml; if there is none, the default
# (empty) feature set is the only configuration.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t"]/,"",a[1]); if(a[1]!="default") print a[1]}' \
    "$CRATE/Cargo.toml"
)
COMBOS=("default")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("none")
  n=${#FEATURES[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEATURES[$i]}"
    done
    COMBOS+=("$combo")
  done
else
  COMBOS+=("none")   # --no-default-features, equivalent here but verified anyway
fi
step "feature combinations to verify: ${COMBOS[*]}  (declared features: ${FEATURES[*]:-<none>})"

feature_flags() {
  case "$1" in
    default) echo "" ;;
    none)    echo "--no-default-features" ;;
    *)       echo "--no-default-features --features $1" ;;
  esac
}

# ------------------------------------------------------------ cargo check ----
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  if timeout "$TIMEOUT" cargo check --manifest-path "$CRATE/Cargo.toml" \
       $(feature_flags "$combo") >/dev/null 2>&1; then
    ok "cargo check [$combo]"
  else
    bad "cargo check [$combo]"
  fi
done

# --------------------------------------------- build + test each config ------
for combo in "${COMBOS[@]}"; do
  flags="$(feature_flags "$combo")"
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"
    step "config: features=$combo profile=$profile"

    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo build --manifest-path "$CRATE/Cargo.toml" \
           $flags $relflag >/dev/null 2>&1; then
      bad "cargo build [$combo/$profile]"; continue
    fi
    RUST_SO="$CRATE/target/$profile/libflac_validate_lib.so"
    [ -f "$RUST_SO" ] || { bad "missing $RUST_SO"; continue; }

    # symbol parity, checked directly here as well as inside the test suite
    missing="$(comm -13 \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u))"
    if [ -z "$missing" ]; then
      ok "symbol parity [$combo/$profile]"
    else
      bad "symbols missing from Rust .so [$combo/$profile]: $(echo "$missing" | tr '\n' ' ')"
    fi

    # shellcheck disable=SC2086
    if C_SO="$C_SO" RUST_SO="$RUST_SO" \
       timeout "$TIMEOUT" cargo test --manifest-path "$CRATE/Cargo.toml" \
         $flags -- --test-threads=4 >"/tmp/vt-$combo-$profile.log" 2>&1; then
      ok "cargo test [$combo/$profile]  ($(grep -c '^test .* ok$' "/tmp/vt-$combo-$profile.log") tests)"
    else
      bad "cargo test [$combo/$profile] — see /tmp/vt-$combo-$profile.log"
      tail -30 "/tmp/vt-$combo-$profile.log"
    fi
  done
done

step "summary"
if [ "$fails" -eq 0 ]; then
  printf '\033[32mall configurations verified\033[0m\n'
else
  printf '\033[31m%d check(s) failed\033[0m\n' "$fails"
fi
exit "$fails"
