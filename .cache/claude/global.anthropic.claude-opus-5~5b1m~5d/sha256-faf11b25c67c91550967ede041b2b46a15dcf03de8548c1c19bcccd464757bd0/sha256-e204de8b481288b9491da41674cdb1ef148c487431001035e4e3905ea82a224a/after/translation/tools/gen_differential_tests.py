"""Regenerate translation/tests/differential.rs.

For every enumerated input this runs the compiled C program REPS times to make
sure the C program itself answers reproducibly (its two stack buffers are
uninitialised, so some inputs do not - see ERRORS.md), compares the answer with
the Rust translation, and only then writes the test file.

    cd c_src && cmake -S . -B build && cmake --build build
    cd translation && cargo build --release
    python3 translation/tools/gen_differential_tests.py
"""
import subprocess

import os

# <repo>/translation/tools/gen_differential_tests.py -> <repo>
W = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
C = W + "/c_src/build/driver"
R = W + "/translation/target/release/driver"

CASES = []  # (name, comment, spec) ; spec = ("raw", text) | ("enc", op, flags, ib, rb)


def raw(name, text, comment=""):
    CASES.append((name, comment, ("raw", text)))


def enc(name, op, flags, ib, rb, comment=""):
    CASES.append((name, comment, ("enc", str(op), str(flags), list(ib), list(rb))))


def b(s, nul=True):
    return list(s.encode()) + ([0] if nul else [])


def text_of(spec):
    if spec[0] == "raw":
        return spec[1]
    _, op, flags, ib, rb = spec
    return (
        f"{op} {flags} {len(ib)} "
        + " ".join(map(str, ib))
        + f" {len(rb)} "
        + " ".join(map(str, rb))
        + "\n"
    )


# =========================================================== main.c: input parsing
raw("empty_stdin", "", "scanf(\"%d\") hits EOF -> 'Error reading operation'")
raw("whitespace_only", "   \n\t\n", "scanf skips whitespace, then EOF")
raw("operation_not_a_number", "hello\n", "matching failure on %d")
raw("operation_only", "3\n", "EOF while reading flags")
raw("flags_not_a_number", "3 zzz\n", "matching failure on %u")
raw("operation_and_flags_only", "3 0\n", "EOF while reading input length")
raw("input_len_not_a_number", "3 0 abc\n", "matching failure on %zu")
raw("input_len_1025", "3 0 1025 1 0\n", "input_len > MAX_BUFFER_SIZE")
raw("input_len_negative", "3 0 -1\n", "%zu accepts '-1' -> SIZE_MAX")
raw("input_len_overflow", "3 0 99999999999999999999\n", "strtoul clamps to ULONG_MAX")
raw("input_byte_missing_at_0", "1 0 3\n", "EOF on the first input byte")
raw("input_byte_missing_at_2", "1 0 3 65 66\n", "EOF on input byte 2")
raw("input_byte_not_a_number", "1 0 2 65 x\n", "matching failure on input byte 1")
raw("ref_len_missing", "1 0 2 65 66\n", "EOF while reading reference length")
raw("ref_len_not_a_number", "1 0 1 65 zz\n", "matching failure on reference length")
raw("ref_len_1025", "1 0 1 65 1025\n", "ref_len > MAX_BUFFER_SIZE")
raw("ref_len_negative", "1 0 1 65 -5\n", "%zu accepts '-5' -> SIZE_MAX-4")
raw("ref_len_ulong_max", "1 0 1 65 18446744073709551615\n", "ULONG_MAX reference length")
raw("ref_byte_missing_at_2", "0 0 1 65 3 65 66\n", "EOF on reference byte 2")
raw("ref_byte_not_a_number", "0 0 1 65 2 65 q\n", "matching failure on reference byte 1")
raw("trailing_garbage_is_ignored", "0 0 3 79 75 0 3 79 75 0 trailing junk\n", "main never reads past the last byte")
raw("tab_separated", "0\t0\t3\t79\t75\t0\t3\t79\t75\t0\n", "scanf treats every space character alike")
raw("crlf_separated", "0\r\n0\r\n3\r\n79\r\n75\r\n0\r\n3\r\n79\r\n75\r\n0\r\n", "\\r is whitespace for scanf")
raw("no_trailing_newline", "0 0 3 79 75 0 3 79 75 0", "last conversion terminated by EOF")
raw("explicit_plus_signs", "+0 +0 +3 +79 +75 +0 +3 +79 +75 +0\n", "scanf accepts a leading '+'")
raw("byte_value_negative", "0 0 2 -1 0 2 255 0", "(char)(unsigned)-1 == 0xff")
raw("byte_value_321_truncated", "0 0 2 321 0 2 65 0", "(char)321 == 'A'")
raw("byte_value_2_pow_32_plus_1", "0 0 2 4294967297 0 2 1 0", "%u truncates to 32 bits, (char) to 8")
raw("flags_2_pow_32_plus_1", "2 4294967297 3 65 66 0 3 65 66 0", "flags truncated to 32 bits -> bit0 set")
raw("flags_all_bits_set", "2 4294967295 3 65 66 0 3 65 66 0", "every flag bit set")

