#!/usr/bin/env python3
"""Harness self-validation (not part of the deliverable behaviour).

Injects a deliberate bug into translation/src/lib.rs, rebuilds the cdylib, and
checks that the differential suite FAILS. A surviving mutant means the tests do
not actually observe that behaviour, i.e. a blind spot.

src/lib.rs is restored unconditionally.
"""
import os, shutil, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "src/lib.rs")

# (name, [(old_literal, new_literal), ...])  -- every pair must apply
MUTANTS = [
    ("arch_order_aarch64_arm64", [
        ('        c"aarch64",\n        c"arm64",', '        c"arm64",\n        c"aarch64",')]),
    ("arch_drop_i86pc", [
        ('const ARCHS: [&core::ffi::CStr; 12]', 'const ARCHS: [&core::ffi::CStr; 11]'),
        ('        c"i86pc",\n', '')]),
    ("arch_drop_arm64", [
        ('const ARCHS: [&core::ffi::CStr; 12]', 'const ARCHS: [&core::ffi::CStr; 11]'),
        ('        c"arm64",\n', '')]),
    ("arch_drop_x86_64", [
        ('const ARCHS: [&core::ffi::CStr; 12]', 'const ARCHS: [&core::ffi::CStr; 11]'),
        ('        c"x86_64",\n', '')]),
    ("arch_no_break", [
        ('            os_arch = strdup(arch.as_ptr());\n            break;',
         '            os_arch = strdup(arch.as_ptr());')]),
    ("ver_offset_7_to_6", [
        ('str_tmp = (str_tmp as usize).wrapping_add(7) as *mut c_char;',
         'str_tmp = (str_tmp as usize).wrapping_add(6) as *mut c_char;')]),
    ("regexec_return_polarity", [('(result == 0) as c_int', '(result != 0) as c_int')]),
    ("regexec_null_guard_pattern_only", [
        ('if !(!pattern.is_null() && !string.is_null()) {', 'if !(!pattern.is_null()) {')]),
    ("regexec_null_guard_string_only", [
        ('if !(!pattern.is_null() && !string.is_null()) {', 'if !(!string.is_null()) {')]),
    ("regexec_no_stderr", [
        ('        fprintf(\n            stderr,', '        let _ = (\n            stderr,')]),
    ("trim_skip_when_empty", [
        ('    let len = strlen(p);\n    let addr =',
         '    let len = strlen(p);\n    if len == 0 { return; }\n    let addr =')]),
    ("ver_marker_lowercase", [('c" [Ver: ".as_ptr()', 'c" [ver: ".as_ptr()')]),
    ("dup_match_size_plus_two", [
        ('let dst = malloc((match_size as isize + 1) as usize) as *mut c_char;',
         'let dst = malloc((match_size as isize + 2) as usize) as *mut c_char;')]),
    # EQUIVALENT (must survive): `%.*s` with precision `match_size` writes at
    # most match_size bytes + 1 NUL, so any size limit >= match_size + 1 gives
    # byte-identical output. Proven by tests/equivalence_proofs.rs.
    ("EQUIV:dup_match_snprintf_size_plus_two", [
        ('        (match_size as isize + 1) as usize,\n        c"%.*s".as_ptr(),',
         '        (match_size as isize + 2) as usize,\n        c"%.*s".as_ptr(),')]),
    ("dup_match_precision_off_by_one", [
        ('        c"%.*s".as_ptr(),\n        match_size,',
         '        c"%.*s".as_ptr(),\n        match_size - 1,')]),
    ("dup_match_uses_rm_eo", [
        ('        (base as usize).wrapping_add(m.rm_so as isize as usize) as *const c_char,',
         '        (base as usize).wrapping_add(m.rm_eo as isize as usize) as *const c_char,')]),
    ("platform_string_case", [('strdup(c"windows".as_ptr())', 'strdup(c"Windows".as_ptr())')]),
    ("bracket_offset_2_to_1", [
        ('str_tmp = (str_tmp as usize).wrapping_add(2) as *mut c_char;\n            (*osd).os_name = strdup(str_tmp);',
         'str_tmp = (str_tmp as usize).wrapping_add(1) as *mut c_char;\n            (*osd).os_name = strdup(str_tmp);')]),
    ("colon_offset_2_to_3", [
        ('str_tmp = (str_tmp as usize).wrapping_add(2) as *mut c_char;\n                (*osd).os_version = strdup(str_tmp);',
         'str_tmp = (str_tmp as usize).wrapping_add(3) as *mut c_char;\n                (*osd).os_version = strdup(str_tmp);')]),
    ("codename_offset_2_to_1", [
        ('str_tmp = (str_tmp as usize).wrapping_add(2) as *mut c_char;\n                    (*osd).os_codename = strdup(str_tmp);',
         'str_tmp = (str_tmp as usize).wrapping_add(1) as *mut c_char;\n                    (*osd).os_codename = strdup(str_tmp);')]),
    ("pipe_offset_1_to_2", [
        ('str_tmp = (str_tmp as usize).wrapping_add(1) as *mut c_char;\n                (*osd).os_platform = strdup(str_tmp);',
         'str_tmp = (str_tmp as usize).wrapping_add(2) as *mut c_char;\n                (*osd).os_platform = strdup(str_tmp);')]),
    ("pipe_needle_changed", [('c"|".as_ptr()', 'c"!".as_ptr()')]),
    ("osd_null_guard_removed", [
        ('    if osd.is_null() {\n        return;\n    }', '    if false {\n        return;\n    }')]),
    ("os_name_from_uname_not_str_tmp", [
        ('            (*osd).os_name = strdup(str_tmp);', '            (*osd).os_name = strdup(uname);')]),
    ("minor_regex_swapped", [
        (r'c"^[0-9]+\\.([0-9]+)\\.*".as_ptr()', r'c"^([0-9]+)\\.[0-9]+\\.*".as_ptr()')]),
    ("build_regex_no_multidot", [
        (r'c"^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*".as_ptr()',
         r'c"^[0-9]+\\.[0-9]+\\.([0-9]+)\\.*".as_ptr()')]),
    ("major_regex_unanchored", [
        (r'c"^([0-9]+)\\.*".as_ptr()', r'c"([0-9]+)\\.*".as_ptr()')]),
    ("codename_marker", [('c" (".as_ptr()', 'c"(".as_ptr()')]),
    ("colon_marker", [('c": ".as_ptr()', 'c":".as_ptr()')]),
    ("bracket_marker", [('c" [".as_ptr()', 'c"[".as_ptr()')]),
    ("reg_extended_dropped", [('const REG_EXTENDED: c_int = 1;', 'const REG_EXTENDED: c_int = 0;')]),
    ("os_version_from_uname", [
        ('        (*osd).os_version = strdup(str_tmp);\n        (*osd).os_platform',
         '        (*osd).os_version = strdup(uname);\n        (*osd).os_platform')]),
    ("arch_branch_moved_outside_else", [
        ('        str_tmp = get_os_arch(uname);\n        if !str_tmp.is_null() {',
         '        str_tmp = core::ptr::null_mut();\n        if !str_tmp.is_null() {')]),
    ("nmatch_forced_to_2", [
        ('    let result = regexec(regex.as_ptr(), string, nmatch, pmatch, 0);',
         '    let result = regexec(regex.as_ptr(), string, 2, pmatch, 0);')]),
    ("eflags_notbol", [
        ('    let result = regexec(regex.as_ptr(), string, nmatch, pmatch, 0);',
         '    let result = regexec(regex.as_ptr(), string, nmatch, pmatch, 1);')]),
    # EQUIVALENT (must survive): the scratch array is only read after w_regexec
    # returned non-zero, and glibc fills every nmatch slot on a match, so the
    # initial value is dead. Proven by tests/equivalence_proofs.rs.
    ("EQUIV:matches_array_not_zeroed", [
        ('let mut matches: [regmatch_t; 2] = [regmatch_t { rm_so: 0, rm_eo: 0 }; 2];',
         'let mut matches: [regmatch_t; 2] = [regmatch_t { rm_so: 7, rm_eo: 9 }; 2];')]),
    ("trim_os_name_skipped", [
        ('            } else {\n                trim_last_char((*osd).os_name);\n            }',
         '            } else {\n            }')]),
]


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, stdout=subprocess.DEVNULL,
                          stderr=subprocess.DEVNULL, **kw).returncode


