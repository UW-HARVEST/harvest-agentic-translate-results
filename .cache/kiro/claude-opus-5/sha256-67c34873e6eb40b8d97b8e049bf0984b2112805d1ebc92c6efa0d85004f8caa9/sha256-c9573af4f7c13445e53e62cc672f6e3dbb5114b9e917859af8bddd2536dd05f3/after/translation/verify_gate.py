#!/usr/bin/env python3
"""Independent completion-gate verification.

Cross-references CONFIGS.md / ERRORS.md rows against the assertion labels that
actually appear in the test sources, and against the list of tests that the
runner reported as passing. Fails loudly on any unmatched row.
"""
import re
import subprocess
import sys
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent
TESTS = sorted((ROOT / "tests").glob("*.rs"))
SRC = "\n".join(p.read_text() for p in TESTS)

fail = []


def rows(path, prefix):
    """Extract row IDs like C12 / E7 from the leading column of a md table."""
    out = []
    for line in (ROOT / path).read_text().splitlines():
        m = re.match(r"\|\s*\*{0,2}(" + prefix + r"\d+[a-z]?)\*{0,2}\s*\|", line)
        if m:
            out.append(m.group(1))
    return out


def natural(rid):
    m = re.match(r"([A-Z]+)(\d+)([a-z]?)", rid)
    return (m.group(1), int(m.group(2)), m.group(3))


cfg = rows("CONFIGS.md", "C")
err = rows("ERRORS.md", "E")
print(f"CONFIGS.md rows parsed: {len(cfg)}  (C1..{sorted(cfg, key=natural)[-1]})")
print(f"ERRORS.md  rows parsed: {len(err)}  (E1..{sorted(err, key=natural)[-1]})")

# --- contiguity: no gaps in the numbering -----------------------------------
for label, ids, prefix in (("CONFIGS", cfg, "C"), ("ERRORS", err, "E")):
    nums = sorted({natural(i)[1] for i in ids})
    expect = list(range(1, max(nums) + 1))
    missing = sorted(set(expect) - set(nums))
    if missing:
        fail.append(f"{label}.md numbering has gaps: {[prefix+str(n) for n in missing]}")
    else:
        print(f"{label}.md numbering is contiguous 1..{max(nums)}  OK")

# --- every row ID must appear as a label inside the test sources ------------
for label, ids in (("CONFIGS", cfg), ("ERRORS", err)):
    unmatched = []
    for rid in ids:
        # a row is covered if its ID appears as an assertion tag or test name
        if re.search(r'(?<![A-Za-z0-9])' + rid + r'(?![0-9])', SRC):
            continue
        unmatched.append(rid)
    if unmatched:
        fail.append(f"{label}.md rows with no test reference: {unmatched}")
    else:
        print(f"{label}.md: every row is referenced by a test  OK")

# --- run the whole suite and collect the passing test names -----------------
print("\nrunning full suite (release)…")
proc = subprocess.run(
    ["cargo", "test", "--release", "--", "--test-threads", "8"],
    cwd=ROOT, capture_output=True, text=True, timeout=600,
)
combined = proc.stdout + proc.stderr
passed = set(re.findall(r"^test (\S+) \.\.\. ok$", combined, re.M))
failed = set(re.findall(r"^test (\S+) \.\.\. FAILED$", combined, re.M))
results = re.findall(r"^test result: (\w+)\. (\d+) passed; (\d+) failed", combined, re.M)
total_pass = sum(int(p) for _, p, _ in results)
total_fail = sum(int(f) for _, _, f in results)
print(f"binaries: {len(results)}   tests passed: {total_pass}   failed: {total_fail}")
if total_fail or failed:
    fail.append(f"suite has failing tests: {sorted(failed)}")
else:
    print("suite: 0 failing tests  OK")

# --- each row must be referenced by a test function that PASSED -------------
def test_fns_of_file(text):
    """map test-fn name -> its body"""
    out = {}
    parts = re.split(r"\n#\[test\]\n", text)
    for chunk in parts[1:]:
        m = re.search(r"fn (\w+)\s*\(", chunk)
        if m:
            out[m.group(1)] = chunk
    return out

fnbodies = {}
for p in TESTS:
    fnbodies.update(test_fns_of_file(p.read_text()))
# include helper functions' text so rows tagged inside helpers still resolve
helpers = SRC

for label, ids in (("CONFIGS", cfg), ("ERRORS", err)):
    not_passing = []
    for rid in ids:
        pat = re.compile(r'(?<![A-Za-z0-9])' + rid + r'(?![0-9])')
        owners = [fn for fn, body in fnbodies.items() if pat.search(body)]
        # fall back: the row id may be embedded in a shared helper invoked with
        # a tag argument; accept a test whose *name* encodes the row
        owners += [fn for fn in passed | failed if re.match(rid.lower() + r'(_|$)', fn)]
        owners = sorted(set(owners))
        if not owners:
            not_passing.append((rid, "no owning test"))
        elif not any(o in passed for o in owners):
            not_passing.append((rid, f"owners not passing: {owners}"))
    if not_passing:
        for rid, why in not_passing:
            fail.append(f"{label}.md {rid}: {why}")
    else:
        print(f"{label}.md: every row is covered by a PASSING test  OK")

# --- symbol parity ----------------------------------------------------------
print("\nsymbol parity:")
cso = sorted((ROOT.parent / "c_src" / "build").glob("lib*.so"))[0]
rso = ROOT / "target" / "release" / "libagglom_lib.so"


def syms(p):
    out = subprocess.run(["nm", "-D", "--defined-only", str(p)],
                         capture_output=True, text=True).stdout
    return {l.split()[2] for l in out.splitlines() if len(l.split()) >= 3}


cs, rs = syms(cso), syms(rso)
missing = sorted(cs - rs)
print(f"  C exports {len(cs)} symbols; missing from Rust: {len(missing)}")
if missing:
    fail.append(f"symbols missing from Rust .so: {missing}")
else:
    print("  symbol diff is EMPTY  OK")

# every symbol must also be referenced by at least one test
untested = sorted(s for s in cs if not re.search(r'"' + re.escape(s) + r'"', SRC))
print(f"  exported symbols never called from a test: {untested or 0}")
if untested:
    fail.append(f"exported symbols with no differential test: {untested}")

# --- verdict ----------------------------------------------------------------
print("\n" + "=" * 62)
if fail:
    print("COMPLETION GATE: NOT SATISFIED")
    for f in fail:
        print("  - " + f)
    sys.exit(1)
print("COMPLETION GATE: ALL CONDITIONS SATISFIED")
print("=" * 62)