# ====================================================== lib.c: operation dispatch
raw("operation_5_default", "5 0 0 0\n", "default: -> -3")
raw("operation_minus_1", "-1 0 0 0\n", "negative operation -> -3")
raw("operation_int_max", "2147483647 0 0 0\n", "INT_MAX -> -3")
raw("operation_int_min", "-2147483648 0 0 0\n", "INT_MIN -> -3")
raw("operation_overflow_wraps_to_minus_1", "99999999999999999999 0 0 0\n", "LONG_MAX truncated to int == -1 -> -3")

# ============================================================ op 0 validate_token
enc("op0_token_equals_reference", 0, 0, b("SECRET"), b("SECRET"), "strcmp(token, expected) == 0 -> 1")
enc("op0_token_differs", 0, 0, b("SECRET"), b("OTHER"), "no match -> 0")
enc("op0_literal_valid", 0, 0, b("VALID"), b("nope"), "second strcmp -> 1")
enc("op0_literal_ok", 0, 0, b("OK"), b("nope"), "third strcmp -> 1")
enc("op0_both_empty_strings", 0, 0, [0], [0], "\"\" == \"\" -> 1")
enc("op0_token_is_prefix_of_reference", 0, 0, b("SEC"), b("SECRET"), "shorter -> 0")
enc("op0_zero_length_buffers", 0, 0, [], [], "both strcmp operands are pure stack residue")
enc("op0_unterminated_equal_payloads", 0, 0, b("RESUME", False), b("RESUME", False), "strcmp runs into the residue of both buffers (both terminate at offset 6)")
enc("op0_unterminated_token_only", 0, 0, b("VALID", False), b("VALID"), "token continues past its data")
enc("op0_full_input_buffer", 0, 0, [65] * 1024, [65] * 1024, "maximum length for both buffers")
enc("op0_full_input_empty_reference", 0, 0, [66] * 1024, [], "strlen(input) runs into main()'s locals")
enc("op0_full_reference_buffer", 0, 0, b("Z"), [90] * 1024, "strcmp runs off the end of ref_buffer into input_buffer")

# ============================================================= op 1 parse_command
for idx, cmd in enumerate(["START", "STOP", "PAUSE", "RESUME", "RESET"]):
    lo = cmd.lower()
    enc(f"op1_{lo}_nul_terminated", 1, 0, b(cmd), [], f"buffer[cmd_len] == 0 -> {idx}")
    enc(f"op1_{lo}_followed_by_space", 1, 0, b(cmd + " arg"), [], f"buffer[cmd_len] == ' ' -> {idx}")
    if cmd != "STOP":
        # `buffer[4]` is a byte of a randomised address, see ERRORS.md.
        enc(f"op1_{lo}_unterminated", 1, 0, b(cmd, False), [], "buffer[cmd_len] is stack residue")
    enc(f"op1_{lo}_one_byte_short", 1, 0, b(cmd[:-1], False), [], "buf_size < cmd_len, strcmp fallback")
