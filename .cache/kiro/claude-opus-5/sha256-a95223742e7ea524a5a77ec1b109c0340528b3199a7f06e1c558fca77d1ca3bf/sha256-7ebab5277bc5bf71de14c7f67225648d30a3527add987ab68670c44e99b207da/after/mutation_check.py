#!/usr/bin/env python3
"""Sanity-check that the differential suite actually discriminates.

Injects small behavioural mutations into translation/src/lib.rs, runs
`cargo test`, and reports whether each mutation is detected. Always restores
the pristine source.
"""

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent
SRC = ROOT / "translation" / "src" / "lib.rs"

# (label, literal_needle, literal_replacement)
MUTANTS = [
    ("decode() fallthrough 63 -> 62", "\n    63\n}", "\n    62\n}"),
    ("is_base64 rejects '+'", "|| (c == b'+')", "|| (c == 0u8)"),
    ("decode lowercase offset 26 -> 25", "return c - b'a' + 26;", "return c - b'a' + 25;"),
    ("decode digit offset 52 -> 51", "return c - b'0' + 52;", "return c - b'0' + 51;"),
    ("byte1 assembly b2>>4 -> b2>>3", "(b1 << 2) | (b2 >> 4)", "(b1 << 2) | (b2 >> 3)"),
    ("byte2 mask 0xf -> 0x7", "((b2 & 0xf) << 4) | (b3 >> 2)", "((b2 & 0x7) << 4) | (b3 >> 2)"),
    ("byte3 shift 6 -> 5", "((b3 & 0x3) << 6) | b4", "((b3 & 0x3) << 5) | b4"),
    ("c3 padding check inverted", "if c3 != b'=' {", "if c3 != b'+' {"),
    ("c4 padding check inverted", "if c4 != b'=' {", "if c4 != b'+' {"),
    ("dest allocation 13 -> 12", "l0 + 13", "l0 + 12"),
    ("group bound k+2 < l -> <= l", "if k + 2 < l {", "if k + 2 <= l {"),
    ("group bound k+3 < l -> <= l", "if k + 3 < l {", "if k + 3 <= l {"),
    ("stride 4 -> 3", "k += 4;", "k += 3;"),
    ("default c2 'A' -> 'B'", "let mut c2: u8 = b'A';", "let mut c2: u8 = b'B';"),
    ("default c3 'A' -> '='", "let mut c3: u8 = b'A';", "let mut c3: u8 = b'=';"),
    ("empty-string early return removed", "|| unsafe { *src } == 0", "|| false"),
]

pristine = SRC.read_text()


def restore():
    SRC.write_text(pristine)


caught = skipped = survived = 0
try:
    for label, needle, repl in MUTANTS:
        restore()
        if needle not in pristine:
            print(f"SKIP (pattern absent): {label}")
            skipped += 1
            continue
        SRC.write_text(pristine.replace(needle, repl, 1))

        proc = subprocess.run(
            ["cargo", "test"],
            cwd=ROOT / "translation",
            capture_output=True,
            text=True,
            timeout=600,
        )
        out = proc.stdout + proc.stderr
        if "error[" in out or "could not compile" in out:
            print(f"SKIP (does not compile): {label}")
            skipped += 1
        elif "test result: FAILED" in out or proc.returncode != 0:
            n = out.count("... FAILED")
            how = f"{n} tests failed" if n else f"harness aborted (rc={proc.returncode})"
            print(f"CAUGHT ({how}): {label}")
            caught += 1
        else:
            print(f"*** SURVIVED ***: {label}")
            survived += 1
finally:
    restore()

print(f"\ncaught={caught} survived={survived} skipped={skipped}")
sys.exit(1 if survived else 0)
