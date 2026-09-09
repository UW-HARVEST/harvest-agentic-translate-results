#!/bin/bash
# Verify every test name referenced in CONFIGS.md / ERRORS.md exists as a #[test] fn.
cd "$(dirname "$0")/../translation" || exit 1
names=$(grep -ohE '\b(cfg|err|dictb|legacy|fse|huf|hist|xxh|pool|smoke)_[a-zA-Z0-9_]+' CONFIGS.md ERRORS.md | sort -u)
have=$(grep -rhoE '^fn [a-zA-Z0-9_]+' tests/*.rs | sed 's/^fn //' | sort -u)
missing=0
for n in $names; do
  if ! echo "$have" | grep -qx "$n"; then echo "MISSING TEST: $n"; missing=$((missing+1)); fi
done
echo "referenced: $(echo "$names" | wc -l)  missing: $missing"
echo "total #[test] fns in tests/: $(grep -rc '^#\[test\]' tests/*.rs | awk -F: '{s+=$2} END {print s}')"
