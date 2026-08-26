#!/usr/bin/env python3
"""Hand-crafted edge-case differential comparison: C vs Rust executables."""
import subprocess, sys, os

C = os.path.abspath("c_src/build/driver")
R = os.path.abspath("target/release/driver")


def run(binary, args, data):
    p = subprocess.run([binary] + args, input=data, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, timeout=30)
    return p.returncode, p.stdout, p.stderr


CASES = []


def case(name, args, data):
    CASES.append((name, args, data if isinstance(data, bytes) else data.encode("latin1")))


D = ["-", "-", "-", "-"]

# --- empty / trivial input -------------------------------------------------
case("empty stdin", D, "")
case("only newline", D, "\n")
case("only spaces", D, "   ")
case("only ws mix", D, " \t\n\v\f\r ")
case("nul only", D, b"\x00")

# --- number parsing --------------------------------------------------------
case("ts sign only", D, "-")
case("ts plus only", D, "+")
case("ts sign then letter", D, "-X AAA BB JFK LAX c\n")
case("ts plus value", D, "+42 AAA BB JFK LAX c\n")
case("ts negative", D, "-42 AAA BB JFK LAX c\n")
case("ts zero", D, "0 AAA BB JFK LAX c\n")
case("ts leading zeros", D, "000000000000123 AAA BB JFK LAX c\n")
case("ts int max", D, "2147483647 AAA BB JFK LAX c\n")
case("ts int max +1", D, "2147483648 AAA BB JFK LAX c\n")
case("ts uint max", D, "4294967295 AAA BB JFK LAX c\n")
case("ts uint max +1", D, "4294967296 AAA BB JFK LAX c\n")
case("ts long max", D, "9223372036854775807 AAA BB JFK LAX c\n")
case("ts long max+1", D, "9223372036854775808 AAA BB JFK LAX c\n")
case("ts long min", D, "-9223372036854775808 AAA BB JFK LAX c\n")
case("ts long min-1", D, "-9223372036854775809 AAA BB JFK LAX c\n")
case("ts 40 digits", D, "1" * 40 + " AAA BB JFK LAX c\n")
case("ts 40 digits neg", D, "-" + "1" * 40 + " AAA BB JFK LAX c\n")
case("ts hex-ish", D, "0x10 AAA BB JFK LAX c\n")
case("ts float-ish", D, "1.5 AAA BB JFK LAX c\n")
case("ts thousands sep", D, "1,000 AAA BB JFK LAX c\n")
case("ts leading ws lots", D, "\n\n\t  7 AAA BB JFK LAX c\n")

# --- scan set widths -------------------------------------------------------
case("lug 8 exact", D, "1 ABCDEFGH FL1234 JFK LAX c\n")
case("lug 9 chars", D, "1 ABCDEFGHI FL1234 JFK LAX c\n")
case("lug 12 chars", D, "1 ABCDEFGHIJKL FL1234 JFK LAX c\n")
case("flight 7 chars", D, "1 AB ABCDEFG JFK LAX c\n")
case("dep 4 chars", D, "1 AB CD JFKX LAX c\n")
case("arr 4 chars", D, "1 AB CD JFK LAXY c\n")
case("arr 6 chars", D, "1 AB CD JFK LAXYZW c\n")
case("all overlong", D, "1 ABCDEFGHIJ ABCDEFGH JFKL LAXY tail\n")

# --- scan set matching failures -------------------------------------------
case("lowercase lug", D, "1 abc FL1 JFK LAX c\n")
case("lowercase flight", D, "1 ABC fl1 JFK LAX c\n")
case("digits in airport", D, "1 ABC FL1 J1K LAX c\n")
case("digit airport dep", D, "1 ABC FL1 111 LAX c\n")
case("all lowercase line", D, "abc def\n")
case("single letter", D, "A")
case("single lowercase", D, "a")
case("only dash line", D, "-\n")
case("stale buffers", D, "x\n1 ABC FL1 JFK LAX ok\ny\n")
case("stale reuse", D, "1 ABC FL1 JFK LAX ok\nzz\n")
case("no digits at all", D, "QQ WW EE RR TT\n")

# --- EOF in the middle -----------------------------------------------------
case("eof after ts", D, "5")
case("eof after ts sp", D, "5 ")
case("eof after lug", D, "5 ABC")
case("eof after lug sp", D, "5 ABC ")
case("eof after flight", D, "5 ABC FL1")
case("eof after flight sp", D, "5 ABC FL1 ")
case("eof after dep", D, "5 ABC FL1 JFK")
case("eof after dep sp", D, "5 ABC FL1 JFK ")
case("eof after arr", D, "5 ABC FL1 JFK LAX")
case("eof after arr nl", D, "5 ABC FL1 JFK LAX\n")
case("eof after comment", D, "5 ABC FL1 JFK LAX cm")
case("two recs 2nd trunc", D, "5 ABC FL1 JFK LAX cm\n6 DEF")

# --- comments --------------------------------------------------------------
case("comment 80", D, "1 ABC FL1 JFK LAX" + "x" * 80 + "\n")
case("comment 81", D, "1 ABC FL1 JFK LAX" + "x" * 81 + "\n")
case("comment 200", D, "1 ABC FL1 JFK LAX" + "y" * 200 + "\n")
case("comment 200 then rec", D, "1 ABC FL1 JFK LAX" + "y" * 200 + "\n2 ABC FL2 SFO SEA z\n")
case("comment tabs", D, "1 ABC FL1 JFK LAX\t\tcm\n")
case("comment cr", D, "1 ABC FL1 JFK LAX cm\r\n2 ABD FL2 SFO SEA cm2\r\n")
case("comment nul", D, b"1 ABC FL1 JFK LAX ab\x00cd\n")
case("comment high bytes", D, b"1 ABC FL1 JFK LAX \xff\xfe\x80 end\n")
case("no comment nl", D, "1 ABC FL1 JFK LAX\n2 ABD FL2 SFO SEA\n")
case("comment only space", D, "1 ABC FL1 JFK LAX \n")

