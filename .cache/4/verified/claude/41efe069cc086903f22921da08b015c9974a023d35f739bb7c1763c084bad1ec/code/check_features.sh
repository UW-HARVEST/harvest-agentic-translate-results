#!/usr/bin/env bash
#
# Phase A / Phase D helper: mechanically derive every valid build configuration
# from Cargo.toml and run `cargo check` + the full differential test-suite for
# each one.  Nothing is hard-coded, so the loop stays correct if features are
# added later.
#
#   ./check_features.sh            # check + test every feature combination
#   ./check_features.sh check      # only `cargo check`
#
set -uo pipefail
cd "$(dirname "$0")"

MODE="${1:-all}"
FAILED=0

# --- enumerate the feature power set --------------------------------------
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, json, subprocess, sys

md = json.loads(subprocess.run(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"],
    capture_output=True, text=True, check=True).stdout)
pkg = next(p for p in md["packages"] if p["name"] == "driver")
feats = sorted(f for f in pkg["features"] if f != "default")

if not feats:
    # No [features] table at all -> exactly one valid configuration.
    print("")
    sys.exit(0)

if len(feats) <= 8:
    combos = [c for r in range(len(feats) + 1) for c in itertools.combinations(feats, r)]
else:  # keep the sweep tractable: empty, every single, and all-together
    combos = [()] + [(f,) for f in feats] + [tuple(feats)]

seen = set()
for c in combos:
    key = ",".join(c)
    if key not in seen:
        seen.add(key)
        print(key)
PY
) || { echo "failed to enumerate features"; exit 1; }

echo "=== ${#COMBOS[@]} feature combination(s) ==="
for i in "${!COMBOS[@]}"; do
    printf '  [%d] %s\n' "$i" "${COMBOS[$i]:-<none>}"
done
echo

for combo in "${COMBOS[@]}"; do
    label="${combo:-<none>}"

    args=(--no-default-features)
    if [ -n "$combo" ]; then
        args+=(--features "$combo")
    fi

    echo "--- cargo check --no-default-features --features '$label' ---"
    if ! timeout 600 cargo check "${args[@]}" --all-targets 2>&1 | tail -n 5; then
        echo "!!! CHECK FAILED for '$label'"; FAILED=1; continue
    fi

    if [ "$MODE" = check ]; then continue; fi

    echo "--- cargo build (cdylib) for '$label' ---"
    if ! timeout 600 cargo build "${args[@]}" 2>&1 | tail -n 3; then
        echo "!!! BUILD FAILED for '$label'"; FAILED=1; continue
    fi

    echo "--- cargo test for '$label' ---"
    if ! DIFF_TEST_FEATURES="$combo" timeout 600 cargo test "${args[@]}" 2>&1 | tail -n 12; then
        echo "!!! TESTS FAILED for '$label'"; FAILED=1
    fi
    echo
done

# --- the two remaining cargo-level configurations -------------------------
for extra in "--all-features" ""; do
    echo "--- cargo test $extra (default feature selection) ---"
    if ! timeout 600 cargo test $extra 2>&1 | tail -n 8; then
        echo "!!! TESTS FAILED for '$extra'"; FAILED=1
    fi
done

# --- release profile (panic = "abort", overflow checks off) ---------------
#
# `cargo test --release` cannot link a `panic = "abort"` cdylib into an unwinding
# test binary (rust-lang/cargo#6313), so the *release* artefacts are built
# separately and handed to the (dev-profile) harness through DIFF_RUST_SO /
# DIFF_RUST_DRIVER.  What matters is which artefact is under test, not how the
# harness itself was compiled.
echo "--- release artefacts under a dev-profile harness ---"
if ! timeout 600 cargo build --release 2>&1 | tail -n 3; then
    echo "!!! RELEASE BUILD FAILED"; FAILED=1
else
    if ! DIFF_RUST_SO="$PWD/target/release/libdriver.so" \
         DIFF_RUST_DRIVER="$PWD/target/release/driver" \
         timeout 600 cargo test 2>&1 | grep -E 'test result|^error'; then
        echo "!!! RELEASE ARTEFACT TESTS FAILED"; FAILED=1
    fi
fi

# --- the C reference at every optimisation level ---------------------------
# The translated code must match the C semantics, not one compiler's rendering
# of them; a disagreement between -O0 and -O3 would mean the tests are relying
# on undefined behaviour.
CBUILD="$PWD/target/cref"
mkdir -p "$CBUILD"
for opt in -O0 -O1 -O2 -O3 -Os; do
    so="$CBUILD/libcdriver$opt.so"
    exe="$CBUILD/c_driver$opt"
    "${CC:-cc}" "$opt" -shared -fPIC -o "$so" c_src/src/lib.c || { FAILED=1; continue; }
    "${CC:-cc}" "$opt" -o "$exe" c_src/src/main.c c_src/src/lib.c || { FAILED=1; continue; }
    echo "--- C reference built with $opt ---"
    if ! DIFF_C_SO="$so" DIFF_C_DRIVER="$exe" timeout 600 cargo test 2>&1 | grep -E 'test result|^error'; then
        echo "!!! TESTS FAILED against C built with $opt"; FAILED=1
    fi
done

if [ "$FAILED" = 0 ]; then
    echo "ALL FEATURE COMBINATIONS OK"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
