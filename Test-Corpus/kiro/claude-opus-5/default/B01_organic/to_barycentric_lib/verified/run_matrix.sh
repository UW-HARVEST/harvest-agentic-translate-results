#!/usr/bin/env bash
# Runs the full differential suite under EVERY Cargo feature combination and
# both profiles. Feature combinations are enumerated from Cargo.toml rather than
# hard-coded, so adding a feature automatically widens the matrix.
#
# Usage: ./run_matrix.sh            (default fuzz iteration count)
#        FUZZ_ITERS=20000000 ./run_matrix.sh
set -euo pipefail
cd "$(dirname "$0")"

C_BUILD="../c_src/build"
if ! compgen -G "$C_BUILD/lib*.so" > /dev/null; then
  echo "error: C shared library missing. Build it with:" >&2
  echo "  cd ../c_src && mkdir -p build && cd build && \\" >&2
  echo "  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
  exit 1
fi

FEATS=$(cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin)["packages"][0]["features"].keys()))')
echo "== declared features: [${FEATS}]"

mapfile -t COMBOS < <(python3 -c "
import itertools
feats = '''${FEATS}'''.split()
for k in range(len(feats) + 1):
    for c in itertools.combinations(feats, k):
        print(','.join(c))
")

fail=0
for profile_flag in '' '--release'; do
  for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
      feat_flags=(--no-default-features)
      label="<no features>"
    else
      feat_flags=(--no-default-features --features "$combo")
      label="$combo"
    fi
    echo
    echo "=== profile='${profile_flag:-dev}' features='${label}' ==="
    # `cargo test` does not build the cdylib (nothing links it), so build it
    # explicitly for this exact profile/feature combination first.
    timeout 600 cargo build ${profile_flag} "${feat_flags[@]}" > /dev/null 2>&1
    if timeout 600 cargo test ${profile_flag} "${feat_flags[@]}" 2>&1 | tail -n 25; then
      :
    else
      echo "!!! FAILED: profile='${profile_flag:-dev}' features='${label}'"
      fail=1
    fi
  done
done

# Also the default feature set explicitly (identical here, but not in general).
for profile_flag in '' '--release'; do
  echo
  echo "=== profile='${profile_flag:-dev}' features='<default>' ==="
  timeout 600 cargo build ${profile_flag} > /dev/null 2>&1
  timeout 600 cargo test ${profile_flag} 2>&1 | tail -n 25 || fail=1
  echo "=== profile='${profile_flag:-dev}' features='<all>' ==="
  timeout 600 cargo build ${profile_flag} --all-features > /dev/null 2>&1
  timeout 600 cargo test ${profile_flag} --all-features 2>&1 | tail -n 25 || fail=1
done

if [ "$fail" -ne 0 ]; then
  echo
  echo "MATRIX FAILED"
  exit 1
fi
echo
echo "MATRIX OK: every feature combination x profile passed"
