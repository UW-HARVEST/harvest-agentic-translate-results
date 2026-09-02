#!/usr/bin/env bash
# Phase D: enumerate every feature combination declared in Cargo.toml and run the
# full differential suite (plus the nm -D symbol diff) under each one.
#
# Nothing is hard-coded: the feature list is read out of Cargo.toml, so if a
# feature is ever added this script picks it up automatically.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
CRATE_DIR="$PWD"
C_BUILD="$CRATE_DIR/../c_src/build"
RUST_SO="$CRATE_DIR/target/release/libaabb_lib.so"
FAIL=0

# --- 0. Make sure the C .so exists -----------------------------------------
if ! ls "$C_BUILD"/lib*.so >/dev/null 2>&1; then
  echo "building the C shared library..."
  ( mkdir -p "$C_BUILD" && cd "$C_BUILD" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
C_SO=$(ls "$C_BUILD"/lib*.so | head -n1)
echo "C   .so: $C_SO"

# --- 1. Enumerate the declared features ------------------------------------
# Read the [features] table out of Cargo.toml. Keys only, ignoring "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

NF=${#FEATURES[@]}
if [ "$NF" -eq 0 ]; then
  echo "features declared: (none)"
else
  echo "features declared: ${FEATURES[*]}"
fi

# --- 2. Build the combination list -----------------------------------------
# Always include the default build and the no-default build; then every subset of
# the declared features (2^n), capped so the script stays quick.
COMBOS=()
COMBOS+=("")                      # default features
COMBOS+=("--no-default-features")
if [ "$NF" -gt 0 ] && [ "$NF" -le 12 ]; then
  total=$((1 << NF))
  for ((m = 0; m < total; m++)); do
    set=""
    for ((i = 0; i < NF; i++)); do
      if (( (m >> i) & 1 )); then set="$set,${FEATURES[$i]}"; fi
    done
    set="${set#,}"
    if [ -n "$set" ]; then
      COMBOS+=("--no-default-features --features $set")
      COMBOS+=("--features $set")
    fi
  done
elif [ "$NF" -gt 12 ]; then
  echo "WARNING: $NF features -> 2^$NF subsets; testing each feature alone and all together"
  all=$(IFS=,; echo "${FEATURES[*]}")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  COMBOS+=("--no-default-features --features $all")
fi

N=${#COMBOS[@]}
echo "combinations to verify: $N"
echo

# --- 3. Verify each combination --------------------------------------------
k=0
for combo in "${COMBOS[@]}"; do
  k=$((k + 1))
  label=${combo:-"(default features)"}
  echo "=============================================================="
  echo "combination $k/$N: $label"
  echo "=============================================================="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $combo >/tmp/fc_build.log 2>&1; then
    echo "  BUILD FAILED"; tail -n 25 /tmp/fc_build.log; FAIL=1; continue
  fi

  # 3a. Symbol parity for THIS combination.
  nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | grep -v '^_' | sort > /tmp/fc_c.txt
  nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | grep -v '^_' | sort > /tmp/fc_r.txt
  missing=$(comm -23 /tmp/fc_c.txt /tmp/fc_r.txt)
  extra=$(comm -13 /tmp/fc_c.txt /tmp/fc_r.txt)
  echo "  symbols: C=$(wc -l < /tmp/fc_c.txt) Rust=$(wc -l < /tmp/fc_r.txt)"
  if [ -n "$missing" ]; then echo "  MISSING FROM RUST:"; echo "$missing" | sed 's/^/    /'; FAIL=1
  else echo "  missing from Rust: none"; fi
  if [ -n "$extra" ]; then echo "  EXTRA IN RUST:"; echo "$extra" | sed 's/^/    /'; FAIL=1
  else echo "  extra in Rust: none"; fi

  # 3b. No unresolved non-libc symbols.
  undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' \
    | grep -v '^_' | grep -v '@GLIBC' | grep -v '@GCC' \
    | grep -vE '^(abort|bcmp|calloc|close|free|getcwd|getenv|gettid|malloc|memcpy|memmove|memset|open|read|readlink|realloc|realpath|statx|strlen|syscall|write|writev|munmap|mmap|lseek|fstat|stat|posix_memalign|dl_iterate_phdr|pthread_[a-z_]*)$')
  if [ -n "$undef" ]; then echo "  UNRESOLVED NON-LIBC SYMBOLS:"; echo "$undef" | sed 's/^/    /'; FAIL=1
  else echo "  unresolved non-libc symbols: none"; fi

  # 3c. Phases B and C. `--nocapture` so the per-test check counts are visible.
  # shellcheck disable=SC2086
  if timeout 600 cargo test --release $combo -- --nocapture --test-threads=4 >/tmp/fc_test.log 2>&1; then
    grep -h 'test result:' /tmp/fc_test.log \
      | awk -F'[ ;]' '{p+=$4} END {printf "  tests passed: %d\n", p}'
    echo "  total differential checks: $(grep -ho '[0-9]\+ differential checks passed' /tmp/fc_test.log | awk '{s+=$1} END {print s+0}')"
  else
    echo "  TESTS FAILED"; grep -E 'FAILED|diverged|panicked' /tmp/fc_test.log | head -n 20 | sed 's/^/    /'; FAIL=1
  fi
  echo
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "ALL $N FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$FAIL"
