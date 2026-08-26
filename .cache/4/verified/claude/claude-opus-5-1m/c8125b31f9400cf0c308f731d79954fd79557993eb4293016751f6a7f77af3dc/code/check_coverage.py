#!/usr/bin/env python3
"""Mechanically verify the Phase A artifacts against the actual test suite.

Fails (exit 1) if:
  * ERRORS.md / CONFIGS.md rows are not a contiguous 1..N range,
  * a row references a test function that does not exist,
  * a row references an in-test label that does not appear in that test's source,
  * a declared C export is missing from the artifact list,
  * a `cfg*` / `err*` test exists that no row references (untracked coverage).
"""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
TESTS = ROOT / "tests"
problems = []


def note(msg):
    problems.append(msg)
    print(f"FAIL: {msg}")


# --- collect every #[test] fn, and which file it lives in --------------------
test_files = {}
test_src = {}
for f in sorted(TESTS.glob("*.rs")):
    src = f.read_text()
    test_src[f.name] = src
    for m in re.finditer(r"#\[test\]\s*\nfn\s+(\w+)", src):
        test_files[m.group(1)] = f.name

print(f"discovered {len(test_files)} #[test] functions in {len(test_src)} files")


def rows_of(doc):
    """Return {row_number: row_body} for the main table of a Phase A doc."""
    text = (ROOT / doc).read_text()
    out = {}
    for m in re.finditer(r"^\| +(\d+) +\|(.*)$", text, re.M):
        out[int(m.group(1))] = m.group(2)
    return out


def check_doc(doc, expected_n, prefix):
    rows = rows_of(doc)
    nums = sorted(rows)
    if nums != list(range(1, expected_n + 1)):
        note(f"{doc}: rows are not a contiguous 1..{expected_n} range: {nums}")
    else:
        print(f"{doc}: {len(nums)} rows, contiguous 1..{expected_n}  OK")

    for n, body in rows.items():
        names = re.findall(r"`(\w+)`", body)
        fns = [x for x in names if x in test_files]
        # a row must point at some existing #[test]
        candidates = [x for x in names if x.startswith(prefix) or x.endswith("_configurations")
                      or x.endswith("_error_paths")]
        if not fns:
            note(f"{doc} row {n}: references no existing #[test] fn (saw {candidates or names})")
            continue
        fn = fns[0]
        # if the row names an in-test label, that label must appear in the source
        for label in re.findall(r"label `(row[\w/]+)`", body):
            if label not in test_src[test_files[fn]]:
                note(f"{doc} row {n}: label '{label}' not found in {test_files[fn]}")
    return rows


print()
err_rows = check_doc("ERRORS.md", 30, "err")
cfg_rows = check_doc("CONFIGS.md", 40, "cfg")

# --- every cfg*/err* test must be referenced by some row --------------------
referenced = set()
for rows in (err_rows, cfg_rows):
    for body in rows.values():
        referenced.update(re.findall(r"`(\w+)`", body))
for fn in test_files:
    if (fn.startswith("cfg") or fn.startswith("err")) and fn not in referenced:
        note(f"test {fn} is not referenced by any ERRORS.md/CONFIGS.md row")

# --- SYMBOLS.md must list every export of the C .so ------------------------
c_so = next((p for p in (ROOT / "c_src/build").glob("*.so")), None)
if c_so is None:
    note("no C .so found under c_src/build (build it first)")
else:
    nm = subprocess.run(["nm", "-D", "--defined-only", str(c_so)],
                        capture_output=True, text=True, check=True).stdout
    c_syms = sorted({l.split()[-1].split("@")[0] for l in nm.splitlines() if l.split()})
    symdoc = (ROOT / "SYMBOLS.md").read_text()
    for s in c_syms:
        if f"`{s}`" not in symdoc:
            note(f"SYMBOLS.md does not mention C export '{s}'")
    print(f"SYMBOLS.md: all {len(c_syms)} C exports mentioned  OK")

    rust_so = ROOT / "target/debug/libdoubleneg_lib.so"
    if rust_so.exists():
        nm_r = subprocess.run(["nm", "-D", "--defined-only", str(rust_so)],
                              capture_output=True, text=True, check=True).stdout
        r_syms = {l.split()[-1].split("@")[0] for l in nm_r.splitlines() if l.split()}
        missing = [s for s in c_syms if s not in r_syms]
        if missing:
            note(f"Rust .so is missing C exports: {missing}")
        else:
            print(f"symbol parity: all {len(c_syms)} C exports present in the Rust .so  OK")
    else:
        note(f"Rust .so not built at {rust_so} (run `cargo build`)")

print()
if problems:
    print(f"=== {len(problems)} problem(s) found ===")
    sys.exit(1)
print("=== coverage bookkeeping is consistent ===")
