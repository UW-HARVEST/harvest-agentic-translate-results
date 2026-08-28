/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Differential tests: the C program (`c_src/build/driver`) and the Rust
//! program (`translation/target/*/driver`) are both executed as subprocesses
//! on identical stdin, and their stdout, stderr and exit status are compared
//! byte for byte.
//!
//! The `expect` columns below are the values the *C* program actually produces
//! (C is the ground truth), so each case both pins the behaviour down and
//! checks that Rust reproduces it.

mod common;

use common::{check, check_expect};

/// `main`'s three `fgets` calls and the newline-stripping that follows them.
/// `fgets` stops at a newline, so a short stdin makes one of the three reads
/// return NULL and the program writes to stderr and exits 1.
#[test]
fn main_read_paths() {
    check_expect(
        "empty_stdin",
        b"",
        "",
        "Error reading operation\n",
        1,
    );
    check_expect(
        "only_operation_no_newline",
        b"0",
        "",
        "Error reading parameter\n",
        1,
    );
    check_expect(
        "only_operation_with_newline",
        b"0\n",
        "",
        "Error reading parameter\n",
        1,
    );
    check_expect(
        "op_and_param_no_newline",
        b"0\n0",
        "",
        "Error reading decision string\n",
        1,
    );
    check_expect(
        "op_and_param_with_newline",
        b"0\n0\n",
        "",
        "Error reading decision string\n",
        1,
    );
    check_expect(
        "single_newline",
        b"\n",
        "",
        "Error reading parameter\n",
        1,
    );
    check_expect(
        "two_newlines",
        b"\n\n",
        "",
        "Error reading decision string\n",
        1,
    );
    check_expect(
        "three_newlines",
        b"\n\n\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "blank_decision_line_op0",
        b"0\n0\n\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "blank_decision_line_op1",
        b"1\n0\n\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "blank_decision_line_op2",
        b"2\n0\n\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "blank_decision_line_op3",
        b"3\n0\n\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "blank_decision_line_bad_op",
        b"99\n0\n\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "decision_no_trailing_newline_op0",
        b"0\n0\nyyy",
        "107\n",
        "",
        0,
    );
    check_expect(
        "decision_no_trailing_newline_op2",
        b"2\n0\nyyy",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "decision_no_trailing_newline_op3",
        b"3\n0\nyn",
        "2\n",
        "",
        0,
    );
    check_expect(
        "extra_trailing_lines_ignored",
        b"2\n0\nyyy\nnnn\nmore\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "crlf_all_three_lines_op2",
        b"2\r\n0\r\nyyy\r\n",
        "203\n",
        "",
        0,
    );
    check_expect(
        "crlf_all_three_lines_op3",
        b"3\r\n0\r\nyyy\r\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "crlf_op0",
        b"0\r\n0\r\nyyy\r\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "cr_only_decision_line",
        b"3\n0\nyn\r\n",
        "11\n",
        "",
        0,
    );
}

/// `fgets` stores embedded NUL bytes, but `strlen`/`atoi` stop at the first
/// one, so a NUL truncates whatever line it appears in.
#[test]
fn embedded_nul() {
    check_expect(
        "nul_in_operation_line",
        b"2\x000\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "nul_in_param_line",
        b"1\n0\x003\nyyy\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "nul_truncates_decisions_to_two",
        b"0\n0\nyy\x00nn\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "nul_truncates_decisions_op2",
        b"2\n0\nyy\x00nn\n",
        "1002\n",
        "",
        0,
    );
    check_expect(
        "nul_truncates_decisions_op3",
        b"3\n0\ny\x00nn\n",
        "1\n",
        "",
        0,
    );
    check_expect(
        "nul_first_byte_of_decisions",
        b"2\n0\n\x00yyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "nul_only_decision_line",
        b"3\n0\n\x00\n",
        "-1\n",
        "",
        0,
    );
}

/// `atoi` == `(int) strtol(s, NULL, 10)`: leading whitespace and an optional
/// sign are accepted, trailing junk is ignored, out-of-range values saturate
/// at `LONG_MIN`/`LONG_MAX` and are then truncated to `int`.
#[test]
fn atoi() {
    check_expect(
        "atoi_op_leading_spaces",
        b"  2  \n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_leading_tab",
        b"\t2\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_plus_sign",
        b"+2\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_trailing_junk",
        b"2abc\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_all_junk",
        b"abc\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_empty_via_space",
        b" \n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_hex_like",
        b"0x2\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_float_like",
        b"2.9\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_leading_zeros",
        b"0002\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_minus_zero",
        b"-0\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_double_minus",
        b"--2\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_underscore",
        b"1_0\n0\nyyy\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_int_max",
        b"2147483647\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_int_max_plus_1",
        b"2147483648\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_int_min",
        b"-2147483648\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_int_min_minus_1",
        b"-2147483649\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_wraps_to_zero",
        b"4294967296\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_wraps_to_two",
        b"4294967298\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_long_max",
        b"9223372036854775807\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_long_max_plus_1",
        b"9223372036854775808\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_long_min",
        b"-9223372036854775808\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_long_min_minus_1",
        b"-9223372036854775809\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_way_too_big",
        b"99999999999999999999999999\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "atoi_op_way_too_negative",
        b"-99999999999999999999999999\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_int_max",
        b"1\n2147483647\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_wraps_to_zero",
        b"1\n4294967296\nyyy\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_wraps_to_one",
        b"1\n4294967297\nyyy\n",
        "103\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_long_max_plus_1",
        b"1\n9223372036854775808\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_long_min",
        b"1\n-9223372036854775808\nyyy\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_junk",
        b"1\nabc\nyyy\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_negative",
        b"1\n-1\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "atoi_param_spaces",
        b"1\n  3 \nyyy\n",
        "0\n",
        "",
        0,
    );
}

