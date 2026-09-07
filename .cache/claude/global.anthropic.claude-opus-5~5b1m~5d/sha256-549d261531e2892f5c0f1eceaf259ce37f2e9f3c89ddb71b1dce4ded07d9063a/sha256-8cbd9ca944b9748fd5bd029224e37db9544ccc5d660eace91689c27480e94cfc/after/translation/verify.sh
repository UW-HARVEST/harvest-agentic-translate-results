#!/usr/bin/env bash
# Full verification gate: rebuild both libraries, diff their exported symbols,
# and run every differential test under EVERY feature combination and BOTH
# optimisation levels.
#
# Usage:  ./verify.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
FAIL=0
step() { printf '\n=== %s ===\n' "$1"; }
ok()   { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; FAIL=1; }

# ---------------------------------------------------------------------------
step "Building the C shared library"
# ---------------------------------------------------------------------------
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/dev/null 2>&1 \
  && ok "C build" || { bad "C build"; exit 1; }

C_SO=$(find "$ROOT/c_src/build" -name '*.so' | sort | head -1)
[ -n "$C_SO" ] || { bad "no C .so produced"; exit 1; }
printf 'C   .so: %s\n' "$C_SO"

# ---------------------------------------------------------------------------
step "Enumerating feature combinations"
# ---------------------------------------------------------------------------
# Every feature declared in [features], minus the implicit `default`.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1]!="default") print a[1]}' Cargo.toml)
if [ -z "$FEATURES" ]; then
    echo "no [features] declared -> the only configuration is the default"
    COMBOS=("default")
else
    # Full power set of the declared features, plus default and no-default.
    COMBOS=("default" "none")
    set -- $FEATURES
    n=$#; total=$((1 << n))
    for ((mask=1; mask<total; mask++)); do
        sel=""
        for ((b=0; b<n; b++)); do
            if (( mask & (1<<b) )); then
                eval "f=\${$((b+1))}"
                sel="${sel:+$sel,}$f"
            fi
        done
        COMBOS+=("$sel")
    done
fi
printf 'combinations: %s\n' "${COMBOS[*]}"

# ---------------------------------------------------------------------------
run_combo() {
    local combo="$1" profile="$2"
    local featflags=() proflags=()
    case "$combo" in
        default) featflags=() ;;
        none)    featflags=(--no-default-features) ;;
        *)       featflags=(--no-default-features --features "$combo") ;;
    esac
    [ "$profile" = release ] && proflags=(--release)

    local label="[$combo/$profile]"

    cargo build "${proflags[@]}" "${featflags[@]}" >/dev/null 2>&1 \
        || { bad "$label rust build"; return; }

    local rust_so="target/$profile/libpoly_ray_lib.so"
    [ -f "$rust_so" ] || { bad "$label no $rust_so"; return; }

    # --- symbol parity -----------------------------------------------------
    local cs rs missing extra
    cs=$(nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort)
    rs=$(nm -D --defined-only "$rust_so" | awk '$2=="T"{print $3}' | sort)
    missing=$(comm -23 <(echo "$cs") <(echo "$rs"))
    extra=$(comm -13 <(echo "$cs") <(echo "$rs"))
    if [ -n "$missing" ]; then
        bad "$label symbols MISSING from Rust .so:"; echo "$missing"
    elif [ -n "$extra" ]; then
        bad "$label symbols EXTRA in Rust .so:"; echo "$extra"
    else
        ok "$label symbol parity ($(echo "$cs" | wc -l) symbols, diff empty)"
    fi

    # Undefined symbols in the Rust .so must all be libc / unwinder.
    local badundef
    badundef=$(nm -D --undefined-only "$rust_so" | awk '{print $NF}' \
        | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^__cxa_|^__tls_get_addr' || true)
    if [ -n "$badundef" ]; then
        bad "$label non-libc undefined symbols:"; echo "$badundef"
    else
        ok "$label no non-libc undefined symbols"
    fi

    # --- differential tests -------------------------------------------------
    if C_SO="$C_SO" RUST_SO="$PWD/$rust_so" \
       timeout 600 cargo test "${proflags[@]}" "${featflags[@]}" \
       -- --test-threads=4 >"${TMPDIR:-.}/vt.$$" 2>&1; then
        ok "$label tests ($(grep -c '\.\.\. ok' "${TMPDIR:-.}/vt.$$") passed)"
    else
        bad "$label tests"; tail -40 "${TMPDIR:-.}/vt.$$"
    fi
    rm -f "${TMPDIR:-.}/vt.$$"
}

for combo in "${COMBOS[@]}"; do
    for profile in debug release; do
        step "combo=$combo profile=$profile"
        run_combo "$combo" "$profile"
    done
done

printf '\n=========================================\n'
if [ "$FAIL" = 0 ]; then
    echo "ALL CHECKS PASSED"
else
    echo "SOME CHECKS FAILED"
fi
exit "$FAIL"
