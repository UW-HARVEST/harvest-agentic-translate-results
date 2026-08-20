#!/usr/bin/env python3
"""Audit that every row in CONFIGS.md / ERRORS.md is backed by a PASSING test.

  ./audit_rows.py [--release]

Checks:
  1. the row ids in each table are contiguous (no row silently dropped);
  2. every name in the "test function" column of the traceability tables
     resolves to either a test function defined in tests/ or a case-name string
     literal used by one of them;
  3. every such test function reports "ok" in a full `cargo test` run;
  4. every row id is mentioned by the traceability table.
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
os.chdir(ROOT)
RELEASE = "--release" in sys.argv
PROFILE = ["--release"] if RELEASE else []

rc = 0


def rows(path, prefix):
    ids = []
    for line in open(path, encoding="utf-8"):
        m = re.match(r"^\|\s*(" + prefix + r"\d{2})\s*\|", line)
        if m:
            ids.append(m.group(1))
    return ids


def traceability_names(path):
    names, intable = [], False
    for line in open(path, encoding="utf-8"):
        if re.match(r"^## Row .* test-function traceability", line):
            intable = True
            continue
        if intable and line.startswith("##"):
            intable = False
        if intable and line.startswith("|"):
            cols = line.split("|")
            if len(cols) > 3:
                for n in re.findall(r"`([a-z][a-z0-9_]*)`", cols[2]):
                    names.append(n)
    return names


# ---- 1 + 4: row id contiguity / presence -------------------------------------
row_ids = {}
for path, prefix in (("CONFIGS.md", "B"), ("ERRORS.md", "E")):
    ids = rows(path, prefix)
    uniq = sorted(set(ids))
    hi = max(int(i[1:]) for i in uniq)
    print(f"=== {path}: {len(uniq)} distinct {prefix} rows (max {prefix}{hi:02d}) ===")
    for i in range(1, hi + 1):
        rid = f"{prefix}{i:02d}"
        if rid not in uniq:
            print(f"  MISSING ROW: {rid}")
            rc = 1
    row_ids[prefix] = uniq

# every row id must appear somewhere in the traceability tables
trace_text = ""
for p in ("CONFIGS.md", "ERRORS.md"):
    txt = open(p, encoding="utf-8").read()
    idx = txt.find("## Row ")
    trace_text += txt[idx:] if idx >= 0 else ""
for prefix in ("B", "E"):
    for rid in row_ids[prefix]:
        if rid not in trace_text:
            print(f"  ROW NOT IN TRACEABILITY TABLE: {rid}")
            rc = 1

# ---- 2: names resolve ---------------------------------------------------------
test_src = {}
for fn in sorted(os.listdir("tests")):
    if fn.endswith(".rs"):
        test_src[fn] = open(os.path.join("tests", fn), encoding="utf-8").read()
common = os.path.join("tests", "common", "mod.rs")
if os.path.exists(common):
    test_src["common/mod.rs"] = open(common, encoding="utf-8").read()
all_src = "\n".join(test_src.values())

names = sorted(set(traceability_names("CONFIGS.md") + traceability_names("ERRORS.md")))
print(f"=== traceability names: {len(names)} ===")
test_fns, other = [], []
for n in names:
    if re.search(r"fn\s+" + re.escape(n) + r"\s*\(", all_src) or re.search(
        r"!\(\s*" + re.escape(n) + r"\s*,", all_src
    ):
        test_fns.append(n)
    elif f'"{n}"' in all_src:
        other.append(n)
    else:
        print(f"  UNRESOLVED NAME (neither a test fn nor a case string): {n}")
        rc = 1
print(f"    test functions: {len(test_fns)}, case-name strings: {len(other)}")

# ---- 3: run the suite and require "ok" for each named test fn ----------------
subprocess.run(["cargo", "build", "--offline"] + PROFILE, check=True,
               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
out = subprocess.run(
    ["cargo", "test", "--offline"] + PROFILE + ["--", "--test-threads=4"],
    capture_output=True, text=True,
)
log = out.stdout + out.stderr
results = dict(re.findall(r"^test ([a-z0-9_]+) \.\.\. (\w+)", log, re.M))
for n in test_fns:
    if results.get(n) != "ok":
        print(f"  TEST NOT PASSING: {n} -> {results.get(n)!r}")
        rc = 1
passed = sum(1 for v in results.values() if v == "ok")
print(f"=== cargo test{' --release' if RELEASE else ''}: "
      f"{passed} test functions passed, {len(results) - passed} not ok ===")
for line in re.findall(r"^test result:.*$", log, re.M):
    print("   ", line)
if "FAILED" in log or out.returncode != 0:
    print("  CARGO TEST REPORTED FAILURES")
    rc = 1

print("AUDIT: PASS" if rc == 0 else "AUDIT: FAIL")
sys.exit(rc)
