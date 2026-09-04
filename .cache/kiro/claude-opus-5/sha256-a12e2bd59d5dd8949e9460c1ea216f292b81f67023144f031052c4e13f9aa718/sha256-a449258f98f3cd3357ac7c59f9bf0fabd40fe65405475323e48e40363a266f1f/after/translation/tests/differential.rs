//! Differential tests: every input class the C in `c_src/src/main.c` branches on.
//!
//! Each test drives BOTH executables as subprocesses and asserts stdout, stderr
//! and exit status all match byte for byte. Comments name the C source construct
//! the case exists to reach.

mod harness;

use harness::{same, buf};

// ===================================================================
// main(): `scanf("%d", &operation) != 1`
// ===================================================================

#[test]
fn op_read_fails_on_empty_input() {
    // EOF immediately -> scanf returns EOF -> "Failed to read operation", exit 1
    same("empty", "");
}

#[test]
fn op_read_fails_on_whitespace_only() {
    // %d skips whitespace then hits EOF. Covers every C isspace() class.
    same("spaces", "   \n\t  \n");
    same("all whitespace kinds", " \t\n\x0b\x0c\r");
    same("100k spaces", &" ".repeat(100_000));
}

#[test]
fn op_read_fails_on_non_numeric() {
    // Matching failure (scanf returns 0), not EOF.
    same("alpha", "abc");
    same("dot", ".5");
    same("sign only", "+");
    same("minus only", "-");
    same("double minus", "--5");
    same("nul byte first", "\x00 1 1");
}

// ===================================================================
// main(): `scanf("%d", &buffer_count) != 1`
// ===================================================================

#[test]
fn count_read_fails() {
    same("op then eof", "1");
    same("op then junk", "1 xyz");
    same("op then newline eof", "1\n");
    same("op then embedded nul", "1\x001 1");
}

// ===================================================================
// main(): `buffer_count <= 0 || buffer_count > 100`
// ===================================================================

#[test]
fn count_out_of_range() {
    same("count 0", "1 0");
    same("count -1", "1 -1");
    same("count -999", "1 -999");
    same("count 101", "1 101");
    same("count 999999", "1 999999");
}

#[test]
fn count_boundaries_accepted() {
    // 1 and 100 are the inclusive limits.
    same("count 1", "6 1 0");
    let mut s = String::from("6 100");
    for i in 0..100 {
        s.push_str(&format!(" 2 {} {}", i, 255 - i));
    }
    same("count 100", &s);
}

// ===================================================================
// read_buffer(): length scanf failure / range check / byte scanf failure
// ===================================================================

#[test]
fn buffer_length_read_fails() {
    same("len eof", "1 1");
    same("len junk", "1 1 zz");
    same("len hex prefix stops at x", "1 1 0x10");
}

#[test]
fn buffer_length_out_of_range() {
    same("len -1", "1 1 -1");
    same("len -256", "1 1 -256");
    same("len 257", "1 1 257");
    same("len 1000", "1 1 1000");
}

#[test]
fn buffer_length_boundaries() {
    same("len 0", "1 1 0");
    same("len 1", "1 1 1 7");
    // 256 is the maximum the fixed-size buffer_t holds.
    same("len 256", &format!("1 1 {}", buf(256, 0)));
}

#[test]
fn buffer_byte_read_fails() {
    // "Failed to read byte %zu" — index must match (0-based).
    same("byte 0 missing", "1 1 3");
    same("byte 2 missing", "1 1 3 1 2");
    same("byte junk", "1 1 3 1 2 q");
    same("byte truncated float", "1 1 3.5 1 2");
    // 256 bytes promised, only 255 supplied -> "Failed to read byte 255"
    let mut short = String::from("1 1 256");
    for k in 0..255usize {
        short.push_str(&format!(" {}", k % 256));
    }
    same("byte 255 missing of 256", &short);
}

#[test]
fn buffer_bytes_truncate_to_u8() {
    // `buf->data[i] = (uint8_t)byte;`
    same("negative bytes", "1 1 3 -1 -2 -300");
    same("oversized bytes", "1 1 3 256 511 1000");
    for v in [-1i64, -128, -129, -255, -256, -257, 256, 257, 511, 512, 1000,
              65535, 65536, 2147483647, -2147483648, 4294967295, 4294967296] {
        same(&format!("byte {v} via checksum"), &format!("6 1 1 {v}"));
        same(&format!("byte {v} via reverse"), &format!("1 1 1 {v}"));
    }
}

