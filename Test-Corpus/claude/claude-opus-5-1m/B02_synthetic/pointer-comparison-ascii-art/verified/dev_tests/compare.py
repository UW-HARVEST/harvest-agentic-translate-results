#!/usr/bin/env python3
"""Differential tester: runs the C reference and the Rust translation on the
same stdin and compares stdout byte for byte (heap pointer values printed with
%p are normalised, they can never match by construction)."""
import os
import re
import shutil
import subprocess
import sys
import tempfile

C_BIN = os.environ["CBIN"]
RUST_BIN = os.environ["RBIN"]

PTR = re.compile(rb"0x[0-9a-f]+")


def run(binary, data, extra_files=None):
    d = tempfile.mkdtemp()
    try:
        if extra_files:
            for name, content in extra_files.items():
                with open(os.path.join(d, name), "wb") as f:
                    f.write(content)
        p = subprocess.run(
            [binary],
            input=data,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=d,
            timeout=10,
        )
        files = {}
        for name in sorted(os.listdir(d)):
            with open(os.path.join(d, name), "rb") as f:
                files[name] = f.read()
        return p.returncode, p.stdout, p.stderr, files
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", e.stdout or b"", e.stderr or b"", {}
    finally:
        shutil.rmtree(d, ignore_errors=True)


def norm(b):
    """Replace each distinct pointer with a stable id based on first use, so
    pointer *identity* relationships are still compared."""
    seen = {}

    def sub(m):
        key = m.group(0)
        if key not in seen:
            seen[key] = b"0xPTR%d" % len(seen)
        return seen[key]

    return PTR.sub(sub, b)