enc("op1_admin", 1, 0, b("ADMIN"), [], "special admin command -> 99")
enc("op1_admin_with_space", 1, 0, b("ADMIN "), [], "strcmp(\"ADMIN \", \"ADMIN\") != 0 -> -1")
enc("op1_unknown_command", 1, 0, b("NOPE"), [], "no match -> -1")
enc("op1_empty_string", 1, 0, [0], [], "empty command -> -1")
enc("op1_zero_length", 1, 0, [], [], "buf_size 0, every strncmp skipped")
enc("op1_start_with_extra_char", 1, 0, b("STARTX"), [], "buffer[5] == 'X' -> -1")
enc("op1_stopped", 1, 0, b("STOPPED"), [], "prefix of STOP but buffer[4] == 'P' -> -1")
enc("op1_full_buffer", 1, 0, [65] * 1024, [], "1024 bytes without a terminator")
enc("op1_start_exactly_5_bytes", 1, 0, [83, 84, 65, 82, 84], [], "buf_size == cmd_len, buffer[5] is residue")
enc("op1_reset_space_nul", 1, 0, [82, 69, 83, 69, 84, 32, 0], [], "RESET followed by ' ' -> 4")

# ============================================================ op 2 compare_prefix
enc("op2_prefix_matches", 2, 0, b("HELLOWORLD"), b("HELLO"), "strncmp prefix -> 1")
enc("op2_prefix_does_not_match", 2, 0, b("HELLOWORLD"), b("WORLD"), "-> 0")
enc("op2_prefix_empty_reference", 2, 0, b("HELLO"), [0], "prefix_len 0, strncmp(_,_,0) == 0 -> 1")
enc("op2_exact_equal", 2, 1, b("HELLO"), b("HELLO"), "flags bit0 -> strcmp -> 1")
enc("op2_exact_not_equal", 2, 1, b("HELLOX"), b("HELLO"), "no variation matches -> 0")
for i, suf in enumerate(["_v1", "_v2", "_old", "_new", "_tmp"]):
    enc(f"op2_exact_variation_{suf.strip('_')}", 2, 1, b("BASE" + suf), b("BASE"), f"variation {i} -> {2 + i}")
enc("op2_exact_unknown_variation", 2, 1, b("BASE_xx"), b("BASE"), "-> 0")
enc("op2_exact_flags_bit0_and_bit1", 2, 3, b("BASE_v2"), b("BASE"), "only bit0 selects exact matching")
enc("op2_prefix_flags_bit1_only", 2, 2, b("HELLOWORLD"), b("HELLO"), "bit0 clear -> prefix mode")
enc("op2_exact_prefix_62_bytes", 2, 1, b("P" * 62 + "_v1"), b("P" * 62), "62 + 3 fits into expected[64]")
enc("op2_exact_prefix_63_bytes", 2, 1, b("P" * 63 + "_v1"), b("P" * 63), "strncat has no room left")
enc("op2_exact_prefix_64_bytes", 2, 1, b("P" * 63), b("P" * 64), "strncpy truncates the prefix to 63 bytes")
enc("op2_exact_prefix_70_bytes", 2, 1, b("P" * 63), b("P" * 70), "prefix longer than expected[]")
enc("op2_exact_prefix_60_plus_tmp", 2, 1, b("Q" * 60 + "_tmp"), b("Q" * 60), "strncat truncates '_tmp' to '_tm' -> 0")
enc("op2_exact_zero_length_reference", 2, 1, b("HELLO"), [], "expected[] built from residue")
enc("op2_exact_both_empty", 2, 1, [0], [0], "\"\" == \"\" -> 1")
enc("op2_prefix_both_empty", 2, 0, [0], [0], "prefix_len 0 -> 1")
enc("op2_prefix_full_input", 2, 0, [67] * 1024, b("C" * 100), "1024 byte input, 100 byte prefix")
enc("op2_prefix_full_reference", 2, 0, [65] * 1023 + [0], [65] * 1024, "strlen(prefix) runs into input_buffer")
enc("op2_exact_full_both", 2, 1, [65] * 1024, [65] * 1024, "strcmp over both full buffers")

