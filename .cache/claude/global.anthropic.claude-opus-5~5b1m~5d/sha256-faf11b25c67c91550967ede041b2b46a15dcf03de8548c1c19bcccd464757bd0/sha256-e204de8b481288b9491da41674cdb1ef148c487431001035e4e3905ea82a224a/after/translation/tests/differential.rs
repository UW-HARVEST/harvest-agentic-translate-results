//! Differential tests: the translated Rust program is executed as a *binary*
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
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&configure.stderr)
            );
            let compile = Command::new("cmake")
                .arg("--build")
                .arg(&build)
                .output()
                .expect("failed to run `cmake --build`");
            assert!(
                compile.status.success(),
                "cmake build failed:\n{}",
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
        "stdout differs for input {:?}\n  C:    {:?}\n  Rust: {:?}",
        input,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        c.stderr,
        rust.stderr,
        "stderr differs for input {:?}\n  C:    {:?}\n  Rust: {:?}",
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
    s.push('\n');
    s
}


/// scanf("%d") hits EOF -> 'Error reading operation'
#[test]
fn empty_stdin() {
    same("");
}

/// scanf skips whitespace, then EOF
#[test]
fn whitespace_only() {
    same("   \n\t\n");
}

/// matching failure on %d
#[test]
fn operation_not_a_number() {
    same("hello\n");
}

/// EOF while reading flags
#[test]
fn operation_only() {
    same("3\n");
}

/// matching failure on %u
#[test]
fn flags_not_a_number() {
    same("3 zzz\n");
}

/// EOF while reading input length
#[test]
fn operation_and_flags_only() {
    same("3 0\n");
}

/// matching failure on %zu
#[test]
fn input_len_not_a_number() {
    same("3 0 abc\n");
}

/// input_len > MAX_BUFFER_SIZE
#[test]
fn input_len_1025() {
    same("3 0 1025 1 0\n");
}

/// %zu accepts '-1' -> SIZE_MAX
#[test]
fn input_len_negative() {
    same("3 0 -1\n");
}

/// strtoul clamps to ULONG_MAX
#[test]
fn input_len_overflow() {
    same("3 0 99999999999999999999\n");
}

/// EOF on the first input byte
#[test]
fn input_byte_missing_at_0() {
    same("1 0 3\n");
}

/// EOF on input byte 2
#[test]
fn input_byte_missing_at_2() {
    same("1 0 3 65 66\n");
}

/// matching failure on input byte 1
#[test]
fn input_byte_not_a_number() {
    same("1 0 2 65 x\n");
}

/// EOF while reading reference length
#[test]
fn ref_len_missing() {
    same("1 0 2 65 66\n");
}

/// matching failure on reference length
#[test]
fn ref_len_not_a_number() {
    same("1 0 1 65 zz\n");
}

/// ref_len > MAX_BUFFER_SIZE
#[test]
fn ref_len_1025() {
    same("1 0 1 65 1025\n");
}

/// %zu accepts '-5' -> SIZE_MAX-4
#[test]
fn ref_len_negative() {
    same("1 0 1 65 -5\n");
}

/// ULONG_MAX reference length
#[test]
fn ref_len_ulong_max() {
    same("1 0 1 65 18446744073709551615\n");
}

/// EOF on reference byte 2
#[test]
fn ref_byte_missing_at_2() {
    same("0 0 1 65 3 65 66\n");
}

/// matching failure on reference byte 1
#[test]
fn ref_byte_not_a_number() {
    same("0 0 1 65 2 65 q\n");
}

/// main never reads past the last byte
#[test]
fn trailing_garbage_is_ignored() {
    same("0 0 3 79 75 0 3 79 75 0 trailing junk\n");
}

/// scanf treats every space character alike
#[test]
fn tab_separated() {
    same("0\t0\t3\t79\t75\t0\t3\t79\t75\t0\n");
}

/// \r is whitespace for scanf
#[test]
fn crlf_separated() {
    same("0\r\n0\r\n3\r\n79\r\n75\r\n0\r\n3\r\n79\r\n75\r\n0\r\n");
}

/// last conversion terminated by EOF
#[test]
fn no_trailing_newline() {
    same("0 0 3 79 75 0 3 79 75 0");
}

/// scanf accepts a leading '+'
#[test]
fn explicit_plus_signs() {
    same("+0 +0 +3 +79 +75 +0 +3 +79 +75 +0\n");
}

/// (char)(unsigned)-1 == 0xff
#[test]
fn byte_value_negative() {
    same("0 0 2 -1 0 2 255 0");
}

/// (char)321 == 'A'
#[test]
fn byte_value_321_truncated() {
    same("0 0 2 321 0 2 65 0");
}

/// %u truncates to 32 bits, (char) to 8
#[test]
fn byte_value_2_pow_32_plus_1() {
    same("0 0 2 4294967297 0 2 1 0");
}

/// flags truncated to 32 bits -> bit0 set
#[test]
fn flags_2_pow_32_plus_1() {
    same("2 4294967297 3 65 66 0 3 65 66 0");
}

/// every flag bit set
#[test]
fn flags_all_bits_set() {
    same("2 4294967295 3 65 66 0 3 65 66 0");
}

/// default: -> -3
#[test]
fn operation_5_default() {
    same("5 0 0 0\n");
}

/// negative operation -> -3
#[test]
fn operation_minus_1() {
    same("-1 0 0 0\n");
}

/// INT_MAX -> -3
#[test]
fn operation_int_max() {
    same("2147483647 0 0 0\n");
}

