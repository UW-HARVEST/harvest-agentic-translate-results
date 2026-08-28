#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Full verification driver: Phase A artifacts -> Phase B/C tests -> Phase D
# symbol parity, across EVERY feature combination and both codegen profiles.
#
#   cd translation && bash run_all.sh
# ---------------------------------------------------------------------------
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$CRATE_DIR/.." && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
TESTS=(valid_paths error_paths malloc_failure)
FAILED=0

hdr() { echo; echo "############ $* ############"; }

# ---------------------------------------------------------------------------
hdr "0. Build the C shared library"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "FATAL: C build failed"; exit 1; }
[ -f "$C_SO" ] || { echo "FATAL: $C_SO not produced"; exit 1; }
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
hdr "1. Enumerate feature combinations from Cargo.toml"
# Extract feature names from the [features] table, if any.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' "$CRATE_DIR/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "No [features] table -> the only build configurations are the default"
  echo "build and --no-default-features (identical, nothing to disable)."
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  echo "Features found: ${FEATURES[*]}"
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
  COMBOS+=("all-features:--all-features")
  # Full power set of the individual features, with default features off.
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${FEATURES[b]}")
    done
    joined=$(
      IFS=,
      echo "${sel[*]}"
    )
    COMBOS+=("nd+$joined:--no-default-features --features $joined")
  done
fi
printf '  combo: %s\n' "${COMBOS[@]%%:*}"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  name="${combo%%:*}"
  flags="${combo#*:}"

  hdr "2. combo=$name  flags='${flags:-<none>}'"

  # ---- cargo check -------------------------------------------------------
  if ! (cd "$CRATE_DIR" && timeout 600 cargo check --all-targets $flags 2>&1 | tail -5); then
    echo "*** cargo check FAILED for combo=$name"
    FAILED=1
    continue
  fi

  # ---- build both profiles ----------------------------------------------
  for profile in debug release; do
    pflag=""
    [ "$profile" = release ] && pflag="--release"
    if ! (cd "$CRATE_DIR" && timeout 600 cargo build $pflag $flags >/dev/null 2>&1); then
      echo "*** cargo build ($profile) FAILED for combo=$name"
      FAILED=1
      continue
    fi
    RUST_SO="$CRATE_DIR/target/$profile/libdriver.so"
    [ -f "$RUST_SO" ] || {
      echo "*** $RUST_SO missing"
      FAILED=1
      continue
    }

    # ---- Phase D: symbol parity ----------------------------------------
    echo "--- symbol parity ($profile) ---"
    c_syms=$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)
    r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      echo "*** MISSING from Rust .so:"
      echo "$missing" | sed 's/^/      /'
      FAILED=1
    else
      echo "  OK: all $(echo "$c_syms" | wc -l) C-exported symbol(s) present in Rust .so"
      echo "$c_syms" | sed 's/^/      /'
    fi

    # Undefined non-libc symbols in the Rust .so must be empty.
    undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
      | sed 's/@.*//' | sort -u \
      | grep -v -E '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
      | grep -v -E '^(malloc|calloc|realloc|free|posix_memalign|memcpy|memmove|memset|bcmp|strlen|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|mmap64|munmap|syscall|dl_iterate_phdr|pthread_key_create|pthread_key_delete|pthread_setspecific)$')
    if [ -n "$undef" ]; then
      echo "*** UNRESOLVED non-libc symbols in Rust .so:"
      echo "$undef" | sed 's/^/      /'
      FAILED=1
    else
      echo "  OK: 0 undefined non-libc symbols"
    fi
    if ! ldd "$RUST_SO" 2>&1 | grep -q 'not found'; then
      echo "  OK: ldd fully resolves the Rust .so"
    else
      echo "*** ldd reports unresolved libraries"
      ldd "$RUST_SO" | grep 'not found'
      FAILED=1
    fi
  done

  # ---- Phase B + C: differential tests ----------------------------------
  # Test binaries are built in debug (release sets panic=abort, which libtest
  # cannot use), but they are pointed at BOTH the debug and the release cdylib
  # so optimized codegen is verified too.
  if ! (cd "$CRATE_DIR" && timeout 600 cargo test --no-run $flags >/dev/null 2>&1); then
    echo "*** building test binaries FAILED for combo=$name"
    FAILED=1
    continue
  fi

  for target_so in debug release; do
    SO="$CRATE_DIR/target/$target_so/libdriver.so"
    echo "--- differential tests (combo=$name, Rust .so=$target_so) ---"
    for t in "${TESTS[@]}"; do
      out=$(cd "$CRATE_DIR" && RUST_DRIVER_SO="$SO" timeout 600 cargo test --test "$t" $flags 2>&1)
      line=$(echo "$out" | grep -E '^test result:' | tail -1)
      if echo "$out" | grep -qE '^test result: ok\.'; then
        echo "  PASS  $t  ($line)"
      else
        echo "  *** FAIL $t"
        echo "$out" | tail -25 | sed 's/^/       /'
        FAILED=1
      fi
    done
  done
done

# ---------------------------------------------------------------------------
hdr "3. Phase A artifacts present"
for f in SYMBOLS.md ERRORS.md CONFIGS.md; do
  if [ -s "$CRATE_DIR/$f" ]; then
    echo "  OK  $f ($(wc -l <"$CRATE_DIR/$f") lines)"
  else
    echo "  *** MISSING/EMPTY $f"
    FAILED=1
  fi
done

# Every ERRORS.md / CONFIGS.md row must be checked off.
unchecked=$(grep -h '^| *[ECG][0-9]' "$CRATE_DIR/ERRORS.md" "$CRATE_DIR/CONFIGS.md" 2>/dev/null | grep -c '\[ \]')
echo "  unchecked table rows: $unchecked"
[ "$unchecked" -eq 0 ] || FAILED=1

# ---------------------------------------------------------------------------
hdr "RESULT"
if [ $FAILED -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT (see *** lines)"
fi
exit $FAILED
