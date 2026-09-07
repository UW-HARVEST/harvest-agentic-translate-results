#!/usr/bin/env bash
# Full verification driver: builds the C and Rust shared libraries, diffs their
# dynamic symbol tables, and runs every differential test under every feature
# combination declared in Cargo.toml.
#
# Usage:  ./verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
CARGO_FLAGS="--offline"
LOG="${TMPDIR:-/tmp}/verify.$$.log"
FAIL=0

step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
  && cmake --build . > /dev/null ) \
  && ok "C library built" || bad "C library build"

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -n1)"
[ -n "$C_SO" ] && ok "C .so: $C_SO" || bad "no C .so produced"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
# ---------------------------------------------------------------------------
# Every feature name declared under [features] (excluding `default`).
FEATURES=$(awk '
  /^\[features\]/       { inf=1; next }
  /^\[/                 { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, kv, "=");
      gsub(/[[:space:]]/, "", kv[1]);
      if (kv[1] != "default") print kv[1];
  }
' "$HERE/Cargo.toml")

# Build the list of cargo invocations to test.
declare -a COMBOS=()
COMBOS+=("")                              # default features
COMBOS+=("--no-default-features")
if [ -n "$FEATURES" ]; then
  # every individual feature, and the full set (powerset for small counts)
  mapfile -t FLIST <<< "$FEATURES"
  n=${#FLIST[@]}
  echo "  declared features: ${FLIST[*]}"
  total=$(( 1 << n ))
  if [ "$total" -le 64 ]; then
    for (( mask=0; mask<total; mask++ )); do
      sel=""
      for (( i=0; i<n; i++ )); do
        if (( (mask >> i) & 1 )); then sel="$sel,${FLIST[$i]}"; fi
      done
      COMBOS+=("--no-default-features --features ${sel#,}")
    done
  else
    for f in "${FLIST[@]}"; do
      COMBOS+=("--no-default-features --features $f")
    done
    COMBOS+=("--all-features")
  fi
else
  echo "  no [features] declared -> the only configuration is the default one"
  echo "  (\`--no-default-features\` is equivalent and is still exercised)"
fi
echo "  ${#COMBOS[@]} configuration(s) to verify"

# ---------------------------------------------------------------------------
step "Verify each configuration"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  printf '\n--- %s ---\n' "$label"

  # cargo check
  if ( cd "$HERE" && cargo check $CARGO_FLAGS $combo --tests > /dev/null 2>&1 ); then
    ok "cargo check           [$label]"
  else
    bad "cargo check           [$label]"
    continue
  fi

  # build the cdylib we will dlopen
  if ( cd "$HERE" && cargo build $CARGO_FLAGS --release $combo > /dev/null 2>&1 ); then
    ok "cargo build --release [$label]"
  else
    bad "cargo build --release [$label]"
    continue
  fi

  R_SO="$HERE/target/release/libmerge_sort_lib.so"
  [ -f "$R_SO" ] && ok "Rust .so: $R_SO" || { bad "no Rust .so [$label]"; continue; }

  # symbol diff: every C symbol must be present in the Rust .so
  cdef=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u)
  rdef=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u)
  noise='^(_init|_fini|__bss_start|_edata|_end|_ITM_.*|__cxa_finalize|__gmon_start__)$'
  missing=$(comm -23 <(echo "$cdef" | grep -Ev "$noise") <(echo "$rdef" | grep -Ev "$noise"))
  if [ -z "$missing" ]; then
    ok "symbol diff empty    [$label]"
  else
    bad "symbols missing from Rust .so [$label]: $(echo $missing)"
  fi

  # also build the debug-profile cdylib: the struct-assignment / padding
  # behaviour the translation must reproduce is codegen-sensitive, so BOTH
  # optimisation levels of the Rust .so are compared against the C.
  ( cd "$HERE" && cargo build $CARGO_FLAGS $combo > /dev/null 2>&1 ) \
    && ok "cargo build (debug)   [$label]" || bad "cargo build (debug) [$label]"
  D_SO="$HERE/target/debug/libmerge_sort_lib.so"

  # the full differential test suite, against each build of the Rust .so
  for target_so in "$R_SO" "$D_SO"; do
    [ -f "$target_so" ] || { bad "missing Rust .so: $target_so"; continue; }
    prof=$(basename "$(dirname "$target_so")")
    if ( cd "$HERE" && MERGE_SORT_RUST_SO="$target_so" \
           timeout 600 cargo test $CARGO_FLAGS $combo --tests > "$LOG" 2>&1 ); then
      ok "cargo test            [$label / rust .so = $prof]"
      grep -h 'test result:' "$LOG" | sed 's/^/       /'
    else
      bad "cargo test            [$label / rust .so = $prof]"
      tail -n 40 "$LOG" | sed 's/^/       /'
    fi
    rm -f "$LOG"
  done
done

# ---------------------------------------------------------------------------
step "Binary / driver comparison"
# ---------------------------------------------------------------------------
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
  bad "c_src declares an executable but this script does not compare it yet"
else
  ok "c_src builds no executable (add_library SHARED only) -> nothing to diff"
fi
if grep -qE '^\[\[bin\]\]' "$HERE/Cargo.toml"; then
  bad "translation declares a [[bin]] but this script does not compare it yet"
else
  ok "translation declares no [[bin]] -> nothing to diff"
fi

# ---------------------------------------------------------------------------
printf '\n'
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
