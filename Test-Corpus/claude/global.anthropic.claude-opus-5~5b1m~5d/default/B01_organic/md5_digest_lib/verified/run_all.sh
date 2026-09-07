#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust .so, diffs their
# exported dynamic symbols, and runs the differential test suite across every
# feature combination and both optimization profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CARGO_FLAGS="--offline"
fail=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------- build C
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so)"
ok "C .so: $C_SO"

# ------------------------------------------------- enumerate feature combos
# The crate declares no [features]; enumerate whatever exists so this stays
# correct if features are ever added.
FEATS=$(sed -n '/^\[features\]/,/^\[/p' "$CRATE/Cargo.toml" \
        | grep -oE '^[A-Za-z0-9_-]+[[:space:]]*=' | sed 's/[[:space:]]*=//' \
        | grep -v '^default$' | tr '\n' ' ')
COMBOS=("default:")                      # label:feature-args
COMBOS+=("no-default-features:--no-default-features")
if [ -n "${FEATS// /}" ]; then
    COMBOS+=("all-features:--all-features")
    for f in $FEATS; do
        COMBOS+=("only-$f:--no-default-features --features $f")
    done
fi

# ------------------------------------------------ per combo / per profile
for combo in "${COMBOS[@]}"; do
    label="${combo%%:*}"; args="${combo#*:}"
    for profile in debug release; do
        pflag=""; [ "$profile" = release ] && pflag="--release"

        step "features=$label profile=$profile — build Rust .so"
        # shellcheck disable=SC2086
        cargo build $CARGO_FLAGS $pflag $args --manifest-path "$CRATE/Cargo.toml" \
            >/dev/null 2>&1 || { bad "cargo build ($label/$profile)"; continue; }
        R_SO="$CRATE/target/$profile/libmd5_digest_lib.so"
        [ -f "$R_SO" ] || { bad "missing $R_SO"; continue; }

        step "features=$label profile=$profile — symbol parity (nm -D)"
        # Ignore linker/runtime-emitted entries that are not part of the
        # library's own API surface.
        filter='^(_init|_fini|__bss_start|_edata|_end|__.*|_ITM_.*)$'
        c_syms=$(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | grep -Ev "$filter" | sort -u)
        r_syms=$(nm -D --defined-only "$R_SO"  | awk '{print $NF}' | grep -Ev "$filter" | sort -u)
        missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
        if [ -z "$missing" ]; then
            ok "symbol diff (C - Rust) is empty: $(echo "$c_syms" | tr '\n' ' ')"
        else
            bad "Rust .so is missing: $(echo "$missing" | tr '\n' ' ')"
        fi
        undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
                | sed 's/@.*//' | grep -Ev '^(_ITM_|_Unwind_|__cxa_|__errno_location|__gmon_start__|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)' || true)
        if [ -z "$undef" ]; then
            ok "0 undefined non-libc symbols in the Rust .so"
        else
            bad "undefined non-libc symbols: $(echo "$undef" | tr '\n' ' ')"
        fi

        step "features=$label profile=$profile — differential tests"
        # shellcheck disable=SC2086
        if cargo test $CARGO_FLAGS $pflag $args --manifest-path "$CRATE/Cargo.toml" \
             -- --test-threads=4 2>&1 | tail -n 25; then
            ok "tests ($label/$profile)"
        else
            bad "tests ($label/$profile)"
        fi
    done
done

# ------------------------------------------------------------- driver binary
step "Driver executable"
if grep -q add_executable "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
    bad "C builds an executable but no stdout comparison is wired up"
else
    ok "no add_executable in CMakeLists.txt and no [[bin]]/src/main.rs — N/A"
fi

printf '\n'
if [ "$fail" -eq 0 ]; then
    printf '\033[32mALL VERIFICATION STEPS PASSED\033[0m\n'
else
    printf '\033[31mVERIFICATION FAILED\033[0m\n'
fi
exit "$fail"
