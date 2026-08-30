#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination
# and under both cargo profiles.
#
# Feature names are extracted mechanically from Cargo.toml rather than
# hard-coded, so a newly added feature is picked up automatically.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
cd "$HERE"

# --- make sure the C reference library exists -------------------------------
if [[ ! -f "$ROOT/c_src/build/libStaticLoop.so" ]]; then
  echo "### building the C reference library"
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build failed" >&2; exit 2; }
fi

# --- enumerate declared features -------------------------------------------
# Everything under [features] up to the next section header, minus "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      line = $0
      sub(/[[:space:]]*=.*/, "", line)
      gsub(/[[:space:]]/, "", line)
      if (line != "default") print line
    }
  ' Cargo.toml | sort -u
)

echo "### declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the list of combinations to test --------------------------------
# Always include the default build. If features exist, add: no-default, each
# feature alone, and the full power set (capped for sanity).
COMBOS=()               # each entry is a set of extra cargo flags
COMBO_NAMES=()

COMBOS+=("");                       COMBO_NAMES+=("default")

if [[ ${#FEATURES[@]} -gt 0 ]]; then
  COMBOS+=("--no-default-features"); COMBO_NAMES+=("no-default-features")
  n=${#FEATURES[@]}
  total=$((1 << n))
  if [[ $total -gt 64 ]]; then
    echo "### note: $n features -> $total subsets; testing singles + all-features only"
    for f in "${FEATURES[@]}"; do
      COMBOS+=("--no-default-features --features $f"); COMBO_NAMES+=("only:$f")
    done
    COMBOS+=("--all-features");     COMBO_NAMES+=("all-features")
  else
    for ((mask = 0; mask < total; mask++)); do
      set=""
      for ((b = 0; b < n; b++)); do
        if (( mask & (1 << b) )); then set="${set:+$set,}${FEATURES[b]}"; fi
      done
      if [[ -z "$set" ]]; then continue; fi
      COMBOS+=("--no-default-features --features $set"); COMBO_NAMES+=("$set")
    done
    COMBOS+=("--all-features");     COMBO_NAMES+=("all-features")
  fi
fi

# --- run every combination under both profiles ------------------------------
overall=0
for profile_flag in "" "--release"; do
  profile_dir="debug"; [[ -n "$profile_flag" ]] && profile_dir="release"
  for i in "${!COMBOS[@]}"; do
    flags="${COMBOS[$i]}"
    name="${COMBO_NAMES[$i]}"
    echo
    echo "=============================================================="
    echo "### profile=$profile_dir features=$name"
    echo "=============================================================="

    # cargo test does not emit the cdylib, so build it explicitly first.
    # shellcheck disable=SC2086
    if ! cargo build --offline $profile_flag $flags 2>&1 | tail -3; then
      echo "BUILD FAILED (profile=$profile_dir features=$name)" >&2
      overall=1; continue
    fi

    # shellcheck disable=SC2086
    if timeout 600 cargo test --offline --no-fail-fast $profile_flag $flags 2>&1 \
        | grep -E "^(test result|error|warning: unused)" ; then
      :
    fi
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test --offline --no-fail-fast $profile_flag $flags >/dev/null 2>&1; then
      echo "TESTS FAILED (profile=$profile_dir features=$name)" >&2
      overall=1
    fi

    if ! "$HERE/check_symbols.sh" "$profile_dir" | tail -1; then
      echo "SYMBOL PARITY FAILED (profile=$profile_dir features=$name)" >&2
      overall=1
    fi
  done
done

echo
if [[ "$overall" -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS x PROFILES: PASS"
else
  echo "SOME COMBINATIONS FAILED" >&2
fi
exit "$overall"
