#!/usr/bin/env python3
"""Total the differential comparisons the whole suite performed.

Every test process appends `<binary>\t<in-process cases>\t<forked cases>` to
`target/observed/cases-<pid>.txt` from an `atexit` hook, so the numbers come from
the runs themselves rather than from counting loops by hand.
"""
import glob
import collections

per = collections.Counter()
perf = collections.Counter()
for f in sorted(glob.glob('target/observed/cases-*.txt')):
    for line in open(f):
        p = line.rstrip('\n').split('\t')
        if len(p) == 3:
            name = p[0].rsplit('-', 1)[0]
            per[name] += int(p[1])
            perf[name] += int(p[2])

print('%-14s %12s %12s' % ('test binary', 'assert_same', 'forked'))
print('-' * 40)
for k in sorted(set(per) | set(perf)):
    print('%-14s %12d %12d' % (k, per[k], perf[k]))
print('-' * 40)
print('%-14s %12d %12d' % ('TOTAL', sum(per.values()), sum(perf.values())))
print('grand total: %d differential comparisons'
      % (sum(per.values()) + sum(perf.values())))
