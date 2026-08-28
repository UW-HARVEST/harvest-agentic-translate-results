#!/usr/bin/env bash
# Full verification driver: builds the C and Rust shared objects, then runs the
# Phase B/C/D differential suites under EVERY Cargo feature combination.
#
# Feature combinations are enumerated MECHANICALLY from Cargo.toml (never
# hard-coded), so a feature added later is picked up automatically.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
fails=0

echo "############ 1. build the C shared library ############"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . ) >/dev/null 2>&1
C_SO="$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)"
if [ -z "$C_SO" ]; then echo "FATAL: C .so was not built"; exit 1; fi
echo "  C .so = $C_SO"
echo "  exports: $(nm -D --defined-only "$C_SO" | awk '{print $NF}' | tr '\n' ' ')"

echo
echo "############ 2. enumerate feature combinations ############"
FEATURES=()
while IFS= read -r line; do
  [ -n "$line" ] && FEATURES+=("$line")
done < <(python3 - "$HERE/Cargo.toml" <<'PY'
import re, sys
txt = open(sys.argv[1]).read()
# Grab the [features] table, if any.
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
print('\n'.join(feats))
PY
)
NFEAT=${#FEATURES[@]}
if [ "$NFEAT" -eq 0 ]; then
  echo "  optional features declared in Cargo.toml: 0 (none)"
  echo "  -> Cargo.toml has no [features] table, so the default (empty) feature"
  echo "     set is the ONLY configuration; there is no combinatorial surface."
else
  echo "  optional features declared in Cargo.toml: $NFEAT (${FEATURES[*]})"
fi

# Build the list of cargo flag-sets to test.
COMBOS=()
COMBOS+=("default:")                                   # default feature set
COMBOS+=("no-default:--no-default-features")
if [ "$NFEAT" -gt 0 ]; then
  COMBOS+=("all:--all-features")
  # Every non-empty subset of the optional features, on top of no-default.
  total=$(( (1 << NFEAT) - 1 ))
  for mask in $(seq 1 "$total"); do
    sel=()
    for i in $(seq 0 $((NFEAT - 1))); do
      if (( (mask >> i) & 1 )); then sel+=("${FEATURES[$i]}"); fi
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("$joined:--no-default-features --features $joined")
  done
fi
echo "  combinations to verify: ${#COMBOS[@]}"

echo
echo "############ 3. verify each combination ############"
for entry in "${COMBOS[@]}"; do
  name="${entry%%:*}"; flags="${entry#*:}"
  echo
  echo "---- combo [$name]  cargo flags: ${flags:-<none>} ----"

  if ! ( cd "$HERE" && cargo build -q $flags 2>&1 ); then
    echo "  FAIL: debug build"; fails=$((fails+1)); continue
  fi
  if ! ( cd "$HERE" && cargo build -q --release $flags 2>&1 ); then
    echo "  FAIL: release build"; fails=$((fails+1)); continue
  fi

  # --- symbol diff must be EMPTY ---
  dbg="$HERE/target/debug/librgb_to_hsv_lib.so"
  rel="$HERE/target/release/librgb_to_hsv_lib.so"
  for so in "$dbg" "$rel"; do
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
      <(nm -D --defined-only "$so"   | awk '{print $NF}' | sort))
    if [ -n "$missing" ]; then
      echo "  FAIL: $(basename "$so") missing symbols: $(echo "$missing" | tr '\n' ' ')"
      fails=$((fails+1))
    fi
  done
  echo "  symbol diff: EMPTY (debug + release)"

  # --- all differential suites, no-fail-fast so nothing is skipped ---
  # Run twice: once against the unoptimised (debug) Rust artifact and once
  # against the OPTIMISED release artifact, which is what actually ships.
  # Optimisation must not perturb a single result bit.
  for art in debug release; do
    case "$art" in
      debug)   rust_so="$dbg" ;;
      release) rust_so="$rel" ;;
    esac
    log="$HERE/target/verify-$name-$art.log"
    ( cd "$HERE" && HARVEST_C_SO="$C_SO" HARVEST_RUST_SO="$rust_so" \
        cargo test -q --no-fail-fast $flags 2>&1 ) > "$log"
    if grep -q 'test result: FAILED' "$log"; then
      echo "  FAIL: tests failed for combo [$name] against $art Rust .so"
      sed -n '/^failures:$/,$p' "$log" | grep -E '^    [a-z]' | sort -u | sed 's/^/      /'
      fails=$((fails+1))
    else
      passed=$(grep -oE 'test result: ok\. [0-9]+ passed' "$log" \
               | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')
      echo "  tests vs $art Rust .so: ${passed:-0} passed, 0 failed"
    fi
  done
done

echo
echo "############ 3b. cross-check against an -O2 C reference ############"
# The graded reference is the CMake build (no -O flags, i.e. -O0). Building the
# same untouched c_src/src/lib.c at -O2 into target/ (never into c_src/) and
# re-running every row proves the agreement is not an artefact of one particular
# C codegen choice -- e.g. that no result depends on x87 excess precision or on
# a compiler-chosen NaN operand order.
O2_SO="$HERE/target/cref-O2.so"
if cc -O2 -fPIC -shared -I"$ROOT/c_src/include" \
      "$ROOT/c_src/src/lib.c" -o "$O2_SO" 2>/dev/null; then
  log="$HERE/target/verify-cref-O2.log"
  ( cd "$HERE" && HARVEST_C_SO="$O2_SO" HARVEST_RUST_SO="$rel" \
      cargo test -q --no-fail-fast 2>&1 ) > "$log"
  if grep -q 'test result: FAILED' "$log"; then
    echo "  FAIL: Rust diverges from the -O2 C build"
    sed -n '/^failures:$/,$p' "$log" | grep -E '^    [a-z]' | sort -u | sed 's/^/      /'
    fails=$((fails+1))
  else
    passed=$(grep -oE 'test result: ok\. [0-9]+ passed' "$log" \
             | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')
    echo "  release Rust .so vs -O2 C .so: ${passed:-0} passed, 0 failed"
  fi
else
  echo "  SKIP: no C compiler available for the -O2 cross-check"
fi

echo
echo "############ 4. mutation (negative control) ############"
if ( cd "$HERE" && ./mutation_check.sh > "$HERE/target/mutation.log" 2>&1 ); then
  echo "  mutation check PASSED ($(grep -c '^  OK' "$HERE/target/mutation.log") mutants classified correctly)"
else
  echo "  mutation check FAILED -- see target/mutation.log"; fails=$((fails+1))
fi

echo
echo "########################################################"
if [ "$fails" -eq 0 ]; then
  echo "ALL CHECKS PASSED across ${#COMBOS[@]} feature combination(s)."
  exit 0
fi
echo "$fails CHECK(S) FAILED"
exit 1
