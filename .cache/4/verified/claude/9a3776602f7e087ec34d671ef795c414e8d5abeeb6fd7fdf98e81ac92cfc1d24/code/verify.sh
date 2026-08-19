#!/usr/bin/env bash
# Full differential verification: symbol parity + every feature combination,
# in both the debug and the release profile.
#
#   ./verify.sh
#
# Nothing in c_src/ is modified; the C artifacts land in target/cbuild and
# c_src/build (the CMake build dir).
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$PWD
LOG=${TMPDIR:-/tmp}/verify-$$.log
FAIL=0

note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  OK   %s\n' "$*"; }
bad()  { printf '  FAIL %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. enumerate the valid feature combinations (Cargo.toml [features])
# ---------------------------------------------------------------------------
note "feature combinations"
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+ *=/ {sub(/ *=.*/,""); print}
' Cargo.toml)

if [ "${#FEATURES[@]}" -eq 0 ]; then
    echo "  no [features] declared -> the empty set is the only combination"
    COMBOS=("")
else
    # power set of the declared features
    COMBOS=()
    n=${#FEATURES[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi
printf '  combinations: %s\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '    [%s]\n' "${c:-<none>}"; done

# ---------------------------------------------------------------------------
# 2. cargo check for every combination
# ---------------------------------------------------------------------------
note "cargo check (every feature combination)"
for c in "${COMBOS[@]}"; do
    if timeout 600 cargo check --offline --all-targets --no-default-features \
            ${c:+--features "$c"} >"$LOG" 2>&1; then
        ok "cargo check --no-default-features --features '${c}'"
    else
        bad "cargo check --no-default-features --features '${c}'"
        tail -20 "$LOG"
    fi
done
# and the default configuration
if timeout 600 cargo check --offline --all-targets >"$LOG" 2>&1; then
    ok "cargo check (default features)"
else
    bad "cargo check (default features)"; tail -20 "$LOG"
fi

# ---------------------------------------------------------------------------
# 3. build the C artifacts
# ---------------------------------------------------------------------------
note "build C reference"
mkdir -p target/cbuild
if gcc -shared -fPIC -o target/cbuild/libc_driver.so c_src/src/main.c 2>"$LOG"; then
    ok "libc_driver.so"
else
    bad "libc_driver.so"; cat "$LOG"
fi
mkdir -p c_src/build
if (cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOG" 2>&1 \
        && cmake --build . >>"$LOG" 2>&1); then
    ok "cmake executable"
else
    bad "cmake executable"; tail -20 "$LOG"
fi

# ---------------------------------------------------------------------------
# 4. tests for every feature combination, debug + release
# ---------------------------------------------------------------------------
for profile in debug release; do
    relflag=""
    [ "$profile" = release ] && relflag="--release"
    for c in "${COMBOS[@]}"; do
        note "cargo test ($profile, features '${c:-<none>}')"
        if timeout 600 cargo test --offline $relflag --no-default-features \
                ${c:+--features "$c"} >"$LOG" 2>&1; then
            grep -E '^test result' "$LOG" | sed 's/^/    /'
            ok "tests pass ($profile, '${c:-<none>}')"
        else
            grep -E '^test result|FAILED|panicked' "$LOG" | head -20 | sed 's/^/    /'
            bad "tests ($profile, '${c:-<none>}')"
        fi
    done
done

# ---------------------------------------------------------------------------
# 5. symbol parity (per profile)
# ---------------------------------------------------------------------------
for profile in debug release; do
    note "symbol parity ($profile)"
    RSO=target/$profile/libdriver.so
    if [ ! -f "$RSO" ]; then
        timeout 600 cargo build --offline $([ "$profile" = release ] && echo --release) --lib >"$LOG" 2>&1
    fi
    C_SYMS=$(nm -D --defined-only target/cbuild/libc_driver.so | awk '{print $NF}' | sort)
    R_SYMS=$(nm -D --defined-only "$RSO" | awk '{print $NF}' | sort)
    echo "  C   : $(echo "$C_SYMS" | tr '\n' ' ')"
    echo "  Rust: $(echo "$R_SYMS" | tr '\n' ' ')"
    MISSING=$(comm -23 <(echo "$C_SYMS") <(echo "$R_SYMS"))
    EXTRA=$(comm -13 <(echo "$C_SYMS") <(echo "$R_SYMS"))
    if [ -z "$MISSING" ]; then ok "no C symbol missing from the Rust .so"; else bad "missing: $MISSING"; fi
    if [ -z "$EXTRA" ]; then ok "no extra exported symbol"; else echo "  note: extra Rust exports: $EXTRA"; fi
    if ldd -r "$RSO" 2>&1 | grep -q "undefined symbol"; then
        bad "undefined symbols in $RSO"; ldd -r "$RSO" | grep "undefined symbol" | head
    else
        ok "no undefined symbols in $RSO"
    fi
done

# ---------------------------------------------------------------------------
# 6. every row of CONFIGS.md / ERRORS.md must name a test that really exists
#    and that really ran
# ---------------------------------------------------------------------------
note "row -> test cross-check"
ALL_TESTS=$(grep -hoE '^ *fn +[a-z0-9_]+' tests/*.rs | awk '{print $2}' | sort -u)
RAN_TESTS=$(timeout 600 cargo test --offline -- --list 2>/dev/null |
            sed -n 's/^\([a-z0-9_]*\): test$/\1/p' | sort -u)
for md in CONFIGS.md ERRORS.md; do
    rows=0
    unchecked=0
    while IFS= read -r line; do
        case "$line" in
        '| '[CE][0-9]*) ;;
        *) continue ;;
        esac
        rows=$((rows + 1))
        case "$line" in
        *'[x]'*) ;;
        *) unchecked=$((unchecked + 1)); bad "$md: unchecked row: ${line:0:60}" ;;
        esac
        # every backticked identifier that looks like a test name must exist
        for t in $(printf '%s\n' "$line" | grep -oE '`(cfg|err)_[a-z0-9_]+`' | tr -d '`'); do
            if ! printf '%s\n' "$ALL_TESTS" | grep -qx "$t"; then
                bad "$md references a non-existent test: $t"
            elif ! printf '%s\n' "$RAN_TESTS" | grep -qx "$t"; then
                bad "$md references a test that did not run: $t"
            fi
        done
    done <"$md"
    ok "$md: $rows rows, $unchecked unchecked, all referenced tests exist and ran"
done

# every test in tests/ must be referenced by a row (no orphan tests)
for t in $ALL_TESTS; do
    case "$t" in
    cfg_* | err_*)
        if ! grep -qF "\`$t\`" CONFIGS.md ERRORS.md; then
            bad "test $t is not referenced by any CONFIGS.md/ERRORS.md row"
        fi
        ;;
    esac
done
ok "no orphan cfg_/err_ tests"

note "summary"
if [ "$FAIL" -eq 0 ]; then
    echo "  ALL CHECKS PASSED"
else
    echo "  FAILURES PRESENT"
fi
rm -f "$LOG"
exit "$FAIL"
