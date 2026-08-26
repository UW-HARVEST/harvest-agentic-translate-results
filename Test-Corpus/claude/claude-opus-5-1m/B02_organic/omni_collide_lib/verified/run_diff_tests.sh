#!/usr/bin/env bash
# Phase D driver: build both libraries, diff their exported symbols, and run the
# full differential suite against EVERY build configuration.
#
#   ./run_diff_tests.sh
#
# `Cargo.toml` declares no [features], so there is exactly one feature
# combination (default == --no-default-features == --all-features).  To make up
# for that, the same suite is run against the Rust cdylib built at four
# different optimisation levels plus the shipping `release` profile
# (panic = "abort"), because THAT is what actually changes codegen here -- in
# particular the `fmul`/`fadd` inline-asm helpers and the f32 operand ordering.

set -uo pipefail
cd "$(dirname "$0")"

TMP="${TMPDIR:-/tmp}"
mkdir -p "$TMP/difflogs"
FAIL=0
SUMMARY="$TMP/difflogs/summary.txt"
: > "$SUMMARY"

say() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }

# ---------------------------------------------------------------------------
say "=== 1. build the C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$TMP/difflogs/cbuild.log" 2>&1
if [ ! -f c_src/build/libtranslated_rust.so ]; then
    say "FAIL: C .so was not produced (see $TMP/difflogs/cbuild.log)"
    exit 1
fi
C_SO="$PWD/c_src/build/libtranslated_rust.so"
say "     $C_SO"

# ---------------------------------------------------------------------------
say ""
say "=== 2. enumerate feature combinations ==="
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"=");gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
    say "     Cargo.toml declares NO [features] -> exactly 1 combination (default)"
    COMBOS=("")
