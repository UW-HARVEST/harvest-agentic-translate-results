#!/usr/bin/env bash
# Runs the full differential verification (Phases A-D) across every build
# configuration.
#
# `Cargo.toml` has no [features] section, so the complete set of valid feature
# combinations is: default == --no-default-features == --all-features == {}.
# All three invocations are run anyway, plus the release object (which adds
# `panic = "abort"` + optimizations and disables debug-assertions), because the
# same code is shared by all of them.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
FAIL=0
TIMEOUT="${TIMEOUT:-600}"

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()  { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad() { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

run() { # run <label> <cmd...>
  local label="$1"; shift
  if timeout "$TIMEOUT" "$@" >"$ROOT/logs/$(echo "$label" | tr ' /' '__').log" 2>&1; then
    ok "$label"
  else
    bad "$label  (see logs/$(echo "$label" | tr ' /' '__').log)"
    tail -n 30 "$ROOT/logs/$(echo "$label" | tr ' /' '__').log" | sed 's/^/      /'
  fi
}

mkdir -p logs

# ---------------------------------------------------------------------------
say "1. Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
if (cd c_src/build \
      && timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
      && timeout "$TIMEOUT" cmake --build .) >logs/cmake.log 2>&1; then
  ok "cmake build"
else
  bad "cmake build"; tail -n 30 logs/cmake.log; exit 1
fi
C_SO="$ROOT/c_src/build/libtranslated_rust.so"
[[ -f "$C_SO" ]] && ok "C .so at $C_SO" || { bad "C .so missing"; exit 1; }

# ---------------------------------------------------------------------------
say "2. cargo check for EVERY feature combination"
# ---------------------------------------------------------------------------
# Enumerate features mechanically from Cargo.toml.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [[ -z "$FEATURES" ]]; then
  echo "  Cargo.toml declares no [features]; the only combination is the empty set."
  COMBOS=("" "--no-default-features" "--all-features")
else
  # Full power set of the declared features.
  COMBOS=("" "--no-default-features" "--all-features")
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((b=0; b<n; b++)); do
      (( mask & (1<<b) )) && sel="${sel:+$sel,}${FARR[b]}"
    done
    COMBOS+=("--no-default-features --features $sel")
  done
fi

for combo in "${COMBOS[@]}"; do
  run "cargo check [${combo:-default}]" cargo check --offline --tests $combo
done

# ---------------------------------------------------------------------------
say "3. Build the Rust shared library (dev + release)"
# ---------------------------------------------------------------------------
run "cargo build dev"     cargo build --offline
run "cargo build release" cargo build --offline --release
DEV_SO="$ROOT/target/debug/libsiphash_lib.so"
REL_SO="$ROOT/target/release/libsiphash_lib.so"
[[ -f "$DEV_SO" ]] && ok "dev .so"     || bad "dev .so missing"
[[ -f "$REL_SO" ]] && ok "release .so" || bad "release .so missing"

# ---------------------------------------------------------------------------
say "4. Phase D: nm -D symbol diff must be EMPTY"
# ---------------------------------------------------------------------------
csyms=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u)
for so in "$DEV_SO" "$REL_SO"; do
  rsyms=$(nm -D --defined-only "$so" | awk '{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  extra=$(comm -13 <(echo "$csyms") <(echo "$rsyms"))
  if [[ -z "$missing" && -z "$extra" ]]; then
    ok "symbol diff empty for $(basename "$(dirname "$so")")/$(basename "$so")"
  else
    bad "symbol diff for $so -- missing:[$missing] extra:[$extra]"
  fi
done
# Undefined non-libc symbols in the Rust objects.
for so in "$DEV_SO" "$REL_SO"; do
  if ldd "$so" 2>&1 | grep -q "not found"; then
    bad "unresolved shared-library deps in $so"
  else
    ok "all dynamic deps resolve for $(basename "$(dirname "$so")")/$(basename "$so")"
  fi
done

# ---------------------------------------------------------------------------
say "5. Phases B + C: differential tests for every feature combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  run "cargo test [${combo:-default}] vs dev .so" \
    env RUST_SO="$DEV_SO" C_SO="$C_SO" cargo test --offline $combo
done

# ---------------------------------------------------------------------------
say "6. Phases B + C against the RELEASE .so (panic=abort, optimized)"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  run "cargo test [${combo:-default}] vs release .so" \
    env RUST_SO="$REL_SO" C_SO="$C_SO" cargo test --offline $combo
done

# ---------------------------------------------------------------------------
say "6b. Optional: C compiled at every optimization level (OPT_SWEEP=1)"
# ---------------------------------------------------------------------------
# The C code contains signed-overflow UB (`d[3] << 24` with d[3] >= 0x80) whose
# result the translation depends on, so it is worth confirming the behaviour is
# not an artefact of the unoptimized CMake default. c_src/ itself is never
# modified -- the variants are built into a scratch directory.
if [[ "${OPT_SWEEP:-0}" == "1" ]]; then
  SCRATCH="${TMPDIR:-/tmp}/copt_verify.$$"
  mkdir -p "$SCRATCH"
  for o in O0 O1 O2 O3 Os; do
    if gcc "-$o" -fPIC -shared -Ic_src/include -o "$SCRATCH/lib_$o.so" c_src/src/lib.c 2>/dev/null; then
      for prof in debug release; do
        run "cargo test [C -$o vs Rust $prof]" \
          env C_SO="$SCRATCH/lib_$o.so" RUST_SO="$ROOT/target/$prof/libsiphash_lib.so" \
          cargo test --offline
      done
    else
      bad "gcc -$o build of lib.c"
    fi
  done
  rm -rf "$SCRATCH"
else
  echo "  skipped (re-run with OPT_SWEEP=1 to include it)"
fi

# ---------------------------------------------------------------------------
say "7. Artifact presence (Phase A)"
# ---------------------------------------------------------------------------
for f in SYMBOLS.md ERRORS.md CONFIGS.md VERIFICATION.md; do
  [[ -s "$f" ]] && ok "$f present ($(wc -l <"$f") lines)" || bad "$f missing/empty"
done

# ---------------------------------------------------------------------------
if [[ $FAIL -eq 0 ]]; then
  printf '\n\033[1;32mALL CONFIGURATIONS VERIFIED\033[0m\n'
else
  printf '\n\033[1;31mVERIFICATION FAILED\033[0m\n'
fi
exit $FAIL