/// `process_decisions` returns -3 for any `operation` outside 0..=3.
#[test]
fn bad_operation() {
    check_expect(
        "bad_operation_4",
        b"4\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_5",
        b"5\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_6",
        b"6\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_neg1",
        b"-1\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_neg2",
        b"-2\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_neg3",
        b"-3\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_100",
        b"100\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_2147483647",
        b"2147483647\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
    check_expect(
        "bad_operation_neg2147483648",
        b"-2147483648\n0\nyyy\n",
        "-3\n",
        "",
        0,
    );
}

/// operation 0 -> `apply_permissions`: -2 when fewer than 3 decisions,
/// otherwise the full 8-way read/write/execute decision tree.
#[test]
fn op0_permissions() {
    check_expect(
        "perm_too_short_y",
        b"0\n0\ny\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "perm_too_short_yy",
        b"0\n0\nyy\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "perm_too_short_n",
        b"0\n0\nn\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "perm_too_short_nn",
        b"0\n0\nnn\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "perm_yyy",
        b"0\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_yyn",
        b"0\n0\nyyn\n",
        "56\n",
        "",
        0,
    );
    check_expect(
        "perm_yny",
        b"0\n0\nyny\n",
        "35\n",
        "",
        0,
    );
    check_expect(
        "perm_nyy",
        b"0\n0\nnyy\n",
        "23\n",
        "",
        0,
    );
    check_expect(
        "perm_ynn",
        b"0\n0\nynn\n",
        "14\n",
        "",
        0,
    );
    check_expect(
        "perm_nyn",
        b"0\n0\nnyn\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "perm_nny",
        b"0\n0\nnny\n",
        "-20\n",
        "",
        0,
    );
    check_expect(
        "perm_nnn",
        b"0\n0\nnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "perm_yyy_2",
        b"0\n0\nYYY\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_yyn_2",
        b"0\n0\nYYN\n",
        "56\n",
        "",
        0,
    );
    check_expect(
        "perm_yny_2",
        b"0\n0\nYNY\n",
        "35\n",
        "",
        0,
    );
    check_expect(
        "perm_nyy_2",
        b"0\n0\nNYY\n",
        "23\n",
        "",
        0,
    );
    check_expect(
        "perm_ynn_2",
        b"0\n0\nYNN\n",
        "14\n",
        "",
        0,
    );
    check_expect(
        "perm_nyn_2",
        b"0\n0\nNYN\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "perm_nny_2",
        b"0\n0\nNNY\n",
        "-20\n",
        "",
        0,
    );
    check_expect(
        "perm_nnn_2",
        b"0\n0\nNNN\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "perm_yyn_3",
        b"0\n0\nyYn\n",
        "56\n",
        "",
        0,
    );
    check_expect(
        "perm_qqq",
        b"0\n0\nqqq\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "perm_yqq",
        b"0\n0\nyqq\n",
        "14\n",
        "",
        0,
    );
    check_expect(
        "perm_qyq",
        b"0\n0\nqyq\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "perm_qqy",
        b"0\n0\nqqy\n",
        "-20\n",
        "",
        0,
    );
    check_expect(
        "perm_y_sp_n",
        b"0\n0\ny n\n",
        "14\n",
        "",
        0,
    );
    check_expect(
        "perm__sp__sp_y",
        b"0\n0\n  y\n",
        "-20\n",
        "",
        0,
    );
    check_expect(
        "perm__sp_yy",
        b"0\n0\n yy\n",
        "23\n",
        "",
        0,
    );
    check_expect(
        "perm_yyyy",
        b"0\n0\nyyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_yynnn",
        b"0\n0\nyynnn\n",
        "56\n",
        "",
        0,
    );
    check_expect(
        "perm_nnnyyy",
        b"0\n0\nnnnyyy\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "perm_yyyx",
        b"0\n0\nyyy_extra_ignored\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_param_ignored_0",
        b"0\n0\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_param_ignored_1",
        b"0\n1\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_param_ignored_2",
        b"0\n2\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_param_ignored_3",
        b"0\n3\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_param_ignored_4",
        b"0\n4\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_param_ignored_neg1",
        b"0\n-1\nyyy\n",
        "107\n",
        "",
        0,
    );
    check_expect(
        "perm_param_ignored_99",
        b"0\n99\nyyy\n",
        "107\n",
        "",
        0,
    );
}

/// operation 1 -> `evaluate_conditions`: -2 when fewer than 3 decisions,
/// then AND/OR/XOR/NAND selected by `param` (anything else gives -1).
#[test]
fn op1_conditions() {
    check_expect(
        "cond_too_short_y",
        b"1\n0\ny\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "cond_too_short_n",
        b"1\n0\nn\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "cond_too_short_yy",
        b"1\n0\nyy\n",
        "-2\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_yyy",
        b"1\n0\nyyy\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_yyn",
        b"1\n0\nyyn\n",
        "50\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_yny",
        b"1\n0\nyny\n",
        "51\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_nyy",
        b"1\n0\nnyy\n",
        "52\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_ynn",
        b"1\n0\nynn\n",
        "10\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_nyn",
        b"1\n0\nnyn\n",
        "11\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_nny",
        b"1\n0\nnny\n",
        "12\n",
        "",
        0,
    );
    check_expect(
        "cond_p0_nnn",
        b"1\n0\nnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_yyy",
        b"1\n1\nyyy\n",
        "103\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_yyn",
        b"1\n1\nyyn\n",
        "102\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_yny",
        b"1\n1\nyny\n",
        "102\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_nyy",
        b"1\n1\nnyy\n",
        "102\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_ynn",
        b"1\n1\nynn\n",
        "101\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_nyn",
        b"1\n1\nnyn\n",
        "101\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_nny",
        b"1\n1\nnny\n",
        "101\n",
        "",
        0,
    );
    check_expect(
        "cond_p1_nnn",
        b"1\n1\nnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_yyy",
        b"1\n2\nyyy\n",
        "7\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_yyn",
        b"1\n2\nyyn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_yny",
        b"1\n2\nyny\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_nyy",
        b"1\n2\nnyy\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_ynn",
        b"1\n2\nynn\n",
        "1\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_nyn",
        b"1\n2\nnyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_nny",
        b"1\n2\nnny\n",
        "3\n",
        "",
        0,
    );
    check_expect(
        "cond_p2_nnn",
        b"1\n2\nnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_yyy",
        b"1\n3\nyyy\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_yyn",
        b"1\n3\nyyn\n",
        "152\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_yny",
        b"1\n3\nyny\n",
        "151\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_nyy",
        b"1\n3\nnyy\n",
        "150\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_ynn",
        b"1\n3\nynn\n",
        "151\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_nyn",
        b"1\n3\nnyn\n",
        "150\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_nny",
        b"1\n3\nnny\n",
        "150\n",
        "",
        0,
    );
    check_expect(
        "cond_p3_nnn",
        b"1\n3\nnnn\n",
        "200\n",
        "",
        0,
    );
    check_expect(
        "cond_bad_logic_4",
        b"1\n4\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "cond_bad_logic_5",
        b"1\n5\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "cond_bad_logic_neg1",
        b"1\n-1\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "cond_bad_logic_neg2",
        b"1\n-2\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "cond_bad_logic_100",
        b"1\n100\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "cond_bad_logic_2147483647",
        b"1\n2147483647\nyyy\n",
        "-1\n",
        "",
        0,
    );
    check_expect(
        "cond_case_yyy",
        b"1\n0\nYYY\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "cond_case_nnn",
        b"1\n0\nNNN\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_case_qqq",
        b"1\n0\nqqq\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "cond_case_yqn",
        b"1\n0\nyqn\n",
        "10\n",
        "",
        0,
    );
    check_expect(
        "cond_case_qyy",
        b"1\n0\nQyy\n",
        "52\n",
        "",
        0,
    );
}

/// operation 2 -> `configure_flags`, which clamps the decision count at 32
/// and tests all-false / all-true / exactly-one-true / exactly-one-false /
/// alternating / >=3 consecutive / plain count rules in that order.
#[test]
fn op2_flags() {
    check_expect(
        "flags_len1_y",
        b"2\n0\ny\n",
        "1001\n",
        "",
        0,
    );
    check_expect(
        "flags_len1_n",
        b"2\n0\nn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len2_yy",
        b"2\n0\nyy\n",
        "1002\n",
        "",
        0,
    );
    check_expect(
        "flags_len2_nn",
        b"2\n0\nnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len2_yn",
        b"2\n0\nyn\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "flags_len2_ny",
        b"2\n0\nny\n",
        "101\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_yyy",
        b"2\n0\nyyy\n",
        "1003\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_nnn",
        b"2\n0\nnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_ynn",
        b"2\n0\nynn\n",
        "100\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_nyn",
        b"2\n0\nnyn\n",
        "101\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_nny",
        b"2\n0\nnny\n",
        "102\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_yny",
        b"2\n0\nyny\n",
        "201\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_yyn",
        b"2\n0\nyyn\n",
        "202\n",
        "",
        0,
    );
    check_expect(
        "flags_len3_nyy",
        b"2\n0\nnyy\n",
        "200\n",
        "",
        0,
    );
    check_expect(
        "flags_len4_yyyy",
        b"2\n0\nyyyy\n",
        "1004\n",
        "",
        0,
    );
    check_expect(
        "flags_len4_nnnn",
        b"2\n0\nnnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len4_ynyn",
        b"2\n0\nynyn\n",
        "502\n",
        "",
        0,
    );
    check_expect(
        "flags_len4_nyny",
        b"2\n0\nnyny\n",
        "502\n",
        "",
        0,
    );
    check_expect(
        "flags_len4_yynn",
        b"2\n0\nyynn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "flags_len4_nnyy",
        b"2\n0\nnnyy\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "flags_len5_yynyy",
        b"2\n0\nyynyy\n",
        "202\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_ynynyn",
        b"2\n0\nynynyn\n",
        "503\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_nynyny",
        b"2\n0\nnynyny\n",
        "503\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_yyynnn",
        b"2\n0\nyyynnn\n",
        "303\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_nnnyyy",
        b"2\n0\nnnnyyy\n",
        "303\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_yynnyy",
        b"2\n0\nyynnyy\n",
        "4\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_nnyynn",
        b"2\n0\nnnyynn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "flags_len8_ynnyynny",
        b"2\n0\nynnyynny\n",
        "4\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_yyyynn",
        b"2\n0\nyyyynn\n",
        "304\n",
        "",
        0,
    );
    check_expect(
        "flags_len6_nnyyyy",
        b"2\n0\nnnyyyy\n",
        "304\n",
        "",
        0,
    );
    check_expect(
        "flags_len8_yynnynny",
        b"2\n0\nyynnynny\n",
        "4\n",
        "",
        0,
    );
    check_expect(
        "flags_len31_yyyyyyyyyyyy",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n",
        "1031\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_yyyyyyyyyyyy",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n",
        "1032\n",
        "",
        0,
    );
    check_expect(
        "flags_len33_yyyyyyyyyyyy",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n",
        "1032\n",
        "",
        0,
    );
    check_expect(
        "flags_len40_yyyyyyyyyyyy",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n",
        "1032\n",
        "",
        0,
    );
    check_expect(
        "flags_len31_nnnnnnnnnnnn",
        b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_nnnnnnnnnnnn",
        b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len33_nnnnnnnnnnnn",
        b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len40_nnnnnnnnnnnn",
        b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_nnnnnnnnnnnn_2",
        b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnny\n",
        "131\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_yyyyyyyyyyyy_2",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyn\n",
        "231\n",
        "",
        0,
    );
    check_expect(
        "flags_len33_nnnnnnnnnnnn_2",
        b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnny\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len33_yyyyyyyyyyyy_2",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyn\n",
        "1032\n",
        "",
        0,
    );
    check_expect(
        "flags_len34_yyyyyyyyyyyy",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyn\n",
        "1032\n",
        "",
        0,
    );
    check_expect(
        "flags_len34_nnnnnnnnnnnn",
        b"2\n0\nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnny\n",
        "0\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_ynynynynynyn",
        b"2\n0\nynynynynynynynynynynynynynynynyn\n",
        "516\n",
        "",
        0,
    );
    check_expect(
        "flags_len34_ynynynynynyn",
        b"2\n0\nynynynynynynynynynynynynynynynynyn\n",
        "516\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_nynynynynyny",
        b"2\n0\nnynynynynynynynynynynynynynynyny\n",
        "516\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_yyyyyyyyyyyy_3",
        b"2\n0\nyyyyyyyyyyyyyyyyyyyyyyyyyyyyyynn\n",
        "330\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_yyyyyyyyyyyy_4",
        b"2\n0\nyyyyyyyyyyyyyyyynnnnnnnnnnnnnnnn\n",
        "316\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_yyynyyynnnnn",
        b"2\n0\nyyynyyynnnnnnnnnnnnnnnnnnnnnnnnn\n",
        "303\n",
        "",
        0,
    );
    check_expect(
        "flags_len32_yynnyynnyynn",
        b"2\n0\nyynnyynnyynnyynnyynnyynnyynnyynn\n",
        "16\n",
        "",
        0,
    );
    check_expect(
        "flags_len36_yynnyynnyynn",
        b"2\n0\nyynnyynnyynnyynnyynnyynnyynnyynnyynn\n",
        "16\n",
        "",
        0,
    );
    check_expect(
        "flags_param_ignored_0",
        b"2\n0\nynyn\n",
        "502\n",
        "",
        0,
    );
    check_expect(
        "flags_param_ignored_1",
        b"2\n1\nynyn\n",
        "502\n",
        "",
        0,
    );
    check_expect(
        "flags_param_ignored_2",
        b"2\n2\nynyn\n",
        "502\n",
        "",
        0,
    );
    check_expect(
        "flags_param_ignored_3",
        b"2\n3\nynyn\n",
        "502\n",
        "",
        0,
    );
    check_expect(
        "flags_param_ignored_neg1",
        b"2\n-1\nynyn\n",
        "502\n",
        "",
        0,
    );
    check_expect(
        "flags_param_ignored_99",
        b"2\n99\nynyn\n",
        "502\n",
        "",
        0,
    );
}

/// operation 3 -> `validate_sequence`: must start with 'y', must end with 'n'
/// when longer than one element, at most 3 consecutive equal values, then a
/// transition count bucketed by short (<=3) / medium (<=10) / long lengths.
#[test]
fn op3_validate() {
    check_expect(
        "seq_len1_y",
        b"3\n0\ny\n",
        "1\n",
        "",
        0,
    );
    check_expect(
        "seq_len1_n",
        b"3\n0\nn\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len1_q",
        b"3\n0\nq\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len1_n_2",
        b"3\n0\nN\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len1_y_2",
        b"3\n0\nY\n",
        "1\n",
        "",
        0,
    );
    check_expect(
        "seq_len2_yy",
        b"3\n0\nyy\n",
        "-11\n",
        "",
        0,
    );
    check_expect(
        "seq_len2_yn",
        b"3\n0\nyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_len2_ny",
        b"3\n0\nny\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len2_nn",
        b"3\n0\nnn\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_yyy",
        b"3\n0\nyyy\n",
        "-11\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_yyn",
        b"3\n0\nyyn\n",
        "11\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_ynn",
        b"3\n0\nynn\n",
        "11\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_yny",
        b"3\n0\nyny\n",
        "-11\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_nyy",
        b"3\n0\nnyy\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_nyn",
        b"3\n0\nnyn\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_nny",
        b"3\n0\nnny\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_nnn",
        b"3\n0\nnnn\n",
        "-10\n",
        "",
        0,
    );
    check_expect(
        "seq_len4_ynyn",
        b"3\n0\nynyn\n",
        "30\n",
        "",
        0,
    );
    check_expect(
        "seq_len4_yynn",
        b"3\n0\nyynn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len4_yyyn",
        b"3\n0\nyyyn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len4_ynnn",
        b"3\n0\nynnn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len4_yyyy",
        b"3\n0\nyyyy\n",
        "-11\n",
        "",
        0,
    );
    check_expect(
        "seq_len4_ynyy",
        b"3\n0\nynyy\n",
        "-11\n",
        "",
        0,
    );
    check_expect(
        "seq_len5_yyyyn",
        b"3\n0\nyyyyn\n",
        "-12\n",
        "",
        0,
    );
    check_expect(
        "seq_len5_ynnnn",
        b"3\n0\nynnnn\n",
        "-12\n",
        "",
        0,
    );
    check_expect(
        "seq_len6_yyyynn",
        b"3\n0\nyyyynn\n",
        "-12\n",
        "",
        0,
    );
    check_expect(
        "seq_len6_ynnnny",
        b"3\n0\nynnnny\n",
        "-11\n",
        "",
        0,
    );
    check_expect(
        "seq_len6_yyynnn",
        b"3\n0\nyyynnn\n",
        "20\n",
        "",
        0,
    );
    check_expect(
        "seq_len6_yynynn",
        b"3\n0\nyynynn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len6_ynynyn",
        b"3\n0\nynynyn\n",
        "30\n",
        "",
        0,
    );
    check_expect(
        "seq_len6_yynnyn",
        b"3\n0\nyynnyn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len6_ynnyyn",
        b"3\n0\nynnyyn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len10_yyynnnyynn",
        b"3\n0\nyyynnnyynn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len10_ynynynynyn",
        b"3\n0\nynynynynyn\n",
        "30\n",
        "",
        0,
    );
    check_expect(
        "seq_len9_yyynnnyyn",
        b"3\n0\nyyynnnyyn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_len11_yyynnnyynnn",
        b"3\n0\nyyynnnyynnn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len12_yyynnnyyynnn",
        b"3\n0\nyyynnnyyynnn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len12_ynynynynynyn",
        b"3\n0\nynynynynynyn\n",
        "50\n",
        "",
        0,
    );
    check_expect(
        "seq_len11_yynynynynyn",
        b"3\n0\nyynynynynyn\n",
        "50\n",
        "",
        0,
    );
    check_expect(
        "seq_len11_yyynnnyyynn",
        b"3\n0\nyyynnnyyynn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len18_yyynnnyyynnnyy",
        b"3\n0\nyyynnnyyynnnyyynnn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len18_ynynynynynynyn",
        b"3\n0\nynynynynynynynynyn\n",
        "50\n",
        "",
        0,
    );
    check_expect(
        "seq_len16_yyyynnnnyyyynn",
        b"3\n0\nyyyynnnnyyyynnnn\n",
        "-12\n",
        "",
        0,
    );
    check_expect(
        "seq_len12_yynyynyynyyn",
        b"3\n0\nyynyynyynyyn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len12_ynnynnynnynn",
        b"3\n0\nynnynnynnynn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len12_ynynynynynyn_2",
        b"3\n0\nynynynynynyn\n",
        "50\n",
        "",
        0,
    );
    check_expect(
        "seq_len16_ynynynynynynyn",
        b"3\n0\nynynynynynynynyn\n",
        "50\n",
        "",
        0,
    );
    check_expect(
        "seq_len24_yynyynyynyynyy",
        b"3\n0\nyynyynyynyynyynyynyynyyn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len24_ynnynnynnynnyn",
        b"3\n0\nynnynnynnynnynnynnynnynn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len16_ynnnyyynnnyyyn",
        b"3\n0\nynnnyyynnnyyynnn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_yqq",
        b"3\n0\nyqq\n",
        "11\n",
        "",
        0,
    );
    check_expect(
        "seq_len2_yn_2",
        b"3\n0\nYn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_len2_yn_3",
        b"3\n0\nYN\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_ynn_2",
        b"3\n0\nyNn\n",
        "11\n",
        "",
        0,
    );
    check_expect(
        "seq_len4_yyyy_2",
        b"3\n0\nyyyy\n",
        "-11\n",
        "",
        0,
    );
    check_expect(
        "seq_len11_yyyyyyyyyyn",
        b"3\n0\nyyyyyyyyyyn\n",
        "-12\n",
        "",
        0,
    );
    check_expect(
        "seq_len12_yyynnnyyynnn_2",
        b"3\n0\nyyynnnyyynnn\n",
        "45\n",
        "",
        0,
    );
    check_expect(
        "seq_len2_yn_4",
        b"3\n0\nyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_len3_ynn_3",
        b"3\n0\nynn\n",
        "11\n",
        "",
        0,
    );
    check_expect(
        "seq_len8_yynnyynn",
        b"3\n0\nyynnyynn\n",
        "25\n",
        "",
        0,
    );
    check_expect(
        "seq_param_ignored_0",
        b"3\n0\nyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_param_ignored_1",
        b"3\n1\nyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_param_ignored_2",
        b"3\n2\nyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_param_ignored_3",
        b"3\n3\nyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_param_ignored_neg1",
        b"3\n-1\nyn\n",
        "2\n",
        "",
        0,
    );
    check_expect(
        "seq_param_ignored_99",
        b"3\n99\nyn\n",
        "2\n",
        "",
        0,
    );
}

/// `MAX_INPUT_SIZE` is 1024, so each `fgets` stores at most 1023 bytes.  A
/// decision line at or beyond that limit is truncated *without* a trailing
/// newline, so `main` does not strip anything and `len` stays 1023.
#[test]
fn decision_line_length_boundaries() {
    let mut lengths: Vec<usize> = (0..=40).collect();
    lengths.extend(1015..=1030);
    lengths.extend([2047, 2048, 2049, 4096]);

    for len in lengths {
        for pattern in ["y", "n", "yn", "ny", "yyynnn", "yyn", "ynn", "yyyy", "q"] {
            let s: String = pattern.chars().cycle().take(len).collect();
            for op in -1..=4 {
                for terminator in ["\n", ""] {
                    let input = format!("{op}\n0\n{s}{terminator}").into_bytes();
                    check(
                        &format!(
                            "decision_len{len}_pat{pattern}_op{op}_term{}",
                            terminator.len()
                        ),
                        &input,
                    );
                }
            }
        }
    }
}

/// A first line longer than 1023 bytes is split across the following `fgets`
/// calls, so the leftover becomes the parameter and then the decision string.
#[test]
fn overlong_lines_spill_into_the_next_read() {
    for len in [1_020, 1_021, 1_022, 1_023, 1_024, 1_025, 1_030, 2_050] {
        // operation line too long
        check(
            &format!("overlong_op_zeros_{len}"),
            &format!("{}\n0\nyyy\n", "0".repeat(len)).into_bytes(),
        );
        check(
            &format!("overlong_op_spaces_{len}"),
            &format!("{}2\n0\nyyy\n", " ".repeat(len)).into_bytes(),
        );
        check(
            &format!("overlong_op_junk_{len}"),
            &format!("2{}\n0\nyyy\n", "x".repeat(len)).into_bytes(),
        );
        check(
            &format!("overlong_op_digits_{len}"),
            &format!("{}\n1\nyyy\n", "3".repeat(len)).into_bytes(),
        );
        // parameter line too long
        check(
            &format!("overlong_param_{len}"),
            &format!("1\n{}\nyyy\n", "0".repeat(len)).into_bytes(),
        );
        check(
            &format!("overlong_param_ynn_{len}"),
            &format!("2\n{}\nynynyn\n", "9".repeat(len)).into_bytes(),
        );
    }
}

/// Every decision string of length 1..=3 over the interesting alphabet
/// (true, false and "invalid, therefore false") crossed with every operation
/// including the out-of-range ones, and every logic operator including the
/// out-of-range ones.
#[test]
fn exhaustive_short_strings_all_operations() {
    const ALPHABET: [char; 5] = ['y', 'n', 'Y', 'N', 'q'];

    let mut strings: Vec<String> = Vec::new();
    for a in ALPHABET {
        strings.push(a.to_string());
        for b in ALPHABET {
            strings.push(format!("{a}{b}"));
            for c in ALPHABET {
                strings.push(format!("{a}{b}{c}"));
            }
        }
    }

    for s in &strings {
        for op in -1..=4 {
            for param in -1..=4 {
                check(
                    &format!("short_{s}_op{op}_p{param}"),
                    &common::stdin3(op, param, s),
                );
            }
        }
    }
}

/// Every y/n sequence up to length 12 against `configure_flags` (operation 2)
/// and `validate_sequence` (operation 3) - these are the two operations whose
/// result depends on the whole string, so they need exhaustive coverage.
#[test]
fn exhaustive_yn_sequences() {
    for len in 1u32..=12 {
        for bits in 0u32..(1u32 << len) {
            let s: String = (0..len)
                .map(|i| if bits >> i & 1 == 1 { 'y' } else { 'n' })
                .collect();
            for op in [2, 3] {
                check(&format!("yn_op{op}_{s}"), &common::stdin3(op, 0, &s));
            }
        }
    }
}

/// Sequences long enough to exercise `configure_flags`' 32-element clamp and
/// `validate_sequence`'s "long" (> 10) length bucket, including the runs that
/// trip the "more than 3 consecutive" rule.
#[test]
fn long_sequences_around_the_clamp() {
    let mut strings: Vec<String> = Vec::new();
    for len in 13..=70usize {
        for pattern in [
            "y", "n", "yn", "ny", "yyn", "ynn", "yyyn", "ynnn", "yyynnn", "yynn", "yyyynnnn",
            "yyynnnn", "yq", "qy", "ynq",
        ] {
            strings.push(pattern.chars().cycle().take(len).collect());
        }
        strings.push(format!("{}n", "y".repeat(len - 1)));
        strings.push(format!("{}y", "n".repeat(len - 1)));
        strings.push(format!("y{}", "n".repeat(len - 1)));
        strings.push(format!("n{}", "y".repeat(len - 1)));
        strings.push(format!(
            "{}{}",
            "y".repeat(len / 2),
            "n".repeat(len - len / 2)
        ));
    }
    for s in &strings {
        for op in 0..=3 {
            check(
                &format!("long_op{op}_len{}_{}", s.len(), &s[..s.len().min(10)]),
                &common::stdin3(op, 0, s),
            );
        }
    }
}

/// Deterministic pseudo-random sweep: random y/n/invalid strings, random
/// operations and parameters, and completely random byte streams (which also
/// covers stdin that has fewer than three lines).
#[test]
fn pseudorandom_sweep() {
    // xorshift64* so the case list is reproducible without a dependency.
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    };

    // 1. structured three-line inputs
    for i in 0..3_000 {
        let len = (next() % 80) as usize;
        let alphabet = b"ynYNqQ \t01";
        let s: Vec<u8> = (0..len)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        let op = (next() % 11) as i64 - 5;
        let param = (next() % 11) as i64 - 5;
        let mut input = format!("{op}\n{param}\n").into_bytes();
        input.extend_from_slice(&s);
        input.push(b'\n');
        check(&format!("rand_structured_{i}"), &input);
    }

    // 2. arbitrary byte streams
    for i in 0..3_000 {
        let len = (next() % 64) as usize;
        let input: Vec<u8> = (0..len).map(|_| (next() % 256) as u8).collect();
        check(&format!("rand_bytes_{i}"), &input);
    }

    // 3. arbitrary byte streams biased towards newlines and digits
    for i in 0..2_000 {
        let len = (next() % 40) as usize;
        let alphabet = b"\n\n\r\0yn0123-+ ";
        let input: Vec<u8> = (0..len)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        check(&format!("rand_lines_{i}"), &input);
    }
}

