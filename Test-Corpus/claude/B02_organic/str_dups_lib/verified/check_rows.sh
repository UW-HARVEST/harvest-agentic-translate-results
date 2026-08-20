#!/usr/bin/env bash
# Verify that every test name referenced by CONFIGS.md / ERRORS.md really exists
# as a #[test] function and really passed in the last full run.
set -uo pipefail
cd "$(dirname "$0")"

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT

# All #[test] fns actually defined (ignoring the #[ignore]d child cases).
grep -hoE '^fn (cfg|err|abi)_[a-z0-9_]+' tests/*.rs | sed 's/^fn //' | sort -u > "$tmp/defined.txt"
grep -hoE '^fn child_[a-z0-9_]+' tests/*.rs | sed 's/^fn //' | sort -u > "$tmp/children.txt"

# Names referenced from the tables (backticked identifiers starting cfg_/err_).
grep -hoE '`(cfg|err|abi)_[a-z0-9_]+`' CONFIGS.md ERRORS.md | tr -d '`' | sort -u > "$tmp/referenced.txt"

echo "defined tests:      $(wc -l < "$tmp/defined.txt")"
echo "ignored child cases: $(wc -l < "$tmp/children.txt")"
echo "referenced in docs: $(wc -l < "$tmp/referenced.txt")"

rc=0
undef="$(comm -13 "$tmp/defined.txt" "$tmp/referenced.txt")"
if [[ -n "$undef" ]]; then
  echo "REFERENCED BUT NOT DEFINED:"; echo "$undef" | sed 's/^/  /'; rc=1
else
  echo "every referenced test exists"
fi
undoc="$(comm -23 "$tmp/defined.txt" "$tmp/referenced.txt")"
if [[ -n "$undoc" ]]; then
  echo "DEFINED BUT NOT REFERENCED (informational):"; echo "$undoc" | sed 's/^/  /'
fi

# Row counts
echo "CONFIGS.md rows: $(grep -cE '^\| *[0-9]+[a-z]? *\|' CONFIGS.md)"
echo "ERRORS.md  rows: $(grep -cE '^\| *[0-9]+[a-z]? *\|' ERRORS.md)"
echo "unchecked rows in CONFIGS.md: $(grep -E '^\| *[0-9]+[a-z]? *\|' CONFIGS.md | grep -cv '\[x\]')"
echo "unchecked rows in ERRORS.md : $(grep -E '^\| *[0-9]+[a-z]? *\|' ERRORS.md  | grep -cv '\[x\]')"
if grep -E '^\| *[0-9]+[a-z]? *\|' CONFIGS.md ERRORS.md | grep -v '\[x\]' | grep -q .; then
  echo "UNCHECKED ROWS PRESENT:"
  grep -E '^\| *[0-9]+[a-z]? *\|' CONFIGS.md ERRORS.md | grep -v '\[x\]' | sed 's/^/  /'
  rc=1
fi

# Confirm each defined test passed in a fresh run.
echo "--- running the suite to confirm ---"
cargo test --offline -- --test-threads=1 2>&1 | grep -E '^test [a-z]' > "$tmp/run.txt"
grep -E '\.\.\. ok$' "$tmp/run.txt" | awk '{print $2}' | sort -u > "$tmp/passed.txt"
grep -E 'ignored$' "$tmp/run.txt" | awk '{print $2}' | sort -u > "$tmp/ignored.txt"
echo "passed: $(wc -l < "$tmp/passed.txt")  ignored(child cases): $(wc -l < "$tmp/ignored.txt")"
notpassed="$(comm -23 "$tmp/defined.txt" "$tmp/passed.txt")"
if [[ -n "$notpassed" ]]; then
  echo "DEFINED BUT DID NOT PASS:"; echo "$notpassed" | sed 's/^/  /'; rc=1
else
  echo "every defined non-child test passed"
fi
# every child case must be exercised by some parent test
for c in $(cat "$tmp/children.txt"); do
  if ! grep -q "\"$c\"" tests/errors.rs; then
    echo "CHILD CASE NEVER DRIVEN BY A PARENT: $c"; rc=1
  fi
done
[[ $rc == 0 ]] && echo "ROW/TEST BOOKKEEPING OK"
exit $rc
