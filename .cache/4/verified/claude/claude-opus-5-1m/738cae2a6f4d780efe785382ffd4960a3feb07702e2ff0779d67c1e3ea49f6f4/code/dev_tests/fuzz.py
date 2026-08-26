#!/usr/bin/env python3
"""Randomised differential test, including inputs that make the C program spin
forever in `while (getchar() != '\\n');` (stdin at EOF)."""
import os
import random
import re
import shutil
import subprocess
import tempfile

C_BIN = os.environ["CBIN"]
RUST_BIN = os.environ["RBIN"]
PTR = re.compile(rb"0x[0-9a-f]+")


def norm(b):
    seen = {}

    def sub(m):
        k = m.group(0)
        if k not in seen:
            seen[k] = b"0xPTR%d" % len(seen)
        return seen[k]

    return PTR.sub(sub, b)


def run(binary, data):
    d = tempfile.mkdtemp()
    try:
        p = subprocess.Popen(
            [binary],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=d,
        )
        try:
            out, err = p.communicate(data, timeout=3)
            rc = p.returncode
        except subprocess.TimeoutExpired:
            p.kill()
            out, err = p.communicate()
            rc = "TIMEOUT"
        files = {}
        for name in sorted(os.listdir(d)):
            with open(os.path.join(d, name), "rb") as f:
                files[name] = f.read()
        return rc, out, err, files
    finally:
        shutil.rmtree(d, ignore_errors=True)


TOKENS = [
    b"", b"0", b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8", b"9", b"10",
    b"11", b"12", b"-1", b"99", b"abc", b"  3", b"1 2", b"+4", b"3x",
    b"name with spaces", b"f.txt", b"a" * 70, b"2147483647", b"-2147483648",
    b"\t8", b"0 1 2",
]

random.seed(1234)
fails = 0
N = 400
for trial in range(N):
    n = random.randint(1, 25)
    lines = [random.choice(TOKENS) for _ in range(n)]
    data = b"\n".join(lines)
    if random.random() < 0.7:
        data += b"\n"
    rc_c, out_c, err_c, files_c = run(C_BIN, data)
    rc_r, out_r, err_r, files_r = run(RUST_BIN, data)
    bad = []
    if norm(out_c) != norm(out_r):
        bad.append("stdout")
    if norm(err_c) != norm(err_r):
        bad.append("stderr")
    if rc_c != rc_r:
        bad.append("rc %r/%r" % (rc_c, rc_r))
    if files_c != files_r:
        bad.append("files")
    if bad:
        fails += 1
        print("FAIL trial %d: %s\n  input=%r" % (trial, ",".join(bad), data))
        if "stdout" in bad:
            a, b = norm(out_c).split(b"\n"), norm(out_r).split(b"\n")
            shown = 0
            for i in range(max(len(a), len(b))):
                x = a[i] if i < len(a) else b"<missing>"
                y = b[i] if i < len(b) else b"<missing>"
                if x != y and shown < 6:
                    shown += 1
                    print("   line %d:\n     C: %r\n     R: %r" % (i + 1, x, y))
            print("   lens: C=%d R=%d" % (len(out_c), len(out_r)))
        if "stderr" in bad:
            print("   C err=%r R err=%r" % (err_c[:200], err_r[:200]))
        if "files" in bad:
            print("   C files=%r R files=%r" % (files_c, files_r))
        if fails >= 5:
            break

print("%d failures / %d trials" % (fails, N))