/// When stdout is a pipe with no reader, the C program is killed by `SIGPIPE`
/// and prints nothing to stderr.  The Rust program must not panic (which would
/// abort with `SIGABRT` and print a panic message instead).
#[cfg(unix)]
#[test]
fn broken_stdout_matches_c() {
    // Success paths: these are the ones that actually write to stdout.
    for (name, input) in [
        ("op0", &b"0\n0\nyyy\n"[..]),
        ("op1", &b"1\n2\nynn\n"[..]),
        ("op2", &b"2\n0\nynynyn\n"[..]),
        ("op3", &b"3\n0\nyyynnn\n"[..]),
        ("bad_op", &b"9\n0\nyyy\n"[..]),
        ("empty_decisions", &b"0\n0\n\n"[..]),
    ] {
        common::check_broken_stream(name, input, common::BrokenStream::Stdout);
    }

    // Error paths write only to stderr, so stdout is never touched and the
    // programs must exit 1 exactly as they do with a working stdout.
    for (name, input) in [
        ("no_operation", &b""[..]),
        ("no_param", &b"0\n"[..]),
        ("no_decisions", &b"0\n0\n"[..]),
    ] {
        common::check_broken_stream(name, input, common::BrokenStream::Stdout);
    }
}

/// When stderr is a pipe with no reader, the error paths' `fprintf` must kill
/// the process with `SIGPIPE` in both programs.
#[cfg(unix)]
#[test]
fn broken_stderr_matches_c() {
    for (name, input) in [
        ("no_operation", &b""[..]),
        ("no_param", &b"0\n"[..]),
        ("no_decisions", &b"0\n0\n"[..]),
        // Success paths never touch stderr, so these must still exit 0.
        ("op2_ok", &b"2\n0\nyyy\n"[..]),
        ("op3_ok", &b"3\n0\nyn\n"[..]),
    ] {
        common::check_broken_stream(name, input, common::BrokenStream::Stderr);
    }
}