def main():
    only = sys.argv[1] if len(sys.argv) > 1 else None
    with open(SRC) as fh:
        original = fh.read()
    backup = tempfile.mktemp(suffix=".rs")
    with open(backup, "w") as fh:
        fh.write(original)

    caught, survived, skipped = [], [], []
    try:
        for name, pairs in MUTANTS:
            if only and only != name:
                continue
            missing = [o for o, _ in pairs if original.count(o) < 1]
            if missing:
                skipped.append((name, "pattern absent"))
                print(f"SKIP     {name}  (pattern absent)", flush=True)
                continue
            mutated = original
            for o, n in pairs:
                mutated = mutated.replace(o, n, 1)
            assert mutated != original, name
            with open(SRC, "w") as fh:
                fh.write(mutated)
            if run(["cargo", "build", "--offline"]) != 0:
                skipped.append((name, "does not compile"))
                print(f"SKIP     {name}  (mutant does not compile)", flush=True)
                continue
            rc = run(["timeout", "600", "cargo", "test", "--offline", "-q"])
            equivalent = name.startswith("EQUIV:")
            if rc == 0 and equivalent:
                caught.append(name)
                print(f"SURVIVED {name}  (expected: provably equivalent)", flush=True)
            elif rc == 0:
                survived.append(name)
                print(f"SURVIVED {name}  <-- BLIND SPOT", flush=True)
            elif equivalent:
                survived.append(name)
                print(f"CAUGHT   {name}  <-- equivalence claim is WRONG", flush=True)
            else:
                caught.append(name)
                print(f"CAUGHT   {name}", flush=True)
    finally:
        with open(SRC, "w") as fh:
            fh.write(original)
        run(["cargo", "build", "--offline"])

    print(f"\ncaught={len(caught)} survived={len(survived)} skipped={len(skipped)}")
    for n, why in skipped:
        print(f"  skipped: {n} ({why})")
    if survived:
        print("FAIL: surviving mutants -> " + ", ".join(survived))
        return 1
    if not caught:
        print("FAIL: no mutant was applied")
        return 1
    print("OK: every applicable mutant was detected by the differential suite.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