/// INT_MIN -> -3
#[test]
fn operation_int_min() {
    same("-2147483648 0 0 0\n");
}

/// LONG_MAX truncated to int == -1 -> -3
#[test]
fn operation_overflow_wraps_to_minus_1() {
    same("99999999999999999999 0 0 0\n");
}

/// strcmp(token, expected) == 0 -> 1
#[test]
fn op0_token_equals_reference() {
    same(&encode("0", "0", &nul(b"SECRET"), &nul(b"SECRET")));
}

/// no match -> 0
#[test]
fn op0_token_differs() {
    same(&encode("0", "0", &nul(b"SECRET"), &nul(b"OTHER")));
}

/// second strcmp -> 1
#[test]
fn op0_literal_valid() {
    same(&encode("0", "0", &nul(b"VALID"), &nul(b"nope")));
}

/// third strcmp -> 1
#[test]
fn op0_literal_ok() {
    same(&encode("0", "0", &nul(b"OK"), &nul(b"nope")));
}

/// "" == "" -> 1
#[test]
fn op0_both_empty_strings() {
    same(&encode("0", "0", &nul(b""), &nul(b"")));
}

/// shorter -> 0
#[test]
fn op0_token_is_prefix_of_reference() {
    same(&encode("0", "0", &nul(b"SEC"), &nul(b"SECRET")));
}

/// both strcmp operands are pure stack residue
#[test]
fn op0_zero_length_buffers() {
    same(&encode("0", "0", &[], &[]));
}

/// strcmp runs into the residue of both buffers (both terminate at offset 6)
#[test]
fn op0_unterminated_equal_payloads() {
    same(&encode("0", "0", b"RESUME", b"RESUME"));
}

/// token continues past its data
#[test]
fn op0_unterminated_token_only() {
    same(&encode("0", "0", b"VALID", &nul(b"VALID")));
}

