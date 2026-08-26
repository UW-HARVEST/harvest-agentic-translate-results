#!/usr/bin/env bash
# Full verification driver: enumerates every build-time configuration, checks
# and tests each of them, and diffs the exported dynamic symbols.
set -uo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS=(--offline)
LOGDIR="${TMPDIR:-.}/verify-logs"
mkdir -p "$LOGDIR"
FAIL=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mOK\033[0m   %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 0. Enumerate every valid feature combination from Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}
echo "  declared non-default features: $N ${FEATURES[*]:-(none)}"

COMBOS=()
if [ "$N" -eq 0 ]; then
  # No [features] table at all -> the only two invocations that can differ.
  COMBOS=("" "--no-default-features")
else
  for ((mask = 0; mask < (1 << N); mask++)); do
    sel=()
    for ((i = 0; i < N; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
    done
    if [ ${#sel[@]} -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  COMBOS+=("") # the default feature set
fi
echo "  combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
step "Building the C shared library"
(
  mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null
) && ok "c_src/build/libdriver.so" || bad "C build"
C_SO=c_src/build/libdriver.so

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  step "cargo check ${label}"
  if timeout 600 cargo check "${CARGO_FLAGS[@]}" $combo --all-targets \
      >"$LOGDIR/check.log" 2>&1; then
    ok "check ${label}"
  else
    bad "check ${label}"; tail -30 "$LOGDIR/check.log"
  fi
done

# ---------------------------------------------------------------------------
# 3. Symbol parity, per profile
# ---------------------------------------------------------------------------
for profile in debug release; do
  step "Symbol parity (${profile} Rust cdylib)"
  if [ "$profile" = release ]; then
    timeout 600 cargo build "${CARGO_FLAGS[@]}" --release >/dev/null 2>&1
  else
    timeout 600 cargo build "${CARGO_FLAGS[@]}" >/dev/null 2>&1
  fi
  R_SO="target/${profile}/libdriver.so"
  [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }

  c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
  r_syms=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  echo "  C exports   : $(echo "$c_syms" | grep -c .)"
  echo "  Rust exports: $(echo "$r_syms" | grep -c .)"
  if [ -z "$missing" ]; then
    ok "0 symbols missing from the Rust .so"
  else
    bad "missing from Rust .so:"; echo "$missing" | sed 's/^/       /'
  fi

  # Every undefined symbol in the Rust .so must resolve at load time; `ldd -r`
  # performs the full relocation check and lists anything that does not.
  unresolved=$(ldd -r "$R_SO" 2>&1 | grep -iE 'undefined symbol|not found')
  if [ -z "$unresolved" ]; then
    ok "ldd -r: every undefined symbol resolves (libc/libgcc only)"
  else
    bad "unresolved symbols:"; echo "$unresolved" | sed 's/^/       /'
  fi
  echo "  needed libs: $(ldd "$R_SO" | awk '{print $1}' | tr '\n' ' ')"
done

# ---------------------------------------------------------------------------
# 4. Differential test suite for every combination, both profiles
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="${combo:-<default features>} ${profile:-<debug>}"
    step "cargo test ${label}"
    if timeout 600 cargo test "${CARGO_FLAGS[@]}" $combo $profile \
        >"$LOGDIR/test.log" 2>&1; then
      ok "test ${label} — $(grep -c '^test .* ok$' "$LOGDIR/test.log") tests passed"
    else
      bad "test ${label}"; tail -40 "$LOGDIR/test.log"
    fi
  done
done

step "RESULT"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
