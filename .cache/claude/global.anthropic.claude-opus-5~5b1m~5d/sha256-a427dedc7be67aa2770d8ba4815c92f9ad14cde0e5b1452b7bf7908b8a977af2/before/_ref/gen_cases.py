#!/usr/bin/env python3
"""Generate differential-test inputs and data files."""
import os
import shutil

base = os.environ.get(
    "REFBASE",
    os.path.join("$HARVEST_WORKDIR", "_ref"),
)
cases = os.path.join(base, "cases")
data = os.path.join(base, "data")
shutil.rmtree(cases, ignore_errors=True)
shutil.rmtree(data, ignore_errors=True)
os.makedirs(cases)
os.makedirs(data)


def case(name, content):
    with open(os.path.join(cases, name), "wb") as f:
        f.write(content if isinstance(content, bytes) else content.encode())


def datafile(name, content):
    p = os.path.join(data, name)
    with open(p, "wb") as f:
        f.write(content if isinstance(content, bytes) else content.encode())
    return p


# ---------------------------------------------------------------- data files
f_small = datafile("small.c", "int main(void) { return 0; } // done\n")
f_code = datafile(
    "code.c",
    "/* multi\n * line comment\n */\nint x = 1.5.5; char *s = \"a\\\"b\";\n"
    "if (a >= b && c || d) { x <<= 2; y-->z; }\n\tunsigned long q = 0x1f;\n",
)
f_empty = datafile("empty.txt", "")
f_8192 = datafile("exact8192.txt", "a" * 8192)
f_8191 = datafile("exact8191.txt", "b" * 8191)
f_8193 = datafile("big8193.txt", "c" * 8193)
f_nul = datafile("withnul.bin", b"abc\x00def ghi\n")
f_hi = datafile("highbytes.bin", bytes(range(128, 256)) + b"\n")
f_dir = os.path.join(data, "adir")
os.makedirs(f_dir)
datafile("adir/inner.txt", "hello\n")

# ---------------------------------------------------------------- cases
case("01_basic", "1\nint x = 42; // hi\nfloat y = 3.14;\n\n3\n4\n5\nx\n6\nif (a==b) { c++; }\n\n7\n")
case("02_empty_stdin", "")
case("03_only_newline", "\n")
case("04_invalid_input", "abc\n\nxyz\n7\n")
case("05_invalid_choice", "0\n8\n-1\n99\n7\n")
case("06_eof_after_2", "2\n")
case("07_eof_after_5", "5\n")
case("08_eof_in_case1", "1\nsome text here\nmore text")
case("09_eof_after_1", "1\n")
case("10_file_ok", "2\n%s\n3\n4\n7\n" % f_small)
case("11_file_missing", "2\n/nonexistent/path/xyz\n7\n")
case("12_file_empty", "2\n%s\n3\n7\n" % f_empty)
case("13_file_8192", "2\n%s\n3\n7\n" % f_8192)
case("14_file_8191", "2\n%s\n4\n7\n" % f_8191)
case("15_file_8193", "2\n%s\n7\n" % f_8193)
case("16_file_nul", "2\n%s\n3\n7\n" % f_nul)
case("17_file_highbytes", "2\n%s\n3\n7\n" % f_hi)
case("18_file_dir", "2\n%s\n3\n7\n" % f_dir)
case("19_file_code", "2\n%s\n3\n4\n5\n\n7\n" % f_code)
case("20_pattern_empty", "1\nfoo bar foo\n\n5\n\n7\n")
case("21_pattern_multi", "1\nalpha beta alphabet alpha\n\n5\nalpha\n5\nzzz\n7\n")
case("22_no_analysis_yet", "3\n4\n5\nx\n7\n")

# long single line (fgets splits at 255)
long_line = "w" * 300 + "\n"
case("23_long_line", "1\n" + long_line + "\n3\n7\n")

# identifier longer than MAX_TOKEN_LENGTH
case("24_long_token", "6\n" + "i" * 400 + "\n\n7\n")

# text longer than MAX_INPUT_SIZE (4096)
big_text = "".join("word%d " % i for i in range(1200)) + "\n"
case("25_big_text", "1\n" + big_text + "\n3\n4\n7\n")

# more than 100 distinct words
many_words = " ".join("w%d" % i for i in range(150)) + "\n"
case("26_many_words", "1\n" + many_words + "\n3\n7\n")

