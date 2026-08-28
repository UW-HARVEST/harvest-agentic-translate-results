#!/usr/bin/env python3
"""Differential tester: runs the C reference and the Rust translation on the
same stdin + argv and compares stdout, stderr and exit status byte for byte.

Usage: difftest.py <path-to-c-binary> <path-to-rust-binary>
"""
import itertools
import random
import subprocess
import sys

C_BIN, RUST_BIN = sys.argv[1], sys.argv[2]

ARGSETS = [
    ["-", "-", "-", "-"],
    ["LUG00001", "-", "-", "-"],
    ["-", "FL1234", "-", "-"],
    ["-", "-", "JFK", "-"],
    ["-", "-", "-", "LAX"],
    ["LUG00001", "FL1234", "JFK", "LAX"],
    ["NOPE", "-", "-", "-"],
    ["", "", "", ""],
    ["-x", "-", "-", "-"],
    ["-", "-", "jfk", "-"],
]

BAD_ARGSETS = [[], ["-"], ["-", "-"], ["-", "-", "-"], ["-", "-", "-", "-", "-"]]

CASES = {
    "empty": b"",
    "only_newline": b"\n",
    "whitespace_only": b"   \t\n\n  \n",
    "one_line": b"0000000001 LUG00001 FL1234 JFK LAX first comment\n",
    "one_line_no_nl": b"0000000001 LUG00001 FL1234 JFK LAX first comment",
    "no_comment_nl": b"0000000001 LUG00001 FL1234 JFK LAX\n",
    # last record has no comment and no trailing newline -> dropped by C
    "no_comment_no_nl": b"0000000001 LUG00001 FL1234 JFK LAX",
    "trailing_space_no_nl": b"0000000001 LUG00001 FL1234 JFK LAX ",
    "multi": (
        b"0000000003 LUG00001 FL0003 JFK LAX third\n"
        b"0000000001 LUG00001 FL0001 JFK BOS first\n"
        b"0000000002 LUG00002 FL0002 BOS SFO second\n"
    ),
    "equal_timestamps": (
        b"0000000005 LUGAAAAA FL0001 JFK LAX a\n"
        b"0000000005 LUGBBBBB FL0002 JFK LAX b\n"
        b"0000000005 LUGCCCCC FL0003 JFK LAX c\n"
    ),
    # supersession: later directive, same luggage, same departure
    "superseded_same_dep": (
        b"0000000001 LUG00001 FL0001 JFK LAX v1\n"
        b"0000000002 LUG00001 FL0002 JFK BOS v2\n"
    ),
    # later directive, same luggage, different departure -> not superseded
    "superseded_diff_dep": (
        b"0000000001 LUG00001 FL0001 JFK LAX v1\n"
        b"0000000002 LUG00001 FL0002 BOS SFO v2\n"
    ),
    # the "stop at first same-luggage directive" quirk
    "supersede_quirk": (
        b"0000000001 LUG00001 FL0001 JFK LAX v1\n"
        b"0000000002 LUG00001 FL0002 BOS SFO v2\n"
        b"0000000003 LUG00001 FL0003 JFK ORD v3\n"
    ),
    "zero_ts": b"0 LUG00001 FL1234 JFK LAX zero\n",
    "big_ts": b"4294967295 LUG00001 FL1234 JFK LAX big\n",
    "huge_ts": b"99999999999999999999 LUG00001 FL1234 JFK LAX huge\n",
    "neg_ts": b"-5 LUG00001 FL1234 JFK LAX neg\n",
    "plus_ts": b"+7 LUG00001 FL1234 JFK LAX plus\n",
    "wrap_ts": b"12345678901 LUG00001 FL1234 JFK LAX wrap\n",
    "leading_zeros": b"0000000042 LUG00001 FL1234 JFK LAX lz\n",
    "spread_over_lines": (
        b"0000000001\nLUG00001\nFL1234\nJFK\nLAX\n"
        b"0000000002 LUG00002\n FL5678  BOS   SFO trailing\n"
    ),
    "extra_blank_lines": (
        b"\n\n0000000001 LUG00001 FL1234 JFK LAX one\n\n\n"
        b"0000000002 LUG00002 FL5678 BOS SFO two\n\n"
    ),
    "tabs": b"0000000001\tLUG00001\tFL1234\tJFK\tLAX\tcomment with tab\n",
    "crlf": b"0000000001 LUG00001 FL1234 JFK LAX c\r\n0000000002 LUG00002 FL5678 BOS SFO d\r\n",
    "crlf_no_comment": b"0000000001 LUG00001 FL1234 JFK LAX\r\n",
    "long_luggage": b"0000000001 LUG0000123456 FL1234 JFK LAX long\n",
    "long_airport": b"0000000001 LUG00001 FL1234 JFKKK LAXXX long\n",
    "long_comment": b"0000000001 LUG00001 FL1234 JFK LAX " + b"x" * 200 + b"\n",
    "comment_exactly_80": b"0000000001 LUG00001 FL1234 JFK LAX" + b" " + b"y" * 79 + b"\n",
    "lowercase_ids": b"0000000001 lug00001 fl1234 jfk lax bad\n",
    "lowercase_airport": b"0000000001 LUG00001 FL1234 jfk lax bad\n",
    "nonnumeric_ts": b"abc LUG00001 FL1234 JFK LAX bad\n",
    "missing_fields": b"0000000001 LUG00001\n",
    "missing_arrival": b"0000000001 LUG00001 FL1234 JFK\n",
    "ts_only": b"0000000001\n",
    "ts_then_eof": b"0000000001 ",
    "double_record_one_line": (
        b"0000000001 LUG00001 FL1234 JFK LAX one 0000000002 LUG00002 FL5678 BOS SFO two\n"
    ),
    "many": b"".join(
        b"%010d LUG%05d FL%04d %s %s comment %d\n"
        % (i * 7 % 23, i % 5, i, b"JFK" if i % 2 else b"BOS", b"LAX" if i % 3 else b"SFO", i)
        for i in range(40)
    ),
    "dup_all_identical": b"0000000009 LUG00001 FL0001 JFK LAX same\n" * 5,
    "utf8_comment": "0000000001 LUG00001 FL1234 JFK LAX café ünïcødé ✈\n".encode(),
    "binary_comment": b"0000000001 LUG00001 FL1234 JFK LAX \x01\x02\xff\xfe\x80 bin\n",
    "nul_in_comment": b"0000000001 LUG00001 FL1234 JFK LAX a\x00b\n",
    "digits_in_airport": b"0000000001 LUG00001 FL1234 JF1 LAX bad\n",
    "sign_only_ts": b"- LUG00001 FL1234 JFK LAX bad\n",
    "ts_hex": b"0x10 LUG00001 FL1234 JFK LAX hex\n",
}


