#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and runs `cargo check`,
# `cargo build --release` and `cargo test` against each one.
#
# The crate currently declares no `[features]`, so the enumeration collapses to
# the single default configuration. The script is written generically so that
# adding features does not require touching it.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# Feature names declared in [features], excluding `default`.
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { in_f = 1; next }
        /^\[/           { in_f = 0 }
        in_f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            sub(/[[:space:]]*=.*/, "")
            gsub(/[[:space:]]/, "")
            if ($0 != "default") print
        }
    ' Cargo.toml
)

# Every subset of FEATURES, as comma-separated strings ("" == no features).
combos=("")
for f in "${FEATURES[@]}"; do
    new=()
    for c in "${combos[@]}"; do
        if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    combos+=("${new[@]}")
done

# The default feature set is also a valid configuration in its own right.
LABELS=("<default>")
ARGS=("")
for c in "${combos[@]}"; do
    if [ -z "$c" ]; then
        LABELS+=("--no-default-features")
        ARGS+=("--no-default-features")
    else
        LABELS+=("--no-default-features --features $c")
        ARGS+=("--no-default-features --features $c")
    fi
done

echo "Declared features: ${#FEATURES[@]} (${FEATURES[*]:-none})"
echo "Configurations to verify: ${#LABELS[@]}"
echo

rc=0
for i in "${!LABELS[@]}"; do
    label="${LABELS[$i]}"
    # shellcheck disable=SC2206 # deliberate word splitting of the flag string
    args=(${ARGS[$i]})

    echo "=============================================================="
    echo "CONFIG: $label"
    echo "=============================================================="

    for step in "check" "build --release" "test" "test --release"; do
        # shellcheck disable=SC2206
        step_args=($step)
        printf '  cargo %s ... ' "$step"
        if out=$(timeout 600 cargo "${step_args[@]}" "${args[@]}" 2>&1); then
            echo "ok"
        else
            echo "FAILED"
            echo "$out" | tail -n 40 | sed 's/^/    /'
            rc=1
        fi
    done
    echo
done

if [ "$rc" -eq 0 ]; then
    echo "All ${#LABELS[@]} configuration(s) passed."
else
    echo "At least one configuration failed."
fi
exit "$rc"
