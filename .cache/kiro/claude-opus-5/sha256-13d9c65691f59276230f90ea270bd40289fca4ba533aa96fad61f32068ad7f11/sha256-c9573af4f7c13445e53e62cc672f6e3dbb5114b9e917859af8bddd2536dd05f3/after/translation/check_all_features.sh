#!/usr/bin/env bash
# Phase D driver: build both libraries, enumerate every Cargo feature
# combination declared in Cargo.toml, and run `cargo check` + the full
# differential test-suite under each one.
#
#   ./check_all_features.sh
#
# Every command is wrapped in `timeout` so no single step can hang the run.
set -uo pipefail

cd "$(dirname "$0")"
CRATE_DIR="$PWD"
C_DIR="$CRATE_DIR/../c_src"
FAIL=0
TIMEOUT=600

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C ground truth and the Rust cdylib
# ---------------------------------------------------------------------------
say "building C libdriver.so"
timeout $TIMEOUT bash -c "mkdir -p '$C_DIR/build' && cd '$C_DIR/build' && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . " \
  || { echo "C build FAILED"; exit 1; }

say "building Rust libdriver.so (release)"
timeout $TIMEOUT cargo build --release >/dev/null 2>&1 \
  || { echo "cargo build --release FAILED"; exit 1; }

# ---------------------------------------------------------------------------
# 2. Symbol diff (the SYMBOLS.md check, done here too so it is scriptable)
# ---------------------------------------------------------------------------
say "nm -D symbol diff"
nm -D --defined-only "$C_DIR/build/libdriver.so"        | awk '{print $NF}' | sort > /tmp/sym_c.$$
nm -D --defined-only "$CRATE_DIR/target/release/libdriver.so" | awk '{print $NF}' | sort > /tmp/sym_r.$$
if diff -u /tmp/sym_c.$$ /tmp/sym_r.$$; then
  echo "symbol diff EMPTY: $(wc -l < /tmp/sym_c.$$) exported symbols match"
else
  echo "SYMBOL PARITY FAILURE"; FAIL=1
fi
rm -f /tmp/sym_c.$$ /tmp/sym_r.$$

# ---------------------------------------------------------------------------
# 3. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[a-zA-Z0-9_-]+[ ]*=/ { split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1]!="default") print a[1] }
' Cargo.toml | sort -u)

say "declared features"
if [ -z "$FEATURES" ]; then
  echo "(none - the crate declares no [features], so 'default' is the only configuration)"
else
  echo "$FEATURES"
fi

# Build the list of feature sets to exercise: the powerset of declared
# features, plus the default build.
COMBOS=()
COMBOS+=("__default__")
COMBOS+=("__none__")
if [ -n "$FEATURES" ]; then
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    set=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then set="${set:+$set,}${FARR[$b]}"; fi
    done
    [ -n "$set" ] && COMBOS+=("$set")
  done
fi

# ---------------------------------------------------------------------------
# 4. cargo check + cargo test for every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) ARGS=(); label="default features" ;;
    __none__)    ARGS=(--no-default-features); label="--no-default-features" ;;
    *)           ARGS=(--no-default-features --features "$combo"); label="--features $combo" ;;
  esac

  say "cargo check ($label)"
  if ! timeout $TIMEOUT cargo check --release "${ARGS[@]}" 2>&1 | tail -5; then
    echo "cargo check FAILED for $label"; FAIL=1; continue
  fi

  # MUST come before cargo test: `cargo test` builds only the test harnesses,
  # never the cdylib, so without this the suite would load a stale .so.
  say "cargo build --release ($label)"
  timeout $TIMEOUT cargo build --release "${ARGS[@]}" >/dev/null 2>&1 \
    || { echo "build FAILED for $label"; FAIL=1; continue; }

  say "cargo test ($label)"
  # --test-threads=1: the harness redirects fd 1/2, chdir's and sets
  # $LOG_FILE/$MAX_TASKS, all of which are process-wide.
  timeout $TIMEOUT cargo test --release "${ARGS[@]}" -- --test-threads=1 >/tmp/ct.$$ 2>&1
  rc=$?
  grep -E '^(     Running|test result|test .* FAILED|error)' /tmp/ct.$$ \
    | sed 's/^     Running /  /'
  echo "  total: $(grep -c '^test .* \.\.\. ok$' /tmp/ct.$$) passed, \
$(grep -c '^test .* \.\.\. FAILED$' /tmp/ct.$$) failed"
  if [ $rc -ne 0 ]; then
    echo "TESTS FAILED for $label"; sed -n '/^failures:/,$p' /tmp/ct.$$ | head -40; FAIL=1
  fi
  rm -f /tmp/ct.$$
done

say "summary"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED (${#COMBOS[@]} feature sets)"
else
  echo "FAILURES PRESENT"
fi
exit $FAIL
