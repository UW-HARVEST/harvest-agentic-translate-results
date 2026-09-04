#!/usr/bin/env python3
"""Second batch of differential-test inputs (boundary conditions)."""
import os

base = "$HARVEST_WORKDIR/_ref"
cases = os.path.join(base, "cases")
data = os.path.join(base, "data")
os.makedirs(cases, exist_ok=True)
os.makedirs(data, exist_ok=True)


def case(name, content):
    with open(os.path.join(cases, name), "wb") as f:
        f.write(content if isinstance(content, bytes) else content.encode())


def datafile(name, content):
    p = os.path.join(data, name)
    with open(p, "wb") as f:
        f.write(content if isinstance(content, bytes) else content.encode())
    return p


# string whose scanned length reaches exactly 256 (1 byte past the C buffer)
s256 = '"' + "a" * 252 + "\\q" + '"' + "\n"
case("61_string_len256", "6\n" + s256 + "\n7\n")
# and 255 / 254
case("62_string_len255", "6\n" + '"' + "b" * 253 + '"' + "\n" + "\n7\n")
case("63_string_len254", "6\n" + '"' + "c" * 252 + '"' + "\n" + "\n7\n")
# escape at the very end of the buffer
case("64_string_escape_eof", "6\n\"abc\\", )
case("65_string_backslash_nl", "6\n\"abc\\\n\n7\n")

# identifier of exactly 255 / 256 characters
case("66_ident255", "6\n" + "i" * 255 + "\n\n7\n")
case("67_ident256", "6\n" + "j" * 256 + "\n\n7\n")

# line comment hitting exactly 255
case("68_linecomment255", "6\n//" + "e" * 253 + "\n\n7\n")
case("69_linecomment256", "6\n//" + "f" * 254 + "\n\n7\n")

# multi line comment hitting the 254/255 boundary
case("70_mlcomment", "6\n/*" + "g" * 251 + "*/x\n\n7\n")
case("71_mlcomment2", "6\n/*" + "h" * 252 + "*/x\n\n7\n")
case("72_mlcomment_unterminated", "6\n/* not closed\nstill\n\n7\n")
case("73_mlcomment_star", "6\n/*a*b*/x\n\n7\n")

# choice line longer than the 256 byte fgets buffer: the remainder is parsed
# as the next choice
case("74_choice_split", "3" + "z" * 300 + "\n7\n")
case("75_choice_split_num", "3" + " " * 254 + "4\n7\n")

# filename longer than the fgets buffer
case("76_filename_split", "2\n" + "/tmp/" + "n" * 300 + "\n7\n")

# a line of exactly 255 characters plus newline in case 1 (fgets splits so the
# continuation line starts with '\n' and stops the loop)
case("77_line255", "1\n" + "k" * 255 + "\n\n3\n7\n")
case("78_line254", "1\n" + "l" * 254 + "\n\n3\n7\n")
case("79_line256", "1\n" + "m" * 256 + "\n\n3\n7\n")

# \v and \f whitespace
case("80_vf_ws", b"6\na\x0bb\x0cc\td\re\n\n7\n")

# analyze, then a failing load, then pattern search over the stale buffer
f8192 = os.path.join(data, "exact8192.txt")
case("81_stale_buffer", "1\nint zz = 5;\n\n2\n" + f8192 + "\n5\nzz\n3\n4\n7\n")

# pattern with high bytes and long pattern
case("82_pattern_high", b"1\nabc\n\n5\n\xc3\xa9\n5\n" + b"q" * 200 + b"\n7\n")

# interactive tokenizer with immediate EOF
case("83_interactive_eof", "6\n")
case("84_interactive_eof2", "6")

# empty text then distribution then complexity
case("85_empty_then_stats", "1\n\n3\n4\n1\n\n3\n4\n7\n")

# 100 distinct words exactly, then more
case("86_words100", "1\n" + " ".join("q%d" % i for i in range(100)) + "\n\n3\n7\n")
case("87_words101", "1\n" + " ".join("r%d" % i for i in range(101)) + "\n\n3\n7\n")

# words with counts to exercise the bubble sort ordering
freq = []
for i in range(12):
    freq.extend(["w%d" % i] * (i + 1))
case("88_word_freq", "1\n" + " ".join(freq) + "\n\n3\n3\n7\n")

# repeated distribution printing (sorting is destructive/persistent)
case("89_repeat_dist", "1\na b b c c c\n\n3\n3\n3\n7\n")

# file with only newlines
fnl = datafile("newlines.txt", "\n" * 50)
case("90_file_newlines", "2\n" + fnl + "\n3\n4\n7\n")

# file with mixed binary
fbin = datafile("bin.dat", bytes(range(1, 128)) * 3)
case("91_file_bin", "2\n" + fbin + "\n3\n4\n7\n")

# file exactly 4096 and 8000 bytes
f4096 = datafile("f4096.txt", "int a; " * 585 + "x")
case("92_file_4096", "2\n" + f4096 + "\n3\n7\n")
f8000 = datafile("f8000.txt", ("abc def " * 1000)[:8000])
case("93_file_8000", "2\n" + f8000 + "\n3\n4\n7\n")

# file that starts with NUL
fnul0 = datafile("nulfirst.bin", b"\x00abc def\n")
case("94_file_nul_first", "2\n" + fnul0 + "\n3\n7\n")

# choice 0 padded, negative, float-ish
case("95_choice_float", "1.9\n7\n")
case("96_choice_minus0", "-0\n7\n")
case("97_choice_plus7", "+7\n")
case("98_choice_space7", " 7 \n")

# many rounds of everything
seq = ""
for i in range(3):
    seq += "1\nint v%d = %d; /* c%d */\n\n3\n4\n5\nv\n6\nv%d++\n\n" % (i, i, i, i)
seq += "7\n"
case("99_many_rounds", seq)

# unknown/error tokens
case("A0_error_tokens", b"6\n@ # $ \x01 \x7f \xff\n\n7\n")

# text ending without newline in interactive mode
case("A1_no_nl_interactive", "6\nabc")

# a quote as the last char
case("A2_quote_last", "6\n\"\n\n7\n")
case("A3_single_quote", "6\n'a' 'bc' '\n\n7\n")

print("total cases:", len(os.listdir(cases)))
