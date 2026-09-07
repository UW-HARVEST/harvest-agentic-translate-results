#!/usr/bin/env bash
# Full verification pipeline: builds both shared objects, gates on the symbol
# diff, and runs the Phase B/C differential suites across every build
# configuration. Intended to be the single command that reproduces the whole
# result.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE="$PWD"
ROOT="$(cd .. && pwd)"
FAIL=0

step() { printf '\n=== %s ===\n' "$1"; }
ok()   { printf '  [ OK ] %s\n' "$1"; }
bad()  { printf '  [FAIL] %s\n' "$1"; FAIL=1; }

# --------------------------------------------------------------------------
step "Build the C shared library"
# --------------------------------------------------------------------------
(
  mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && timeout 600 cmake --build . >/dev/null
) && ok "cmake build" || bad "cmake build"

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
[ -n "$C_SO" ] && ok "C .so: $(basename "$C_SO")" || { bad "no C .so produced"; exit 1; }

# --------------------------------------------------------------------------
step "Enumerate build configurations"
# --------------------------------------------------------------------------
# Cargo features declared in Cargo.toml, if any. With no [features] table the
# only configurations are the default and --no-default-features.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+ *=/{print $1}' Cargo.toml)
if [ -z "$FEATURES" ]; then
  ok "no [features] declared -> configurations: default, --no-default-features"
  CONFIGS=("" "--no-default-features")
else
  ok "features found: $FEATURES"
  CONFIGS=("" "--no-default-features")
  for f in $FEATURES; do
    CONFIGS+=("--no-default-features --features $f")
  done
  # All features together.
  CONFIGS+=("--all-features")
fi

# --------------------------------------------------------------------------
step "Per-configuration: build, symbol diff, differential tests"
# --------------------------------------------------------------------------
for cfg in "${CONFIGS[@]}"; do
  label="${cfg:-default}"
  printf '\n--- configuration: %s ---\n' "$label"

  for profile in release debug; do
    if [ "$profile" = release ]; then
      # shellcheck disable=SC2086
      timeout 600 cargo build --release $cfg >/dev/null 2>&1 \
        && ok "cargo build --release $cfg" || { bad "cargo build --release $cfg"; continue; }
      RUST_SO="$CRATE/target/release/libspec_ray_lib.so"
    else
      # shellcheck disable=SC2086
      timeout 600 cargo build $cfg >/dev/null 2>&1 \
        && ok "cargo build $cfg" || { bad "cargo build $cfg"; continue; }
      RUST_SO="$CRATE/target/debug/libspec_ray_lib.so"
    fi

    # ---- symbol parity gate -------------------------------------------------
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
    extra=$(comm -13 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u))
    if [ -z "$missing" ]; then
      n=$(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u | wc -l)
      ok "symbol diff empty ($n symbols) [$profile]"
    else
      bad "symbols missing from the Rust .so [$profile]: $(echo "$missing" | tr '\n' ' ')"
    fi
    [ -n "$extra" ] && printf '  [note] Rust exports extra symbols: %s\n' "$(echo "$extra" | tr '\n' ' ')"

    # Any undefined symbol that is not libc / libgcc-unwind.
    undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $2}' \
      | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_' || true)
    [ -z "$undef" ] && ok "no non-libc undefined symbols [$profile]" \
      || bad "non-libc undefined symbols [$profile]: $(echo "$undef" | tr '\n' ' ')"

    # ---- differential suites ----------------------------------------------
    # shellcheck disable=SC2086
    C_SO_PATH="$C_SO" RUST_SO_PATH="$RUST_SO" \
      timeout 600 cargo test --release $cfg --test configs -- --test-threads=4 >/tmp/vb.$$ 2>&1 \
      && ok "Phase B (CONFIGS.md): $(grep -c '^test cfg_' /tmp/vb.$$) rows passed against the $profile .so" \
      || { bad "Phase B failed [$profile/$label]"; tail -30 /tmp/vb.$$; }

    # shellcheck disable=SC2086
    C_SO_PATH="$C_SO" RUST_SO_PATH="$RUST_SO" \
      timeout 600 cargo test --release $cfg --test errors -- --test-threads=4 >/tmp/vc.$$ 2>&1 \
      && ok "Phase C (ERRORS.md): $(grep -c '^test err_' /tmp/vc.$$) rows passed against the $profile .so" \
      || { bad "Phase C failed [$profile/$label]"; tail -30 /tmp/vc.$$; }

    rm -f /tmp/vb.$$ /tmp/vc.$$
  done
done

# --------------------------------------------------------------------------
step "Binary / driver stdout comparison"
# --------------------------------------------------------------------------
if grep -q add_executable "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
  bad "c_src declares an executable but this script does not compare stdout"
else
  ok "n/a: c_src builds only a SHARED library (no add_executable)"
fi
if [ -d src/bin ] || grep -q '\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ]; then
  bad "the crate declares a binary but this script does not compare stdout"
else
  ok "n/a: the crate is cdylib-only (no [[bin]], no src/main.rs, no src/bin/)"
fi

# --------------------------------------------------------------------------
step "Stub / placeholder scan"
# --------------------------------------------------------------------------
stubs=$(grep -nE 'unimplemented!|todo!|unreachable!\(\)' src/lib.rs || true)
[ -z "$stubs" ] && ok "no unimplemented!/todo! stubs in src/lib.rs" \
  || bad "stubs present: $stubs"

# --------------------------------------------------------------------------
printf '\n=========================================\n'
if [ "$FAIL" -eq 0 ]; then
  printf 'VERIFICATION PASSED\n'
else
  printf 'VERIFICATION FAILED\n'
fi
printf '=========================================\n'
exit "$FAIL"
