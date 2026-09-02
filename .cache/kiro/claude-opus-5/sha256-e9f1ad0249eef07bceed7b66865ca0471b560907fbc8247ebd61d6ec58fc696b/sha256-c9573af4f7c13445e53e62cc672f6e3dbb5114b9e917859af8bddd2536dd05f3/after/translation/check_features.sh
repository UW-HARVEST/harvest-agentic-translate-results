#!/usr/bin/env bash
# Phase D — symbol parity + feature-combination sweep.
#
#   ./check_features.sh
#
# 1. Rebuilds the C .so and the Rust cdylib.
# 2. Diffs `nm -D` on both; the C-side symbol set must be a subset of the Rust
#    side (the diff of C-exported symbols must be EMPTY).
# 3. Enumerates every feature combination declared in Cargo.toml and runs
#    `cargo check` + the full differential suite for each.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)

echo "== building C shared library =="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }

C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "C  .so: $C_SO"

echo
echo "== enumerating feature combinations from Cargo.toml =="
mapfile -t FEATURES < <(python3 - Cargo.toml <<'PY'
import re, sys, itertools
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if '=' in line:
            name = line.split('=', 1)[0].strip().strip('"')
            if name and name != 'default':
                feats.append(name)
for r in range(len(feats) + 1):
    for combo in itertools.combinations(feats, r):
        print(','.join(combo))
PY
)
NFEAT=${#FEATURES[@]}
if [ "$NFEAT" -eq 1 ] && [ -z "${FEATURES[0]}" ]; then
  echo "no [features] table in Cargo.toml -> configurations are: default, --no-default-features"
fi
printf 'combination: %s\n' "${FEATURES[@]/#/[}" | sed 's/$/]/'

fail=0

run_config() {
  local label="$1"; shift
  echo
  echo "---- configuration: $label ----"
  if ! timeout 300 cargo check "$@" >/dev/null 2>&1; then
    echo "  cargo check FAILED"; fail=1; return
  fi
  echo "  cargo check ok"
  if ! timeout 300 cargo build --release "$@" >/dev/null 2>&1; then
    echo "  cargo build --release FAILED"; fail=1; return
  fi
  R_SO=target/release/libread_side_info_lib.so
  if [ ! -f "$R_SO" ]; then echo "  cdylib missing"; fail=1; return; fi

  # ---- symbol parity ----
  local cs rs missing
  cs=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
  rs=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$cs") <(echo "$rs"))
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY FAILED - missing from Rust .so:"
    echo "$missing" | sed 's/^/    /'
    fail=1
  else
    echo "  symbol parity ok ($(echo "$cs" | wc -l) C symbol(s), all present in Rust .so)"
  fi

  # ---- undefined non-libc symbols in the Rust .so ----
  # Keep only STRONG undefined symbols with no version suffix: everything with
  # an '@VERSION' tag comes from libc/libgcc_s, and weak ('w') entries such as
  # __gmon_start__ / _ITM_* are toolchain hooks that the C .so also has.
  local undef
  undef=$(nm -D --undefined-only "$R_SO" | awk '$1=="U" || $2=="U" {print $NF}' \
          | grep -v '@' || true)
  if [ -n "$undef" ]; then
    echo "  UNRESOLVED non-libc undefined symbols:"; echo "$undef" | sed 's/^/    /'
    fail=1
  else
    echo "  no undefined non-libc symbols (all imports are versioned libc/libgcc)"
  fi

  # ---- differential suite ----
  if timeout 600 cargo test --release "$@" >/tmp/ft.$$.log 2>&1; then
    echo "  differential suite PASSED ($(grep -c '^test .* ok$' /tmp/ft.$$.log) tests)"
  else
    echo "  differential suite FAILED"; tail -30 /tmp/ft.$$.log | sed 's/^/    /'; fail=1
  fi
  rm -f /tmp/ft.$$.log
}

run_config "default features" 
run_config "--no-default-features" --no-default-features
for combo in "${FEATURES[@]}"; do
  [ -z "$combo" ] && continue
  run_config "--no-default-features --features $combo" --no-default-features --features "$combo"
done

echo
if [ "$fail" -ne 0 ]; then echo "RESULT: FAILURES above."; exit 1; fi
echo "RESULT: symbol parity empty + differential suite green in every configuration."
