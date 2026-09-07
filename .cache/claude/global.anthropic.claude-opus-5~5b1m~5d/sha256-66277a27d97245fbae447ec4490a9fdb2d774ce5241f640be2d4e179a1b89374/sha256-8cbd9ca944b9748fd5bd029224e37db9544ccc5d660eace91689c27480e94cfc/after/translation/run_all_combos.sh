#!/usr/bin/env bash
# Phase D — run the full differential suite under every feature combination and
# both optimisation profiles.
#
# `Cargo.toml` declares no `[features]` table and no optional dependencies, so
# the complete set of combinations is {default, no-default-features,
# all-features}; they are all enumerated anyway so that adding a feature later
# cannot silently go untested.
set -u
cd "$(dirname "$0")"

echo "== feature combinations declared in Cargo.toml =="
if grep -q '^\[features\]' Cargo.toml; then
  sed -n '/^\[features\]/,/^\[/p' Cargo.toml
else
  echo "(none)"
fi
echo

# Rebuild the C reference library first.
( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

fail=0
for profile in release debug; do
  for combo in "" "--no-default-features" "--all-features"; do
    [ "$profile" = release ] && prof_flag="--release" || prof_flag=""
    label="profile=$profile features=${combo:-default}"
    echo "---- $label ----"
    # shellcheck disable=SC2086
    if timeout 580 cargo test $prof_flag $combo 2>&1 | tee /dev/stderr \
         | grep -q '^test result: FAILED'; then
      echo "FAILED: $label"
      fail=1
    else
      echo "OK: $label"
    fi
    echo
  done
done

echo "== symbol parity =="
syms() { nm -D --defined-only "$1" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort; }
echo "C-only symbols (must be empty):"
missing=$(comm -23 <(syms ../c_src/build/libdriver.so) <(syms target/release/libdriver.so))
printf '%s\n' "$missing"
[ -n "$missing" ] && fail=1

[ $fail -eq 0 ] && echo "ALL COMBINATIONS PASSED" || echo "SOME COMBINATIONS FAILED"
exit $fail
