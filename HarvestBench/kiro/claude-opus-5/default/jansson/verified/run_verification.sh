#!/usr/bin/env bash
# Full differential verification: build both libraries, diff their exported
# symbols, then run every phase's tests under every feature combination.
#
# The C library is NOT thread safe (dtoa.c uses a global freelist and
# hashtable_seed / the allocator hooks are process-global), so the test
# binaries must run with --test-threads=1.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
OUT="$ROOT/verif"
mkdir -p "$OUT"
rc=0

echo "=== building C shared library ==="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    timeout 600 cmake --build . 
) >"$OUT/c_build.log" 2>&1 || { echo "C build FAILED"; tail -20 "$OUT/c_build.log"; exit 1; }

# ---------------------------------------------------------------- features
# Extract every feature declared in Cargo.toml (excluding "default").
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1]!="default" && a[1]!="") print a[1]}' \
    "$CRATE/Cargo.toml"
)

COMBOS=("")            # default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="$combo,${FEATURES[$i]}"; fi
    done
    COMBOS+=("--no-default-features --features ${combo#,}")
  done
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  COMBOS+=("--no-default-features")
  echo "note: Cargo.toml declares no [features]; only the default configuration exists"
fi

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "############################################################"
  echo "### feature combination: $label"
  echo "############################################################"

  echo "--- cargo build --release $combo ---"
  # shellcheck disable=SC2086
  (cd "$CRATE" && timeout 600 cargo build --release $combo) \
    >"$OUT/rust_build.log" 2>&1 || { echo "Rust build FAILED"; tail -20 "$OUT/rust_build.log"; rc=1; continue; }

  echo "--- symbol parity ---"
  nm -D --defined-only "$ROOT/c_src/build/libjansson.so" | awk '{print $3}' | sort -u >"$OUT/c_syms.txt"
  nm -D --defined-only "$CRATE/target/release/libjansson.so" | awk '{print $3}' | sort -u >"$OUT/rust_syms.txt"
  missing=$(comm -23 "$OUT/c_syms.txt" "$OUT/rust_syms.txt")
  extra=$(comm -13 "$OUT/c_syms.txt" "$OUT/rust_syms.txt")
  printf 'C symbols   : %s\n' "$(wc -l <"$OUT/c_syms.txt")"
  printf 'Rust symbols: %s\n' "$(wc -l <"$OUT/rust_syms.txt")"
  if [ -n "$missing" ]; then echo "MISSING FROM RUST:"; echo "$missing"; rc=1; else echo "missing: none"; fi
  if [ -n "$extra" ]; then echo "EXTRA IN RUST:"; echo "$extra"; fi
  # undefined symbols that are not libc / libgcc-unwind imports
  nm -D --undefined-only "$CRATE/target/release/libjansson.so" | awk '{print $2}' | sort -u \
    | grep -vE '@|^_ITM_|^__gmon_start__$' >"$OUT/rust_undef_nonlibc.txt" || true
  if [ -s "$OUT/rust_undef_nonlibc.txt" ]; then
    echo "unversioned undefined symbols (review):"; cat "$OUT/rust_undef_nonlibc.txt"
  fi

  echo "--- tests ---"
  # shellcheck disable=SC2086
  (cd "$CRATE" && timeout 3000 cargo test --release $combo -- --test-threads=1) 2>&1 \
    | tee "$OUT/test_${label//[^A-Za-z0-9]/_}.log" | grep -E "^(test |running|test result|error)"
  if grep -qE "^(error|test result: FAILED)" "$OUT/test_${label//[^A-Za-z0-9]/_}.log"; then rc=1; fi
  if ! grep -q "test result: ok" "$OUT/test_${label//[^A-Za-z0-9]/_}.log"; then rc=1; fi
done

echo
if [ "$rc" -eq 0 ]; then echo "ALL VERIFICATION PASSED"; else echo "VERIFICATION FAILED"; fi
exit "$rc"
