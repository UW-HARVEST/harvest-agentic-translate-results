#!/usr/bin/env python3
"""Negative controls for the differential test suite.

Each entry below deliberately breaks the Rust translation in one specific way
and records which tests catch it. A test suite that passes every mutation is
worthless, so this script is what proves the suite has teeth.

src/main.rs is restored (and verified byte-identical) after every mutation.
"""
import hashlib
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent
MAIN = ROOT / "src" / "main.rs"
ORIG = MAIN.read_text()
ORIG_SHA = hashlib.sha256(ORIG.encode()).hexdigest()

MUTATIONS = [
    (
        "M1 rem_euclid instead of C truncating %",
        "if val % 10 == 9 {",
        "if val.rem_euclid(10) == 9 {",
        ["cfg_05", "cfg_06", "err_25"],
    ),
    (
        "M2 saturating narrowing instead of truncation",
        "let mut val = parsed as i32;",
        "let mut val = parsed.clamp(i32::MIN as i64, i32::MAX as i64) as i32;",
        ["cfg_15", "err_21", "err_22"],
    ),
    (
        "M3 do not restore SIGPIPE disposition",
        "    reset_sigpipe();\n\n    let argv",
        "    // mutated: no reset_sigpipe()\n\n    let argv",
        ["cfg_25", "err_28"],
    ),
    (
        "M4 stop instead of wrapping on signed overflow",
        "        val = val.wrapping_add(1);",
        "        match val.checked_add(1) { Some(v) => val = v, None => break }",
        ["cfg_13", "err_26"],
    ),
    (
        "M5 int error message to stderr instead of stdout",
        'let _ = out.write_all(b"Error: first argument must be an integer!\\n");',
        'eprint!("Error: first argument must be an integer!\\n");',
        ["err_07", "err_13"],
    ),
    (
        "M6 reject trailing garbage that C accepts",
        "    if i == digits_start {",
        "    if i < s.len() { return (0, 0); }\n    if i == digits_start {",
        ["err_20", "cfg_11"],
    ),
    (
        "M7 wrap instead of clamping on strtol overflow",
        "        if negative {\n            i64::MIN\n        } else {\n            i64::MAX\n        }",
        "        acc as i64",
        ["err_21", "cfg_17"],
    ),
    (
        "M8 argc error message text altered by one word",
        "Error: should only be a single (integer) argument!",
        "Error: should only be a single integer argument!",
        ["err_02", "err_03"],
    ),
    (
        "M9 print without the trailing newline shape (space separator)",
        "    buf[len] = b'\\n';",
        "    buf[len] = b' ';",
        ["cfg_01", "cfg_04"],
    ),
    (
        "M10 treat '+' sign as invalid (extra validation C never had)",
        "    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {",
        "    if i < s.len() && s[i] == b'-' {",
        ["cfg_08", "err_09"],
    ),
]


def run(cmd, timeout=600):
    return subprocess.run(
        cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout
    )


def restore():
    MAIN.write_text(ORIG)
    assert hashlib.sha256(MAIN.read_text().encode()).hexdigest() == ORIG_SHA


def main():
    results = []
    try:
        for name, old, new, filters in MUTATIONS:
            src = ORIG
            assert old in src, f"{name}: pattern not found"
            MAIN.write_text(src.replace(old, new, 1))
            build = run(["cargo", "build", "--offline", "--tests"])
            if build.returncode != 0:
                results.append((name, "BUILD-FAILED", build.stderr[-400:]))
                restore()
                continue
            caught = []
            for f in filters:
                r = run(
                    [
                        "cargo",
                        "test",
                        "--offline",
                        "--test",
                        "phase_b_valid",
                        "--test",
                        "phase_c_errors",
                        f,
                        "--",
                        "--test-threads=4",
                    ]
                )
                ran = "0 filtered out" in r.stdout or "running" in r.stdout
                caught.append((f, r.returncode != 0, ran))
            restore()
            results.append((name, caught, None))
    finally:
        restore()

    print("\n=== mutation results (a mutation MUST be caught) ===")
    ok = True
    for name, caught, err in results:
        if err is not None or caught == "BUILD-FAILED":
            print(f"[SKIP/BUILD-FAIL] {name}: {err}")
            ok = False
            continue
        detail = " ".join(
            f"{f}:{'CAUGHT' if failed else 'MISSED'}" for f, failed, _ in caught
        )
        any_caught = any(failed for _, failed, _ in caught)
        print(f"[{'OK ' if any_caught else 'BAD'}] {name}: {detail}")
        ok = ok and any_caught
    print("\nALL MUTATIONS CAUGHT" if ok else "\nSOME MUTATIONS ESCAPED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