# =========================================================== op 3 find_delimiter
enc("op3_zero_length", 3, 0, [], b(":"), "len == 0 -> -1")
enc("op3_delimiter_at_0", 3, 0, b(":abc"), b(":"), "-> 0")
enc("op3_delimiter_at_3", 3, 0, b("abc:def"), b(":"), "-> 3")
enc("op3_delimiter_absent", 3, 0, b("abcdef"), b(":"), "-> -1")
enc("op3_nul_before_delimiter", 3, 0, b("abc") + [58], b(":"), "scan breaks at the NUL -> -1")
enc("op3_default_delimiter_ref_len_0", 3, 0, b("ab:cd"), [], "ref_len == 0 -> delim ':'")
enc("op3_delimiter_is_nul_of_reference", 3, 0, b("ab:cd"), [0, 124], "reference[0] == 0 -> delim '\\0'")
enc("op3_pipe_none_special", 3, 0, b("NONE"), b("|"), "delim '|' and \"NONE\" -> -2")
enc("op3_colon_empty_special", 3, 0, b("EMPTY"), b(":"), "delim ':' and \"EMPTY\" -> -3")
enc("op3_colon_empty_unterminated", 3, 0, b("EMPTY", False), b(":"), "strcmp runs into residue -> -1")
enc("op3_pipe_with_empty_text", 3, 0, b("EMPTY"), b("|"), "wrong delimiter for the special case -> -1")
enc("op3_colon_with_none_text", 3, 0, b("NONE"), b(":"), "wrong delimiter for the special case -> -1")
enc("op3_pipe_found", 3, 0, b("a|b"), b("|"), "-> 1")
enc("op3_delimiter_nul_byte", 3, 0, b("abc"), [0], "delim '\\0' matches the terminator -> 3")
enc("op3_multi_byte_reference", 3, 0, b("a#b"), b("#!"), "only reference[0] is used")
enc("op3_delimiter_at_last_index", 3, 0, [65] * 1023 + [58], b(":"), "-> 1023")
enc("op3_full_buffer_no_delimiter", 3, 0, [65] * 1024, b(":"), "scan all 1024 bytes -> -1")
enc("op3_leading_nul", 3, 0, [0, 58, 58], b(":"), "break on data[0] == 0 -> -1")
enc("op3_single_nul", 3, 0, [0], b(":"), "len 1, data[0] == 0 -> -1")

