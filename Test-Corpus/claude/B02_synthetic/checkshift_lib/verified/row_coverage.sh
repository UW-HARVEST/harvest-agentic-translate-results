#!/usr/bin/env bash
# Gate: every row in CONFIGS.md and ERRORS.md must name a test that (a) exists
# in tests/ and (b) actually ran and passed. Also fails if any row is unchecked.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

PROFILE_FLAG=""
[[ ${1:-debug} == release ]] && PROFILE_FLAG="--release"

tmp=${TMPDIR:-/tmp}

# Collect the list of tests that actually passed.
timeout 600 cargo test --no-default-features $PROFILE_FLAG -- --test-threads=1 \
  > "$tmp/row_tests.log" 2>&1
grep -oE '^test [a-z0-9_]+ \.\.\. ok$' "$tmp/row_tests.log" \
  | awk '{print $2}' | sort -u > "$tmp/passed.txt"
echo "tests passed: $(wc -l < "$tmp/passed.txt")"

rc=0

check_file() {
  local md=$1 label=$2
  # rows look like: | C7 | ... | `test_name` | [x] |
  local rows
  rows=$(grep -cE '^\| (C|E|G)[0-9]+[a-z]? \|' "$md")
  echo "== $label: $rows rows =="
  local n=0
  while IFS= read -r line; do
    local id names
    id=$(echo "$line" | awk -F'|' '{gsub(/ /,"",$2); print $2}')
    # every `backticked` identifier in the row that looks like a test name
    names=$(echo "$line" | grep -oE '`[a-z][a-z0-9_]{4,}`' | tr -d '`' | sort -u)
    if [[ -z $names ]]; then
      echo "  $id: NO TEST NAMED"; rc=1; continue
    fi
    local found=0
    for t in $names; do
      if grep -q "^$t$" "$tmp/passed.txt"; then found=1; fi
    done
    if (( found == 0 )); then
      echo "  $id: none of [$(echo $names | tr '\n' ' ')] is a passing test"; rc=1
    fi
    # the checkbox must be ticked
    if ! echo "$line" | grep -q '\[x\]'; then
      echo "  $id: row is NOT checked off"; rc=1
    fi
    n=$((n+1))
  done < <(grep -E '^\| (C|E|G)[0-9]+[a-z]? \|' "$md")
  echo "  verified $n rows"
}

check_file CONFIGS.md "CONFIGS.md (Phase B)"
check_file ERRORS.md  "ERRORS.md (Phase C)"

# Every test that exists must be referenced by a row (no orphan coverage claims
# and, more importantly, no test silently absent from the tables).
awk '/^#\[test\]/{want=1;next} want && /^fn /{gsub(/[(].*/,"",$2);print $2;want=0}' \
  tests/phase_b_valid.rs tests/phase_c_errors.rs | sort -u > "$tmp/defined.txt"
echo "== tests defined in phase_b/phase_c: $(wc -l < "$tmp/defined.txt") =="
while read -r t; do
  if ! grep -qF "\`$t\`" CONFIGS.md ERRORS.md; then
    echo "  orphan test not listed in any row: $t"; rc=1
  fi
done < "$tmp/defined.txt"

if (( rc == 0 )); then echo "ROW COVERAGE: OK"; else echo "ROW COVERAGE: FAILED"; fi
exit $rc
