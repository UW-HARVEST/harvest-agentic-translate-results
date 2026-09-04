#!/usr/bin/env python3
"""Assemble translation/src/match_engine.rs from the skeleton plus the
per-chunk fragment files _gen/frag_match_[ABC].rs.

Each fragment file is divided into sections by marker lines:
    // ==== EXTRA_STATE_CONSTS ====
    // ==== EXTRA_LOCALS ====
    // ==== ARMS ====
    // ==== STATES ====
Sections may be empty or absent.
"""
import os, sys

GEN = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(GEN)
SKEL = os.path.join(GEN, 'match_engine_skeleton.rs')
OUT = os.path.join(ROOT, 'translation', 'src', 'match_engine.rs')
SECTIONS = ['EXTRA_STATE_CONSTS', 'EXTRA_LOCALS', 'ARMS', 'STATES']

def parse(path):
    out = {s: [] for s in SECTIONS}
    cur = None
    for line in open(path):
        st = line.strip()
        if st.startswith('// ====') and st.endswith('===='):
            name = st.strip('/= ').strip()
            if name in SECTIONS:
                cur = name
                continue
        if cur is not None:
            out[cur].append(line.rstrip('\n'))
    return out

frags = {}
for tag in ['A', 'B', 'C']:
    p = os.path.join(GEN, 'frag_match_%s.rs' % tag)
    frags[tag] = parse(p) if os.path.exists(p) else {s: [] for s in SECTIONS}

skel = open(SKEL).read()
report = []
for sec in SECTIONS:
    body = []
    for tag in ['A', 'B', 'C']:
        lines = frags[tag][sec]
        if lines:
            body.append('/* ---- chunk %s: %s ---- */' % (tag, sec))
            body.extend(lines)
            report.append('%s/%s: %d lines' % (tag, sec, len(lines)))
    marker = '// <<<%s>>>' % sec
    if marker not in skel:
        sys.exit('marker %s missing from skeleton' % marker)
    skel = skel.replace(marker, '\n'.join(body) if body else '')

open(OUT, 'w').write(skel)
print('assembled %s (%d lines): %s' % (OUT, skel.count('\n') + 1, ', '.join(report) or 'no fragments'))