else
    say "     features found: $FEATURES"
    # power set
    COMBOS=()
    feats=($FEATURES)
    n=${#feats[@]}
    for ((mask=0; mask<(1<<n); mask++)); do
        combo=""
        for ((i=0; i<n; i++)); do
            if (( mask & (1<<i) )); then
                combo="${combo:+$combo,}${feats[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

say ""
say "=== 3. cargo check every feature combination ==="
for combo in "${COMBOS[@]}"; do
    label="${combo:-<none>}"
    if timeout 300 cargo check --no-default-features ${combo:+--features "$combo"} \
            > "$TMP/difflogs/check-${combo:-none}.log" 2>&1; then
        say "     OK   cargo check --no-default-features --features '$label'"
    else
        say "     FAIL cargo check --no-default-features --features '$label'"
        tail -30 "$TMP/difflogs/check-${combo:-none}.log" | tee -a "$SUMMARY"
        FAIL=1
    fi
    if timeout 300 cargo check --no-default-features ${combo:+--features "$combo"} --tests \
            > "$TMP/difflogs/checktests-${combo:-none}.log" 2>&1; then
        say "     OK   cargo check --tests   --features '$label'"
    else
        say "     FAIL cargo check --tests   --features '$label'"
        tail -30 "$TMP/difflogs/checktests-${combo:-none}.log" | tee -a "$SUMMARY"
        FAIL=1
    fi
done

# ---------------------------------------------------------------------------
# Build the Rust cdylib in every configuration we want to verify.
# name|cargo args|env
CONFIGS=(
  "release|--release|"
  "dev-O0|:|RUSTFLAGS=-Copt-level=0"
  "dev-O1|:|RUSTFLAGS=-Copt-level=1"
  "dev-O2|:|RUSTFLAGS=-Copt-level=2"
  "dev-O3|:|RUSTFLAGS=-Copt-level=3"
  "dev-Os|:|RUSTFLAGS=-Copt-level=s"
)

say ""
say "=== 4. build the Rust cdylib in every configuration ==="
declare -A SO_PATH
for cfg in "${CONFIGS[@]}"; do
    IFS='|' read -r name args envs <<< "$cfg"
    [ "$args" = ":" ] && args=""
    outdir="$TMP/difflogs/so-$name"
    mkdir -p "$outdir"
    if [ -n "$envs" ]; then
        env "$envs" timeout 300 cargo build $args --target-dir "$TMP/tgt-$name" \
            > "$TMP/difflogs/build-$name.log" 2>&1
    else
        timeout 300 cargo build $args --target-dir "$TMP/tgt-$name" \
            > "$TMP/difflogs/build-$name.log" 2>&1
    fi
    prof=debug
    [ -n "$args" ] && prof=release
    so="$TMP/tgt-$name/$prof/libomni_collide_lib.so"
    if [ -f "$so" ]; then
        cp "$so" "$outdir/libomni_collide_lib.so"
        SO_PATH[$name]="$outdir/libomni_collide_lib.so"
        say "     OK   $name -> ${SO_PATH[$name]}"
    else
        say "     FAIL $name: no .so produced (see $TMP/difflogs/build-$name.log)"
        tail -20 "$TMP/difflogs/build-$name.log" | tee -a "$SUMMARY"
        FAIL=1
    fi
done

# ---------------------------------------------------------------------------
say ""
say "=== 5. symbol parity (nm -D) for every configuration ==="
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "$TMP/difflogs/c_syms.txt"
NC=$(wc -l < "$TMP/difflogs/c_syms.txt")
say "     C .so exports $NC global symbols"
for name in "${!SO_PATH[@]}"; do
    nm -D --defined-only "${SO_PATH[$name]}" | awk '{print $3}' | sort \
        > "$TMP/difflogs/rust_syms-$name.txt"
    missing=$(comm -23 "$TMP/difflogs/c_syms.txt" "$TMP/difflogs/rust_syms-$name.txt")
    extra=$(comm -13 "$TMP/difflogs/c_syms.txt" "$TMP/difflogs/rust_syms-$name.txt")
    nr=$(wc -l < "$TMP/difflogs/rust_syms-$name.txt")
    if [ -z "$missing" ]; then
        say "     OK   $name: $nr symbols, 0 missing"
    else
        say "     FAIL $name: MISSING from Rust .so:"
        printf '%s\n' "$missing" | sed 's/^/            /' | tee -a "$SUMMARY"
        FAIL=1
    fi
    if [ -n "$extra" ]; then
        say "     note $name: extra (non-C) exported symbols:"
        printf '%s\n' "$extra" | sed 's/^/            /' | tee -a "$SUMMARY"
    fi
    # undefined symbols must all be libc / libgcc-unwind
    nm -D --undefined-only "${SO_PATH[$name]}" | awk '{print $2}' | sed 's/@.*//' \
        | grep -v -E '^(_ITM_|_Unwind_|__cxa_|__errno_location|__gmon_start__|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|fstat64|getcwd|getenv|gettid|lseek|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap|mmap64|munmap|open|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|sqrt|sqrtf|stat|stat64|statx|strlen|syscall|write|writev)' \
        > "$TMP/difflogs/undef-$name.txt" || true
    if [ -s "$TMP/difflogs/undef-$name.txt" ]; then
        say "     FAIL $name: non-libc undefined symbols:"
        sed 's/^/            /' "$TMP/difflogs/undef-$name.txt" | tee -a "$SUMMARY"
        FAIL=1
    else
        say "     OK   $name: 0 undefined non-libc symbols"
    fi
done

# ---------------------------------------------------------------------------
say ""
say "=== 6. run the full differential suite against every configuration ==="
# The test harness itself is always built with the dev profile (so that
# `catch_unwind` in harness_sanity works); only the library UNDER TEST changes.
for combo in "${COMBOS[@]}"; do
  for name in release dev-O0 dev-O1 dev-O2 dev-O3 dev-Os; do
    so="${SO_PATH[$name]:-}"
    [ -z "$so" ] && continue
    label="features='${combo:-<none>}' so=$name"
    log="$TMP/difflogs/test-${combo:-none}-$name.log"
    if C2_C_SO="$C_SO" C2_RUST_SO="$so" \
       timeout 600 cargo test --no-default-features ${combo:+--features "$combo"} \
       -- --test-threads="$(nproc)" --nocapture > "$log" 2>&1; then
        # authoritative counts: sum the per-binary "test result:" lines
        read -r passed failed ignored < <(grep "^test result:" "$log" \
            | awk '{p+=$4; f+=$6; i+=$8} END{print p, f, i}')
        if [ "${failed:-1}" != "0" ] || [ "${passed:-0}" -lt 98 ]; then
            say "     FAIL $label: passed=$passed failed=$failed ignored=$ignored (expected 98/0/3)"
            FAIL=1
            continue
        fi
        # prove the intended libraries were the ones actually dlopen()ed
        if ! grep -q "\[diff\] Rust .so = $(readlink -f "$so")$" "$log"; then
            say "     FAIL $label: harness did not load the expected Rust .so"
            FAIL=1
            continue
        fi
        if ! grep -q "\[diff\] C   .so = $(readlink -f "$C_SO")$" "$log"; then
            say "     FAIL $label: harness did not load the expected C .so"
            FAIL=1
            continue
        fi
        say "     OK   $label ($passed passed / $failed failed / $ignored ignored, both .so paths verified)"
    else
        say "     FAIL $label (see $log)"
        grep -E '^(test .* FAILED|failures:|DIVERGENCE|thread .* panicked|error)' "$log" \
            | head -30 | sed 's/^/            /' | tee -a "$SUMMARY"
        FAIL=1
    fi
  done
done

# ---------------------------------------------------------------------------
say ""
if [ "$FAIL" -eq 0 ]; then
    say "=== ALL CONFIGURATIONS PASSED ==="
else
    say "=== FAILURES PRESENT -- see $SUMMARY ==="
fi
exit "$FAIL"