// ===================================================================
// main(): case OP_COPY
// ===================================================================

#[test]
fn op_copy() {
    // buffer_count < 2 -> "Copy needs at least 2 buffers"
    same("copy 1 buffer", "0 1 3 1 2 3");
    // buffer_count >= 2 -> copies buffers[0] into a fresh temp
    same("copy 2 buffers", "0 2 3 1 2 3 2 9 8");
    same("copy empty source", "0 2 0 2 9 8");
    same("copy full source", &format!("0 2 {} 1 0", buf(256, 1)));
    same("copy 3 buffers", "0 3 2 1 2 2 3 4 2 5 6");
}

// ===================================================================
// main(): case OP_REVERSE  +  buffer_reverse()'s `length == 0` early return
// ===================================================================

#[test]
fn op_reverse() {
    same("reverse odd", "1 1 5 1 2 3 4 5");
    same("reverse even", "1 1 4 1 2 3 4");
    same("reverse single", "1 1 1 42");
    same("reverse empty (early return)", "1 1 0");
    same("reverse many", "1 3 2 1 2 0 4 7 7 7 7");
    same("reverse 256", &format!("1 1 {}", buf(256, 0)));
}

#[test]
fn op_reverse_all_lengths() {
    for l in [0usize, 1, 2, 3, 4, 5, 7, 8, 127, 128, 129, 200, 255, 256] {
        same(&format!("reverse len {l}"), &format!("1 1 {}", buf(l, 3)));
    }
}

// ===================================================================
// main(): case OP_MERGE  +  buffer_merge()'s length overflow check
// ===================================================================

#[test]
fn op_merge() {
    same("merge 1 buffer", "2 1 3 1 2 3");
    same("merge ok", "2 2 3 1 2 3 2 9 8");
    same("merge two empties", "2 2 0 0");
    same("merge empty + full", &format!("2 2 0 {}", buf(256, 1)));
}

#[test]
fn op_merge_length_boundary() {
    // 128+128 == 256 is allowed; anything above trips
    // "Merged length %zu exceeds maximum".
    same("merge exactly 256", &format!("2 2 {} {}", buf(128, 1), buf(128, 9)));
    same("merge 257", &format!("2 2 {} {}", buf(129, 1), buf(128, 9)));
    same("merge 512", &format!("2 2 {} {}", buf(256, 0), buf(256, 7)));
    same("merge 256+1", &format!("2 2 {} {}", buf(256, 0), buf(1, 7)));
}

#[test]
fn op_merge_length_pairs() {
    for a in [0usize, 1, 2, 5, 127, 128, 129, 200, 255, 256] {
        for b in [0usize, 1, 2, 5, 127, 128, 129, 200, 255, 256] {
            same(
                &format!("merge {a}+{b}"),
                &format!("2 2 {} {}", buf(a, 5), buf(b, 9)),
            );
        }
    }
}

// ===================================================================
// main(): case OP_SPLIT  +  buffer_split()'s bounds check
// ===================================================================

#[test]
fn op_split_position_read_fails() {
    same("split pos eof", "3 1 3 1 2 3");
    same("split pos junk", "3 1 3 1 2 3 nope");
}

#[test]
fn op_split_in_range() {
    same("split 0", "3 1 3 1 2 3 0");
    same("split middle", "3 1 5 1 2 3 4 5 2");
    same("split at length", "3 1 3 1 2 3 3");
    same("split empty at 0", "3 1 0 0");
    same("split 256 at 128", &format!("3 1 {} 128", buf(256, 0)));
    // Extra buffers are read but only buffers[0] is split.
    same("split ignores later buffers", "3 3 2 1 2 2 3 4 2 5 6 1");
}

#[test]
fn op_split_out_of_range() {
    same("split beyond length", "3 1 3 1 2 3 4");
    same("split empty at 1", "3 1 0 1");
    same("split 257", &format!("3 1 {} 257", buf(256, 0)));
}

#[test]
fn op_split_negative_position_becomes_huge() {
    // The C passes an `int` into a `size_t` parameter, so a negative position
    // sign-extends into a giant value and is reported verbatim by "%zu".
    same("split -1", "3 1 3 1 2 3 -1");
    same("split -2", "3 1 3 1 2 3 -2");
    same("split INT_MIN", "3 1 3 1 2 3 -2147483648");
    same("split -1000000", "3 1 3 1 2 3 -1000000");
}

