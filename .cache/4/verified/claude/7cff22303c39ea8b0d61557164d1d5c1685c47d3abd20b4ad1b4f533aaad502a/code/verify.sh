#!/usr/bin/env bash
# Full verification driver: Phases A -> D.
#
#   1. Builds the C reference shared object exactly as c_src/CMakeLists.txt says.
#   2. Enumerates EVERY valid feature combination from Cargo.toml (the power set
#      of [features], minus `default`, plus the default build itself).
#   3. For each combination: cargo check, cargo build, `nm -D` symbol diff
#      against the C .so, and the full differential test suite in both the
#      `dev` and the `release` profile.
#
# Usage: ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"

LOGDIR="${TMPDIR:-/tmp}/tfm-verify"
mkdir -p "$LOGDIR"
rc=0
step() { printf '\n=== %s ===\n' "$*"; }
run() { # run <logfile> <cmd...>
    local log="$1"; shift
    if timeout 600 "$@" >"$log" 2>&1; then
        return 0
    fi
    echo "    FAILED: $*"
    echo "    ---- last 40 lines of $log ----"
    tail -n 40 "$log" | sed 's/^/    /'
    rc=1
    return 1
}

# ---------------------------------------------------------------------------
step "Phase A.1 — build the C reference shared object"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) >"$LOGDIR/cmake.log" 2>&1
if [ $? -ne 0 ]; then
    echo "  C build FAILED"; tail -n 30 "$LOGDIR/cmake.log" | sed 's/^/    /'; exit 1
fi
C_SO="$(ls c_src/build/*.so 2>/dev/null | head -1)"
[ -n "$C_SO" ] || { echo "  no C .so produced"; exit 1; }
echo "  C .so: $C_SO"

# ---------------------------------------------------------------------------
step "Phase A.2 — enumerate every valid feature combination"
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip()
        if name and name != 'default':
            feats.append(name)
print('\n'.join(feats))
PY
)
# Drop empty lines produced when there are no non-default features.
CLEAN=()
if [ "${#FEATURES[@]}" -gt 0 ]; then
    for f in "${FEATURES[@]}"; do [ -n "$f" ] && CLEAN+=("$f"); done
