#!/usr/bin/env bash
# Full verification driver: enumerates every build-time configuration and runs
# Phases B, C and D against each of them.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }

# Scratch dir that is guaranteed writable (sandboxes often mount /tmp read-only).
WORKTMP="${TMPDIR:-/tmp}"
mkdir -p "$WORKTMP" 2>/dev/null
if ! : > "$WORKTMP/.wtest" 2>/dev/null; then
  WORKTMP="$PWD/target/verify-tmp"; mkdir -p "$WORKTMP"
fi
rm -f "$WORKTMP/.wtest"
echo "scratch dir: $WORKTMP"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating [features] from Cargo.toml"
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "" && a[1] !~ /^#/) print a[1]}
' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "No [features] section -> exactly one valid feature combination: <none>"
  COMBOS=("")
else
  echo "Declared features: $FEATURES"
  # power set
  read -r -a F <<<"$(echo "$FEATURES" | tr '\n' ' ')"
  n=${#F[@]}
  COMBOS=()
  for ((m = 0; m < (1 << n); m++)); do
    c=""
    for ((i = 0; i < n; i++)); do
      if (((m >> i) & 1)); then c="${c:+$c,}${F[i]}"; fi
    done
    COMBOS+=("$c")
  done
fi
echo "Combination count: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. cargo check every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  step "cargo check --no-default-features --features '${combo:-<none>}'"
  if [ -z "$combo" ]; then
    cargo check --offline --no-default-features --all-targets 2>&1 | tail -5 || FAIL=1
  else
    cargo check --offline --no-default-features --features "$combo" --all-targets 2>&1 | tail -5 || FAIL=1
  fi
  # shellcheck disable=SC2181
  [ "${PIPESTATUS[0]}" = 0 ] || FAIL=1
done

# ---------------------------------------------------------------------------
# 3. Build the C reference shared library (default configuration)
# ---------------------------------------------------------------------------
step "Building C reference .so (default configuration)"
(mkdir -p c_src/build && cd c_src/build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null) || FAIL=1
ls -l c_src/build/libtranslated_rust.so || FAIL=1

# ---------------------------------------------------------------------------
# 4. Phases B + C + D for every combination, in both profiles
# ---------------------------------------------------------------------------
for profile in debug release; do
  RELFLAG=""
  [ "$profile" = release ] && RELFLAG="--release"
  for combo in "${COMBOS[@]}"; do
    FEATFLAG=()
    [ -n "$combo" ] && FEATFLAG=(--features "$combo")
    step "profile=$profile features='${combo:-<none>}' : build cdylib"
    cargo build --offline --no-default-features $RELFLAG "${FEATFLAG[@]}" 2>&1 | tail -3
    [ "${PIPESTATUS[0]}" = 0 ] || FAIL=1

    step "profile=$profile features='${combo:-<none>}' : nm -D symbol diff"
    CSYMS="$WORKTMP/csyms.$$"; RSYMS="$WORKTMP/rsyms.$$"; SDIFF="$WORKTMP/symdiff.$$"
    nm -D --defined-only c_src/build/libtranslated_rust.so \
      | awk '{print $NF}' | grep -v '^_' | sort -u > "$CSYMS" || FAIL=1
    nm -D --defined-only "target/$profile/libfallcalc_lib.so" \
      | awk '{print $NF}' | sort -u > "$RSYMS" || FAIL=1
    nC=$(wc -l < "$CSYMS"); nR=$(wc -l < "$RSYMS")
    echo "C exports (API): $nC   Rust exports (all): $nR"
    if [ "$nC" -lt 6 ]; then
      echo "ERROR: expected >=6 C API symbols, got $nC -- nm/awk pipeline broken"; FAIL=1
    fi
    comm -23 "$CSYMS" "$RSYMS" > "$SDIFF"
    if [ ! -f "$SDIFF" ]; then
      echo "ERROR: could not write symbol diff to $SDIFF"; FAIL=1
    elif [ -s "$SDIFF" ]; then
      echo "MISSING SYMBOLS IN RUST .so:"; cat "$SDIFF"; FAIL=1
    else
      echo "symbol diff EMPTY -- all $nC C API symbols exported by Rust: OK"
      paste -sd' ' "$CSYMS"
    fi
    rm -f "$SDIFF" "$CSYMS" "$RSYMS"

    step "profile=$profile features='${combo:-<none>}' : cargo test (Phases B, C, D)"
    timeout 600 cargo test --offline --no-default-features $RELFLAG "${FEATFLAG[@]}" 2>&1 \
      | grep -E '^(test result|running|error)|FAILED|panicked'
    [ "${PIPESTATUS[0]}" = 0 ] || FAIL=1
  done
done

step "RESULT"
if [ "$FAIL" = 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES DETECTED"; fi
exit "$FAIL"
