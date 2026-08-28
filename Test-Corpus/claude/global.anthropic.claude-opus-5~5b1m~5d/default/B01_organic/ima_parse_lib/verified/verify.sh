#!/usr/bin/env bash
# Full four-phase verification driver.
#
#   ./verify.sh
#
# Builds the C shared library and the Rust cdylib, checks symbol parity
# (Phase A / D), then runs the Phase B + Phase C differential test suites for
# every Cargo feature combination and for both the dev and release profiles.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
fail=0
note() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '   \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '   \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) && ok "c_src" || bad "c_src build"
ls -l "$root"/c_src/build/lib*.so

# ---------------------------------------------------------------------------
# Enumerate the Cargo feature combinations. `cargo test` does NOT build a
# `crate-type = ["cdylib"]` target, so each combination needs an explicit
# `cargo build` first — that is what produces the .so the tests dlopen.
note "Enumerating Cargo feature combinations"
features="$(awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /^[A-Za-z0-9_-]+[ \t]*=/ {
        split($0, a, "="); gsub(/[ \t]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
' "$here/Cargo.toml" | sort -u)"

combos=()
if [[ -z "$features" ]]; then
    echo "   (no [features] declared -> default and --no-default-features are"
    echo "    the only two configurations, and they are identical)"
    combos+=("" "--no-default-features" "--all-features")
else
    echo "   features: $features"
    # Full power set of the declared features, plus the two baselines.
    mapfile -t farr <<<"$features"
    n=${#farr[@]}
    combos+=("" "--no-default-features" "--all-features")
    for ((mask = 1; mask < (1 << n); mask++)); do
        sel=()
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && sel+=("${farr[i]}")
        done
        combos+=("--no-default-features --features $(
            IFS=,
            echo "${sel[*]}"
        )")
    done
fi
printf '   combination: [%s]\n' "${combos[@]}"

# ---------------------------------------------------------------------------
cd "$here"
for profile in dev release; do
    prof_flag=""
    prof_dir="debug"
    if [[ $profile == release ]]; then
        prof_flag="--release"
        prof_dir="release"
    fi
    for combo in "${combos[@]}"; do
        label="profile=$profile combo=[${combo:-<default>}]"
        note "$label"

        # shellcheck disable=SC2086
        if cargo build $prof_flag $combo >/dev/null 2>&1; then
            ok "cargo build"
        else
            bad "cargo build ($label)"
            continue
        fi

        so="$here/target/$prof_dir/libima_parse_lib.so"
        if [[ -f $so ]]; then ok "cdylib present: $so"; else bad "cdylib missing"; continue; fi

        if "$here/check_symbols.sh" "$so" >/dev/null 2>&1; then
            ok "symbol parity"
        else
            bad "symbol parity ($label)"
            "$here/check_symbols.sh" "$so" || true
        fi

        # shellcheck disable=SC2086
        if timeout 600 cargo test $prof_flag $combo >"$here/verify-$profile.log" 2>&1; then
            ok "cargo test  ($(grep -c '^test .* ok$' "$here/verify-$profile.log") tests ok)"
        else
            bad "cargo test ($label)"
            tail -60 "$here/verify-$profile.log"
        fi
    done
done

note "Detailed symbol report"
"$here/check_symbols.sh" || fail=1

note "RESULT"
if ((fail)); then
    echo "VERIFICATION FAILED"
    exit 1
fi
echo "VERIFICATION COMPLETE — all phases passed."
