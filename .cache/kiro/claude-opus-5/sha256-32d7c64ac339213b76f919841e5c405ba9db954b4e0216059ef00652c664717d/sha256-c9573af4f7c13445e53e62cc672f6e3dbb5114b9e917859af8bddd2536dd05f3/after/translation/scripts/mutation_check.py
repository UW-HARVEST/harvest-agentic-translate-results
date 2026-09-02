#!/usr/bin/env python3
"""Mutation-test the differential suite itself.

Injects a known bug into the Rust translation, rebuilds the .so, and asserts
that `cargo test` FAILS. A surviving mutant means the test suite has a blind
spot, which is the thing this exercise is supposed to rule out.

Never touches c_src/.
"""
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "src", "lib.rs")

# (name, needle, replacement) — all literal, so nothing can silently not apply.
MUTATIONS = [
    (
        "archs_order",
        '    b"x86_64\\0",\n    b"i386\\0",',
        '    b"i386\\0",\n    b"x86_64\\0",',
    ),
    (
        "archs_wrong_literal",
        'b"i86pc\\0",',
        'b"i86pd\\0",',
    ),
    (
        "regexec_polarity",
        "(result == 0) as c_int",
        "(result != 0) as c_int",
    ),
    (
        "platform_case",
        'strdup(b"windows\\0".as_ptr() as *const c_char)',
        'strdup(b"Windows\\0".as_ptr() as *const c_char)',
    ),
    (
        "ver_marker_offset",
        "str_tmp = str_tmp.add(7);",
        "str_tmp = str_tmp.add(6);",
    ),
    (
        "unix_marker_offset",
        "str_tmp = str_tmp.add(2);\n            (*osd).os_name = strdup(str_tmp);",
        "str_tmp = str_tmp.add(1);\n            (*osd).os_name = strdup(str_tmp);",
    ),
    (
        "strip_guard",
        "*p.offset(len as isize - 1) = 0;",
        "if len > 0 { *p.offset(len as isize - 1) = 0; }",
    ),
    (
        "capture_size_off_by_one",
        "let size = (match_size + 1) as usize;",
        "let size = match_size as usize;",
    ),
    (
        "nmatch_hardcoded_1",
        "if w_regexec(pattern.as_ptr() as *const c_char, s, 2, m) != 0 {",
        "if w_regexec(pattern.as_ptr() as *const c_char, s, 1, m) != 0 {",
    ),
    (
        "null_check_removed",
        "if !(!pattern.is_null() && !string.is_null()) {",
        "if false {",
    ),
    (
        "null_check_or",
        "if !(!pattern.is_null() && !string.is_null()) {",
        "if !(!pattern.is_null() || !string.is_null()) {",
    ),
    (
        "arch_no_strdup",
        "os_arch = strdup(needle);",
        "os_arch = needle as *mut c_char;",
    ),
    (
        "skip_build_regex",
        "        {\n            (*osd).os_build = p;\n        }",
        "        {\n            let _ = p;\n        }",
    ),
    (
        "skip_arch",
        "(*osd).os_arch = strdup(str_tmp);",
        "let _: *mut c_char = str_tmp;",
    ),
    (
        "no_regfree",
        "regfree(regex.as_mut_ptr());",
        "",
    ),
    (
        "major_regex_unanchored",
        'b"^([0-9]+)\\\\.*\\0"',
        'b"([0-9]+)\\\\.*\\0"',
    ),
    (
        "minor_regex_changed",
        'b"^[0-9]+\\\\.([0-9]+)\\\\.*\\0"',
        'b"^[0-9]*\\\\.([0-9]+)\\\\.*\\0"',
    ),
    (
        "build_regex_no_multidot",
        'b"^[0-9]+\\\\.[0-9]+\\\\.([0-9]+(\\\\.[0-9]+)*)\\\\.*\\0"',
        'b"^[0-9]+\\\\.[0-9]+\\\\.([0-9]+)\\\\.*\\0"',
    ),
    (
        "colon_marker",
        'strstr((*osd).os_name, b": \\0".as_ptr() as *const c_char)',
        'strstr((*osd).os_name, b":\\0".as_ptr() as *const c_char)',
    ),
    (
        "codename_marker",
        'strstr((*osd).os_version, b" (\\0".as_ptr() as *const c_char)',
        'strstr((*osd).os_version, b"(\\0".as_ptr() as *const c_char)',
    ),
    (
        "version_strdup_order",
        "(*osd).os_version = strdup(str_tmp);\n        (*osd).os_platform =",
        "(*osd).os_version = strdup(uname);\n        (*osd).os_platform =",
    ),
    (
        "reg_extended_dropped",
        "const REG_EXTENDED: c_int = 1;",
        "const REG_EXTENDED: c_int = 0;",
    ),
    (
        "osd_null_check_removed",
        "if osd.is_null() {\n        return;\n    }",
        "if false {\n        return;\n    }",
    ),
    (
        "pipe_platform_offset",
        "str_tmp = str_tmp.add(1);\n                (*osd).os_platform = strdup(str_tmp);",
        "str_tmp = str_tmp.add(0);\n                (*osd).os_platform = strdup(str_tmp);",
    ),
]


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def main():
    original = open(SRC).read()
    backup = tempfile.NamedTemporaryFile("w", delete=False, suffix=".rs")
    backup.write(original)
    backup.close()

    killed, survivors, skipped = 0, [], []
    try:
        for name, needle, repl in MUTATIONS:
            if needle not in original:
                print("BAD   %-26s needle not found in src/lib.rs" % name)
                survivors.append((name, "needle not found"))
                continue
            mutated = original.replace(needle, repl, 1)
            assert mutated != original
            open(SRC, "w").write(mutated)

            b = run(["cargo", "build", "--quiet"])
            if b.returncode != 0:
                print("SKIP  %-26s (does not compile)" % name)
                skipped.append((name, "does not compile"))
                continue

            t = run(["cargo", "test", "--quiet"], timeout=600)
            if t.returncode == 0:
                print("ALIVE %-26s <-- BLIND SPOT" % name)
                survivors.append((name, "suite still passed"))
            else:
                print("KILL  %-26s" % name)
                killed += 1
    finally:
        shutil.copyfile(backup.name, SRC)
        os.unlink(backup.name)
        run(["cargo", "build", "--quiet"])
        run(["cargo", "build", "--release", "--quiet"])

    total = len(MUTATIONS)
    print("\nkilled %d / %d  (skipped-uncompilable: %d)" % (killed, total, len(skipped)))
    if survivors:
        print("survivors:")
        for n, why in survivors:
            print("  %s: %s" % (n, why))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