# ============================================================ op 4 match_pattern
enc("op4_ci_exact_equal", 4, 0, b("Hello"), b("Hello"), "strcmp -> 1")
enc("op4_ci_case_folded_equal", 4, 0, b("HELLO"), b("hello"), "manual tolower compare -> 6")
enc("op4_ci_prefix", 4, 0, b("HELLOWORLD"), b("HELLO"), "text_len != pattern_len, strncmp -> 5")
enc("op4_ci_prefix_wrong_case", 4, 0, b("helloworld"), b("HELLO"), "strncmp is case sensitive -> 0")
enc("op4_ci_same_length_no_match", 4, 0, b("abcde"), b("vwxyz"), "-> 0")
enc("op4_ci_different_length_no_match", 4, 0, b("abcde"), b("zz"), "-> 0")
enc("op4_ci_empty_pattern", 4, 0, b("abc"), [0], "pattern_len 0 -> strncmp(_,_,0) -> 5")
enc("op4_ci_both_empty", 4, 0, [0], [0], "strcmp -> 1")
enc("op4_ci_non_alphabetic_bytes", 4, 0, b("@[`{"), b("@[`{"), "bytes around 'A'-'Z' are not folded")
enc("op4_ci_z_vs_brace", 4, 0, b("Z["), b("z{"), "'[' + 32 == '{' must not fold -> 0")
enc("op4_ci_full_length_folded", 4, 0, b("K" * 1000), b("k" * 1000), "1000 byte case-insensitive match -> 6")
enc("op4_ci_pattern_longer", 4, 0, b("zz"), b("PATTERNTOOLONGXYZ"), "no size_t underflow on this path -> 0")
enc("op4_cs_exact_equal", 4, 2, b("Hello"), b("Hello"), "flags bit1 -> strcmp -> 1")
enc("op4_cs_wildcard_surrounded", 4, 2, b("*pat*"), b("pat"), "snprintf \"*%s*\" -> 2")
enc("op4_cs_wildcard_suffix", 4, 2, b("pat*"), b("pat"), "snprintf \"%s*\" -> 3")
enc("op4_cs_wildcard_prefix", 4, 2, b("*pat"), b("pat"), "snprintf \"*%s\" -> 4")
enc("op4_cs_substring_at_0", 4, 2, b("abcdef"), b("abc"), "-> 10")
enc("op4_cs_substring_at_1", 4, 2, b("xabcx"), b("abc"), "-> 11")
enc("op4_cs_substring_at_2", 4, 2, b("xxabc"), b("abc"), "-> 12")
enc("op4_cs_substring_at_40", 4, 2, b("x" * 40 + "abc"), b("abc"), "-> 50")
enc("op4_cs_no_substring", 4, 2, b("abcdef"), b("xyz"), "loop finishes -> 0")
enc("op4_cs_same_length_no_match", 4, 2, b("abc"), b("xyz"), "text_len == pattern_len, one iteration -> 0")
enc("op4_cs_empty_pattern", 4, 2, b("abc"), [0], "pattern_len 0 -> 10")
enc("op4_cs_both_empty", 4, 2, [0], [0], "strcmp -> 1")
enc("op4_cs_flags_bit0_and_bit1", 4, 3, b("xxabc"), b("abc"), "bit0 is ignored by operation 4 -> 12")
enc("op4_cs_wildcard_61_bytes", 4, 2, b("*" + "y" * 61 + "*"), b("y" * 61), "\"*%s*\" fills exactly 63 bytes")
enc("op4_cs_wildcard_62_bytes", 4, 2, b("*" + "y" * 62), b("y" * 62), "snprintf truncates the trailing '*'")
enc("op4_cs_wildcard_63_bytes", 4, 2, b("*" + "y" * 62), b("y" * 63), "the pattern itself is truncated")
enc("op4_cs_pattern_longer_underflows", 4, 2, b("A", False), b("ABCDEFGHIJ"), "text_len - pattern_len underflows -> SIGSEGV")
enc("op4_cs_pattern_longer_underflows_2", 4, 2, b("zz"), b("PATTERNTOOLONGXYZ"), "same underflow with a terminated text")
enc("op4_cs_full_buffers_underflow", 4, 2, [65] * 1024, [65] * 1024, "ref_buffer runs into input_buffer: pattern_len 2048 > text_len 1024")
enc("op4_ci_full_input_8_byte_pattern", 4, 0, [65] * 1024, [65] * 8, "strlen(input) reads ref_len from the stack frame")
enc("op4_ci_full_both_buffers", 4, 0, [65] * 1024, [65] * 1024, "pattern_len 2048 != text_len 1024 -> 0")
enc("op4_ci_full_input_257_byte_pattern", 4, 0, [65] * 1024, [65] * 257, "ref_len 0x101 leaks two non-zero bytes")
enc("op4_cs_locals_leak_after_input_buffer", 4, 2, [65] * 1024, [65, 8, 0, 1, 1, 1, 1, 1],
    "the two byte pattern 'A' 0x08 exists only where input_buffer ends and the "
    "little endian ref_len local begins: 10 + 1023 == 1033")
enc("op2_prefix_len_crosses_into_input_buffer", 2, 0, [65] * 1024, [65] * 1024,
    "strlen(prefix) is 2048 because ref_buffer is immediately followed by input_buffer")