fi
FEATURES=()
if [ "${#CLEAN[@]}" -gt 0 ]; then FEATURES=("${CLEAN[@]}"); fi
NF=${#FEATURES[@]}
if [ "$NF" -gt 0 ]; then
    echo "  non-default features declared: $NF -> ${FEATURES[*]}"
else
    echo "  non-default features declared: 0 (none)"
fi

# Power set of FEATURES -> combos for --no-default-features --features <combo>
COMBOS=()
for ((mask = 0; mask < (1 << NF); mask++)); do
    combo=""
    for ((b = 0; b < NF; b++)); do
        if (((mask >> b) & 1)); then
            combo="${combo:+$combo,}${FEATURES[$b]}"
        fi
    done
    COMBOS+=("$combo")
done
echo "  ${#COMBOS[@]} --no-default-features combination(s) + 1 default build"

# ---------------------------------------------------------------------------
verify_one() { # verify_one <label> <cargo-feature-flags...>
    local label="$1"; shift
    local flags=()
    if [ "$#" -gt 0 ]; then flags=("$@"); fi
    local tag
    tag="$(printf '%s' "$label" | tr -c 'A-Za-z0-9_.-' '_')"

    step "Phase B/C/D — configuration: $label"

    run "$LOGDIR/check-$tag.log" cargo check --offline ${flags[@]+"${flags[@]}"} --all-targets \
        && echo "  cargo check              ok"

    for profile in dev release; do
        local pflag=() pdir=debug
        if [ "$profile" = release ]; then pflag=(--release); pdir=release; fi

        run "$LOGDIR/build-$tag-$profile.log" \
            cargo build --offline ${flags[@]+"${flags[@]}"} ${pflag[@]+"${pflag[@]}"} \
            && echo "  cargo build ($profile)      ok"

        local R_SO="target/$pdir/libtfm_lib.so"
        if [ ! -f "$R_SO" ]; then
            echo "  MISSING Rust .so: $R_SO"; rc=1; continue
        fi

        # ---- Phase D: symbol parity -------------------------------------
        nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u >"$LOGDIR/c.syms"
        nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u >"$LOGDIR/r-$tag-$profile.syms"
        local missing
        missing="$(comm -23 "$LOGDIR/c.syms" "$LOGDIR/r-$tag-$profile.syms")"
        if [ -n "$missing" ]; then
            echo "  SYMBOL PARITY FAILED ($profile) — missing from the Rust .so:"
            printf '%s\n' "$missing" | sed 's/^/      /'
            rc=1
        else
            echo "  symbol parity ($profile)    ok ($(wc -l <"$LOGDIR/c.syms") C symbol(s), 0 missing)"
        fi
        # No non-libc undefined symbols in the Rust .so.
        local undef
        undef="$(nm -D --undefined-only "$R_SO" \
                 | awk '$1 == "U" {print $2}' \
                 | sed 's/@.*//' \
                 | grep -vxF -f <(printf '%s\n' \
                     _Unwind_Backtrace _Unwind_GetDataRelBase _Unwind_GetIP \
                     _Unwind_GetIPInfo _Unwind_GetLanguageSpecificData \
                     _Unwind_GetRegionStart _Unwind_GetTextRelBase _Unwind_Resume \
                     _Unwind_SetGR _Unwind_SetIP __errno_location __tls_get_addr \
                     abort bcmp calloc close dl_iterate_phdr free fstat64 getcwd \
                     getenv lseek64 malloc memcpy memmove memset mmap64 munmap \
                     open64 posix_memalign pthread_key_create pthread_key_delete \
                     pthread_setspecific read readlink realloc realpath stat64 \
                     strlen syscall write writev sqrtf sqrt memcmp \
                     pthread_getspecific __cxa_thread_atexit_impl \
                     pthread_mutex_lock pthread_mutex_unlock pthread_self \
                     pthread_getattr_np pthread_attr_getstack pthread_attr_destroy \
                     sysconf getrandom poll sigaltstack mprotect signal \
                     __libc_start_main environ pipe2 fcntl dup3 sched_getaffinity \
                     _Unwind_RaiseException _Unwind_DeleteException \
                     _Unwind_GetCFA _Unwind_FindEnclosingFunction \
                     pthread_rwlock_rdlock pthread_rwlock_unlock \
                     __register_atfork gnu_get_libc_version \
                    ) || true)"
        if [ -n "$undef" ]; then
            echo "  NOTE: non-allowlisted undefined symbols in the Rust .so ($profile):"
            printf '%s\n' "$undef" | sed 's/^/      /'
            rc=1
        else
            echo "  no non-libc undefined   ok ($profile)"
        fi

        # ---- Phases B + C: the differential suite -----------------------
        if run "$LOGDIR/test-$tag-$profile.log" \
               cargo test --offline ${flags[@]+"${flags[@]}"} ${pflag[@]+"${pflag[@]}"}; then
            local passed
            passed="$(grep -c '^test .* ok$' "$LOGDIR/test-$tag-$profile.log")"
            echo "  differential tests ($profile) ok ($passed test(s) passed)"
        fi
    done
}

for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
        verify_one "--no-default-features" --no-default-features
    else
        verify_one "--no-default-features --features $combo" \
            --no-default-features --features "$combo"
    fi
done
verify_one "default features (implicit)"

# ---------------------------------------------------------------------------
step "SUMMARY"
if [ "$rc" -eq 0 ]; then
    echo "  ALL PHASES PASSED for every feature combination."
    echo "  Logs: $LOGDIR"
else
    echo "  VERIFICATION FAILED — see the output above and $LOGDIR"
fi
exit "$rc"
