/*
 * Rust translation of c_src/src/main.c
 *
 * Reads the same whitespace separated decimal fields as the C driver's scanf()
 * calls, in the same order, with the same error messages and exit codes.
 *
 * The two 1024 byte buffers are modelled as one contiguous byte array, because
 * the C library functions called by lib.c read past the end of the data that
 * was actually written into them. In the reference build (c_src/CMakeLists.txt,
 * i.e. no optimisation flags) main()'s frame is
 *
 *     rbp-0x830  ref_buffer[1024]
 *     rbp-0x430  input_buffer[1024]
 *     rbp-0x30   ref_len, input_len, flags, operation, ...
 *
 * so ref_buffer sits immediately *below* input_buffer. Bytes never written by
 * main() keep the start-up residue captured in stack_residue.rs.
 */

mod stack_residue;
mod strcpy_fun;

use std::io::{Read, Write};

use strcpy_fun::{process_strings, MAX_BUFFER_SIZE};

/* Offsets inside the modelled stack region. */
const REF_OFF: usize = 0;
const INPUT_OFF: usize = MAX_BUFFER_SIZE;
/* main()'s scalar locals live directly above input_buffer (rbp-0x30 .. rbp),
 * followed by the saved frame pointer. Modelled so that a strlen() walking off
 * the end of a completely unterminated input_buffer sees the same bytes. */
const LOCALS_OFF: usize = 2 * MAX_BUFFER_SIZE;
const MEM_SIZE: usize = LOCALS_OFF + 64;

/* ------------------------------------------------------------------ */
/* scanf() emulation                                                   */
/* ------------------------------------------------------------------ */

struct Scanner {
    data: Vec<u8>,
    pos: usize,
    /// `true` once stdin has been read to completion (or for a fixed buffer).
    eof: bool,
}

/// Bytes glibc's scanf treats as whitespace (isspace).
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Result of scanning an integer field: sign plus magnitude, with saturation
/// tracking so that glibc's strtol/strtoul range clamping can be reproduced.
struct RawInt {
    negative: bool,
    magnitude: u64,
    overflow: bool,
}