# --- separators ------------------------------------------------------------
case("tabs everywhere", D, "1\tABC\tFL1\tJFK\tLAX\tcm\n")
case("newlines between", D, "1\nABC\nFL1\nJFK\nLAX\ncm\n")
case("many spaces", D, "1     ABC     FL1     JFK     LAX     cm\n")
case("no space dep arr", D, "1 ABC FL1 JFKLAX cm\n")
case("vertical tab", D, "1\x0bABC\x0bFL1\x0bJFK\x0bLAX cm\n")
case("formfeed", D, "1\x0cABC\x0cFL1\x0cJFK\x0cLAX cm\n")
case("cr separators", D, "1\rABC\rFL1\rJFK\rLAX cm\n")

# --- sorting and ties ------------------------------------------------------
case("ties stable", D, "1 A1 F1 AAA BBB one\n1 A2 F2 AAA BBB two\n1 A3 F3 AAA BBB three\n")
case("descending", D, "5 A5 F AAA BBB e\n4 A4 F AAA BBB d\n3 A3 F AAA BBB c\n2 A2 F AAA BBB b\n1 A1 F AAA BBB a\n")
case("mixed order", D, "3 C F AAA BBB c\n1 A F AAA BBB a\n5 E F AAA BBB e\n2 B F AAA BBB b\n4 D F AAA BBB d\n")
case("zero ts first", D, "0 Z F AAA BBB z\n1 A F AAA BBB a\n")

# --- supersedes / superseded ----------------------------------------------
case("same lug same dep", D, "1 L1 F1 AAA BBB first\n2 L1 F2 AAA CCC second\n")
case("same lug diff dep", D, "1 L1 F1 AAA BBB first\n2 L1 F2 XXX CCC second\n")
case("same lug three", D, "1 L1 F1 AAA BBB a\n2 L1 F2 XXX CCC b\n3 L1 F3 AAA DDD c\n")
case("interleaved lugs", D, "1 L1 F1 AAA BBB a\n2 L2 F2 AAA BBB b\n3 L1 F3 AAA CCC c\n4 L2 F4 ZZZ DDD d\n")
case("dup identical", D, "1 L1 F1 AAA BBB a\n1 L1 F1 AAA BBB a\n")

# --- filters (argv) --------------------------------------------------------
BASE = "1 L1 F1 AAA BBB one\n2 L2 F2 CCC DDD two\n3 L1 F3 XXX BBB three\n"
case("filter exact lug", ["L1", "-", "-", "-"], BASE)
case("filter exact flight", ["-", "F2", "-", "-"], BASE)
case("filter dep", ["-", "-", "CCC", "-"], BASE)
case("filter arr", ["-", "-", "-", "BBB"], BASE)
case("filter all exact", ["L2", "F2", "CCC", "DDD"], BASE)
case("filter no match", ["ZZ", "-", "-", "-"], BASE)
case("filter empty strings", ["", "", "", ""], BASE)
case("filter dash prefix", ["-L1", "-x", "-", "-"], BASE)
case("filter dash only mixed", ["-", "F1", "-", "BBB"], BASE)
case("filter case sensitive", ["l1", "-", "-", "-"], BASE)
case("filter long", ["L1XXXXXXXXXXXX", "-", "-", "-"], BASE)
case("filter empty vs empty field", ["", "-", "-", "-"], "1 L1 F1 AAA BBB c\n")

# --- argc errors -----------------------------------------------------------
CASES.append(("argc 0 args", [], b"1 L1 F1 AAA BBB c\n"))
CASES.append(("argc 1 arg", ["-"], b"1 L1 F1 AAA BBB c\n"))
CASES.append(("argc 2 args", ["-", "-"], b""))
CASES.append(("argc 3 args", ["-", "-", "-"], b""))
CASES.append(("argc 5 args", ["-", "-", "-", "-", "-"], b"1 L1 F1 AAA BBB c\n"))
CASES.append(("argc 6 args", ["-"] * 6, b""))

# --- larger inputs ---------------------------------------------------------
big = "".join("%d L%d F%d AAA BBB c%d\n" % (1000 - i, i % 7, i, i) for i in range(300))
case("300 records", D, big)
big2 = "".join("%d L%d F%d %s %s cm\n" % (i % 13, i % 5, i % 3,
        "ABC" if i % 2 else "XYZ", "DEF" if i % 3 else "UVW") for i in range(500))
case("500 records dup ts", D, big2)


def main():
    fails = 0
    for name, args, data in CASES:
        try:
            c = run(C, args, data)
        except subprocess.TimeoutExpired:
            print("[TIMEOUT-C] %s" % name)
            continue
        try:
            r = run(R, args, data)
        except subprocess.TimeoutExpired:
            print("[TIMEOUT-R] %s" % name)
            fails += 1
            continue
        if c != r:
            fails += 1
            print("[FAIL] %-24s args=%r stdin=%r" % (name, args, data[:120]))
            print("   C  rc=%d out=%r err=%r" % c)
            print("   RS rc=%d out=%r err=%r" % r)
    print("%d cases, %d failures" % (len(CASES), fails))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