def run(binary, args, data):
    p = subprocess.run([binary] + args, input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=30)
    return p.returncode, p.stdout, p.stderr


def compare(name, args, data, failures):
    try:
        c = run(C_BIN, args, data)
        r = run(RUST_BIN, args, data)
    except subprocess.TimeoutExpired:
        failures.append((name, args, data, "TIMEOUT", ""))
        return
    if c != r:
        failures.append((name, args, data, c, r))


def main():
    failures = []
    checks = 0

    for name, data in CASES.items():
        for args in ARGSETS:
            compare(name, args, data, failures)
            checks += 1

    for args in BAD_ARGSETS:
        compare("argc", args, b"0000000001 LUG00001 FL1234 JFK LAX x\n", failures)
        checks += 1

    # Structured fuzzing over well-formed-ish records.
    random.seed(1234)
    tokens_ts = [b"0", b"1", b"0000000007", b"42", b"-3", b"4294967295", b"99999999999999999999", b"12345678901"]
    lug = [b"LUG00001", b"LUG00002", b"A", b"ABCDEFGH", b"ABCDEFGHIJ", b"9"]
    fl = [b"FL0001", b"FL0002", b"F", b"ABCDEF", b"ABCDEFGH"]
    ap = [b"JFK", b"BOS", b"LAX", b"SFO", b"A", b"AB", b"ABCD"]
    cm = [b"", b" hello", b" a b c", b"   ", b" " + b"z" * 90, b" \xff\x01"]
    seps = [b" ", b"  ", b"\t", b"\n", b" \n ", b"\r\n"]
    for _ in range(1500):
        n = random.randint(0, 6)
        parts = []
        for _ in range(n):
            rec = [random.choice(tokens_ts), random.choice(lug), random.choice(fl),
                   random.choice(ap), random.choice(ap)]
            line = b""
            for tok in rec:
                line += tok + random.choice(seps)
            line = line.rstrip(b" \t") if random.random() < 0.3 else line
            line += random.choice(cm)
            if random.random() < 0.9:
                line += b"\n"
            parts.append(line)
        data = b"".join(parts)
        args = random.choice(ARGSETS)
        compare("fuzz", args, data, failures)
        checks += 1

    # Byte-level fuzzing (may hit the C program's uninitialised-buffer paths).
    alphabet = b"0123456789ABCDEFGHJKLXabc -\n\t\r"
    for _ in range(1500):
        data = bytes(random.choice(alphabet) for _ in range(random.randint(0, 60)))
        compare("bytefuzz", random.choice(ARGSETS), data, failures)
        checks += 1

    print("checks: %d, failures: %d" % (checks, len(failures)))
    for name, args, data, c, r in failures[:15]:
        print("--- %s args=%r stdin=%r\n  C   : %r\n  Rust: %r" % (name, args, data, c, r))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
