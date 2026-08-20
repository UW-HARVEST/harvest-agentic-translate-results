#!/bin/bash
# Full verification driver:
#   1. build the C shared library (default cmake configuration)
#   2. enumerate every valid feature combination from Cargo.toml
#   3. `cargo check` each combination
#   4. build the cdylib and run the whole differential suite for each combination,
#      in both the dev and the release profile (release adds opt-level + panic=abort)
#   5. diff `nm -D` between the C and the Rust shared libraries
set -u
cd "$(dirname "$0")"
TMP="${TMPDIR:-/tmp}"
rc=0

echo "=============== 1. build C shared library ==============="
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$TMP/cmake.log" 2>&1 \
  && cmake --build . >>"$TMP/cmake.log" 2>&1 ) || { tail -20 "$TMP/cmake.log"; exit 1; }
ls -l c_src/build/libSieve.so

echo
echo "=============== 2. feature combinations ================="
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*(.*?)(^\[|\Z)', txt, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name and name != 'default':
                feats.append(name)
if len(feats) > 10:
    print("too many features to enumerate exhaustively", file=sys.stderr)
for n in range(len(feats) + 1):
    for c in itertools.combinations(feats, n):
        print(','.join(c))
PY
)
echo "Cargo.toml declares ${#COMBOS[@]} feature combination(s) (empty line = no features):"
for c in "${COMBOS[@]}"; do echo "  --no-default-features --features '$c'"; done

echo
echo "=============== 3. cargo check per combination =========="
for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then args=(--no-default-features); else args=(--no-default-features --features "$c"); fi
  for extra in "" "--tests"; do
    if timeout 600 cargo check "${args[@]}" $extra --offline >"$TMP/check.log" 2>&1; then
      echo "  OK   cargo check ${args[*]} $extra"
    else
      echo "  FAIL cargo check ${args[*]} $extra"; tail -20 "$TMP/check.log"; rc=1
    fi
  done
done
# also the default feature set, as a consumer would build it
if timeout 600 cargo check --all-targets --offline >"$TMP/check.log" 2>&1; then
  echo "  OK   cargo check --all-targets (default features)"
else
  echo "  FAIL cargo check --all-targets (default features)"; tail -20 "$TMP/check.log"; rc=1
fi

echo
echo "=============== 4. differential suite per combination ==="
for profile in dev release; do
  for c in "${COMBOS[@]}"; do
    if [ -z "$c" ]; then args=(--no-default-features); else args=(--no-default-features --features "$c"); fi
    if [ "$profile" = release ]; then args+=(--release); fi
    # build the cdylib first: `cargo test` does NOT rebuild cdylib targets
    timeout 600 cargo build "${args[@]}" --offline >"$TMP/build.log" 2>&1 \
      || { echo "  FAIL cargo build ${args[*]}"; tail -20 "$TMP/build.log"; rc=1; continue; }
    if timeout 600 cargo test "${args[@]}" --offline -- --test-threads=1 >"$TMP/test.log" 2>&1; then
      echo "  OK   cargo test ${args[*]}  ($(grep -c '^test .* ok$' "$TMP/test.log") tests passed)"
    else
      echo "  FAIL cargo test ${args[*]}"; grep -E "^test .*FAILED|^test result|panicked" "$TMP/test.log" | head -20; rc=1
    fi
  done
done

echo
echo "=============== 5. nm -D symbol parity ================="
for profile in debug release; do
  so="target/$profile/libSieve.so"
  [ -f "$so" ] || continue
  nm -D --defined-only c_src/build/libSieve.so | awk '{print $NF}' | sort -u >"$TMP/c.syms"
  nm -D --defined-only "$so"                   | awk '{print $NF}' | sort -u >"$TMP/r.syms"
  missing=$(comm -23 "$TMP/c.syms" "$TMP/r.syms")
  echo "  C exports:    $(tr '\n' ' ' <"$TMP/c.syms")"
  echo "  Rust ($profile) exports: $(tr '\n' ' ' <"$TMP/r.syms")"
  if [ -z "$missing" ]; then
    echo "  OK   no C symbol is missing from the Rust .so ($profile)"
  else
    echo "  FAIL missing from Rust .so ($profile): $missing"; rc=1
  fi
  if ldd -r "$so" 2>&1 | grep -q "undefined symbol"; then
    echo "  FAIL unresolved symbols in $so"; ldd -r "$so" 2>&1 | grep "undefined symbol" | head; rc=1
  else
    echo "  OK   no unresolved symbols in $so"
  fi
done

echo
echo "========================================================"
if [ "$rc" -eq 0 ]; then echo "VERIFY: ALL CHECKS PASSED"; else echo "VERIFY: FAILURES PRESENT"; fi
exit "$rc"
