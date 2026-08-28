#!/usr/bin/env bash
# Enumerates every valid feature combination declared in translation/Cargo.toml
# and runs `cargo check` + `cargo test` for each, in both dev and release
# profiles. Automates steps 2 and 9 of the verification plan.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
LOG=/tmp/verify_all.log
: > "$LOG"

# --- Enumerate features -------------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys, pathlib
txt = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
for f in feats:
    print(f)
PY
)

DEFAULT_FEATURES=$(python3 - <<'PY'
import re, pathlib
txt = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\s*default\s*=\s*\[(.*?)\]', txt, re.M | re.S)
print(','.join(x.strip().strip('"\'') for x in m.group(1).split(',') if x.strip()) if m else '')
PY
)

echo "Declared non-default features: ${FEATURES[*]:-<none>}"
echo "Default feature set: ${DEFAULT_FEATURES:-<empty>}"

# Build the powerset of declared features. With no [features] table the only
# valid configuration is the empty set, which is also the default build.
COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
    COMBOS=("")
else
    for (( mask = 0; mask < (1 << n); mask++ )); do
        combo=""
        for (( i = 0; i < n; i++ )); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

echo "Enumerated ${#COMBOS[@]} feature combination(s)."
echo

# --- Build the C ground truth -------------------------------------------------
echo "== building C shared library =="
( mkdir -p "$ROOT/c_src/build" \
  && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) >> "$LOG" 2>&1 || { echo "C BUILD FAILED (see $LOG)"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $C_SO"
echo

FAIL=0

run_step() {  # run_step <label> <cmd...>
    local label="$1"; shift
    printf '  %-46s' "$label"
    if timeout 600 "$@" >> "$LOG" 2>&1; then
        echo "PASS"
    else
        echo "FAIL"
        FAIL=1
        { echo "### FAILED: $label"; echo "### cmd: $*"; } >> "$LOG"
    fi
}

for combo in "${COMBOS[@]}"; do
    label="${combo:-<no features>}"
    echo "== feature combination: $label =="
    {
        echo
        echo "############################################################"
        echo "# feature combination: $label"
        echo "############################################################"
    } >> "$LOG"

    feat_args=(--no-default-features)
    [[ -n "$combo" ]] && feat_args+=(--features "$combo")

    run_step "cargo check (dev)"   cargo check "${feat_args[@]}" --all-targets
    run_step "cargo build (dev)"   cargo build "${feat_args[@]}"
    run_step "cargo test  (dev)"   cargo test  "${feat_args[@]}"
    run_step "cargo build (release)" cargo build --release "${feat_args[@]}"
    run_step "cargo test  (release)" cargo test  --release "${feat_args[@]}"

    # Symbol parity, checked against the very artifacts the tests load. The test
    # harness builds the cdylib into target/harness-<profile>/ precisely because
    # `cargo test` does not refresh a cdylib on its own.
    for profile in debug release; do
        rust_so="target/harness-$profile/$profile/liboverunder_lib.so"
        printf '  %-46s' "symbol parity ($profile)"
        if [[ ! -f "$rust_so" ]]; then
            echo "FAIL (missing $rust_so)"; FAIL=1; continue
        fi
        missing=$(comm -23 \
            <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u) \
            <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u))
        if [[ -z "$missing" ]]; then
            echo "PASS"
        else
            echo "FAIL"; FAIL=1
            echo "missing symbols in $rust_so:"$'\n'"$missing" | tee -a "$LOG"
        fi
    done
    echo
done

# The default configuration must be exercised too, in case `default` pulls in
# features that the powerset above covered only via explicit opt-in.
echo "== default configuration =="
run_step "cargo test (dev, default features)" cargo test
run_step "cargo test (release, default features)" cargo test --release
echo

if (( FAIL )); then
    echo "RESULT: FAILURES PRESENT — see $LOG"
    exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) pass in dev and release, with full symbol parity."
