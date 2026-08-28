#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth across every
# build-time configuration.
#
# `translation/Cargo.toml` declares no `[features]` table, so the complete set
# of valid feature combinations is the empty one. It is still exercised both as
# `--no-default-features` and with defaults enabled, in debug and release, to
# cover every code path the crate can be compiled into.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
log_dir=/tmp/wcscat-verify
mkdir -p "$log_dir"

fail=0

step() { printf '\n=== %s ===\n' "$1"; }

# --- Enumerate feature combinations ----------------------------------------
step 'feature combinations'
features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}' \
    "$root/translation/Cargo.toml")
if [[ -z "$features" ]]; then
    echo 'no [features] declared -> single configuration (empty feature set)'
else
    echo "declared features: $features"
fi

# --- Build the C reference --------------------------------------------------
step 'build C shared library'
(
    cd "$root/c_src" && mkdir -p build && cd build &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
        cmake --build .
) >"$log_dir/cmake.log" 2>&1 || { echo 'FAILED (see cmake.log)'; fail=1; }
c_so=$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $c_so"

# --- cargo check / test for each configuration ------------------------------
for feat_flag in "--no-default-features" ""; do
    for profile_flag in "" "--release"; do
        label="cargo${feat_flag:+ $feat_flag}${profile_flag:+ $profile_flag}"
        slug=$(echo "$label" | tr -c 'a-zA-Z0-9' '-')

        step "check: $label"
        if timeout 600 cargo check $feat_flag $profile_flag \
            --manifest-path "$root/translation/Cargo.toml" \
            >"$log_dir/check$slug.log" 2>&1; then
            echo 'OK'
        else
            echo "FAILED (see check$slug.log)"; tail -20 "$log_dir/check$slug.log"; fail=1; continue
        fi

        step "build cdylib: $label"
        timeout 600 cargo build $feat_flag $profile_flag \
            --manifest-path "$root/translation/Cargo.toml" \
            >"$log_dir/build$slug.log" 2>&1 || { echo "FAILED"; fail=1; continue; }

        profile_dir=$([[ -n "$profile_flag" ]] && echo release || echo debug)
        rust_so="$root/translation/target/$profile_dir/libwcscat_lib.so"
        [[ -f "$rust_so" ]] || { echo "MISSING artifact: $rust_so"; fail=1; continue; }

        step "symbol parity: $label"
        c_syms=$(nm -D --defined-only "$c_so" | awk '{print $3}' | sort)
        r_syms=$(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort)
        missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
        if [[ -n "$missing" ]]; then
            echo "MISSING from Rust .so:"; echo "$missing"; fail=1
        else
            echo "OK (C exports: $(echo $c_syms | tr '\n' ' '))"
        fi

        step "test: $label"
        if timeout 600 cargo test $feat_flag $profile_flag \
            --manifest-path "$root/translation/Cargo.toml" \
            >"$log_dir/test$slug.log" 2>&1; then
            grep -E '^test result:' "$log_dir/test$slug.log"
        else
            echo "FAILED (see test$slug.log)"; tail -40 "$log_dir/test$slug.log"; fail=1
        fi
    done
done

step 'summary'
if [[ $fail -eq 0 ]]; then
    echo 'ALL CONFIGURATIONS PASS'
else
    echo 'FAILURES PRESENT'
fi
exit $fail
