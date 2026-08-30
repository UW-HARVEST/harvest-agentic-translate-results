#!/usr/bin/env python3
import subprocess, sys, os
BASE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = os.path.join(BASE, "c_src/build/driver")
R = os.path.join(BASE, "translation/target/release/driver")

CASES = [
    ("empty", b""),
    ("one_line_5", b"5\n"),
    ("two_lines", b"5\n4\n"),
    ("zero_zero", b"0\n0\n"),
    ("neg_zero", b"-0\n-0\n"),
    ("neg_zero_f", b"-0.0\n-0.0\n"),
    ("tiny_below_eps", b"0.0000001\n0.0000001\n"),
    ("eps_boundary", b"0.000001\n0.000001\n"),
    ("just_above_eps", b"0.0000011\n0.0000011\n"),
    ("garbage", b"abc\nxyz\n"),
    ("inf", b"inf\ninf\n"),
    ("neg_inf", b"-inf\n-inf\n"),
    ("infinity", b"INFINITY\nInfinity\n"),
    ("inf_partial", b"infin\ninfi\n"),
    ("nan", b"nan\nnan\n"),
    ("neg_nan", b"-NAN\n-nan\n"),
    ("nan_paren", b"nan(123)\nnan(x)\n"),
    ("hex", b"0x1p3\n0x10\n"),
    ("hex_nodigits", b"0x\n0xg\n"),
    ("hex_dot", b"0x1.8p1\n0x.8p2\n"),
    ("hex_nop", b"0x1f\n0x1f\n"),
    ("exp", b"1e2\n1E-2\n"),
    ("exp_bad", b"1e\n1e+\n"),
    ("leading_ws", b"   7  \n\t 8\n"),
    ("plus", b"+3\n+3.5\n"),
    ("dot_only", b".\n.\n"),
    ("trailing_dot", b"5.\n5.\n"),
    ("leading_dot", b".5\n.25\n"),
    ("huge", b"1e300\n1e300\n"),
    ("huge_overflow", b"1e999\n1e999\n"),
    ("tiny_underflow", b"1e-999\n1e-999\n"),
    ("f32_overflow", b"1e39\n1e39\n"),
    ("f32_subnormal", b"1e-44\n1e-44\n"),
    ("no_trailing_newline", b"5"),
    ("one_line_only", b"5\n"),
    ("long_line_20plus", b"111111111111111111112222\n"),
    ("exactly_19", b"1234567890123456789\n"),
    ("exactly_20", b"12345678901234567890\n"),
    ("big_int_result", b"0.00001\n0.00001\n"),
    ("negative", b"-4\n-4\n"),
    ("int_min_edge1", b"0.0000000465661287\n0.0000000465661287\n"),
    ("int_min_edge2", b"4.65661287e-8\n4.65661287e-8\n"),
    ("windows_crlf", b"5\r\n4\r\n"),
    ("null_byte", b"5\x006\n7\x008\n"),
    ("space_only", b" \n \n"),
    ("many_lines", b"2\n4\n8\n16\n"),
    ("1e-46", b"1e-46\n1e-46\n"),
    ("near_eps", b"0.0000010000001\n0.0000010000001\n"),
    ("hexbig", b"0x1p1000\n0x1p1000\n"),
    ("hexneg", b"-0x1p-200\n-0x1p-200\n"),
    ("underscore", b"1_000\n1_000\n"),
    ("digits_then_junk", b"12abc\n34xyz\n"),
    ("only_newlines", b"\n\n"),
    ("only_one_newline", b"\n"),
    ("minus_only", b"-\n-\n"),
    ("plus_only", b"+\n+\n"),
    ("many_zeros", b"0.000000000000000000\n0000000000000000000\n"),
    ("long_precise", b"3.14159265358979323846\n2.718281828459045\n"),
    ("d_suffix", b"5d\n5f\n"),
    ("2_147_483_647", b"4.656612873077e-08\n4.656612873077e-08\n"),
    ("just_over_intmax", b"4.6566128e-08\n4.6566128e-08\n"),
    ("100", b"100\n100\n"),
    ("0.5", b"0.5\n0.5\n"),
    ("3", b"3\n3\n"),
    ("neg_tiny", b"-0.0000001\n-0.0000001\n"),
    ("neg_huge", b"-1e39\n-1e39\n"),
    ("tab_number", b"\t42\n\t42\n"),
    ("vtab", b"\x0b42\n\x0c42\n"),
    ("cr_number", b"\r42\n\r42\n"),
    ("18_spaces_then_num", b"                  1\n"),
    ("19_spaces", b"                   \n"),
    ("split_across_buf", b"0.000000000000000005\n"),
    ("e_only_second", b"e5\ne5\n"),
    ("hex_upper", b"0X1P3\n0X1P3\n"),
    ("hex_p_bad", b"0x1p\n0x1p\n"),
    ("nan_then_num", b"nan\n5\n"),
    ("inf_then_zero", b"inf\n0\n"),
    ("zero_then_inf", b"0\ninf\n"),
    ("neg_inf_bad", b"-inf\n-inf\n"),
    ("hex_intmax", b"0x1p-25\n0x1p-25\n"),
    ("subnormal_f32", b"7e-46\n7e-46\n"),
    ("exact_1em6_f32", b"1.0000001e-6\n9.999999e-7\n"),
    ("binary_junk", b"\x01\x02\x03\n\xff\xfe\n"),
    ("high_bytes", b"\xc3\xa9 5\n\xc3\xa9 5\n"),
]


def run(exe, data):
    p = subprocess.run([exe], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return p.stdout, p.stderr, p.returncode


fails = 0
for name, data in CASES:
    a = run(C, data)
    b = run(R, data)
    if a == b:
        print("PASS %-24s %r" % (name, a[0]))
    else:
        fails += 1
        print("FAIL %s input=%r" % (name, data))
        print("   C: out=%r err=%r rc=%s" % a)
        print("   R: out=%r err=%r rc=%s" % b)
print("\n%d/%d failed" % (fails, len(CASES)))
sys.exit(1 if fails else 0)
