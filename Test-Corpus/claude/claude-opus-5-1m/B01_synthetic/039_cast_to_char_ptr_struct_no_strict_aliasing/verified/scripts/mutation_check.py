#!/usr/bin/env python3
"""Harness validation ("who tests the tests?").

Injects a behavioural bug into the Rust translation, rebuilds, and requires the
differential suite to FAIL. A mutation that survives is either
  * semantically equivalent to the original (must be justified), or
  * a genuine blind spot in the test suite.

usage: python3 scripts/mutation_check.py [--quick]
"""
import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "src/imp.rs")

# (name, needle, replacement, expected: "caught" | "equivalent")
MUTATIONS = [
    ("bedrooms 3 -> 4", "bedrooms: 3,", "bedrooms: 4,", "caught"),
    ("bathrooms 2.0 -> 2.5", "bathrooms: 2.0,", "bathrooms: 2.5,", "caught"),
    ("floors big-endian", "self.floors.to_le_bytes()", "self.floors.to_be_bytes()", "caught"),
    ("bathrooms big-endian", "self.bathrooms.to_le_bytes()", "self.bathrooms.to_be_bytes()",
     "caught"),
    ("hex uppercase",
     "line.push(char::from_digit(u32::from(b >> 4), 16).unwrap());",
     "line.push(char::from_digit(u32::from(b >> 4), 16).unwrap().to_ascii_uppercase());",
     "caught"),
    ("no trailing newline", "line.push('\\n');", "line.push(' ');", "caught"),
    ("print only 15 bytes", "p.iter().take(count)", "p.iter().take(count.saturating_sub(1))",
     "caught"),
    ("nibbles swapped",
     "line.push(char::from_digit(u32::from(b >> 4), 16).unwrap());\n        line.push(char::from_digit(u32::from(b & 0x0f), 16).unwrap());",
     "line.push(char::from_digit(u32::from(b & 0x0f), 16).unwrap());\n        line.push(char::from_digit(u32::from(b >> 4), 16).unwrap());",
     "caught"),
    ("isspace drops \\v",
     "matches!(b, b' ' | b'\\t' | b'\\n' | b'\\x0b' | b'\\x0c' | b'\\r')",
     "matches!(b, b' ' | b'\\t' | b'\\n' | b'\\x0c' | b'\\r')",
     "caught"),
    ("isspace drops \\n (fgets-like)",
     "matches!(b, b' ' | b'\\t' | b'\\n' | b'\\x0b' | b'\\x0c' | b'\\r')",
     "matches!(b, b' ' | b'\\t' | b'\\x0b' | b'\\x0c' | b'\\r')",
     "caught"),
    ("'+' not accepted as a sign", "        b'+' => {", "        b'+' if false => {", "caught"),
    ("'+' treated as '-'", "        b'-' => {\n            negative = true;",
     "        b'-' | b'+' => {\n            negative = true;", "caught"),
    ("digit run accepts hex letters", "if b.is_ascii_digit() => c = b",
     "if b.is_ascii_hexdigit() => c = b", "caught"),
    ("first digit check accepts hex letters", "if !c.is_ascii_digit() {",
     "if !c.is_ascii_hexdigit() {", "caught"),
    ("positive overflow saturates to INT_MAX", "            i64::MAX\n        }",
     "            i64::from(i32::MAX)\n        }", "caught"),
    ("negative overflow saturates to LONG_MAX", "            i64::MIN\n        } else {",
     "            i64::MAX\n        } else {", "caught"),
    ("clamp instead of truncate", "    Some(as_long as i32)",
     "    Some(as_long.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)", "caught"),
    ("overflow detection dropped (wrapping)",
     "            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {\n                Some(v) => acc = v,\n                None => overflow = true,\n            }",
     "            acc = acc.wrapping_mul(10).wrapping_add(digit);",
     "caught"),
    ("slurps stdin to EOF before parsing",
     "        let stdin = std::io::stdin();\n        let mut input = stdin.lock();\n        if let Some(v) = scanf_i32(&mut input) {",
     "        let mut slurped = Vec::new();\n        let _ = std::io::stdin().read_to_end(&mut slurped);\n        let mut input: &[u8] = &slurped;\n        if let Some(v) = scanf_i32(&mut input) {",
     "caught"),
    ("read error not treated as EOF",
     "            Err(_) => return None, // read error behaves like EOF for scanf",
     '            Err(_) => panic!("read error"),',
     "caught"),
    ("main returns 1", "fn main_impl() -> c_int {\n    let mut x: i32 = 0;",
     "fn main_impl() -> c_int {\n    if true {\n        let mut x: i32 = 0;\n        {\n            let stdin = std::io::stdin();\n            let mut input = stdin.lock();\n            if let Some(v) = scanf_i32(&mut input) {\n                x = v;\n            }\n        }\n        driver_impl(x);\n        return 1;\n    }\n    let mut x: i32 = 0;",
     "caught"),
    # deliberately equivalent mutants: they must survive, which proves the
    # suite is not failing for spurious reasons
    ("EQUIVALENT: negative overflow -> 0 (LONG_MIN truncates to 0 anyway)",
     "            i64::MIN\n        } else {", "            0\n        } else {", "equivalent"),
    ("EQUIVALENT: -acc instead of wrapping_neg (acc is always >= 0)",
     "acc.wrapping_neg()", "0i64.wrapping_sub(acc)", "equivalent"),
]


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def main():
    quick = "--quick" in sys.argv
    # A crash-safe backup inside the repo: if a previous run was killed, restore
    # the pristine source before doing anything else.
    backup = os.path.join(ROOT, "cbuild/imp.rs.mutation-backup")
    os.makedirs(os.path.dirname(backup), exist_ok=True)
    if os.path.exists(backup):
        print("restoring src/imp.rs from a previous interrupted run", flush=True)
        shutil.copyfile(backup, SRC)
    shutil.copyfile(SRC, backup)
    original = open(SRC).read()
    problems = []
    try:
        for name, needle, repl, expect in MUTATIONS:
            if needle not in original:
                problems.append(f"PATTERN NOT FOUND for {name!r}")
                print(f"!! pattern not found: {name}", flush=True)
                continue
            if original.count(needle) != 1:
                problems.append(f"AMBIGUOUS PATTERN for {name!r}")
                print(f"!! ambiguous pattern: {name}", flush=True)
                continue
            with open(SRC, "w") as f:
                f.write(original.replace(needle, repl, 1))
            b = run(["cargo", "build", "--offline", "--quiet"])
            if b.returncode != 0:
                print(f"!! did not compile: {name}\n{b.stderr[-800:]}", flush=True)
                problems.append(f"MUTATION DID NOT COMPILE: {name!r}")
                continue
            args = ["cargo", "test", "--offline"]
            if quick:
                args += ["--test", "differential"]
            try:
                t = run(args, timeout=180)
                failed = t.returncode != 0
            except subprocess.TimeoutExpired:
                # a hang is also a detected divergence, but report it distinctly
                print(f"ok   caught(HANG) (want {expect:10s}) {name}", flush=True)
                if expect != "caught":
                    problems.append(f"{name!r}: expected {expect}, got hang")
                continue
            verdict = "caught" if failed else "survived"
            ok = (verdict == "caught") if expect == "caught" else (verdict == "survived")
            names = sorted(
                set(
                    line.split()[1]
                    for line in (t.stdout + t.stderr).splitlines()
                    if line.startswith("test ") and "FAILED" in line
                )
            )
            detail = ("killed by: " + ", ".join(names[:6])) if names else ""
            print(f"{'ok  ' if ok else 'FAIL'} {verdict:9s} (want {expect:10s}) {name}  {detail}", flush=True)
            if not ok:
                problems.append(f"{name!r}: expected {expect}, got {verdict}")
                log = os.path.join(ROOT, "cbuild",
                                   "mutation-unexpected-%d.log" % len(problems))
                with open(log, "w") as fh:
                    fh.write(t.stdout + "\n===== stderr =====\n" + t.stderr)
                print(f"     details written to {log}", flush=True)
    finally:
        shutil.copyfile(backup, SRC)
        run(["cargo", "build", "--offline", "--quiet"])
        os.remove(backup)

    print()
    if problems:
        print("MUTATION CHECK FAILED:")
        for p in problems:
            print("  -", p)
        return 1
    print(f"MUTATION CHECK PASSED: {len(MUTATIONS)} mutants behaved as expected")
    return 0


if __name__ == "__main__":
    sys.exit(main())
