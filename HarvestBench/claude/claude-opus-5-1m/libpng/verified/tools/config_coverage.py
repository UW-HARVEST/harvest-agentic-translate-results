#!/usr/bin/env python3
"""Stamp the CONFIGS.md check-marks from the results of an actual test run.

    ./check.sh                                   # writes target/test_output.txt
    python3 tools/config_coverage.py [--stamp]

Each CONFIGS.md row names the test that covers it ("Row → test mapping").  A row
is checked off only when that test is present in the run *and* passed.  Tests that
were split into `name`, `name_2`, `name_3`, … all count for the row (a prefix
match), and every part must have passed.

Exits non-zero if any row's test is missing or failed, so an unchecked row cannot
slip through unnoticed.
"""
import re
import sys
import collections

CONFIGS = 'CONFIGS.md'
OUTPUT = 'target/test_output.txt'

# ---- what actually ran -------------------------------------------------------
# cargo test prints:   Running tests/<file>.rs (target/release/deps/<file>-<hash>)
# then:                test <name> ... ok | FAILED | ignored
results = {}          # "file::test" -> "ok" | "FAILED" | "ignored"
cur = None
for line in open(OUTPUT, encoding='utf-8', errors='replace'):
    m = re.search(r'Running (?:unittests )?tests[/\\]([A-Za-z0-9_]+)\.rs', line)
    if m:
        cur = m.group(1)
        continue
    m = re.match(r'^test ([A-Za-z0-9_:]+) \.\.\. (ok|FAILED|ignored)', line.strip())
    if m and cur:
        results['%s::%s' % (cur, m.group(1))] = m.group(2)

if not results:
    sys.exit('no test results found in %s -- run ./check.sh first' % OUTPUT)

# ---- the row -> test mapping -------------------------------------------------
text = open(CONFIGS, encoding='utf-8').read()
mapping = {}
for m in re.finditer(r'^\| (C-\d+) \| `([A-Za-z0-9_:]+)` \|$', text, re.M):
    mapping[m.group(1)] = m.group(2)

if not mapping:
    sys.exit('no "Row -> test mapping" table found in %s' % CONFIGS)


def status_of(test):
    """A row's test may have been split into name, name_2, name_3 ..."""
    if '::' not in test:
        return 'MISSING', []
    f, n = test.split('::', 1)
    hits = [k for k in results
            if k == test or re.match(re.escape(test) + r'(_\d+)?$', k)]
    if not hits:
        return 'MISSING', []
    sts = {results[h] for h in hits}
    if 'FAILED' in sts:
        return 'FAILED', hits
    if sts == {'ignored'}:
        return 'IGNORED', hits
    return 'ok', hits


ok, bad = 0, []
per_row = {}
for row, test in sorted(mapping.items(), key=lambda kv: int(kv[0][2:])):
    st, hits = status_of(test)
    per_row[row] = (test, st, hits)
    if st == 'ok':
        ok += 1
    else:
        bad.append((row, test, st))

print('CONFIGS.md: %d/%d rows covered by a passing test' % (ok, len(mapping)))
for row, test, st in bad:
    print('  %-7s %-40s %s' % (row, test, st))

if '--stamp' in sys.argv:
    lines = text.split('\n')
    out = []
    n = 0
    for l in lines:
        m = re.match(r'^\| (C-\d+) \| (.*) \| \[[ x]\] \|$', l)
        if m and m.group(1) in per_row:
            mark = 'x' if per_row[m.group(1)][1] == 'ok' else ' '
            l = '| %s | %s | [%s] |' % (m.group(1), m.group(2), mark)
            n += 1
        out.append(l)
    open(CONFIGS, 'w').write('\n'.join(out))
    print('stamped %d rows in CONFIGS.md' % n)

sys.exit(0 if not bad else 1)
