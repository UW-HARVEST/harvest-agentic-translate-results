#!/usr/bin/env bash
# Full verification matrix: every feature combination x every cargo profile,
# plus the C library rebuilt at several optimisation levels (to confirm the C
# ground truth itself is stable and that the Rust matches all of them).
#
# Usage: cd translation && ./run_all.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
TMP="${TMPDIR:-/tmp}/pow43-verify"
mkdir -p "$TMP"

fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf 'PASS  %s\n' "$*"; }
bad()  { printf 'FAIL  %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# 0. Build the C reference .so (default flags) exactly as the brief specifies.
# ---------------------------------------------------------------------------
step "build C reference (.so)"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$TMP/c-build.log" 2>&1 \
  && ok "C reference built" || { bad "C reference build"; tail -20 "$TMP/c-build.log"; }

C_SO="$(find "$ROOT/c_src/build" -name 'lib*.so' | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml (powerset).
# ---------------------------------------------------------------------------
step "enumerate feature combinations"
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]);if(a[1]!="default")print a[1]}' Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "no [features] declared -> single configuration"
  COMBOS=("" "--no-default-features" "--all-features")
else
  COMBOS=("" "--no-default-features" "--all-features")
  # shellcheck disable=SC2206
  arr=($FEATS); n=${#arr[@]}
  for ((m=1; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      (( m & (1<<i) )) && sel="$sel,${arr[i]}"
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
  done
fi
printf 'combination: [%s]\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. cargo check every combination.
# ---------------------------------------------------------------------------
step "cargo check every feature combination"
for c in "${COMBOS[@]}"; do
  # shellcheck disable=SC2086
  if timeout 300 cargo check --offline --all-targets $c >"$TMP/check.log" 2>&1; then
    ok "cargo check $c"
  else
    bad "cargo check $c"; tail -20 "$TMP/check.log"
  fi
done

# ---------------------------------------------------------------------------
# 3. Symbol parity for each combination (the .so must be rebuilt per combo).
# ---------------------------------------------------------------------------
step "symbol parity per feature combination"
for c in "${COMBOS[@]}"; do
  # shellcheck disable=SC2086
  timeout 300 cargo build --offline --release $c >"$TMP/build.log" 2>&1 || { bad "build $c"; continue; }
  RS_SO=target/release/libpow43_lib.so
  d=$(diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
           <(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort))
  if [ -z "$d" ]; then ok "symbol diff empty  [$c]"; else bad "symbol diff [$c]:"; echo "$d"; fi
done

# ---------------------------------------------------------------------------
# 4. Run the whole test suite for every combination x profile.
# ---------------------------------------------------------------------------
step "differential test suite: every combination x profile"
for c in "${COMBOS[@]}"; do
  for prof in "" "--release"; do
    # The cdylib is dlopen'd, not linked, so `cargo test` alone won't emit it.
    # shellcheck disable=SC2086
    timeout 300 cargo build --offline --lib $prof $c >"$TMP/lib.log" 2>&1 \
      || { bad "build --lib [$c] [$prof]"; tail -10 "$TMP/lib.log"; continue; }
    # shellcheck disable=SC2086
    if timeout 600 cargo test --offline $prof $c -- --test-threads=4 >"$TMP/test.log" 2>&1; then
      n=$(grep -c '^test .* ok$' "$TMP/test.log")
      ok "tests [${c:---default}] [${prof:---debug}]  ($n tests ok)"
    else
      bad "tests [${c:---default}] [${prof:---debug}]"
      grep -E '^(test .*FAILED|failures:|thread|assertion)' "$TMP/test.log" | head -30
    fi
  done
done

# ---------------------------------------------------------------------------
# 5. C ground-truth stability: rebuild the C at several -O levels (out of tree,
#    c_src is never modified) and diff each against the Rust .so.
# ---------------------------------------------------------------------------
step "C optimisation-level stability vs Rust"
for opt in -O0 -O1 -O2 -O3 -Os; do
  bdir="$TMP/c$opt"
  rm -rf "$bdir"
  if ! cmake -S "$ROOT/c_src" -B "$bdir" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$opt" >"$TMP/c$opt.log" 2>&1 \
     || ! cmake --build "$bdir" >>"$TMP/c$opt.log" 2>&1; then
    bad "C build $opt"; tail -10 "$TMP/c$opt.log"; continue
  fi
  so="$(find "$bdir" -name 'lib*.so' | head -1)"
  if POW43_C_SO="$so" timeout 600 cargo test --offline --release --test valid_paths \
        -- --test-threads=4 >"$TMP/opt.log" 2>&1; then
    ok "Rust matches C built with $opt"
  else
    bad "Rust vs C built with $opt"; grep -E 'FAILED|diverged' "$TMP/opt.log" | head -10
  fi
done

printf '\n===============================\n'
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$fail"