# repeated words with ties for the bubble sort
case(
    "27_word_ties",
    "1\na b c a b c a d e\n\n3\n1\na a b\n\n3\n7\n",
)

# more than 100 tokens in interactive tokenizer
toks = " ".join("t%d + " % i for i in range(80)) + "\n"
case("28_interactive_trunc", "6\n" + toks + "\n7\n")

# multi-line comment spanning newlines -> negative columns
case("29_multiline_comment", "6\n/* abc\ndef */ x\n\n7\n")

# unterminated string / char literals, escapes
case("30_strings", "6\n\"abc\\\"def\" 'x' \"unterminated\n\n7\n")

# operators and punctuation coverage
case("31_operators", "6\n+ - * / % = < > ! & | ^ ~ ? : == != <= >= && || ++ -- -> << >>\n\n7\n")
case("32_punct", "6\n( ) { } [ ] ; , . @ # $ ` \\ \n\n7\n")

# numbers
case("33_numbers", "6\n1 12.34 1.2.3 .5 0. 007 9999999999999999999\n\n7\n")

# CRLF input
case("34_crlf", "1\r\nint x;\r\n\r\n3\r\n7\r\n")

# NUL bytes in stdin
case("35_nul_stdin", b"1\nab\x00cd\nef\n\n3\n7\n")

# choice parsing oddities
case("36_choice_parsing", "  3\n+4\n3abc\n7xyz\n")
case("37_choice_overflow", "99999999999999999999\n-99999999999999999999\n7\n")
case("38_choice_leading_ws", "\t\t 4 \n7\n")

# whitespace-only text for case 1
case("39_ws_text", "1\n   \t  \n\n3\n7\n")

# repeat analysis several times (cumulative statistics)
case("40_cumulative", "1\nint a;\n\n1\nfloat b;\n\n1\nchar c;\n\n3\n4\n7\n")

# high bytes typed interactively
case("41_high_bytes_stdin", b"6\n\xc3\xa9\xe2\x82\xac abc\n\n7\n")

# exactly 100 tokens then 101 tokens
case("42_exactly_101", "6\n" + " ".join(["a"] * 101) + "\n\n7\n")
case("43_exactly_100", "6\n" + " ".join(["a"] * 100) + "\n\n7\n")

# no trailing newline on last choice
case("44_no_trailing_nl", "3\n4\n7")

# choice 6 with immediate empty line
case("45_interactive_empty", "6\n\n7\n")

# choice 1 with immediate empty line
case("46_analyze_empty", "1\n\n3\n7\n")

# string token that hits the 256 byte boundary exactly
s = '"' + "x" * 252 + '"' + "\n"
case("47_string_boundary", "6\n" + s + "\n7\n")
s2 = '"' + "y" * 253 + '"' + "\n"
case("48_string_boundary2", "6\n" + s2 + "\n7\n")
s3 = '"' + "z" * 254 + '"' + "\n"
case("49_string_boundary3", "6\n" + s3 + "\n7\n")
s4 = '"' + ("a" * 251) + "\\\"" + '"' + "\n"
case("50_string_escape_boundary", "6\n" + s4 + "\n7\n")

# comment boundary cases
case("51_comment_boundary", "6\n/*" + "c" * 260 + "*/\n\n7\n")
case("52_line_comment_boundary", "6\n//" + "d" * 300 + "\n\n7\n")
case("53_lone_slash", "6\na / b\n\n7\n")

# pattern search with special chars
case("54_pattern_special", "1\nx=y;\n\n5\n=\n5\n;\n7\n")

# very many menu iterations
case("55_menu_spam", "".join("3\n4\n" for _ in range(5)) + "7\n")

# filename with trailing spaces / empty filename
case("56_empty_filename", "2\n\n7\n")
case("57_filename_ws", "2\n   \n7\n")

# interactive tokenizer where load fails (text >= 4096 is impossible: capped)
case("58_interactive_big", "6\n" + big_text + "\n7\n")

# keywords all
kw = ("if else while for return int char float double void struct typedef const "
      "static extern auto register sizeof break continue switch case default do "
      "goto enum union signed unsigned long short\n")
case("59_keywords", "1\n" + kw + "\n3\n4\n7\n")

# mixed everything, several rounds
case(
    "60_mixed",
    "1\nint main() { /* c */ return \"s\"; }\n\n3\n4\n5\nmain\n6\nx+y\n\n2\n"
    + f_code
    + "\n3\n4\n5\n\n7\n",
)
print("cases:", len(os.listdir(cases)))
