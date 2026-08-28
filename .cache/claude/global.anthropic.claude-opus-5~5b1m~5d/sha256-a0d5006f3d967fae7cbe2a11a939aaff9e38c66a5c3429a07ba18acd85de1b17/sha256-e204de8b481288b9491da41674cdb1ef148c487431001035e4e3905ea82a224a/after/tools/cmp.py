#!/usr/bin/env python3
"""Exploration helper: run the C and Rust binaries on the same stdin and diff."""
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
C = os.path.join(ROOT, "c_src", "build", "driver")
R = os.path.join(ROOT, "translation", "target", "release", "driver")

PTR = re.compile(rb"0x[0-9a-f]+")


def run(binary, data, files=None, timeout=5):
    d = tempfile.mkdtemp(prefix="run", dir=os.environ.get("TMPDIR", "/tmp"))
    for name, content in (files or {}).items():
        with open(os.path.join(d, name), "wb") as f:
            f.write(content)
    try:
        p = subprocess.run([binary], input=data, cwd=d,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           timeout=timeout)
        rc, out, err = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        rc, out, err = "TIMEOUT", e.stdout or b"", e.stderr or b""
    extra = {}
    for fn in sorted(os.listdir(d)):
        with open(os.path.join(d, fn), "rb") as f:
            extra[fn] = f.read()
    for name in (files or {}):
        extra.pop(name, None)
    return rc, out, err, extra


def norm(b):
    seen = {}

    def sub(m):
        seen.setdefault(m.group(0), b"<ptr%d>" % len(seen))
        return seen[m.group(0)]

    return PTR.sub(sub, b)


def main():
    data = sys.stdin.buffer.read()
    files = {}
    for a in sys.argv[1:]:
        name, _, content = a.partition("=")
        files[name] = content.encode().decode("unicode_escape").encode("latin-1")
    c = run(C, data, files)
    r = run(R, data, files)
    ok = True
    for i, label in enumerate(["status", "stdout", "stderr", "files"]):
        cv, rv = c[i], r[i]
        if label in ("stdout", "stderr"):
            cv, rv = norm(cv), norm(rv)
        if label == "files":
            cv = {k: v for k, v in cv.items()}
            rv = {k: v for k, v in rv.items()}
        if cv != rv:
            ok = False
            print("MISMATCH %s:\n  C   =%r\n  RUST=%r" % (label, cv, rv))
    print("OK" if ok else "FAIL")
    if "-v" in os.environ.get("V", ""):
        pass
    if os.environ.get("SHOW"):
        sys.stdout.write("--- C stdout ---\n")
        sys.stdout.flush()
        os.write(1, c[1])
        sys.stdout.write("--- C stderr ---\n")
        sys.stdout.flush()
        os.write(1, c[2])
        print("--- C status: %r  files=%r" % (c[0], c[3]))


if __name__ == "__main__":
    main()
