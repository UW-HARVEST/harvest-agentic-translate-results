#!/usr/bin/env bash
# Build both libraries and run the whole differential suite.
#
#   ./run_all.sh            # default features
#   ./run_all.sh --quick    # skip the slowest test binaries
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "=== building C reference shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . -j"$(nproc)" >/dev/null )

echo "=== building Rust shared library (release: overflow-checks off, like C) ==="
timeout 600 cargo build --release

echo "=== regenerating ERRORS.md / errors_index.tsv from the C sources ==="
python3 ./gen_errors.py

echo "=== symbol parity ==="
bash ./symcheck.sh

# Every cargo feature combination.  The crate declares no [features] table, so
# the only combination is the default one; the loop still enumerates whatever
# Cargo reports so new features are picked up automatically.
COMBOS=$(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name != 'default':
                feats.append(name)
if not feats:
    print('__default__')
else:
    import itertools
    print('__default__')
    print('__none__')
    for r in range(1, len(feats) + 1):
        for c in itertools.combinations(feats, r):
            print(','.join(c))
PY
)

# Binaries that redirect fd 1 must not run in parallel with the test harness.
SERIAL_TESTS="b_stdout"   # redirect fd 1 -> must not run in parallel

run_combo() {
  local combo="$1"
  local args=()
  case "$combo" in
    __default__) ;;
    __none__)    args=(--no-default-features) ;;
    *)           args=(--no-default-features --features "$combo") ;;
  esac
  echo "=== cargo check (${combo}) ==="
  timeout 600 cargo check "${args[@]}" >/dev/null
  echo "=== tests (${combo}) ==="
  for t in $(ls tests/*.rs | xargs -n1 basename | sed 's/\.rs$//'); do
    extra=()
    for s in $SERIAL_TESTS; do
      [ "$t" = "$s" ] && extra=(-- --test-threads=1)
    done
    echo "--- $t ---"
    timeout 600 cargo test --release "${args[@]}" --test "$t" "${extra[@]}" 2>&1 \
      | grep -E '^(test |running|test result|error)' || true
  done
}

for combo in $COMBOS; do
  run_combo "$combo"
done

echo "=== done ==="

# Optional: prove the suite discriminates.  ./mutants.sh runs a mutation sweep
# (each mutation must make at least one test binary fail).  It is slow, so it is
# not part of the default run:
#   ./mutants.sh          # all mutations
#   ./mutants.sh 7        # a single mutation
