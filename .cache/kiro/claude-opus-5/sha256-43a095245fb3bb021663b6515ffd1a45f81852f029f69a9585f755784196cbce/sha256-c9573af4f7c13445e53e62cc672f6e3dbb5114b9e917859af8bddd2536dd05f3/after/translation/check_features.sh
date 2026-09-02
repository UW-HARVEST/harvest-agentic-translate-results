#!/usr/bin/env bash
# Phase D automation: enumerate every feature combination declared in
# Cargo.toml, then for each one run cargo check, build the cdylib, diff `nm -D`
# against the C .so, and run the full differential suite.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/lib*.so 2>/dev/null | head -1)
if [ -z "${C_SO:-}" ]; then
  echo "FATAL: C .so not built (expected ../c_src/build/lib*.so)"; exit 1
fi
RUST_SO=target/release/liboverunder_lib.so

# --- enumerate features -----------------------------------------------------
# Feature names are the keys of the [features] table, excluding "default".
FEATURES=$(awk '
  /^\[features\]/ {inb=1; next}
  /^\[/           {inb=0}
  inb && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features] table -> the default build is the"
  echo "only configuration. Verifying it."
  COMBOS+=("__default__")
else
  # default build plus the full power set of the declared features
  COMBOS+=("__default__")
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${feats[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__default__" ]; then
    label="(default features)"; flags=()
  elif [ -z "$combo" ]; then
    label="(no default features, no features)"; flags=(--no-default-features)
  else
    label="--no-default-features --features $combo"; flags=(--no-default-features --features "$combo")
  fi
  echo "=============================================================="
  echo "CONFIG: $label"
  echo "=============================================================="

  if ! timeout 300 cargo check "${flags[@]}" -q 2>&1 | tail -5; then
    echo "  cargo check FAILED"; fail=1; continue
  fi
  echo "  cargo check           OK"

  if ! timeout 300 cargo build --release "${flags[@]}" -q 2>&1 | tail -5; then
    echo "  cargo build FAILED"; fail=1; continue
  fi
  echo "  cargo build --release OK"

  # --- symbol parity --------------------------------------------------------
  csyms=$(nm -D --defined-only "$C_SO"    | awk '{print $3}' | sort -u)
  rsyms=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
  if [ -n "$missing" ]; then
    echo "  SYMBOL DIFF NOT EMPTY -- missing from Rust .so:"
    echo "$missing" | sed 's/^/    /'
    fail=1
  else
    echo "  symbol diff           EMPTY ($(echo "$csyms" | wc -l) C symbols all exported by Rust)"
  fi

  # non-libc undefined symbols in the Rust .so
  undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' \
    | grep -v -E '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
    | grep -v -E '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|pthread_key_create|pthread_key_delete|pthread_setspecific|putchar|read|readlink|realloc|realpath|sqrt|stat64|strlen|strncpy|syscall|write|writev)$')
  if [ -n "$undef" ]; then
    echo "  UNRESOLVED non-libc undefined symbols:"; echo "$undef" | sed 's/^/    /'; fail=1
  else
    echo "  undefined non-libc    NONE"
  fi

  # --- full differential suite ---------------------------------------------
  if timeout 500 cargo test --release "${flags[@]}" 2>&1 | tail -12; then
    echo "  differential suite    PASS"
  else
    echo "  differential suite    FAIL"; fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS VERIFIED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
