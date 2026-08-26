#!/usr/bin/env bash
#
# Build the C .so and the Rust cdylib, then run the whole differential suite for
# every valid feature combination x profile.
#
#   ./run_all.sh
#
# Phase A established that there is exactly ONE valid build configuration (no
# [features] in Cargo.toml, no build.rs, no #ifdef in the C, no CMake options),
# but the combination list below is derived from Cargo.toml programmatically so
# this script stays correct if features are ever added.

set -uo pipefail
cd "$(dirname "$0")"

log() { printf '\n=== %s\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. C shared object  ->  c_src/build/libtranslated_rust.so
# ---------------------------------------------------------------------------
log "building the C shared library"
mkdir -p c_src/build
if ! ( cd c_src/build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
         && cmake --build . >/dev/null ); then
  echo "C build FAILED" >&2
  exit 1
fi
ls -l c_src/build/libtranslated_rust.so

# ---------------------------------------------------------------------------
# 2. enumerate every valid feature combination (powerset of [features])
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import itertools, re
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name and name != 'default':
                feats.append(name)
combos = {",".join(c) for r in range(len(feats) + 1)
                     for c in itertools.combinations(feats, r)}
for c in sorted(combos):
    print(c)
PY
)
log "feature combinations to verify: ${#FEATURES[@]}"
for f in "${FEATURES[@]}"; do echo "  '${f:-<none>}'"; done

# ---------------------------------------------------------------------------
# 3. check + test every combination x profile
# ---------------------------------------------------------------------------
RC=0
for prof in "" "--release"; do
  for feat in "${FEATURES[@]}"; do
    label="profile=${prof:---dev} features='${feat:-<none>}'"

    log "cargo check --tests  [$label]"
    if ! cargo check --offline --tests --no-default-features --features "$feat" \
           $prof 2>&1 | tail -3; then
      echo "CHECK FAILED: $label" >&2; RC=1; continue
    fi

    # the tests dlopen target/<profile>/libintput_lib.so, so build it first
    if ! cargo build --offline --no-default-features --features "$feat" \
           $prof >/dev/null 2>&1; then
      echo "BUILD FAILED: $label" >&2; RC=1; continue
    fi

    log "cargo test  [$label]"
    out=$(timeout 600 cargo test --offline --no-default-features \
            --features "$feat" $prof 2>&1)
    status=$?
    echo "$out" | grep -E '^(running|test result)|FAILED|panicked'
    if [ $status -ne 0 ]; then
      echo "TEST FAILED: $label" >&2
      echo "$out" | tail -40 >&2
      RC=1
    else
      total=$(echo "$out" | grep -oE '^test result: ok\. [0-9]+ passed' \
                | awk '{s+=$4} END {print s+0}')
      echo "  -> $total tests passed [$label]"
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. symbol parity (also asserted by tests/d_symbols.rs)
# ---------------------------------------------------------------------------
log "nm -D symbol parity"
for p in debug release; do
  so="target/$p/libintput_lib.so"
  [ -f "$so" ] || continue
  nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort >"${TMPDIR:-/tmp}/c.syms"
  nm -D --defined-only "$so" | grep -v ' [wWuv] ' | awk '{print $NF}' | sort >"${TMPDIR:-/tmp}/r.syms"
  if diff -u "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms" >/dev/null; then
    echo "  $p: IDENTICAL ($(wc -l < "${TMPDIR:-/tmp}/c.syms") symbols)"
  else
    echo "  $p: SYMBOL DIFF" >&2
    diff -u "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms" >&2
    RC=1
  fi
done

echo
if [ $RC -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $RC