# ================================================= phase C: paths still untouched
raw("byte_written_as_hex_literal", "0 0 2 0x41 0 2 65 0", "%u stops at 'x' -> failure on byte 1")
raw("leading_zeros_everywhere", "0000 0000 0003 079 075 000 0003 079 075 000\n", "leading zeros are accepted by %u")
raw("byte_overflow_becomes_0xff", "0 0 1 99999999999999999999 2 255 0", "ULONG_MAX -> (char)0xff")
raw("operation_with_leading_whitespace", "\n\n\t  0 0 3 79 75 0 3 79 75 0\n", "%d skips leading whitespace")
raw("negative_zero_operation", "-0 0 0 0\n", "'-0' is a valid %d -> operation 0")
enc("op0_single_byte_nul", 0, 0, [0], [65], "one byte input holding just the terminator")
enc("op0_single_byte_a", 0, 0, [65], [65], "single unterminated byte in both buffers")
enc("op0_embedded_nul", 0, 0, b("AB") + b("CD"), b("AB") + b("CD"), "strcmp stops at the embedded NUL -> 1")
enc("op0_embedded_nul_differing_tail", 0, 0, b("AB") + b("CD"), b("AB") + b("ZZ"), "the tail behind the NUL is invisible -> 1")
enc("op1_start_embedded_nul_tail", 1, 0, b("START") + b("XYZ"), [], "buffer[5] == 0 -> 0")
enc("op1_start_padded_with_nuls", 1, 0, b("START") + [0] * 1018, [], "full buffer, terminator at index 5 -> 0")
enc("op1_space_only", 1, 0, b(" "), [], "no command starts with a space -> -1")
enc("op2_exact_embedded_nul", 2, 1, b("BASE_v1") + b("tail"), b("BASE"), "strcmp ignores everything past the NUL -> 2")
enc("op3_delim_255", 3, 0, [65, 255, 66, 0], [255], "delimiter 0xff -> 1")
enc("op3_delim_newline_byte", 3, 0, b("ab\ncd"), [10], "delimiter '\\n' -> 2")
enc("op3_delim_space_byte", 3, 0, b("ab cd"), [32], "delimiter ' ' -> 2")
enc("op3_all_nul_buffer", 3, 0, [0] * 16, [65], "break at index 0 -> -1")
enc("op4_cs_substring_at_500", 4, 2, b("x" * 500 + "abc"), b("abc"), "10 + 500 -> 510")
enc("op4_cs_substring_last_position", 4, 2, b("abcdef"), b("f"), "match at text_len - 1 -> 15")
enc("op4_ci_high_bytes", 4, 0, [200, 201, 0], [200, 201, 0], "bytes >= 0x80 are compared as chars")
enc("op4_ci_pattern_equals_text_len_1", 4, 0, [65, 0], [97, 0], "'A' folds to 'a' -> 6")
enc("op4_cs_text_len_equals_pattern_len", 4, 2, [65, 66, 0], [65, 67, 0], "single strncmp iteration -> 0")


# ================================================ pseudo random cross check table
def build_corpus():
    import random

    rng = random.Random(20250828)
    payloads = [
        b"", b"A", b"OK", b"VALID", b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET",
        b"ADMIN", b"NONE", b"EMPTY", b"BASE", b"BASE_v1", b"BASE_tmp", b"abc", b"ABC",
        b"aBc", b"abcdef", b"x:y", b"a|b", b"START x", b"STOPX", b"hello world",
        b"\x01\x02\x03", b"\xff\xfe", b"  ", b"::", b"||", b"START ", b"RESUME", b"ADMIN12", b"abcdef", b"BASE_v1",
    ]
    corpus = []
    while len(corpus) < 140:
        op = rng.choice([0, 1, 2, 3, 4, 4, 5, -2])
        flags = rng.choice([0, 1, 2, 3, 4, 6])
        ip = rng.choice(payloads)
        rp = rng.choice(payloads)
        # `match_pattern()` underflows `text_len - pattern_len` when the pattern is
        # longer than the text and then walks the whole stack, which even the C
        # program does not do reproducibly - keep that class out of the corpus and
        # cover it with the dedicated SIGSEGV tests instead.
        # An unterminated payload only behaves reproducibly when the first NUL of
        # the stack residue behind it is one of the two that are always NUL (the
        # high order bytes of a 48 bit address, at offsets 6 and 7 of both
        # buffers) - so leave the terminator out only for those two lengths.
        nul_i = rng.random() < 0.6 or len(ip) not in (6, 7)
        nul_r = rng.random() < 0.6 or len(rp) not in (6, 7)
        if op == 4 and (flags & 0x02) != 0:
            # `match_pattern()` underflows `text_len - pattern_len` when the
            # pattern is longer than the text and then walks the whole stack,
            # which not even the C program does reproducibly.
            nul_i = nul_r = True
            if len(ip) < len(rp):
                continue
        ib = list(ip) + ([0] if nul_i else [])
        rb = list(rp) + ([0] if nul_r else [])
        text = (
            f"{op} {flags} {len(ib)} "
            + " ".join(map(str, ib))
            + f" {len(rb)} "
            + " ".join(map(str, rb))
            + "\n"
        )
        if text not in corpus:
            corpus.append(text)
    return corpus