/// `main` is declared `int main(void)`, so command-line arguments are ignored.
#[test]
fn command_line_arguments_are_ignored() {
    for (name, args) in [
        ("none", &[][..]),
        ("help", &["--help"][..]),
        ("many", &["--help", "extra", "-1", "2"][..]),
        ("looks_like_input", &["3", "0", "yyy"][..]),
        ("empty_arg", &[""][..]),
    ] {
        common::check_with_args(name, args, b"2\n0\nyyy\n");
        common::check_with_args(name, args, b"");
    }
}

/// stdin as a regular file rather than a pipe: `fgets` must behave the same.
#[test]
fn stdin_from_a_regular_file() {
    for (name, input) in [
        ("empty", &b""[..]),
        ("op_only", &b"0\n"[..]),
        ("op0", &b"0\n0\nyyy\n"[..]),
        ("op1", &b"1\n3\nnnn\n"[..]),
        ("op2", &b"2\n0\nynynyn\n"[..]),
        ("op3", &b"3\n0\nyyynnn\n"[..]),
        ("no_final_newline", &b"3\n0\nyn"[..]),
        ("blank_decisions", &b"2\n0\n\n"[..]),
    ] {
        common::check_file_stdin(name, input);
    }
}

/// stdin far larger than anything the program reads: it consumes at most three
/// lines and exits, leaving the rest unread.
#[test]
fn stdin_much_larger_than_the_program_reads() {
    let mut big = b"2\n0\n".to_vec();
    big.extend(std::iter::repeat(b'y').take(1024 * 1024));
    big.push(b'\n');
    check("one_mib_decision_line", &big);

    let mut big3 = Vec::new();
    for i in 0..20_000 {
        big3.extend_from_slice(format!("{}\n", i % 7).as_bytes());
    }
    check("twenty_thousand_lines", &big3);

    let mut nulls = b"3\n0\n".to_vec();
    nulls.extend(std::iter::repeat(0u8).take(100_000));
    nulls.push(b'\n');
    check("hundred_thousand_nul_bytes", &nulls);
}

