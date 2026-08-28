#!/usr/bin/env bash
# Builds the C shared library and runs the differential test suite for every
# valid Cargo feature combination and both build profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== building C shared library =="
mkdir -p c_src/build
( cd c_src/build \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
  && timeout 600 cmake --build . >>/tmp/cmake.log 2>&1 ) \
  || { tail -40 /tmp/cmake.log; exit 1; }

cd translation

# Enumerate every feature combination declared in Cargo.toml (the power set of
# the non-meta features). With no [features] table this yields the single empty
# combination, i.e. the default configuration.
mapfile -t FEATURES < <(python3 - <<'PY'
import tomllib, itertools
d = tomllib.load(open("Cargo.toml", "rb"))
feats = [f for f in d.get("features", {}) if f != "default"]
combos = []
for r in range(len(feats) + 1):
    combos += ["".join(",".join(c)) for c in itertools.combinations(feats, r)]
print("\n".join(combos))
PY
)

status=0
for combo in "${FEATURES[@]}"; do
  label="${combo:-<none>}"
  for profile in debug release; do
    flags=(--no-default-features)
    [ -n "$combo" ] && flags+=(--features "$combo")
    [ "$profile" = release ] && flags+=(--release)

    echo "== cargo check   [features=$label profile=$profile] =="
    timeout 600 cargo check "${flags[@]}" >/tmp/check.log 2>&1 \
      || { echo "CHECK FAILED"; tail -40 /tmp/check.log; status=1; continue; }

    echo "== cargo build   [features=$label profile=$profile] =="
    timeout 600 cargo build "${flags[@]}" >/tmp/build.log 2>&1 \
      || { echo "BUILD FAILED"; tail -40 /tmp/build.log; status=1; continue; }

    echo "== nm symbol parity =="
    C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
    R_SO="target/$profile/libgotomach_lib.so"
    csyms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRWGS]$/ {print $3}' | sort -u)
    rsyms=$(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TDBRWGS]$/ {print $3}' | sort -u)
    missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
    if [ -n "$missing" ]; then
      echo "MISSING EXPORTS in $R_SO:"; echo "$missing"; status=1
    else
      echo "all $(echo "$csyms" | wc -l) C exports present"
    fi

    echo "== cargo test    [features=$label profile=$profile] =="
    RUST_TEST_THREADS=1 timeout 600 cargo test "${flags[@]}" >/tmp/test.log 2>&1 \
      || { echo "TESTS FAILED"; tail -60 /tmp/test.log; status=1; continue; }
    grep -E '^test result:' /tmp/test.log
  done
done

echo
if [ "$status" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$status"