CORPUS = build_corpus()


# ------------------------------------------------------------------- verification
def run(exe, text):
    r = subprocess.run([exe], input=text.encode(), capture_output=True)
    return (r.stdout, r.stderr, r.returncode)


def rust_bytes(vals):
    """Render a byte list as a compact Rust expression for `s(...)`."""
    n = len(vals)
    if n == 0:
        return "&[]"
    printable = all(32 <= v < 127 for v in vals)
    if printable:
        return "b\"" + "".join(chr(v).replace("\\", "\\\\").replace('"', '\\"') for v in vals) + "\""
    if vals[-1] == 0 and all(32 <= v < 127 for v in vals[:-1]):
        return "&nul(b\"" + "".join(chr(v) for v in vals[:-1]) + "\")"
    if len(set(vals)) == 1:
        return f"&[{vals[0]}u8; {n}]"
    if vals[-1] == 0 and len(set(vals[:-1])) == 1:
        return f"&nul(&[{vals[0]}u8; {n - 1}])"
    return "&[" + ", ".join(str(v) for v in vals) + "]"


HEADER = '''//! Differential tests: the translated Rust program is executed as a *binary*
//! next to the original C program and both are compared byte for byte.
//!
//! Every test feeds identical bytes to both processes on stdin and asserts that
//! stdout, stderr and the exit status (including a fatal signal) are the same.
//! The Rust code is never linked in as a library: `driver` is driven exactly the
//! way a shell drives it.
//!
//! The inputs enumerate the branches of `c_src/src/main.c` and
//! `c_src/src/lib.c`: the eight `scanf`/length error paths, all five operations
//! plus the `default` case, every `return` inside the five static helpers, the
//! empty buffer, the single byte buffer, the 1024 byte maximum, and the
//! `size_t` underflow in `match_pattern()` that kills the process with SIGSEGV.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Path of the compiled C reference program, built on demand with CMake.
fn c_program() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate directory has a parent")
            .join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.is_file() {
            let configure = Command::new("cmake")
                .arg("-S")
                .arg(&c_src)
                .arg("-B")
                .arg(&build)
                .output()
                .expect("failed to run `cmake` - is it installed?");
            assert!(
                configure.status.success(),
                "cmake configure failed:\\n{}",
                String::from_utf8_lossy(&configure.stderr)
            );
            let compile = Command::new("cmake")
                .arg("--build")
                .arg(&build)
                .output()
                .expect("failed to run `cmake --build`");
            assert!(
                compile.status.success(),
                "cmake build failed:\\n{}",
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(exe.is_file(), "the C program was not built at {:?}", exe);
        exe
    })
}

/// Path of the translated Rust program (built by cargo for this test).
fn rust_program() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

#[derive(PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit code {:?}, signal {:?}, stdout {:?}, stderr {:?}",
            self.code,
            self.signal,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn run(program: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {:?}: {e}", program));
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input)
        .or_else(|e| {
            // A process that dies before reading everything closes the pipe.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                Ok(())
            } else {
                Err(e)
            }
        })
        .expect("writing to the child's stdin");
    let out = child.wait_with_output().expect("waiting for the child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Feed `input` to both programs and require identical observable behaviour.
#[track_caller]
fn same(input: &str) {
    let bytes = input.as_bytes();
    let c = run(c_program(), bytes);
    let rust = run(rust_program(), bytes);
    assert_eq!(
        c.stdout,
        rust.stdout,
        "stdout differs for input {:?}\\n  C:    {:?}\\n  Rust: {:?}",
        input,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "stderr differs for input {:?}\\n  C:    {:?}\\n  Rust: {:?}",
        input,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(
        (c.code, c.signal),
        (rust.code, rust.signal),
        "exit status differs for input {:?}: C {:?} vs Rust {:?}",
        input,
        (c.code, c.signal),
        (rust.code, rust.signal)
    );
}

/// `payload` with a NUL terminator appended.
fn nul(payload: &[u8]) -> Vec<u8> {
    let mut v = payload.to_vec();
    v.push(0);
    v
}

/// Render the stdin text `main()` expects: operation, flags, then each buffer
/// as a length followed by that many decimal byte values.
fn encode(operation: &str, flags: &str, input: &[u8], reference: &[u8]) -> String {
    let mut s = format!("{operation} {flags} {}", input.len());
    for byte in input {
        s.push_str(&format!(" {byte}"));
    }
    s.push_str(&format!(" {}", reference.len()));
    for byte in reference {
        s.push_str(&format!(" {byte}"));
    }
    s.push('\\n');
    s
}
'''


