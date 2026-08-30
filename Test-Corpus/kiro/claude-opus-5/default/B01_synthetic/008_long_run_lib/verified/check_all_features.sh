#!/usr/bin/env bash
# Enumerate every valid build-time configuration and `cargo check` each one.
#
# Findings for this crate:
#   * translation/Cargo.toml declares no `[features]` table at all, so there are
#     no optional features and no default feature set.
#   * c_src/CMakeLists.txt declares no `option()`/`add_definitions` and the C
#     source has no `#ifdef`s, so the C library likewise has a single
#     configuration.
# The powerset of features is therefore just the empty set, which means exactly
# one configuration. The loop below still derives that from Cargo.toml rather
# than hard-coding it, so it keeps working if features are ever added.
set -euo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Feature names from `cargo metadata` (empty here).
features=$(cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print("\n".join(json.load(sys.stdin)["packages"][0]["features"].keys()))')

mapfile -t names < <(printf '%s\n' "$features" | sed '/^$/d' | sort)
echo "declared features: ${names[*]:-<none>}"

# Build the powerset of feature names.
combos=("")
for name in "${names[@]:-}"; do
    [[ -z "$name" ]] && continue
    for existing in "${combos[@]}"; do
        if [[ -z "$existing" ]]; then
            combos+=("$name")
        else
            combos+=("$existing,$name")
        fi
    done
done

status=0
for combo in "${combos[@]}"; do
    label="${combo:-<no features>}"
    echo "== cargo check --no-default-features --features '$combo'  ($label) =="
    if ! cargo check --no-default-features --features "$combo" --all-targets; then
        echo "FAILED: $label"
        status=1
    fi
done

# The default feature set is a distinct configuration whenever `default` exists;
# check it unconditionally so it is never skipped.
echo "== cargo check (default features) =="
cargo check --all-targets || status=1

exit "$status"
