#!/usr/bin/env bash
# Runs the complete verification matrix: the C library is rebuilt, then every
# feature combination declared in Cargo.toml (full powerset, plus the
# default / --no-default-features / --all-features units) is built and tested in
# both the debug and the release profile.
#
#   translation/tests/run_all.sh [extra cargo test args...]
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE=$PWD

echo "### 1. building the C shared library"
(
  cd ../c_src || exit 1
  mkdir -p build && cd build || exit 1
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
ls -1 ../c_src/build/lib*.so

echo
echo "### 2. enumerating feature combinations"
mapfile -t FEATS < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {
         split($0,a,"="); gsub(/[ \t]/,"",a[1]);
         if (a[1] != "default" && a[1] != "") print a[1] }' Cargo.toml
)
declare -a COMBOS=("" "--no-default-features" "--all-features")
n=${#FEATS[@]}
if (( n > 0 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    sel=()
    for (( i = 0; i < n; i++ )); do
      (( mask & (1 << i) )) && sel+=("${FEATS[i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
fi
# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
echo "declared features: ${FEATS[*]:-<none>}"
printf 'combination: %s\n' "${COMBOS[@]/#/}" | sed 's/combination: $/combination: <default>/'

fail=0
for profile in debug release; do
  rflag=""
  [ "$profile" = release ] && rflag="--release"
  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=${combo:-<default>}"
    echo
    echo "=============================================================="
    echo "### $label"
    echo "=============================================================="
    # shellcheck disable=SC2086
    if ! cargo check $rflag $combo --all-targets 2>&1 | tail -3; then
      echo "CHECK FAILED: $label"; fail=1; continue
    fi
    # shellcheck disable=SC2086
    if ! cargo build $rflag $combo 2>&1 | tail -3; then
      echo "BUILD FAILED: $label"; fail=1; continue
    fi
    # shellcheck disable=SC2086
    if ! cargo test $rflag $combo "$@" 2>&1 | grep -vE '^\s*$'; then
      echo "TEST FAILED: $label"; fail=1
    fi
  done
done

echo
echo "### 3. symbol diff (must be empty)"
nm -D --defined-only ../c_src/build/lib*.so | awk '{print $3}' | sort >"${TMPDIR:-/tmp}/c.syms"
for p in debug release; do
  so="target/$p/libcleanup_lib.so"
  [ -f "$so" ] || continue
  nm -D --defined-only "$so" | awk '{print $3}' | sort >"${TMPDIR:-/tmp}/r.syms"
  missing=$(comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms")
  if [ -n "$missing" ]; then
    echo "MISSING from $so:"; echo "$missing"; fail=1
  else
    echo "$so: 0 symbols missing ($(wc -l <"${TMPDIR:-/tmp}/c.syms") C exports all present)"
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
