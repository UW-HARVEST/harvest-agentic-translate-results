#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination declared in Cargo.toml,
# build the Rust cdylib for each, diff its exported symbols against the C .so,
# and run the full differential suite against that exact .so.
#
# Usage:  bash tests/feature_matrix.sh
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"
C_SO="$(ls "$ROOT"/c_src/build/*.so 2>/dev/null | head -1)"

if [[ -z "$C_SO" ]]; then
    echo "FATAL: C .so not built. Run:"
    echo "  cd $ROOT/c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    exit 1
fi
echo "C  .so: $C_SO"

# ---- enumerate features declared in [features] -----------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inf=1; next }
        /^\[/           { inf=0 }
        inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
            sub(/[[:space:]]*=.*/, ""); if ($0 != "default") print
        }
    ' Cargo.toml
)
NF_COUNT=${#FEATURES[@]}
echo "declared non-default features: $NF_COUNT ${FEATURES[*]:-(none)}"

# ---- build the list of combinations to test --------------------------------
# Always test the default build. Then every subset of the declared features
# (with --no-default-features), which for N=0 is just the empty subset.
COMBOS=("default::")
if (( NF_COUNT > 0 )); then
    total=$(( 1 << NF_COUNT ))
    for (( mask = 0; mask < total; mask++ )); do
        sel=""
        for (( i = 0; i < NF_COUNT; i++ )); do
            if (( mask & (1 << i) )); then
                sel="${sel:+$sel,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("nodefault:$sel")
    done
else
    COMBOS+=("nodefault:")
fi

FAIL=0
C_SYMS="$(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u)"

for profile in release debug; do
  for combo in "${COMBOS[@]}"; do
    kind="${combo%%:*}"
    feats="${combo#*:}"; feats="${feats#:}"

    args=()
    [[ "$profile" == "release" ]] && args+=(--release)
    if [[ "$kind" == "nodefault" ]]; then
        args+=(--no-default-features)
        [[ -n "$feats" ]] && args+=(--features "$feats")
    fi

    label="profile=$profile kind=$kind features=[${feats:-}]"
    echo
    echo "=================================================================="
    echo "### $label"
    echo "=================================================================="

    if ! timeout 600 cargo build "${args[@]}" >/tmp/fm_build.log 2>&1; then
        echo "BUILD FAILED -- $label"; tail -20 /tmp/fm_build.log; FAIL=1; continue
    fi
    if ! timeout 600 cargo check "${args[@]}" >/tmp/fm_check.log 2>&1; then
        echo "CHECK FAILED -- $label"; tail -20 /tmp/fm_check.log; FAIL=1; continue
    fi

    RS_SO="target/$profile/libsynth_pair_lib.so"
    if [[ ! -f "$RS_SO" ]]; then
        echo "MISSING $RS_SO -- $label"; FAIL=1; continue
    fi

    # ---- symbol parity for THIS build ------------------------------------
    RS_SYMS="$(nm -D --defined-only "$RS_SO" | awk '$2 ~ /^[TDBRWiu]$/ {print $NF}' | sort -u)"
    MISSING="$(comm -23 <(printf '%s\n' "$C_SYMS") <(printf '%s\n' "$RS_SYMS"))"
    if [[ -n "$MISSING" ]]; then
        echo "SYMBOL PARITY FAILED -- missing from Rust .so:"; printf '  %s\n' $MISSING; FAIL=1
    else
        echo "symbol parity: OK (0 missing of $(printf '%s\n' "$C_SYMS" | wc -l) C symbols)"
    fi

    # ---- undefined non-libc symbols --------------------------------------
    UNDEF="$(nm -D --undefined-only "$RS_SO" | awk '{print $NF}' | sed 's/@.*//' \
        | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
        | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev|sysconf|__libc_start_main|memrchr|qsort|getauxval|pthread_getattr_np|pthread_self|pthread_attr_getstack|pthread_attr_destroy|sigaltstack|sigaction|sigemptyset|sigaddset|mprotect|__stack_chk_fail|environ|_exit)$' \
        || true)"
    if [[ -n "$UNDEF" ]]; then
        echo "NOTE: undefined symbols not in the libc/unwind allowlist:"; printf '  %s\n' $UNDEF
    else
        echo "undefined non-libc symbols: 0"
    fi

    # ---- run the differential suite against THIS .so ---------------------
    export SYNTH_RUST_SO="$PWD/$RS_SO"
    if timeout 600 cargo test "${args[@]}" >/tmp/fm_test.log 2>&1; then
        grep -E 'test result' /tmp/fm_test.log | sed 's/^/  /'
    else
        echo "TESTS FAILED -- $label"; tail -40 /tmp/fm_test.log; FAIL=1
    fi
    unset SYNTH_RUST_SO
  done
done

# ---- driver binary check ---------------------------------------------------
echo
echo "=================================================================="
NEXE=$(grep -c 'add_executable' "$ROOT/c_src/CMakeLists.txt")
NBIN=$(grep -c '\[\[bin\]\]' Cargo.toml)
HASMAIN=$([[ -f src/main.rs ]] && echo 1 || echo 0)
echo "c_src add_executable count : $NEXE"
echo "Cargo [[bin]] count        : $NBIN"
echo "src/main.rs present        : $HASMAIN"
if (( NEXE == 0 && NBIN == 0 && HASMAIN == 0 )); then
    echo "no driver binary on either side -> stdout comparison N/A"
else
    echo "DRIVER BINARY EXISTS -- stdout must be compared!"; FAIL=1
fi

echo
if (( FAIL == 0 )); then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $FAIL
