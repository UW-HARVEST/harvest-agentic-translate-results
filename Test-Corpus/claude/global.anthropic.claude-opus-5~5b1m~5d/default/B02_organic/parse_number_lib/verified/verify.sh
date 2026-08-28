#!/usr/bin/env bash
# Full verification sweep: builds the C and Rust shared objects, diffs their
# dynamic symbol tables, then runs the whole differential suite under every
# feature combination x profile.
#
# Usage: ./verify.sh
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
CARGO_OFFLINE="${CARGO_OFFLINE:---offline}"
LOGDIR="${TMPDIR:-/tmp}/verify-logs"
mkdir -p "$LOGDIR"
rc=0

say() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL\033[0m %s\n' "$*"; rc=1; }
ok() { printf '\033[32mOK\033[0m   %s\n' "$*"; }

# ---------------------------------------------------------------- build C ----
say "Building C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$LOGDIR"/cbuild.log 2>&1 \
  || { cat "$LOGDIR"/cbuild.log; fail "C build"; exit 1; }
C_SO="$root/c_src/build/libdriver.so"
[[ -f $C_SO ]] || { fail "missing $C_SO"; exit 1; }
ok "$C_SO"

# ------------------------------------------------------- feature combos ------
# Enumerate the [features] table from Cargo.toml (there may be none).
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/{print $1}' \
    "$here/Cargo.toml"
)
say "Declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

# Build the combination list: default, no-default-features, all-features, and
# every individual feature plus the full power set when it is small enough.
COMBOS=("" "--no-default-features" "--all-features")
if (( ${#FEATURES[@]} > 0 && ${#FEATURES[@]} <= 8 )); then
  n=${#FEATURES[@]}
  for (( mask=1; mask < (1<<n); mask++ )); do
    sel=()
    for (( b=0; b<n; b++ )); do
      (( mask & (1<<b) )) && sel+=("${FEATURES[b]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi

# ---------------------------------------------------------------- sweep ------
for profile in debug release; do
  prof_flag=""
  [[ $profile == release ]] && prof_flag="--release"

  for combo in "${COMBOS[@]}"; do
    label="profile=$profile combo=${combo:-<default>}"
    say "$label"

    # The cdylib must exist for the profile under test, because the harness
    # dlopen()s the artifact rather than linking it.
    # shellcheck disable=SC2086
    if ! cargo build $CARGO_OFFLINE $prof_flag $combo >"$LOGDIR"/rbuild.log 2>&1; then
      tail -30 "$LOGDIR"/rbuild.log; fail "cargo build [$label]"; continue
    fi
    RUST_SO="$here/target/$profile/libdriver.so"
    if [[ ! -f $RUST_SO ]]; then fail "missing $RUST_SO [$label]"; continue; fi

    # ---- symbol parity -----------------------------------------------------
    missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))"
    if [[ -n $missing ]]; then
      fail "symbols exported by C but not Rust [$label]:"; printf '  %s\n' $missing
    else
      ok "symbol parity: 0 missing [$label]"
    fi

    unresolved="$(ldd -r "$RUST_SO" 2>&1 | grep 'undefined symbol' || true)"
    if [[ -n $unresolved ]]; then
      fail "unresolved symbols in Rust .so [$label]"; printf '  %s\n' "$unresolved"
    else
      ok "no unresolved symbols [$label]"
    fi

    # ---- differential suite ------------------------------------------------
    # shellcheck disable=SC2086
    if RUST_DRIVER_SO="$RUST_SO" timeout 600 \
         cargo test $CARGO_OFFLINE $prof_flag $combo -- --test-threads=4 \
         >"$LOGDIR"/rtest.log 2>&1; then
      ok "tests passed [$label]  ($(grep -c '^test .* ok$' "$LOGDIR"/rtest.log) cases)"
    else
      tail -60 "$LOGDIR"/rtest.log; fail "tests [$label]"
    fi

    # ---- heavy exhaustive sweeps (~4.4M differential calls) ----------------
    # shellcheck disable=SC2086
    if RUST_DRIVER_SO="$RUST_SO" timeout 600 \
         cargo test $CARGO_OFFLINE $prof_flag $combo --test heavy_exhaustive -- \
         --ignored --nocapture --test-threads=4 \
         >"$LOGDIR"/rheavy.log 2>&1; then
      ok "heavy sweeps passed [$label]"
      grep -E '0 divergences' "$LOGDIR"/rheavy.log | sed 's/^/       /'
    else
      tail -60 "$LOGDIR"/rheavy.log; fail "heavy sweeps [$label]"
    fi
  done
done

say "RESULT"
if (( rc == 0 )); then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit $rc