/// Line terminators and whitespace forms that are *not* `\n`: `fgets` only
/// stops at `\n`, while `atoi`/`strtol` skip the full `isspace` set
/// (space, `\t`, `\n`, `\v`, `\f`, `\r`).
#[test]
fn exotic_whitespace_and_line_terminators() {
    check_expect(
        "cr_only_terminators",
        b"2\r0\ryyy\r",
        "",
        "Error reading parameter\n",
        1,
    );
    check_expect("vtab_and_formfeed_skipped", b"\x0b2\n\x0c1\nyyy\n", "1003\n", "", 0);
    check_expect("mixed_leading_whitespace", b"\x0c\x0b 2\n0\nyyy\n", "1003\n", "", 0);
    check_expect("whitespace_only_lines", b"\x0b\n\x0c\n\x0b\n", "-2\n", "", 0);
    check_expect("tabs_only_lines", b"\t\n\t\n\t\t\t\n", "0\n", "", 0);
    check_expect("spaces_only_decisions", b"2\n0\n   \n", "0\n", "", 0);
    check_expect("cr_inside_decisions_op2", b"2\n0\nyyy\r\n", "203\n", "", 0);
    check_expect("cr_inside_decisions_op3", b"3\n0\nyyy\r\n", "25\n", "", 0);
    // Every single byte value as a one-character decision string.
    for b in 0u16..=255 {
        let byte = b as u8;
        if byte == b'\n' || byte == 0 {
            continue; // covered separately; these terminate the line
        }
        for op in 0..=3 {
            let mut input = format!("{op}\n0\n").into_bytes();
            input.push(byte);
            input.push(b'\n');
            check(&format!("single_byte_{byte:#04x}_op{op}"), &input);
        }
    }
}
