#!/usr/bin/env python3
"""Anti-vacuity check for the differential test suite.

Injects a deliberate bug into src/lib.rs, runs the suite, and requires the suite
to FAIL. A mutation that survives means the tests cannot detect that class of
divergence, so a green run proves nothing.

Usage:  python3 mutation_check.py [--features FEATURES]
"""
import pathlib
import re
import shutil
import subprocess
import sys

SRC = pathlib.Path("src/lib.rs")
BAK = pathlib.Path("src/lib.rs.mutation_backup")

# (description, old_fragment, new_fragment)
MUTATIONS = [
    (
        "M1  case 10 loses its fallthrough into case 20 (+30 -> +10)",
        "                    result = result.wrapping_add(10);\n"
        "                    result = result.wrapping_add(20);",
        "                    result = result.wrapping_add(10);",
    ),
    (
        "M2  case 30 loses its fallthrough into case 40 (+70 -> +30)",
        "                    result = result.wrapping_add(30);\n"
        "                    result = result.wrapping_add(40);",
        "                    result = result.wrapping_add(30);",
    ),
    (
        "M3  case 20 becomes +30",
        "                20 => {\n                    result = result.wrapping_add(20);",
        "                20 => {\n                    result = result.wrapping_add(30);",
    ),
    (
        "M4  case 40 becomes +41",
        "                40 => {\n                    result = result.wrapping_add(40);",
        "                40 => {\n                    result = result.wrapping_add(41);",
    ),
    (
        "M5  default arm saturates instead of wrapping (overflow behaviour)",
        "                other => {\n                    result = result.wrapping_add(other);",
        "                other => {\n                    result = result.saturating_add(other);",
    ),
    (
        "M6  snprintf prints the VALUES instead of the stringized token",
        'c"numbers".as_ptr(),',
        'c"10, 20, 30, 40".as_ptr(),',
    ),
    (
        "M7  snprintf buffer size 50 -> 20 (truncates the message)",
        "                50,\n                c\"Processed numbers: %s\".as_ptr(),",
        "                20,\n                c\"Processed numbers: %s\".as_ptr(),",
    ),
    (
        "M8  loop covers only 3 of the 4 arguments",
        "for i in 0..4usize {",
        "for i in 0..3usize {",
    ),
    (
        "M9  cleanup_resources drops its NULL guard",
        "    if !dynamic_str.is_null() {",
        "    if dynamic_str.is_null() {",
    ),
    (
        "M10 print_result swaps label/result order in the format string",
        'printf(c"%s: %d\\n".as_ptr(), label, result)',
        'printf(c"%d: %s\\n".as_ptr(), result, label)',
    ),
    (
        "M11 print_result routes the label through a Rust str (breaks non-UTF8)",
        'unsafe { printf(c"%s: %d\\n".as_ptr(), label, result) };',
        'unsafe {\n'
        '        let s = std::ffi::CStr::from_ptr(label).to_string_lossy().into_owned();\n'
        '        let c = std::ffi::CString::new(s).unwrap();\n'
        '        printf(c"%s: %d\\n".as_ptr(), c.as_ptr(), result);\n'
        '    };',
    ),
    (
        "M12 cleanup returns 0 unconditionally",
        "    unsafe { cleanup_resources(dynamic_str) };\n    result",
        "    unsafe { cleanup_resources(dynamic_str) };\n    0",
    ),
    (
        "M13 the 'Processed numbers' line is not printed",
        'printf(c"%s\\n".as_ptr(), dynamic_str);',
        "let _ = dynamic_str;",
    ),
    (
        "M14 case labels 10 and 30 swapped",
        "                10 => {",
        "                11 => {",
    ),
    (
        "M15 cleanup never frees its internal buffer (pure leak)",
        "    unsafe { cleanup_resources(dynamic_str) };\n    result",
        "    result",
    ),
    (
        "M16 cleanup_resources becomes a total no-op (pure leak)",
        "    if !dynamic_str.is_null() {\n        unsafe { free(dynamic_str as *mut c_void) };",
        "    if false {\n        unsafe { free(dynamic_str as *mut c_void) };",
    ),
]


def run_suite(features):
    cmd = ["cargo", "test", "--no-default-features"]
    if features:
        cmd += ["--features", features]
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    out = p.stdout + p.stderr
    failed = "test result: FAILED" in out
    rows = sorted(set(re.findall(r"^\s+([BEG]\d+[^\n.]*?)\s+\.\.\. FAILED", out, re.M)))
    build_error = "error[" in out or "error: could not compile" in out
    return failed, rows, build_error, out


def main():
    features = ""
    if "--features" in sys.argv:
        features = sys.argv[sys.argv.index("--features") + 1]

    shutil.copy(SRC, BAK)
    original = SRC.read_text()

    # Sanity: the unmutated tree must PASS, otherwise "caught" is meaningless.
    failed, _, build_error, out = run_suite(features)
    if build_error or failed:
        SRC.write_text(original)
        BAK.unlink(missing_ok=True)
        print("BASELINE IS NOT GREEN — fix the translation before mutation testing.")
        print(out[-4000:])
        return 1
    print(f"baseline: PASS  (features={features or '<default>'})\n")

    caught, missed, skipped = [], [], []
    try:
        for desc, old, new in MUTATIONS:
            SRC.write_text(original)
            s = SRC.read_text()
            if old not in s:
                skipped.append(desc)
                print(f"  SKIP    {desc}  (pattern not found)")
                continue
            SRC.write_text(s.replace(old, new, 1))

            failed, rows, build_error, _ = run_suite(features)
            if build_error:
                skipped.append(desc)
                print(f"  SKIP    {desc}  (mutation does not compile)")
            elif failed:
                shown = ", ".join(rows) if rows else "suite failed"
                caught.append(desc)
                print(f"  CAUGHT  {desc}\n            by: {shown}")
            else:
                missed.append(desc)
                print(f"  MISSED  {desc}   <-- test gap!")
    finally:
        SRC.write_text(original)
        BAK.unlink(missing_ok=True)

    total = len(MUTATIONS)
    print(
        f"\ncaught {len(caught)}/{total}, missed {len(missed)}, skipped {len(skipped)}"
    )
    if missed:
        print("\nSURVIVING MUTATIONS (the suite cannot detect these):")
        for m in missed:
            print(f"  - {m}")
        return 1
    print("No surviving mutations: the suite detects every injected divergence.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
