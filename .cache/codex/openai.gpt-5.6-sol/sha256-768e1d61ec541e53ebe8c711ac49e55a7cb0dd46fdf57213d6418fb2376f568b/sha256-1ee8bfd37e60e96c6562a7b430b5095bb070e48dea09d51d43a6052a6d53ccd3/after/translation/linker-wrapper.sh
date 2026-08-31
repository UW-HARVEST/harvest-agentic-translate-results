#!/usr/bin/env bash
set -euo pipefail

filtered=()
export_map="$(dirname "$0")/exports.map"
for argument in "$@"; do
    case "$argument" in
        -Wl,--version-script=*)
            filtered+=("-Wl,--version-script=${export_map}")
            ;;
        *)
            filtered+=("$argument")
            ;;
    esac
done

exec cc "${filtered[@]}"
