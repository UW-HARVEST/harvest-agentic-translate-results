#!/usr/bin/env bash
# Full verification run: build both libraries, then run every phase under every
# feature combination declared in translation/Cargo.toml.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
CRATE="$ROOT/translation"
fails=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
# Mechanically derived from Cargo.toml rather than hard-coded.
mapfile -t FEATURES < <(
  cd "$CRATE" && cargo metadata --format-version 1 --no-deps 2>/dev/null |
  python3 -c '
import json,sys
m=json.load(sys.stdin)
feats=set()
for p in m["packages"]:
    feats |= set(p["features"].keys())
for f in sorted(feats):
    print(f)
'
)
echo "declared features: ${FEATURES[*]:-<none>}"

# Build the list of cargo flag-sets to test. With no declared features the
# powerset is just the empty set, but --all-features / --no-default-features are
# exercised too so the claim "all combinations are identical" is checked, not
# assumed.
COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("" "--no-default-features" "--all-features")
else
  # Full powerset of the declared features, plus the default build.
  mapfile -t COMBOS < <(
    python3 - "${FEATURES[@]}" <<'PY'
import itertools, sys
feats = sys.argv[1:]
print("")
for n in range(len(feats) + 1):
    for c in itertools.combinations(feats, n):
        print("--no-default-features" + ("" if not c else " --features " + ",".join(c)))
print("--all-features")
PY
  )
fi
printf 'combination: [%s]\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  step "Combination: $label"

  # shellcheck disable=SC2086
  ( cd "$CRATE" && cargo build --release $combo >/dev/null 2>&1 \
                && cargo build           $combo >/dev/null 2>&1 ) \
    || { echo "  build FAILED"; fails=$((fails+1)); continue; }

  R_REL="$CRATE/target/release/libcolourblind_lib.so"
  R_DBG="$CRATE/target/debug/libcolourblind_lib.so"
  echo "  rust release: $(nm -D --defined-only "$R_REL" | wc -l) exported symbol(s)"
  echo "  rust debug  : $(nm -D --defined-only "$R_DBG" | wc -l) exported symbol(s)"

  # Symbol diff, computed here as well as inside the test, so the gate does not
  # depend on the test harness agreeing with itself.
  diff_out=$(diff \
    <(nm -D --defined-only --format=posix "$C_SO" | awk '{print $1}' | sed 's/@.*//' | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort) \
    <(nm -D --defined-only --format=posix "$R_REL" | awk '{print $1}' | sed 's/@.*//' | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$' | sort))
  if [ -n "$diff_out" ]; then
    echo "  SYMBOL DIFF NOT EMPTY:"; echo "$diff_out" | sed 's/^/    /'
    fails=$((fails+1))
  else
    echo "  symbol diff: EMPTY"
  fi

  for t in phase_d_symbols phase_b_valid phase_c_errors; do
    # shellcheck disable=SC2086
    if ( cd "$CRATE" && timeout 600 cargo test --test "$t" $combo >/dev/null 2>&1 ); then
      echo "  $t: PASS"
    else
      echo "  $t: FAIL"
      fails=$((fails+1))
    fi
  done
done

# ---------------------------------------------------------------------------
step "Summary"
if [ "$fails" -eq 0 ]; then
  echo "ALL CHECKS PASSED across ${#COMBOS[@]} feature combination(s)"
  exit 0
fi
echo "$fails CHECK(S) FAILED"
exit 1
