#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY cargo feature
# combination and under both codegen profiles.
#
# Feature combinations are extracted from Cargo.toml, not hard-coded.
set -uo pipefail
cd "$(dirname "$0")"

C_BUILD=../c_src/build
if [ ! -d "$C_BUILD" ] || ! ls "$C_BUILD"/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library =="
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || exit 1
fi

# --- enumerate features declared in Cargo.toml -----------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] table => exactly one configuration. --no-default-features is
  # still exercised explicitly to prove it is equivalent.
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
  COMBOS+=("all:--all-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    combo=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("$combo:--no-default-features --features $combo")
  done
fi

echo "== ${#COMBOS[@]} feature combination(s) x 2 profiles =="
fail=0
for entry in "${COMBOS[@]}"; do
  label=${entry%%:*}
  flags=${entry#*:}
  for profile in "" "--release"; do
    pname=${profile:---debug}
    echo
    echo "############ features=[$label] profile=${pname#--} ############"
    # the .so under test must be rebuilt with the same flags first
    if ! timeout 600 cargo build $profile $flags >/dev/null 2>&1; then
      echo "BUILD FAILED: features=[$label] $pname"; fail=1; continue
    fi
    # keep only the .so for the profile being tested so the loader picks it
    if [ -z "$profile" ]; then
      mv -f target/release/libarrayfunc_lib.so target/release/.libarrayfunc_lib.so.hidden 2>/dev/null
    fi
    if ! timeout 600 cargo test $profile $flags 2>&1 | grep -E "^(test|running|error|thread)|test result|panicked" ; then
      echo "TEST FAILED: features=[$label] $pname"; fail=1
    fi
    if [ -z "$profile" ]; then
      mv -f target/release/.libarrayfunc_lib.so.hidden target/release/libarrayfunc_lib.so 2>/dev/null
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit $fail