#[test]
fn op_split_all_positions() {
    for l in [0usize, 1, 2, 3, 5, 128, 256] {
        for p in -3i64..=(l as i64 + 3) {
            same(
                &format!("split len {l} at {p}"),
                &format!("3 1 {} {}", buf(l, 2), p),
            );
        }
    }
}

// ===================================================================
// main(): case OP_INTERLEAVE  +  buffer_interleave()'s overflow check
// ===================================================================

#[test]
fn op_interleave() {
    same("interleave 1 buffer", "4 1 3 1 2 3");
    same("interleave equal", "4 2 3 1 2 3 3 7 8 9");
    same("interleave first longer", "4 2 5 1 2 3 4 5 2 9 8");
    same("interleave second longer", "4 2 2 9 8 5 1 2 3 4 5");
    same("interleave both empty", "4 2 0 0");
    same("interleave first empty", "4 2 0 3 1 2 3");
    same("interleave second empty", "4 2 3 1 2 3 0");
}

#[test]
fn op_interleave_length_boundary() {
    // "Interleaved length exceeds maximum" has no format specifier — the
    // message must not gain one.
    same("interleave exactly 256", &format!("4 2 {} {}", buf(128, 1), buf(128, 9)));
    same("interleave 257", &format!("4 2 {} {}", buf(129, 1), buf(128, 9)));
    same("interleave 300", &format!("4 2 {} {}", buf(200, 1), buf(100, 9)));
    same("interleave 512", &format!("4 2 {} {}", buf(256, 0), buf(256, 7)));
}

#[test]
fn op_interleave_length_pairs() {
    for a in [0usize, 1, 2, 5, 127, 128, 129, 200, 255, 256] {
        for b in [0usize, 1, 2, 5, 127, 128, 129, 200, 255, 256] {
            same(
                &format!("interleave {a}+{b}"),
                &format!("4 2 {} {}", buf(a, 5), buf(b, 9)),
            );
        }
    }
}

// ===================================================================
// main(): case OP_ROTATE  +  buffer_rotate()'s early return and normalization
// ===================================================================

#[test]
fn op_rotate_amount_read_fails() {
    same("rotate amount eof", "5 1 3 1 2 3");
    same("rotate amount junk", "5 1 3 1 2 3 zz");
}

#[test]
fn op_rotate_early_returns() {
    // `buf->length == 0 || positions == 0`
    same("rotate empty buffer", "5 1 0 4");
    same("rotate by 0", "5 1 3 1 2 3 0");
    same("rotate empty by 0", "5 1 0 0");
}

#[test]
fn op_rotate_normalization() {
    same("rotate 1", "5 1 3 1 2 3 1");
    same("rotate == length", "5 1 3 1 2 3 3");
    same("rotate > length", "5 1 3 1 2 3 5");
    same("rotate negative", "5 1 3 1 2 3 -1");
    same("rotate very negative", "5 1 3 1 2 3 -100");
    same("rotate INT_MIN", "5 1 3 1 2 3 -2147483648");
    same("rotate INT_MAX", "5 1 3 1 2 3 2147483647");
    same("rotate length 1", "5 1 1 42 7");
    same("rotate multiple buffers", "5 3 3 1 2 3 0 4 4 5 6 7 2");
    same("rotate 256 buffer", &format!("5 1 {} 100", buf(256, 0)));
}

#[test]
fn op_rotate_all_amounts() {
    // C's `%` truncates toward zero, then negatives get one `+= length`.
    for l in [1usize, 2, 3, 5, 8, 256] {
        for p in -(l as i64 * 2 + 2)..=(l as i64 * 2 + 2) {
            same(
                &format!("rotate len {l} by {p}"),
                &format!("5 1 {} {}", buf(l, 1), p),
            );
        }
    }
}

// ===================================================================
// main(): case OP_CHECKSUM  (prints the stored checksum with "%u")
// ===================================================================

#[test]
fn op_checksum() {
    same("checksum multiple", "6 3 3 1 2 3 0 2 255 255");
    same("checksum empty", "6 1 0");
    same("checksum 256", &format!("6 1 {}", buf(256, 0)));
    // (sum << 3) ^ byte overflows uint32 well before 256 bytes; check the wrap.
    same("checksum 11 x 255", "6 1 11 255 255 255 255 255 255 255 255 255 255 255");
    for v in 0..256 {
        same(&format!("checksum single byte {v}"), &format!("6 1 1 {v}"));
    }
}

// ===================================================================
// main(): default case — "Unknown operation %d"
// ===================================================================