def emit():
    parts = [HEADER]
    for name, comment, spec in CASES:
        parts.append("")
        if comment:
            parts.append(f"/// {comment}")
        parts.append("#[test]")
        parts.append(f"fn {name}() {{")
        if spec[0] == "raw":
            lit = spec[1].replace("\\", "\\\\").replace('"', '\\"').replace("\t", "\\t").replace("\r", "\\r").replace("\n", "\\n")
            parts.append(f'    same("{lit}");')
        else:
            _, op, flags, ib, rb = spec
            parts.append(
                f'    same(&encode("{op}", "{flags}", {rust_bytes(ib)}, {rust_bytes(rb)}));'
            )
        parts.append("}")
    parts.append("")
    parts.append("/// A fixed pseudo random cross check over the whole input space:")
    parts.append("/// every operation, every flag combination and terminated as well as")
    parts.append("/// unterminated payloads of every interesting shape.")
    parts.append("#[test]")
    parts.append("fn randomized_corpus() {")
    parts.append("    for input in CORPUS {")
    parts.append("        same(input);")
    parts.append("    }")
    parts.append("}")
    parts.append("")
    parts.append(f"const CORPUS: [&str; {len(CORPUS)}] = [")
    for text in CORPUS:
        lit = text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
        parts.append(f'    "{lit}",')
    parts.append("];")
    return "\n".join(parts) + "\n"


REPS = 30
CORPUS_REPS = 8

if __name__ == "__main__":
    names = set()
    bad, unstable = [], []
    for name, _c, spec in CASES:
        assert name not in names, "duplicate case name " + name
        names.add(name)
        text = text_of(spec)
        outs = {run(C, text) for _ in range(REPS)}
        if len(outs) != 1:
            unstable.append((name, outs))
            continue
        c = outs.pop()
        r = run(R, text)
        if c != r:
            bad.append((name, text, c, r))
    for i, text in enumerate(CORPUS):
        outs = {run(C, text) for _ in range(CORPUS_REPS)}
        if len(outs) != 1:
            unstable.append((f"corpus[{i}] {text!r}", outs))
            continue
        c = outs.pop()
        r = run(R, text)
        if c != r:
            bad.append((f"corpus[{i}]", text, c, r))
    print(f"cases={len(CASES)} corpus={len(CORPUS)} mismatch={len(bad)} unstable={len(unstable)}")
    for n, t, c, r in bad:
        print("  MISMATCH", n, repr(t[:100]), "C", c, "R", r)
    for n, o in unstable:
        print("  UNSTABLE", n, o)
    if not bad and not unstable:
        open(W + "/translation/tests/differential.rs", "w").write(emit())
        print("wrote translation/tests/differential.rs")