/// maximum length for both buffers
#[test]
fn op0_full_input_buffer() {
    same(&encode("0", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

/// strlen(input) runs into main()'s locals
#[test]
fn op0_full_input_empty_reference() {
    same(&encode("0", "0", b"BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB", &[]));
}

/// strcmp runs off the end of ref_buffer into input_buffer
#[test]
fn op0_full_reference_buffer() {
    same(&encode("0", "0", &nul(b"Z"), b"ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ"));
}

/// buffer[cmd_len] == 0 -> 0
#[test]
fn op1_start_nul_terminated() {
    same(&encode("1", "0", &nul(b"START"), &[]));
}

/// buffer[cmd_len] == ' ' -> 0
#[test]
fn op1_start_followed_by_space() {
    same(&encode("1", "0", &nul(b"START arg"), &[]));
}

/// buffer[cmd_len] is stack residue
#[test]
fn op1_start_unterminated() {
    same(&encode("1", "0", b"START", &[]));
}

/// buf_size < cmd_len, strcmp fallback
#[test]
fn op1_start_one_byte_short() {
    same(&encode("1", "0", b"STAR", &[]));
}

/// buffer[cmd_len] == 0 -> 1
#[test]
fn op1_stop_nul_terminated() {
    same(&encode("1", "0", &nul(b"STOP"), &[]));
}

/// buffer[cmd_len] == ' ' -> 1
#[test]
fn op1_stop_followed_by_space() {
    same(&encode("1", "0", &nul(b"STOP arg"), &[]));
}

/// buf_size < cmd_len, strcmp fallback
#[test]
fn op1_stop_one_byte_short() {
    same(&encode("1", "0", b"STO", &[]));
}

/// buffer[cmd_len] == 0 -> 2
#[test]
fn op1_pause_nul_terminated() {
    same(&encode("1", "0", &nul(b"PAUSE"), &[]));
}

/// buffer[cmd_len] == ' ' -> 2
#[test]
fn op1_pause_followed_by_space() {
    same(&encode("1", "0", &nul(b"PAUSE arg"), &[]));
}

/// buffer[cmd_len] is stack residue
#[test]
fn op1_pause_unterminated() {
    same(&encode("1", "0", b"PAUSE", &[]));
}

/// buf_size < cmd_len, strcmp fallback
#[test]
fn op1_pause_one_byte_short() {
    same(&encode("1", "0", b"PAUS", &[]));
}

/// buffer[cmd_len] == 0 -> 3
#[test]
fn op1_resume_nul_terminated() {
    same(&encode("1", "0", &nul(b"RESUME"), &[]));
}

/// buffer[cmd_len] == ' ' -> 3
#[test]
fn op1_resume_followed_by_space() {
    same(&encode("1", "0", &nul(b"RESUME arg"), &[]));
}

/// buffer[cmd_len] is stack residue
#[test]
fn op1_resume_unterminated() {
    same(&encode("1", "0", b"RESUME", &[]));
}

/// buf_size < cmd_len, strcmp fallback
#[test]
fn op1_resume_one_byte_short() {
    same(&encode("1", "0", b"RESUM", &[]));
}

/// buffer[cmd_len] == 0 -> 4
#[test]
fn op1_reset_nul_terminated() {
    same(&encode("1", "0", &nul(b"RESET"), &[]));
}

/// buffer[cmd_len] == ' ' -> 4
#[test]
fn op1_reset_followed_by_space() {
    same(&encode("1", "0", &nul(b"RESET arg"), &[]));
}

/// buffer[cmd_len] is stack residue
#[test]
fn op1_reset_unterminated() {
    same(&encode("1", "0", b"RESET", &[]));
}

/// buf_size < cmd_len, strcmp fallback
#[test]
fn op1_reset_one_byte_short() {
    same(&encode("1", "0", b"RESE", &[]));
}

/// special admin command -> 99
#[test]
fn op1_admin() {
    same(&encode("1", "0", &nul(b"ADMIN"), &[]));
}

/// strcmp("ADMIN ", "ADMIN") != 0 -> -1
#[test]
fn op1_admin_with_space() {
    same(&encode("1", "0", &nul(b"ADMIN "), &[]));
}

/// no match -> -1
#[test]
fn op1_unknown_command() {
    same(&encode("1", "0", &nul(b"NOPE"), &[]));
}

/// empty command -> -1
#[test]
fn op1_empty_string() {
    same(&encode("1", "0", &nul(b""), &[]));
}

/// buf_size 0, every strncmp skipped
#[test]
fn op1_zero_length() {
    same(&encode("1", "0", &[], &[]));
}

/// buffer[5] == 'X' -> -1
#[test]
fn op1_start_with_extra_char() {
    same(&encode("1", "0", &nul(b"STARTX"), &[]));
}

/// prefix of STOP but buffer[4] == 'P' -> -1
#[test]
fn op1_stopped() {
    same(&encode("1", "0", &nul(b"STOPPED"), &[]));
}

/// 1024 bytes without a terminator
#[test]
fn op1_full_buffer() {
    same(&encode("1", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &[]));
}

/// buf_size == cmd_len, buffer[5] is residue
#[test]
fn op1_start_exactly_5_bytes() {
    same(&encode("1", "0", b"START", &[]));
}

/// RESET followed by ' ' -> 4
#[test]
fn op1_reset_space_nul() {
    same(&encode("1", "0", &nul(b"RESET "), &[]));
}

/// strncmp prefix -> 1
#[test]
fn op2_prefix_matches() {
    same(&encode("2", "0", &nul(b"HELLOWORLD"), &nul(b"HELLO")));
}

/// -> 0
#[test]
fn op2_prefix_does_not_match() {
    same(&encode("2", "0", &nul(b"HELLOWORLD"), &nul(b"WORLD")));
}

/// prefix_len 0, strncmp(_,_,0) == 0 -> 1
#[test]
fn op2_prefix_empty_reference() {
    same(&encode("2", "0", &nul(b"HELLO"), &nul(b"")));
}

/// flags bit0 -> strcmp -> 1
#[test]
fn op2_exact_equal() {
    same(&encode("2", "1", &nul(b"HELLO"), &nul(b"HELLO")));
}

/// no variation matches -> 0
#[test]
fn op2_exact_not_equal() {
    same(&encode("2", "1", &nul(b"HELLOX"), &nul(b"HELLO")));
}

/// variation 0 -> 2
#[test]
fn op2_exact_variation_v1() {
    same(&encode("2", "1", &nul(b"BASE_v1"), &nul(b"BASE")));
}

/// variation 1 -> 3
#[test]
fn op2_exact_variation_v2() {
    same(&encode("2", "1", &nul(b"BASE_v2"), &nul(b"BASE")));
}

/// variation 2 -> 4
#[test]
fn op2_exact_variation_old() {
    same(&encode("2", "1", &nul(b"BASE_old"), &nul(b"BASE")));
}

/// variation 3 -> 5
#[test]
fn op2_exact_variation_new() {
    same(&encode("2", "1", &nul(b"BASE_new"), &nul(b"BASE")));
}

/// variation 4 -> 6
#[test]
fn op2_exact_variation_tmp() {
    same(&encode("2", "1", &nul(b"BASE_tmp"), &nul(b"BASE")));
}

/// -> 0
#[test]
fn op2_exact_unknown_variation() {
    same(&encode("2", "1", &nul(b"BASE_xx"), &nul(b"BASE")));
}

/// only bit0 selects exact matching
#[test]
fn op2_exact_flags_bit0_and_bit1() {
    same(&encode("2", "3", &nul(b"BASE_v2"), &nul(b"BASE")));
}

/// bit0 clear -> prefix mode
#[test]
fn op2_prefix_flags_bit1_only() {
    same(&encode("2", "2", &nul(b"HELLOWORLD"), &nul(b"HELLO")));
}

/// 62 + 3 fits into expected[64]
#[test]
fn op2_exact_prefix_62_bytes() {
    same(&encode("2", "1", &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP_v1"), &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP")));
}

/// strncat has no room left
#[test]
fn op2_exact_prefix_63_bytes() {
    same(&encode("2", "1", &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP_v1"), &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP")));
}

/// strncpy truncates the prefix to 63 bytes
#[test]
fn op2_exact_prefix_64_bytes() {
    same(&encode("2", "1", &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP"), &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP")));
}

