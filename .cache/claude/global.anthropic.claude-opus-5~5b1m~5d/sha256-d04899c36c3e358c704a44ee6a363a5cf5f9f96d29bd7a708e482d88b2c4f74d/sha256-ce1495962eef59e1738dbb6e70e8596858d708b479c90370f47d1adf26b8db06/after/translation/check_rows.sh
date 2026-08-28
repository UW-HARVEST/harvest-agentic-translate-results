#!/usr/bin/env bash
# Cross-check the Phase A artifacts against reality:
#   * every test name named in ERRORS.md / CONFIGS.md must exist in tests/
#   * every row must be checked off ([x])
#   * every named test must actually have run and passed
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here" || exit 2

rc=0

rows() { # file -> "row-id<TAB>test-name<TAB>checkbox"
    grep -E '^\| (C|E)[0-9]+ ' "$1" \
      | awk -F'|' '{ gsub(/^ +| +$/,"",$2); n=NF; box=$(n-1); test=$(n-2);
                     gsub(/^ +| +$/,"",box); gsub(/^ +| +$/,"",test);
                     gsub(/`/,"",test); print $2"\t"test"\t"box }'
}

# collect the list of tests that passed in the last run
passed="$(mktemp)"; trap 'rm -f "$passed"' EXIT
cargo test --offline 2>&1 | grep -E '^test [a-z0-9_]+ \.\.\. ok$' \
    | awk '{print $2}' | sort -u > "$passed"
echo "tests that passed: $(wc -l < "$passed")"

for f in ERRORS.md CONFIGS.md; do
    n=0
    while IFS=$'\t' read -r id test box; do
        n=$((n+1))
        [ -z "$test" ] && continue
        if [ "$box" != "[x]" ]; then
            echo "!! $f $id: row not checked off (box='$box')"; rc=1
        fi
        if ! grep -qE "^fn ${test}\(" tests/*.rs; then
            echo "!! $f $id: no test function named '$test' in tests/"; rc=1
        fi
        if ! grep -qx "$test" "$passed"; then
            echo "!! $f $id: test '$test' did not pass in the last run"; rc=1
        fi
    done < <(rows "$f")
    echo "$f: $n rows checked"
done

if [ "$rc" -eq 0 ]; then echo "ROW CROSS-CHECK: OK"; else echo "ROW CROSS-CHECK: FAILED"; fi
exit "$rc"