impl Scanner {
    fn new() -> Scanner {
        Scanner {
            data: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    #[cfg(test)]
    fn from_bytes(data: Vec<u8>) -> Scanner {
        Scanner {
            data,
            pos: 0,
            eof: true,
        }
    }

    /// Pull more of stdin in when the next byte has not been buffered yet.
    /// scanf() only ever needs one byte of lookahead, and like scanf this stops
    /// reading as soon as the conversions are satisfied, so a caller that keeps
    /// the pipe open after an early error is not blocked on.
    fn fill(&mut self) {
        while !self.eof && self.pos >= self.data.len() {
            let mut chunk = [0u8; 4096];
            match std::io::stdin().read(&mut chunk) {
                Ok(0) => self.eof = true,
                Ok(n) => self.data.extend_from_slice(&chunk[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => self.eof = true,
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.fill();
        self.data.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if is_space(c) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Scan one integer conversion: optional whitespace, optional sign, then
    /// one or more decimal digits. `None` means a matching failure or EOF, i.e.
    /// scanf returned something other than 1.
    fn scan_int(&mut self) -> Option<RawInt> {
        self.skip_ws();

        let mut negative = false;
        if let Some(c) = self.peek() {
            if c == b'+' || c == b'-' {
                negative = c == b'-';
                self.pos += 1;
            }
        }

        let digits_start = self.pos;
        let mut magnitude: u64 = 0;
        let mut overflow = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            let d = u64::from(c - b'0');
            match magnitude.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => magnitude = v,
                None => overflow = true,
            }
            self.pos += 1;
        }

        if self.pos == digits_start {
            /* no digits consumed: matching failure */
            return None;
        }

        Some(RawInt {
            negative,
            magnitude,
            overflow,
        })
    }

    /// `scanf("%d", &x)`: glibc converts with strtol (clamping to LONG_MIN /
    /// LONG_MAX on overflow) and then stores into an int, truncating.
    fn scan_i32(&mut self) -> Option<i32> {
        let r = self.scan_int()?;
        let as_long: i64 = if r.negative {
            if r.overflow || r.magnitude >= (i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                -(r.magnitude as i64)
            }
        } else if r.overflow || r.magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            r.magnitude as i64
        };
        Some(as_long as i32)
    }

    /// `scanf("%u", &x)`: strtoul semantics (a leading '-' wraps around),
    /// clamped to ULONG_MAX, then truncated to unsigned int.
    fn scan_u32(&mut self) -> Option<u32> {
        let v = self.scan_ulong()?;
        Some(v as u32)
    }

    /// `scanf("%zu", &x)`: strtoul semantics stored into a 64 bit size_t.
    fn scan_usize(&mut self) -> Option<usize> {
        let v = self.scan_ulong()?;
        Some(v as usize)
    }

    /// `strtoul()` semantics shared by `%u` and `%zu`.
    ///
    /// glibc's strtoul returns ULONG_MAX (and sets ERANGE) as soon as the digit
    /// string overflows, *without* applying the leading sign. Only values that
    /// fit are negated, which is why "-1" yields ULONG_MAX but
    /// "-99999999999999999999999999" also yields ULONG_MAX rather than a small
    /// wrapped-around number.
    fn scan_ulong(&mut self) -> Option<u64> {
        let r = self.scan_int()?;
        if r.overflow {
            return Some(u64::MAX);
        }
        Some(if r.negative {
            0u64.wrapping_sub(r.magnitude)
        } else {
            r.magnitude
        })
    }
}

/* ------------------------------------------------------------------ */

fn err(msg: &str) -> ! {
    let mut stderr = std::io::stderr();
    let _ = stderr.write_all(msg.as_bytes());
    let _ = stderr.flush();
    let _ = std::io::stdout().flush();
    std::process::exit(1);
}

/// The modelled stack region, pre-filled with the start-up residue that the
/// reference binary's two buffers contain before main() writes to them.
fn new_stack_region() -> Vec<u8> {
    let mut mem = vec![0u8; MEM_SIZE];
    mem[REF_OFF..REF_OFF + MAX_BUFFER_SIZE].copy_from_slice(&stack_residue::ref_residue());
    mem[INPUT_OFF..INPUT_OFF + MAX_BUFFER_SIZE].copy_from_slice(&stack_residue::input_residue());
    mem
}

/// main()'s scalar locals, in the order the reference build lays them out:
/// ref_len at rbp-0x30, input_len at rbp-0x28, flags at rbp-0x1c,
/// operation at rbp-0x18. They sit directly above input_buffer, so a strlen()
/// running off the end of a fully unterminated input_buffer reads them.
fn store_locals(mem: &mut [u8], ref_len: usize, input_len: usize, flags: u32, operation: i32) {
    mem[LOCALS_OFF..LOCALS_OFF + 8].copy_from_slice(&(ref_len as u64).to_le_bytes());
    mem[LOCALS_OFF + 8..LOCALS_OFF + 16].copy_from_slice(&(input_len as u64).to_le_bytes());
    mem[LOCALS_OFF + 20..LOCALS_OFF + 24].copy_from_slice(&flags.to_le_bytes());
    mem[LOCALS_OFF + 24..LOCALS_OFF + 28].copy_from_slice(&operation.to_le_bytes());
}

fn main() {
    let mut sc = Scanner::new();

    /* ref_buffer, then input_buffer, then main()'s scalar locals. */
    let mut mem = new_stack_region();

    /* Read operation */
    let operation: i32 = match sc.scan_i32() {
        Some(v) => v,
        None => err("Error reading operation\n"),
    };

    /* Read flags */
    let flags: u32 = match sc.scan_u32() {
        Some(v) => v,
        None => err("Error reading flags\n"),
    };

    /* Read input length */
    let input_len: usize = match sc.scan_usize() {
        Some(v) => v,
        None => err("Error reading input length\n"),
    };

    if input_len > MAX_BUFFER_SIZE {
        err(&format!(
            "Error: input length {} exceeds maximum {}\n",
            input_len, MAX_BUFFER_SIZE
        ));
    }

    /* Read input buffer data */
    for i in 0..input_len {
        let byte: u32 = match sc.scan_u32() {
            Some(v) => v,
            None => err(&format!("Error reading input byte {}\n", i)),
        };
        mem[INPUT_OFF + i] = byte as u8; /* (char)byte */
    }

    /* Read reference length */
    let ref_len: usize = match sc.scan_usize() {
        Some(v) => v,
        None => err("Error reading reference length\n"),
    };

    if ref_len > MAX_BUFFER_SIZE {
        err(&format!(
            "Error: reference length {} exceeds maximum {}\n",
            ref_len, MAX_BUFFER_SIZE
        ));
    }

    /* Read reference buffer data */
    for i in 0..ref_len {
        let byte: u32 = match sc.scan_u32() {
            Some(v) => v,
            None => err(&format!("Error reading reference byte {}\n", i)),
        };
        mem[REF_OFF + i] = byte as u8; /* (char)byte */
    }

    /* main()'s scalar locals sit directly above input_buffer. */
    store_locals(&mut mem, ref_len, input_len, flags, operation);

    /* Call the library function. A C `char *` into the region is modelled as a
     * slice running from that offset to the end of the region, so reads past a
     * buffer's end see the neighbouring bytes just like the C original. */
    let result = process_strings(
        Some(&mem[INPUT_OFF..]),
        input_len,
        Some(&mem[REF_OFF..]),
        ref_len,
        operation,
        flags,
    );

    /* Print result to stdout */
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(format!("{}\n", result).as_bytes());
    let _ = stdout.flush();
}

/* ------------------------------------------------------------------ */
/* In-crate model tests.
 *
 * These exercise the ported logic directly, without spawning a process. The
 * real correctness check is tests/differential.rs, which runs the CMake-built C
 * binary side by side with this one.
 *
 * Every expected value below was captured from the reference C binary built as
 * c_src/CMakeLists.txt specifies, EXCEPT the two cases marked
 * "residue-dependent": those declare a buffer length with no NUL inside it, so
 * the C code reads pre-main stack residue. The values recorded for them were
 * observed to be stable over 80 consecutive runs on the reference build, but
 * they are not reproducible in general (see ERRORS.md), which is why they are
 * not part of the differential suite.                                 */
/* ------------------------------------------------------------------ */

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive one case the way main() does: parse a whitespace separated
    /// payload, fill the modelled stack, call process_strings.
    fn drive(payload: &str) -> i32 {
        let mut sc = Scanner::from_bytes(payload.as_bytes().to_vec());
        let mut mem = new_stack_region();

        let operation = sc.scan_i32().expect("operation");
        let flags = sc.scan_u32().expect("flags");
        let input_len = sc.scan_usize().expect("input_len");
        assert!(input_len <= MAX_BUFFER_SIZE);
        for i in 0..input_len {
            mem[INPUT_OFF + i] = sc.scan_u32().expect("input byte") as u8;
        }
        let ref_len = sc.scan_usize().expect("ref_len");
        assert!(ref_len <= MAX_BUFFER_SIZE);
        for i in 0..ref_len {
            mem[REF_OFF + i] = sc.scan_u32().expect("ref byte") as u8;
        }
        store_locals(&mut mem, ref_len, input_len, flags, operation);

        process_strings(
            Some(&mem[INPUT_OFF..]),
            input_len,
            Some(&mem[REF_OFF..]),
            ref_len,
            operation,
            flags,
        )
    }

    #[test]
    fn matches_c_reference() {
        let cases: &[(&str, &str, i32)] = &[
            ("op0_valid", "0 0 6 86 65 76 73 68 0 4 82 69 70 0", 1),
            ("op0_exact", "0 0 6 83 84 65 82 84 0 6 83 84 65 82 84 0", 1),
            ("op0_ok", "0 0 3 79 75 0 4 82 69 70 0", 1),
            ("op0_no", "0 0 5 110 111 112 101 0 4 82 69 70 0", 0),
            ("op1_stop", "1 0 5 83 84 79 80 0 1 0", 1),
            /* residue-dependent: buffers with no NUL inside the declared
             * length, so the C code reads pre-main stack residue */
            ("op1_stop_unterminated", "1 0 4 83 84 79 80 1 0", -1),
            ("op1_resume_unterminated", "1 0 6 82 69 83 85 77 69 1 0", 3),
            ("op1_start_space", "1 0 8 83 84 65 82 84 32 120 0 1 0", 0),
            ("op1_admin", "1 0 6 65 68 77 73 78 0 1 0", 99),
            ("op1_none", "1 0 5 110 111 112 101 0 1 0", -1),
            ("op2_prefix", "2 0 7 112 114 101 102 105 120 0 4 112 114 101 0", 1),
            ("op2_exact", "2 1 4 112 114 101 0 4 112 114 101 0", 1),
            ("op2_v1", "2 1 7 112 114 101 95 118 49 0 4 112 114 101 0", 2),
            ("op2_old", "2 1 8 112 114 101 95 111 108 100 0 4 112 114 101 0", 4),
            ("op2_tmp", "2 1 8 112 114 101 95 116 109 112 0 4 112 114 101 0", 6),
            ("op3_colon", "3 0 4 97 58 98 0 2 58 0", 1),
            ("op3_pipe", "3 0 4 120 124 121 0 2 124 0", 1),
            ("op3_none", "3 0 5 78 79 78 69 0 2 124 0", -2),
            ("op3_empty_special", "3 0 6 69 77 80 84 89 0 2 58 0", -3),
            ("op3_default_delim", "3 0 4 97 58 98 0 0", 1),
            ("op3_zero_len", "3 0 0 2 58 0", -1),
            ("op4_cs_exact", "4 2 4 112 97 116 0 4 112 97 116 0", 1),
            ("op4_cs_wild_both", "4 2 6 42 112 97 116 42 0 4 112 97 116 0", 2),
            ("op4_cs_wild_suffix", "4 2 5 112 97 116 42 0 4 112 97 116 0", 3),
            ("op4_cs_wild_prefix", "4 2 5 42 112 97 116 0 4 112 97 116 0", 4),
            ("op4_cs_contains", "4 2 6 120 120 112 97 116 0 4 112 97 116 0", 12),
            ("op4_ci_case", "4 0 4 80 97 84 0 4 112 97 116 0", 6),
            ("op4_ci_prefix", "4 0 5 112 97 116 120 0 4 112 97 116 0", 5),
            ("op4_ci_none", "4 0 4 122 122 122 0 4 112 97 116 0", 0),
            ("op_bad", "5 0 2 120 0 2 120 0", -3),
            ("op_negative", "-1 0 2 120 0 2 120 0", -3),
        ];
        for (name, payload, expected) in cases {
            assert_eq!(drive(payload), *expected, "case {}", name);
        }
    }

    #[test]
    fn compare_prefix_truncates_like_strncpy_strncat() {
        /* prefix of 63 'A's: strncpy fills expected[0..63] completely, so no
         * "_vN" variation can fit and only the plain strcmp can match. */
        let a63 = "65 ".repeat(63);
        let payload = format!("2 1 64 {}0 64 {}0", a63, a63);
        assert_eq!(drive(&payload), 1);

        /* 60 'A's + "_v1" matches variation 0 -> 2 + 0 */
        let a60 = "65 ".repeat(60);
        let payload = format!("2 1 64 {}95 118 49 0 61 {}0", a60, a60);
        assert_eq!(drive(&payload), 2);
    }

    #[test]
    fn scanf_integer_semantics() {
        /* %u applies strtoul semantics: a leading '-' wraps around. */
        let mut sc = Scanner::from_bytes(b"-1".to_vec());
        assert_eq!(sc.scan_u32(), Some(4294967295));

        /* %u then truncation to unsigned int. */
        let mut sc = Scanner::from_bytes(b"4294967296".to_vec());
        assert_eq!(sc.scan_u32(), Some(0));

        /* %d saturates at LONG_MAX and is then truncated to int. */
        let mut sc = Scanner::from_bytes(b"99999999999999999999".to_vec());
        assert_eq!(sc.scan_i32(), Some(-1));

        /* bytes are read with %u and cast to char. */
        let mut sc = Scanner::from_bytes(b"321".to_vec());
        assert_eq!(sc.scan_u32().map(|v| v as u8), Some(65));

        /* whitespace, including newlines, is skipped between conversions. */
        let mut sc = Scanner::from_bytes(b"  \n\t 12\r\n 34".to_vec());
        assert_eq!(sc.scan_usize(), Some(12));
        assert_eq!(sc.scan_usize(), Some(34));

        /* matching failure on non numeric input, and at end of input. */
        let mut sc = Scanner::from_bytes(b"x".to_vec());
        assert_eq!(sc.scan_i32(), None);
        let mut sc = Scanner::from_bytes(b"".to_vec());
        assert_eq!(sc.scan_i32(), None);
        let mut sc = Scanner::from_bytes(b"-".to_vec());
        assert_eq!(sc.scan_i32(), None);
    }
}