CASES = {
    "exit_only": (b"12\n", None),
    "empty": (b"", None),
    "view_shapes": (b"1\n12\n", None),
    "no_newline_end": (b"1", None),
    "invalid_choice": (b"0\n13\n99\n-5\n12\n", None),
    "invalid_input": (b"abc\n\n   \nx1\n12\n", None),
    "create_scenes": (b"2\nMy Scene\n2\nOther\n6\n12\n", None),
    "create_empty_name": (b"2\n\n6\n12\n", None),
    "add_shapes": (b"2\nA\n3\n0\n0\n3\n0\n9\n5\n0\n12\n", None),
    "add_invalid": (b"3\n2\nA\n3\n5\n0\n3\n0\n99\n3\n0\n-1\n3\n0\nzz\n12\n", None),
    "remove_shapes": (
        b"2\nA\n3\n0\n7\n3\n0\n2\n4\n0\n1\n4\n0\n5\n4\n0\n0\n5\n0\n12\n",
        None,
    ),
    "remove_empty": (b"2\nA\n4\n0\n12\n", None),
    "remove_no_scene": (b"4\n12\n", None),
    "view_no_scene": (b"5\n12\n", None),
    "view_bad_idx": (b"2\nA\n5\n7\n5\n-1\n5\n0\n12\n", None),
    "list_empty": (b"6\n12\n", None),
    "save_load": (
        b"2\nScene1\n3\n0\n0\n3\n0\n7\n7\n0\nout.txt\n8\nout.txt\n5\n1\n6\n12\n",
        None,
    ),
    "save_no_scene": (b"7\n12\n", None),
    "save_bad_file": (b"2\nA\n7\n0\n/nonexistent_dir/x/y.txt\n12\n", None),
    "save_empty_name": (b"2\nA\n7\n0\n\n12\n", None),
    "load_missing": (b"8\nnope.txt\n12\n", None),
    "load_ok": (
        b"8\nscene.dat\n5\n0\n6\n12\n",
        {"scene.dat": b"Loaded Scene\n3\n0\n5\n9\n"},
    ),
    "load_bad_count": (b"8\nscene.dat\n6\n12\n", {"scene.dat": b"Name\nxyz\n"}),
    "load_empty_file": (b"8\nscene.dat\n6\n12\n", {"scene.dat": b""}),
    "load_only_name": (b"8\nscene.dat\n6\n12\n", {"scene.dat": b"Only Name\n"}),
    "load_bad_types": (
        b"8\nscene.dat\n5\n0\n12\n",
        {"scene.dat": b"S\n4\n0\n99\n-3\n7\n"},
    ),
    "load_short": (b"8\nscene.dat\n5\n0\n12\n", {"scene.dat": b"S\n5\n1\n2\n"}),
    "load_no_trailing_nl": (
        b"8\nscene.dat\n5\n0\n12\n",
        {"scene.dat": b"S\n2\n1\n2"},
    ),
    "load_crlf": (b"8\nscene.dat\n5\n0\n12\n", {"scene.dat": b"S\r\n1\r\n3\r\n"}),
    "load_51": (
        b"8\nscene.dat\n5\n0\n12\n",
        {"scene.dat": b"Big\n55\n" + b"0\n" * 55},
    ),
    "load_long_name": (
        b"8\nscene.dat\n5\n0\n12\n",
        {"scene.dat": b"N" * 100 + b"\n2\n1\n2\n"},
    ),
    "load_spaces": (
        b"8\nscene.dat\n5\n0\n12\n",
        {"scene.dat": b"S\n  2  \n   1    2   \n"},
    ),
    "compare_shapes": (b"9\n0\n0\n9\n1\n2\n9\n99\n1\n9\n0\n-1\n12\n", None),
    "compare_shapes_bad": (b"9\nabc\n9\n0\nxyz\n12\n", None),
    "compare_scenes_few": (b"10\n2\nA\n10\n12\n", None),
    "compare_scenes": (
        b"2\nA\n2\nB\n3\n0\n1\n3\n1\n1\n10\n0\n1\n3\n0\n2\n10\n0\n1\n12\n",
        None,
    ),
    "compare_scenes_bad": (b"2\nA\n2\nB\n10\n5\n0\n10\n0\n-1\n12\n", None),
    "delete_scene": (b"2\nA\n2\nB\n2\nC\n11\n1\n6\n11\n9\n11\n-1\n11\n0\n6\n12\n", None),
    "delete_none": (b"11\n12\n", None),
    "max_scenes": (b"2\nA\n" * 12 + b"6\n12\n", None),
    "max_scenes_load": (
        b"2\nA\n" * 10 + b"8\nscene.dat\n12\n",
        {"scene.dat": b"S\n1\n0\n"},
    ),
    "long_menu_line": (b"1" + b"x" * 300 + b"\n12\n", None),
    "long_name": (b"2\n" + b"N" * 200 + b"\n6\n12\n", None),
    "huge_numbers": (
        b"2147483648\n99999999999999999999\n-99999999999999999999\n4294967308\n12\n",
        None,
    ),
    "trailing_junk": (b"12abc\n", None),
    "leading_space": (b"   6\n  +12\n", None),
    "scanf_across_lines": (b"3\n2\nA\n3\n\n\n0\n\n\n1\n6\n12\n", None),
    "scanf_multi_on_line": (b"2\nA\n3\n0 5\n5\n0\n12\n", None),
    "shape_50_full": (
        b"2\nA\n" + b"3\n0\n0\n" * 52 + b"5\n0\n12\n",
        None,
    ),
    "tab_input": (b"\t6\n\t12\n", None),
    "mixed": (
        b"1\n2\nFarm\n3\n0\n1\n3\n0\n0\n3\n0\n2\n5\n0\n6\n7\n0\nfarm.sav\n"
        b"8\nfarm.sav\n10\n0\n1\n4\n0\n1\n5\n0\n11\n0\n6\n12\n",
        None,
    ),
}

fails = 0
for name, (data, extra) in sorted(CASES.items()):
    rc_c, out_c, err_c, files_c = run(C_BIN, data, extra)
    rc_r, out_r, err_r, files_r = run(RUST_BIN, data, extra)
    problems = []
    if norm(out_c) != norm(out_r):
        problems.append("stdout")
    if norm(err_c) != norm(err_r):
        problems.append("stderr")
    if rc_c != rc_r:
        problems.append("rc(%r vs %r)" % (rc_c, rc_r))
    if files_c != files_r:
        problems.append("files")
    if problems:
        fails += 1
        print("FAIL %-22s %s" % (name, ",".join(problems)))
        if "stdout" in problems:
            a = norm(out_c).split(b"\n")
            b = norm(out_r).split(b"\n")
            for i in range(max(len(a), len(b))):
                x = a[i] if i < len(a) else b"<missing>"
                y = b[i] if i < len(b) else b"<missing>"
                if x != y:
                    print("   line %d:\n     C: %r\n     R: %r" % (i + 1, x, y))
            print("   C len=%d R len=%d" % (len(out_c), len(out_r)))
        if "stderr" in problems:
            print("   C err: %r\n   R err: %r" % (err_c[:400], err_r[:400]))
        if "files" in problems:
            print("   C files: %r\n   R files: %r" % (files_c, files_r))
    else:
        print("ok   %s" % name)

print("\n%d failure(s) out of %d" % (fails, len(CASES)))
sys.exit(1 if fails else 0)
