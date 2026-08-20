#!/usr/bin/env bash
# Phase D driver: enumerate every valid feature combination from Cargo.toml and
# run `cargo check` + `cargo test` (dev and release) for each, plus the symbol
# parity diff between the C and Rust shared libraries.
set -uo pipefail
cd "$(dirname "$0")"

FAIL=0
# Use a writable scratch dir ($TMPDIR may be set by a sandbox; /tmp can be RO).
WORK=$(mktemp -d "${TMPDIR:-/tmp}/verify_all.XXXXXX") || { echo "cannot create temp dir"; exit 1; }
trap 'rm -rf "$WORK"' EXIT

note() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  [PASS] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate features from the [features] table of Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); gsub(/[[:space:]]/, "");
      if ($0 != "default") print
    }
  ' Cargo.toml
)

note "Feature enumeration"
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  Cargo.toml declares no [features]; the only valid combination is the empty set."
else
  echo "  features: ${FEATURES[*]}"
fi

# Build the powerset of FEATURES as comma-separated combos ("" == no features).
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$((1 << n))
  COMBOS=()
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "  => ${#COMBOS[@]} valid feature combination(s) to verify"

# ---------------------------------------------------------------------------
# 2. Build the C reference shared library
# ---------------------------------------------------------------------------
note "Building C reference shared library"
mkdir -p c_src/build
if (cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null 2>&1 \
      && cmake --build . >/dev/null 2>&1); then
  ok "C .so built"
else
  bad "C .so build"
fi
C_SO=$(find c_src/build -maxdepth 1 -name '*.so' | head -1)
echo "  C_SO=$C_SO"

# ---------------------------------------------------------------------------
# 3. Per-combination: check, build, symbol-diff, test (dev + release)
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  note "Combination: $label"
  FLAGS=(--no-default-features)
  [ -n "$combo" ] && FLAGS+=(--features "$combo")

  if timeout 600 cargo check "${FLAGS[@]}" >/dev/null 2>&1; then
    ok "cargo check"
  else
    bad "cargo check ($label)"
  fi

  for profile in dev release; do
    PFLAGS=("${FLAGS[@]}")
    dir=debug
    if [ "$profile" = release ]; then PFLAGS+=(--release); dir=release; fi

    if timeout 600 cargo build "${PFLAGS[@]}" >/dev/null 2>&1; then
      ok "cargo build ($profile)"
    else
      bad "cargo build ($profile, $label)"
      continue
    fi

    # Symbol parity: every symbol the C .so exports must exist in the Rust .so.
    RUST_SO="target/$dir/librev16_lib.so"
    if [ -f "$C_SO" ] && [ -f "$RUST_SO" ]; then
      nm -D --defined-only "$C_SO"    | awk '{print $3}' | grep -v '^_' | sort -u > "$WORK/c_syms"  || { bad "nm on C .so"; continue; }
      nm -D --defined-only "$RUST_SO" | awk '{print $3}' | grep -v '^_' | sort -u > "$WORK/r_syms" || { bad "nm on Rust .so"; continue; }
      n_c=$(wc -l < "$WORK/c_syms")
      # Guard against a vacuous pass: the C .so must export at least one symbol.
      if [ "$n_c" -lt 1 ]; then
        bad "symbol parity ($profile): extracted 0 C symbols -- diff would be vacuous"
      else
        missing=$(comm -23 "$WORK/c_syms" "$WORK/r_syms")
        if [ -z "$missing" ]; then
          ok "symbol parity ($profile): $n_c C symbol(s), 0 missing"
        else
          bad "symbol parity ($profile) missing: $(echo "$missing" | tr '\n' ' ')"
        fi
      fi
      # Non-libc undefined symbols in the Rust .so must be zero.
      undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' \
                | grep -vE '^(_|__|abort@|bcmp@|calloc@|close@|dl_iterate_phdr@|free@|fstat64@|getcwd@|getenv@|gettid@|lseek64@|malloc@|memcpy@|memmove@|memset@|mmap64@|munmap@|open64@|posix_memalign@|pthread_|read@|readlink@|realloc@|realpath@|stat64@|statx@|strlen@|syscall@|write@|writev@)' | sort -u)
      if [ -z "$undef" ]; then
        ok "no non-libc undefined symbols ($profile)"
      else
        bad "non-libc undefined symbols ($profile): $(echo "$undef" | tr '\n' ' ')"
      fi
    else
      bad "missing .so for symbol diff ($profile)"
    fi

    # Phase B + C differential tests.
    if timeout 600 cargo test "${PFLAGS[@]}" >/dev/null 2>&1; then
      ok "differential tests ($profile)"
    else
      bad "differential tests ($profile, $label)"
    fi
  done

  # Exhaustive 2^32 sweep, release only (fast enough there: ~25s).
  if RUN_EXHAUSTIVE_32=1 timeout 600 cargo test "${FLAGS[@]}" --release \
       --test differential config_c17 >/dev/null 2>&1; then
    ok "exhaustive 2^32 sweep"
  else
    bad "exhaustive 2^32 sweep ($label)"
  fi
done

note "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "  ALL CHECKS PASSED across ${#COMBOS[@]} feature combination(s)."
else
  echo "  FAILURES PRESENT (see [FAIL] lines above)."
fi
exit "$FAIL"