/// prefix longer than expected[]
#[test]
fn op2_exact_prefix_70_bytes() {
    same(&encode("2", "1", &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP"), &nul(b"PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP")));
}

/// strncat truncates '_tmp' to '_tm' -> 0
#[test]
fn op2_exact_prefix_60_plus_tmp() {
    same(&encode("2", "1", &nul(b"QQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ_tmp"), &nul(b"QQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQQ")));
}

/// expected[] built from residue
#[test]
fn op2_exact_zero_length_reference() {
    same(&encode("2", "1", &nul(b"HELLO"), &[]));
}

/// "" == "" -> 1
#[test]
fn op2_exact_both_empty() {
    same(&encode("2", "1", &nul(b""), &nul(b"")));
}

/// prefix_len 0 -> 1
#[test]
fn op2_prefix_both_empty() {
    same(&encode("2", "0", &nul(b""), &nul(b"")));
}

/// 1024 byte input, 100 byte prefix
#[test]
fn op2_prefix_full_input() {
    same(&encode("2", "0", b"CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC", &nul(b"CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC")));
}

/// strlen(prefix) runs into input_buffer
#[test]
fn op2_prefix_full_reference() {
    same(&encode("2", "0", &nul(b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"), b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

/// strcmp over both full buffers
#[test]
fn op2_exact_full_both() {
    same(&encode("2", "1", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

/// len == 0 -> -1
#[test]
fn op3_zero_length() {
    same(&encode("3", "0", &[], &nul(b":")));
}

/// -> 0
#[test]
fn op3_delimiter_at_0() {
    same(&encode("3", "0", &nul(b":abc"), &nul(b":")));
}

/// -> 3
#[test]
fn op3_delimiter_at_3() {
    same(&encode("3", "0", &nul(b"abc:def"), &nul(b":")));
}

/// -> -1
#[test]
fn op3_delimiter_absent() {
    same(&encode("3", "0", &nul(b"abcdef"), &nul(b":")));
}

/// scan breaks at the NUL -> -1
#[test]
fn op3_nul_before_delimiter() {
    same(&encode("3", "0", &[97, 98, 99, 0, 58], &nul(b":")));
}

/// ref_len == 0 -> delim ':'
#[test]
fn op3_default_delimiter_ref_len_0() {
    same(&encode("3", "0", &nul(b"ab:cd"), &[]));
}

/// reference[0] == 0 -> delim '\0'
#[test]
fn op3_delimiter_is_nul_of_reference() {
    same(&encode("3", "0", &nul(b"ab:cd"), &[0, 124]));
}

/// delim '|' and "NONE" -> -2
#[test]
fn op3_pipe_none_special() {
    same(&encode("3", "0", &nul(b"NONE"), &nul(b"|")));
}

/// delim ':' and "EMPTY" -> -3
#[test]
fn op3_colon_empty_special() {
    same(&encode("3", "0", &nul(b"EMPTY"), &nul(b":")));
}

/// strcmp runs into residue -> -1
#[test]
fn op3_colon_empty_unterminated() {
    same(&encode("3", "0", b"EMPTY", &nul(b":")));
}

/// wrong delimiter for the special case -> -1
#[test]
fn op3_pipe_with_empty_text() {
    same(&encode("3", "0", &nul(b"EMPTY"), &nul(b"|")));
}

/// wrong delimiter for the special case -> -1
#[test]
fn op3_colon_with_none_text() {
    same(&encode("3", "0", &nul(b"NONE"), &nul(b":")));
}

/// -> 1
#[test]
fn op3_pipe_found() {
    same(&encode("3", "0", &nul(b"a|b"), &nul(b"|")));
}

/// delim '\0' matches the terminator -> 3
#[test]
fn op3_delimiter_nul_byte() {
    same(&encode("3", "0", &nul(b"abc"), &nul(b"")));
}

/// only reference[0] is used
#[test]
fn op3_multi_byte_reference() {
    same(&encode("3", "0", &nul(b"a#b"), &nul(b"#!")));
}

/// -> 1023
#[test]
fn op3_delimiter_at_last_index() {
    same(&encode("3", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA:", &nul(b":")));
}

/// scan all 1024 bytes -> -1
#[test]
fn op3_full_buffer_no_delimiter() {
    same(&encode("3", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &nul(b":")));
}

/// break on data[0] == 0 -> -1
#[test]
fn op3_leading_nul() {
    same(&encode("3", "0", &[0, 58, 58], &nul(b":")));
}

/// len 1, data[0] == 0 -> -1
#[test]
fn op3_single_nul() {
    same(&encode("3", "0", &nul(b""), &nul(b":")));
}

/// strcmp -> 1
#[test]
fn op4_ci_exact_equal() {
    same(&encode("4", "0", &nul(b"Hello"), &nul(b"Hello")));
}

/// manual tolower compare -> 6
#[test]
fn op4_ci_case_folded_equal() {
    same(&encode("4", "0", &nul(b"HELLO"), &nul(b"hello")));
}

/// text_len != pattern_len, strncmp -> 5
#[test]
fn op4_ci_prefix() {
    same(&encode("4", "0", &nul(b"HELLOWORLD"), &nul(b"HELLO")));
}

/// strncmp is case sensitive -> 0
#[test]
fn op4_ci_prefix_wrong_case() {
    same(&encode("4", "0", &nul(b"helloworld"), &nul(b"HELLO")));
}

/// -> 0
#[test]
fn op4_ci_same_length_no_match() {
    same(&encode("4", "0", &nul(b"abcde"), &nul(b"vwxyz")));
}

/// -> 0
#[test]
fn op4_ci_different_length_no_match() {
    same(&encode("4", "0", &nul(b"abcde"), &nul(b"zz")));
}

/// pattern_len 0 -> strncmp(_,_,0) -> 5
#[test]
fn op4_ci_empty_pattern() {
    same(&encode("4", "0", &nul(b"abc"), &nul(b"")));
}

/// strcmp -> 1
#[test]
fn op4_ci_both_empty() {
    same(&encode("4", "0", &nul(b""), &nul(b"")));
}

/// bytes around 'A'-'Z' are not folded
#[test]
fn op4_ci_non_alphabetic_bytes() {
    same(&encode("4", "0", &nul(b"@[`{"), &nul(b"@[`{")));
}

/// '[' + 32 == '{' must not fold -> 0
#[test]
fn op4_ci_z_vs_brace() {
    same(&encode("4", "0", &nul(b"Z["), &nul(b"z{")));
}

/// 1000 byte case-insensitive match -> 6
#[test]
fn op4_ci_full_length_folded() {
    same(&encode("4", "0", &nul(b"KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK"), &nul(b"kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk")));
}

/// no size_t underflow on this path -> 0
#[test]
fn op4_ci_pattern_longer() {
    same(&encode("4", "0", &nul(b"zz"), &nul(b"PATTERNTOOLONGXYZ")));
}

/// flags bit1 -> strcmp -> 1
#[test]
fn op4_cs_exact_equal() {
    same(&encode("4", "2", &nul(b"Hello"), &nul(b"Hello")));
}

/// snprintf "*%s*" -> 2
#[test]
fn op4_cs_wildcard_surrounded() {
    same(&encode("4", "2", &nul(b"*pat*"), &nul(b"pat")));
}

/// snprintf "%s*" -> 3
#[test]
fn op4_cs_wildcard_suffix() {
    same(&encode("4", "2", &nul(b"pat*"), &nul(b"pat")));
}

/// snprintf "*%s" -> 4
#[test]
fn op4_cs_wildcard_prefix() {
    same(&encode("4", "2", &nul(b"*pat"), &nul(b"pat")));
}

/// -> 10
#[test]
fn op4_cs_substring_at_0() {
    same(&encode("4", "2", &nul(b"abcdef"), &nul(b"abc")));
}

/// -> 11
#[test]
fn op4_cs_substring_at_1() {
    same(&encode("4", "2", &nul(b"xabcx"), &nul(b"abc")));
}

/// -> 12
#[test]
fn op4_cs_substring_at_2() {
    same(&encode("4", "2", &nul(b"xxabc"), &nul(b"abc")));
}

/// -> 50
#[test]
fn op4_cs_substring_at_40() {
    same(&encode("4", "2", &nul(b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxabc"), &nul(b"abc")));
}

/// loop finishes -> 0
#[test]
fn op4_cs_no_substring() {
    same(&encode("4", "2", &nul(b"abcdef"), &nul(b"xyz")));
}

/// text_len == pattern_len, one iteration -> 0
#[test]
fn op4_cs_same_length_no_match() {
    same(&encode("4", "2", &nul(b"abc"), &nul(b"xyz")));
}

/// pattern_len 0 -> 10
#[test]
fn op4_cs_empty_pattern() {
    same(&encode("4", "2", &nul(b"abc"), &nul(b"")));
}

/// strcmp -> 1
#[test]
fn op4_cs_both_empty() {
    same(&encode("4", "2", &nul(b""), &nul(b"")));
}

/// bit0 is ignored by operation 4 -> 12
#[test]
fn op4_cs_flags_bit0_and_bit1() {
    same(&encode("4", "3", &nul(b"xxabc"), &nul(b"abc")));
}

/// "*%s*" fills exactly 63 bytes
#[test]
fn op4_cs_wildcard_61_bytes() {
    same(&encode("4", "2", &nul(b"*yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy*"), &nul(b"yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy")));
}

/// snprintf truncates the trailing '*'
#[test]
fn op4_cs_wildcard_62_bytes() {
    same(&encode("4", "2", &nul(b"*yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"), &nul(b"yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy")));
}

/// the pattern itself is truncated
#[test]
fn op4_cs_wildcard_63_bytes() {
    same(&encode("4", "2", &nul(b"*yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"), &nul(b"yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy")));
}

/// text_len - pattern_len underflows -> SIGSEGV
#[test]
fn op4_cs_pattern_longer_underflows() {
    same(&encode("4", "2", b"A", &nul(b"ABCDEFGHIJ")));
}

/// same underflow with a terminated text
#[test]
fn op4_cs_pattern_longer_underflows_2() {
    same(&encode("4", "2", &nul(b"zz"), &nul(b"PATTERNTOOLONGXYZ")));
}

/// ref_buffer runs into input_buffer: pattern_len 2048 > text_len 1024
#[test]
fn op4_cs_full_buffers_underflow() {
    same(&encode("4", "2", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

/// strlen(input) reads ref_len from the stack frame
#[test]
fn op4_ci_full_input_8_byte_pattern() {
    same(&encode("4", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", b"AAAAAAAA"));
}

/// pattern_len 2048 != text_len 1024 -> 0
#[test]
fn op4_ci_full_both_buffers() {
    same(&encode("4", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

/// ref_len 0x101 leaks two non-zero bytes
#[test]
fn op4_ci_full_input_257_byte_pattern() {
    same(&encode("4", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

/// the two byte pattern 'A' 0x08 exists only where input_buffer ends and the little endian ref_len local begins: 10 + 1023 == 1033
#[test]
fn op4_cs_locals_leak_after_input_buffer() {
    same(&encode("4", "2", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", &[65, 8, 0, 1, 1, 1, 1, 1]));
}

/// strlen(prefix) is 2048 because ref_buffer is immediately followed by input_buffer
#[test]
fn op2_prefix_len_crosses_into_input_buffer() {
    same(&encode("2", "0", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"));
}

/// %u stops at 'x' -> failure on byte 1
#[test]
fn byte_written_as_hex_literal() {
    same("0 0 2 0x41 0 2 65 0");
}

/// leading zeros are accepted by %u
#[test]
fn leading_zeros_everywhere() {
    same("0000 0000 0003 079 075 000 0003 079 075 000\n");
}

/// ULONG_MAX -> (char)0xff
#[test]
fn byte_overflow_becomes_0xff() {
    same("0 0 1 99999999999999999999 2 255 0");
}

/// %d skips leading whitespace
#[test]
fn operation_with_leading_whitespace() {
    same("\n\n\t  0 0 3 79 75 0 3 79 75 0\n");
}

/// '-0' is a valid %d -> operation 0
#[test]
fn negative_zero_operation() {
    same("-0 0 0 0\n");
}

/// one byte input holding just the terminator
#[test]
fn op0_single_byte_nul() {
    same(&encode("0", "0", &nul(b""), b"A"));
}

/// single unterminated byte in both buffers
#[test]
fn op0_single_byte_a() {
    same(&encode("0", "0", b"A", b"A"));
}

/// strcmp stops at the embedded NUL -> 1
#[test]
fn op0_embedded_nul() {
    same(&encode("0", "0", &[65, 66, 0, 67, 68, 0], &[65, 66, 0, 67, 68, 0]));
}

/// the tail behind the NUL is invisible -> 1
#[test]
fn op0_embedded_nul_differing_tail() {
    same(&encode("0", "0", &[65, 66, 0, 67, 68, 0], &[65, 66, 0, 90, 90, 0]));
}

/// buffer[5] == 0 -> 0
#[test]
fn op1_start_embedded_nul_tail() {
    same(&encode("1", "0", &[83, 84, 65, 82, 84, 0, 88, 89, 90, 0], &[]));
}

/// full buffer, terminator at index 5 -> 0
#[test]
fn op1_start_padded_with_nuls() {
    same(&encode("1", "0", &[83, 84, 65, 82, 84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], &[]));
}

/// no command starts with a space -> -1
#[test]
fn op1_space_only() {
    same(&encode("1", "0", &nul(b" "), &[]));
}

/// strcmp ignores everything past the NUL -> 2
#[test]
fn op2_exact_embedded_nul() {
    same(&encode("2", "1", &[66, 65, 83, 69, 95, 118, 49, 0, 116, 97, 105, 108, 0], &nul(b"BASE")));
}

/// delimiter 0xff -> 1
#[test]
fn op3_delim_255() {
    same(&encode("3", "0", &[65, 255, 66, 0], &[255u8; 1]));
}

/// delimiter '\n' -> 2
#[test]
fn op3_delim_newline_byte() {
    same(&encode("3", "0", &[97, 98, 10, 99, 100, 0], &[10u8; 1]));
}

/// delimiter ' ' -> 2
#[test]
fn op3_delim_space_byte() {
    same(&encode("3", "0", &nul(b"ab cd"), b" "));
}

/// break at index 0 -> -1
#[test]
fn op3_all_nul_buffer() {
    same(&encode("3", "0", &[0u8; 16], b"A"));
}

/// 10 + 500 -> 510
#[test]
fn op4_cs_substring_at_500() {
    same(&encode("4", "2", &nul(b"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxabc"), &nul(b"abc")));
}

/// match at text_len - 1 -> 15
#[test]
fn op4_cs_substring_last_position() {
    same(&encode("4", "2", &nul(b"abcdef"), &nul(b"f")));
}

/// bytes >= 0x80 are compared as chars
#[test]
fn op4_ci_high_bytes() {
    same(&encode("4", "0", &[200, 201, 0], &[200, 201, 0]));
}

/// 'A' folds to 'a' -> 6
#[test]
fn op4_ci_pattern_equals_text_len_1() {
    same(&encode("4", "0", &nul(b"A"), &nul(b"a")));
}

/// single strncmp iteration -> 0
#[test]
fn op4_cs_text_len_equals_pattern_len() {
    same(&encode("4", "2", &nul(b"AB"), &nul(b"AC")));
}

/// A fixed pseudo random cross check over the whole input space:
/// every operation, every flag combination and terminated as well as
/// unterminated payloads of every interesting shape.
#[test]
fn randomized_corpus() {
    for input in CORPUS {
        same(input);
    }
}

const CORPUS: [&str; 140] = [
    "1 4 7 66 65 83 69 95 118 49 3 32 32 0\n",
    "1 6 4 97 66 99 0 6 82 69 83 69 84 0\n",
    "4 2 12 104 101 108 108 111 32 119 111 114 108 100 0 6 86 65 76 73 68 0\n",
    "-2 1 8 83 84 65 82 84 32 120 0 7 97 98 99 100 101 102 0\n",
    "-2 2 7 97 98 99 100 101 102 0 2 65 0\n",
    "3 2 6 80 65 85 83 69 0 7 82 69 83 85 77 69 0\n",
    "3 2 4 97 66 99 0 4 97 98 99 0\n",
    "1 4 4 97 124 98 0 5 78 79 78 69 0\n",
    "1 3 3 32 32 0 6 82 69 83 69 84 0\n",
    "4 6 4 120 58 121 0 4 1 2 3 0\n",
    "1 3 7 82 69 83 85 77 69 0 6 65 68 77 73 78 0\n",
    "3 2 7 65 68 77 73 78 49 50 4 120 58 121 0\n",
    "0 0 2 65 0 3 255 254 0\n",
    "3 1 8 65 68 77 73 78 49 50 0 6 86 65 76 73 68 0\n",
    "2 6 6 80 65 85 83 69 0 1 0\n",
    "5 6 3 124 124 0 1 0\n",
    "4 0 5 66 65 83 69 0 5 78 79 78 69 0\n",
    "4 1 6 69 77 80 84 89 0 3 32 32 0\n",
    "0 0 6 69 77 80 84 89 0 3 255 254 0\n",
    "3 0 4 97 98 99 0 6 82 69 83 85 77 69\n",
    "4 3 7 83 84 65 82 84 32 0 7 97 98 99 100 101 102 0\n",
    "5 3 7 97 98 99 100 101 102 0 12 104 101 108 108 111 32 119 111 114 108 100 0\n",
    "5 0 6 82 69 83 69 84 0 4 65 66 67 0\n",
    "-2 6 5 83 84 79 80 0 6 82 69 83 85 77 69\n",
    "4 1 6 65 68 77 73 78 0 6 80 65 85 83 69 0\n",
    "0 0 6 83 84 79 80 88 0 4 97 124 98 0\n",
    "2 6 4 65 66 67 0 6 97 98 99 100 101 102\n",
    "3 3 6 82 69 83 69 84 0 7 66 65 83 69 95 118 49\n",
    "4 4 5 66 65 83 69 0 3 79 75 0\n",
    "2 0 7 83 84 65 82 84 32 120 8 65 68 77 73 78 49 50 0\n",
    "4 6 5 78 79 78 69 0 4 97 66 99 0\n",
    "4 1 4 97 98 99 0 6 83 84 65 82 84 0\n",
    "-2 0 3 79 75 0 6 83 84 65 82 84 0\n",
    "0 3 7 65 68 77 73 78 49 50 6 83 84 65 82 84 0\n",
    "4 1 8 66 65 83 69 95 118 49 0 6 86 65 76 73 68 0\n",
    "5 4 4 97 66 99 0 6 83 84 65 82 84 0\n",
    "0 3 3 32 32 0 6 80 65 85 83 69 0\n",
    "3 2 5 83 84 79 80 0 2 65 0\n",
    "1 4 3 255 254 0 6 83 84 65 82 84 0\n",
    "0 4 6 65 68 77 73 78 0 4 97 98 99 0\n",
    "3 0 4 120 58 121 0 2 65 0\n",
    "3 2 4 97 66 99 0 6 83 84 79 80 88 0\n",
    "-2 0 7 82 69 83 85 77 69 0 7 97 98 99 100 101 102 0\n",
    "5 2 7 83 84 65 82 84 32 120 9 66 65 83 69 95 116 109 112 0\n",
    "4 2 8 66 65 83 69 95 118 49 0 3 124 124 0\n",
    "0 4 4 1 2 3 0 1 0\n",
    "4 1 4 65 66 67 0 6 83 84 65 82 84 0\n",
    "3 4 7 83 84 65 82 84 32 120 7 82 69 83 85 77 69 0\n",
    "-2 0 7 82 69 83 85 77 69 0 6 83 84 65 82 84 0\n",
    "-2 1 3 32 32 0 7 82 69 83 85 77 69 0\n",
    "4 1 6 97 98 99 100 101 102 7 66 65 83 69 95 118 49\n",
    "-2 1 5 83 84 79 80 0 6 82 69 83 69 84 0\n",
    "-2 0 3 255 254 0 4 97 98 99 0\n",
    "2 1 6 86 65 76 73 68 0 6 69 77 80 84 89 0\n",
    "1 3 2 65 0 6 86 65 76 73 68 0\n",
    "4 2 8 65 68 77 73 78 49 50 0 6 83 84 79 80 88 0\n",
    "5 1 4 1 2 3 0 6 69 77 80 84 89 0\n",
    "4 0 3 124 124 0 2 65 0\n",
    "4 3 8 66 65 83 69 95 118 49 0 7 97 98 99 100 101 102 0\n",
    "1 0 5 83 84 79 80 0 1 0\n",
    "3 4 3 58 58 0 6 80 65 85 83 69 0\n",
    "4 0 7 97 98 99 100 101 102 0 6 83 84 65 82 84 0\n",
    "3 1 6 97 98 99 100 101 102 3 255 254 0\n",
    "0 3 3 79 75 0 6 83 84 65 82 84 32\n",
    "0 1 5 66 65 83 69 0 7 83 84 65 82 84 32 0\n",
    "3 4 4 97 98 99 0 3 58 58 0\n",
    "0 1 7 97 98 99 100 101 102 0 9 66 65 83 69 95 116 109 112 0\n",
    "3 0 6 82 69 83 85 77 69 9 66 65 83 69 95 116 109 112 0\n",
    "1 6 5 83 84 79 80 0 7 83 84 65 82 84 32 120\n",
    "0 6 7 97 98 99 100 101 102 0 5 83 84 79 80 0\n",
    "0 1 1 0 4 1 2 3 0\n",
    "0 6 8 83 84 65 82 84 32 120 0 7 82 69 83 85 77 69 0\n",
    "1 3 6 82 69 83 69 84 0 8 83 84 65 82 84 32 120 0\n",
    "3 4 6 82 69 83 69 84 0 8 66 65 83 69 95 118 49 0\n",
    "1 1 6 69 77 80 84 89 0 6 83 84 65 82 84 0\n",
    "1 3 4 1 2 3 0 3 58 58 0\n",
    "4 4 6 65 68 77 73 78 0 6 86 65 76 73 68 0\n",
    "5 1 3 32 32 0 7 97 98 99 100 101 102 0\n",
    "3 4 1 0 6 69 77 80 84 89 0\n",
    "-2 6 5 83 84 79 80 0 5 78 79 78 69 0\n",
    "5 2 3 255 254 0 5 78 79 78 69 0\n",
    "2 4 12 104 101 108 108 111 32 119 111 114 108 100 0 6 82 69 83 69 84 0\n",
    "0 6 8 66 65 83 69 95 118 49 0 5 83 84 79 80 0\n",
    "2 2 3 32 32 0 4 97 98 99 0\n",
    "4 2 6 69 77 80 84 89 0 4 1 2 3 0\n",
    "2 0 4 97 124 98 0 6 86 65 76 73 68 0\n",
    "3 3 4 97 124 98 0 3 58 58 0\n",
    "3 4 4 65 66 67 0 6 82 69 83 69 84 0\n",
    "-2 4 7 83 84 65 82 84 32 0 5 66 65 83 69 0\n",
    "4 1 4 120 58 121 0 3 32 32 0\n",
    "4 0 7 97 98 99 100 101 102 0 5 78 79 78 69 0\n",
    "-2 3 3 79 75 0 7 83 84 65 82 84 32 0\n",
    "-2 3 7 97 98 99 100 101 102 0 4 97 66 99 0\n",
    "2 3 4 97 66 99 0 6 83 84 65 82 84 0\n",
    "5 4 6 86 65 76 73 68 0 2 65 0\n",
    "5 4 6 82 69 83 85 77 69 5 78 79 78 69 0\n",
    "0 3 6 83 84 79 80 88 0 7 65 68 77 73 78 49 50\n",
    "0 6 9 66 65 83 69 95 116 109 112 0 4 97 124 98 0\n",
    "5 1 9 66 65 83 69 95 116 109 112 0 4 120 58 121 0\n",
    "4 1 6 83 84 65 82 84 0 4 97 98 99 0\n",
    "-2 3 3 32 32 0 3 58 58 0\n",
    "0 0 6 83 84 65 82 84 32 6 65 68 77 73 78 0\n",
    "4 2 12 104 101 108 108 111 32 119 111 114 108 100 0 4 65 66 67 0\n",
    "0 3 1 0 2 65 0\n",
    "1 4 4 97 124 98 0 4 1 2 3 0\n",
    "1 2 8 83 84 65 82 84 32 120 0 7 66 65 83 69 95 118 49\n",
    "4 1 8 65 68 77 73 78 49 50 0 3 124 124 0\n",
    "4 2 9 66 65 83 69 95 116 109 112 0 2 65 0\n",
    "4 1 3 255 254 0 3 58 58 0\n",
    "2 1 7 82 69 83 85 77 69 0 7 66 65 83 69 95 118 49\n",
    "-2 2 3 58 58 0 4 120 58 121 0\n",
    "3 6 6 83 84 79 80 88 0 7 66 65 83 69 95 118 49\n",
    "4 1 5 83 84 79 80 0 6 83 84 65 82 84 0\n",
    "4 3 8 83 84 65 82 84 32 120 0 4 97 124 98 0\n",
    "1 4 12 104 101 108 108 111 32 119 111 114 108 100 0 9 66 65 83 69 95 116 109 112 0\n",
    "-2 3 4 97 124 98 0 6 65 68 77 73 78 0\n",
    "4 0 4 97 124 98 0 7 82 69 83 85 77 69 0\n",
    "1 1 5 83 84 79 80 0 2 65 0\n",
    "2 0 4 1 2 3 0 12 104 101 108 108 111 32 119 111 114 108 100 0\n",
    "0 0 7 97 98 99 100 101 102 0 1 0\n",
    "5 0 7 82 69 83 85 77 69 0 3 32 32 0\n",
    "2 2 6 86 65 76 73 68 0 4 97 66 99 0\n",
    "4 6 6 83 84 65 82 84 0 4 120 58 121 0\n",
    "3 0 4 97 66 99 0 1 0\n",
    "5 3 1 0 7 65 68 77 73 78 49 50\n",
    "1 0 7 82 69 83 85 77 69 0 6 83 84 65 82 84 0\n",
    "3 2 8 66 65 83 69 95 118 49 0 5 66 65 83 69 0\n",
    "2 3 3 32 32 0 12 104 101 108 108 111 32 119 111 114 108 100 0\n",
    "5 1 3 255 254 0 6 82 69 83 85 77 69\n",
    "0 2 7 97 98 99 100 101 102 0 5 66 65 83 69 0\n",
    "5 1 6 97 98 99 100 101 102 4 97 124 98 0\n",
    "5 6 8 66 65 83 69 95 118 49 0 7 97 98 99 100 101 102 0\n",
    "4 1 4 120 58 121 0 3 124 124 0\n",
    "1 4 4 1 2 3 0 7 66 65 83 69 95 118 49\n",
    "0 4 6 83 84 79 80 88 0 7 97 98 99 100 101 102 0\n",
    "-2 0 6 97 98 99 100 101 102 6 82 69 83 69 84 0\n",
    "5 3 9 66 65 83 69 95 116 109 112 0 6 82 69 83 85 77 69\n",
    "1 6 7 97 98 99 100 101 102 0 3 58 58 0\n",
    "0 0 5 66 65 83 69 0 7 66 65 83 69 95 118 49\n",
    "5 3 12 104 101 108 108 111 32 119 111 114 108 100 0 6 83 84 65 82 84 32\n",
];