#[test]
fn unknown_operation() {
    for op in ["7", "8", "-1", "-2", "99", "999", "-2147483648", "2147483647"] {
        same(&format!("op {op}"), &format!("{op} 1 0"));
    }
}

#[test]
fn unknown_operation_still_reads_all_buffers_first() {
    // The C validates `operation` only after consuming every buffer, so a bad
    // buffer is reported before the unknown-operation error.
    same("bad op, bad buffer", "77 1 -5");
    same("bad op, short buffer", "77 1 3 1");
    same("bad op, good buffers", "77 2 2 1 2 2 3 4");
}

// ===================================================================
// scanf("%d") integer conversion quirks: glibc parses as `long`, saturates at
// LONG_MAX/LONG_MIN on overflow, then stores the low 32 bits into the `int`.
// ===================================================================

#[test]
fn scanf_overflow_saturates_then_truncates() {
    same("op overflow", "99999999999999999999 1 0");
    same("count overflow", "1 99999999999999999999");
    same("count overflow negative", "1 -99999999999999999999");
    same("length overflow", "1 1 99999999999999999999");
    same("byte overflow", "1 1 1 99999999999999999999");
    same("split pos overflow", "3 1 1 5 99999999999999999999");
    same("rotate overflow", "5 1 3 1 2 3 99999999999999999999");
    same("5000 nines", &format!("1 1 {}", "9".repeat(5000)));
}

#[test]
fn scanf_int_truncation_boundaries() {
    same("LONG_MAX", "1 9223372036854775807");
    same("LONG_MAX + 1", "1 9223372036854775808");
    same("LONG_MIN", "1 -9223372036854775808");
    same("LONG_MIN - 1", "1 -9223372036854775809");
    same("INT_MAX + 1 as count", "1 2147483648");
    same("2^32 as op wraps to OP_COPY", "4294967296 1 0");
    same("2^32+1 as op wraps to OP_REVERSE", "4294967297 1 0");
    same("2^32+6 as op wraps to OP_CHECKSUM", "4294967302 1 0");
}

#[test]
fn scanf_accepts_signs_and_leading_zeros() {
    same("leading zeros", "0001 1 0003 001 002 003");
    same("many leading zeros", &format!("6 1 {}3 1 2 3", "0".repeat(5000)));
    same("plus signs", "+1 +1 +3 +1 +2 +3");
    same("negative zero", "1 1 -0");
}

#[test]
fn scanf_reads_across_newlines_and_whitespace() {
    // %d skips arbitrary whitespace, so line structure is irrelevant.
    same("newline separated", "1\n1\n3\n1\n2\n3\n");
    same("crlf separated", "1\r\n1\r\n3\r\n1\r\n2\r\n3\r\n");
    same("vtab formfeed", "1\x0b1\x0c3 1 2 3");
    same("tabs", "1\t1\t3\t1\t2\t3");
    same("mixed heavy whitespace", "  \n\t 1 \r\n\n 1 \x0b 3 \x0c 1  2   3  \n\n");
    same("no trailing newline", "1 1 3 1 2 3");
}

#[test]
fn trailing_input_is_ignored() {
    same("trailing garbage", "6 1 0 this is never read");
    same("trailing numbers", "6 1 0 1 2 3 4 5");
    same("split ignores trailing", "3 1 2 1 2 1 99 junk");
}

// ===================================================================
// Largest inputs the program handles: 100 buffers x 256 bytes, for every op.
// These also straddle the 4096-byte stdin read chunks.
// ===================================================================

#[test]
fn maximum_size_inputs_for_every_operation() {
    for op in 0..7 {
        for sep in [" ", "\n"] {
            let mut toks: Vec<String> = vec![op.to_string(), "100".to_string()];
            for b in 0..100usize {
                toks.push("256".to_string());
                for k in 0..256usize {
                    toks.push(((b * 7 + k) % 256).to_string());
                }
            }
            if op == 3 || op == 5 {
                toks.push("77".to_string());
            }
            same(
                &format!("max op {op} sep {:?}", sep),
                &(toks.join(sep) + "\n"),
            );
        }
    }
}

#[test]
fn numbers_straddling_stdin_read_chunks() {
    // The Rust reader refills in 4096-byte chunks; a token must not be split.
    for pad in 4080..4100 {
        same(
            &format!("straddle digits pad {pad}"),
            &format!("6 1 1 {}250\n", " ".repeat(pad)),
        );
        same(
            &format!("straddle token pad {pad}"),
            &format!("1 1 3 1 2 {}3\n", " ".repeat(pad)),
        );
    }
}
