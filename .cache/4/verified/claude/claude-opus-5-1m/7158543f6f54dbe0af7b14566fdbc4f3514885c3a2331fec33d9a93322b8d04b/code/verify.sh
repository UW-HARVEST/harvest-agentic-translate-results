#!/usr/bin/env bash
# Full C-vs-Rust differential verification.
#
#   Phase A : build both sides, dump the symbol tables
#   Phase B : valid-path differential tests   (CONFIGS.md rows)
#   Phase C : error-path differential tests   (ERRORS.md rows)
#   Phase D : symbol parity + every Cargo feature combination
#
# Every step is run for EVERY valid feature combination. The combinations are
# enumerated mechanically from Cargo.toml's [features] section, so adding a
# feature later automatically widens the matrix.
set -uo pipefail
cd "$(dirname "$0")"

LOGDIR="${TMPDIR:-target/tmp}/verify-logs"
mkdir -p "$LOGDIR" 2>/dev/null || { LOGDIR=target/verify-logs; mkdir -p "$LOGDIR"; }

fail=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# Phase A.0 — enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No features declared: the empty set is the only combination.
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

step "Phase A.0 — feature combinations"
echo "  declared features : ${FEATURES[*]:-<none>}"
echo "  combinations      : ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "    --no-default-features --features '${c}'"; done

# ---------------------------------------------------------------------------
# Phase A.1 — build the C side (executable per CMakeLists + shared object)
# ---------------------------------------------------------------------------
step "Phase A.1 — build the C reference"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >cmake.log 2>&1 \
  && cmake --build . >>cmake.log 2>&1 ) \
  && ok "cmake executable c_src/build/driver" || bad "cmake build"

mkdir -p cbuild
cc -fPIC -shared -o cbuild/libcdriver.so c_src/src/main.c \
  && ok "shared object cbuild/libcdriver.so" || bad "gcc -shared"

# ---------------------------------------------------------------------------
# Phase A.2 / D — cargo check + build + symbol diff for every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="features='${combo:-<empty>}'"

  step "Phase A.2 — cargo check ($label)"
  if timeout 600 cargo check --no-default-features --features "$combo" \
       --all-targets >"$LOGDIR/check.log" 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"; tail -30 "$LOGDIR/check.log"
  fi

  step "Phase A.3 — cargo build (bin + cdylib) ($label)"
  for profile in "" "--release"; do
    if timeout 600 cargo build $profile --no-default-features --features "$combo" \
         >"$LOGDIR/build.log" 2>&1; then
      ok "cargo build ${profile:---dev} $label"
    else
      bad "cargo build ${profile:---dev} $label"; tail -30 "$LOGDIR/build.log"
    fi
  done

  step "Phase D — symbol diff C .so vs Rust .so ($label)"
  cs=$(nm -D --defined-only cbuild/libcdriver.so | awk '{print $NF}' | sort)
  rs=$(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort)
  missing=$(comm -23 <(echo "$cs") <(echo "$rs"))
  if [ -z "$missing" ]; then
    ok "0 missing symbols ($(echo "$cs" | tr '\n' ' '))"
  else
    bad "missing from Rust .so: $(echo "$missing" | tr '\n' ' ')"
  fi
  if ldd -r target/release/libdriver.so 2>&1 | grep -qi 'undefined symbol'; then
    bad "Rust .so has undefined symbols"
  else
    ok "0 undefined non-libc symbols in the Rust .so"
  fi

  step "Phases B + C — differential tests ($label)"
  if timeout 600 cargo test --no-default-features --features "$combo" \
       -- --test-threads=8 >"$LOGDIR/test.log" 2>&1; then
    ok "cargo test $label"
    grep -E '^(test result|phase_[bc]_ffi:)' "$LOGDIR/test.log" | sed 's/^/    /'
  else
    bad "cargo test $label"; tail -60 "$LOGDIR/test.log"
  fi
done

step "SUMMARY"
if [ "$fail" -eq 0 ]; then
  printf '\033[32mALL PHASES PASSED\033[0m for %d feature combination(s)\n' "${#COMBOS[@]}"
else
  printf '\033[31mVERIFICATION FAILED\033[0m\n'
fi
exit "$fail"
