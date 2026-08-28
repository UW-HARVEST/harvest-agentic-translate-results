#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration.
#
#  1. enumerate all feature combinations declared in translation/Cargo.toml
#  2. `cargo check` each one
#  3. build the C .so (default CMake configuration)
#  4. compare exported symbols (nm -D)
#  5. run the libloading-based comparison suite for each combination,
#     in both dev and release profiles
#  6. replay the suite against C builds made with different optimisation levels
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
LOG=/tmp/verify.log
: > "$LOG"

fail=0
step() { printf '\n=== %s ===\n' "$*"; }
# run_in <dir> <cmd...>  -- runs in a subshell but records failure in $fail
run_in() {
  local dir="$1"; shift
  printf '  %-6s: %s\n' "start" "$*" >> "$LOG"
  if (cd "$dir" && timeout 600 "$@") >>"$LOG" 2>&1; then
    echo "  ok   : $*"
  else
    echo "  FAIL : $*"
    fail=1
  fi
}

# ---------------------------------------------------------------- features ----
step "1. enumerate feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblk=1; next }
    /^\[/           { inblk=0 }
    inblk && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
  ' "$CRATE/Cargo.toml"
)
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  translation/Cargo.toml declares no [features] section, and"
  echo "  c_src/CMakeLists.txt has no options/defines: the only build-time"
  echo "  configuration is the default (empty) feature set."
  COMBOS=("")
else
  echo "  declared features: ${FEATURES[*]}"
  COMBOS=("")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && combo="${combo:+$combo,}${FEATURES[i]}"
    done
    COMBOS+=("$combo")
  done
fi
printf '  %d combination(s): ' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '[%s] ' "${c:-<none>}"; done; echo

# ------------------------------------------------------------------- check ----
step "2. cargo check every combination"
for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then
    run_in "$CRATE" cargo check --no-default-features --all-targets
  else
    run_in "$CRATE" cargo check --no-default-features --features "$c" --all-targets
  fi
done

# ------------------------------------------------------------------- C .so ----
step "3. build the C shared library"
mkdir -p "$ROOT/c_src/build"
run_in "$ROOT/c_src/build" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
run_in "$ROOT/c_src/build" cmake --build .
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)
echo "  C .so: ${C_SO:-<none>}"

# ----------------------------------------------------------------- symbols ----
step "4. exported symbol comparison (nm -D)"
run_in "$CRATE" cargo build --release
R_SO="$CRATE/target/release/libsynth_pair_lib.so"
if [ -z "$C_SO" ] || [ ! -f "$R_SO" ]; then
  echo "  FAIL : missing .so (C='${C_SO:-}' Rust='$R_SO')"; fail=1
else
  nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort > /tmp/c.syms
  nm -D --defined-only "$R_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort > /tmp/r.syms
  echo "  C   exports: $(tr '\n' ' ' < /tmp/c.syms)"
  echo "  Rust exports: $(tr '\n' ' ' < /tmp/r.syms)"
  if ! grep -qx synth_pair /tmp/c.syms; then
    echo "  FAIL : C .so does not export synth_pair (bad baseline)"; fail=1
  fi
  missing=$(comm -23 /tmp/c.syms /tmp/r.syms)
  if [ -n "$missing" ]; then
    echo "  FAIL : Rust .so missing symbols: $(echo "$missing" | tr '\n' ' ')"; fail=1
  else
    echo "  ok   : Rust .so exports every symbol the C .so exports"
  fi
fi

# ------------------------------------------------------------------- tests ----
step "5. comparison suite: every feature combination x profile"
for c in "${COMBOS[@]}"; do
  for prof in dev release; do
    args=(cargo test --no-default-features)
    [ -n "$c" ] && args+=(--features "$c")
    [ "$prof" = release ] && args+=(--release)
    echo "  -- features=[${c:-<none>}] profile=$prof"
    run_in "$CRATE" "${args[@]}"
  done
done

# ------------------------------------------- replay against other C builds ----
step "6. replay against C built at -O0/-O1/-O2/-O3"
for opt in O0 O1 O2 O3; do
  d=/tmp/c_build_$opt
  rm -rf "$d"; mkdir -p "$d"
  if ! (cd "$d" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="-$opt" && cmake --build .) >>"$LOG" 2>&1; then
    echo "  skip : -$opt (build failed)"; continue
  fi
  so=$(find "$d" -maxdepth 1 -name 'lib*.so' | head -1)
  echo "  -- C -$opt"
  if (cd "$CRATE" && SYNTH_C_SO="$so" timeout 600 cargo test --release) >>"$LOG" 2>&1; then
    echo "  ok   : matches C -$opt"
  else
    echo "  FAIL : mismatch vs C -$opt"; fail=1
  fi
done

step "result"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT (see $LOG)"; fi
exit "$fail"
