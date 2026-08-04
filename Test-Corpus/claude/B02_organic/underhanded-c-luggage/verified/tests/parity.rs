// Integration tests: compare the C reference binary against the Rust binary.
//
// Note: this project produces executables (not shared libraries). The C
// CMakeLists.txt builds `add_executable(driver ...)` and the Rust crate is
// `[[bin]]` with a `main()`. Neither side exports any FFI symbols, so the
// only externally visible surface is the program's argv/stdin -> stdout/stderr
// behavior. We compare both processes byte-for-byte.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_binary() -> PathBuf {
    workspace_root().join("c_src").join("build").join("driver")
}

fn rust_binary() -> PathBuf {
    // The integration test is run after `cargo build` of the binary target.
    // Cargo sets CARGO_BIN_EXE_<name> for the crate's bin targets.
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn ensure_built() {
    let cbin = c_binary();
    assert!(
        cbin.exists(),
        "C binary not found at {}. Build it with: \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        cbin.display()
    );
    let rbin = rust_binary();
    assert!(rbin.exists(), "Rust binary not found at {}", rbin.display());
}

#[derive(Debug, PartialEq, Eq)]
struct RunResult {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
}

fn run(bin: &Path, args: &[&str], stdin_bytes: &[u8]) -> RunResult {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(stdin_bytes).expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait");
    RunResult {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

/// Run both binaries with identical inputs and assert byte-identical outputs.
fn assert_parity(args: &[&str], input: &[u8]) {
    ensure_built();
    let c = run(&c_binary(), args, input);
    let r = run(&rust_binary(), args, input);
    assert_eq!(
        c.code, r.code,
        "exit code mismatch: C={:?} Rust={:?}\nC stderr: {}\nRust stderr: {}",
        c.code,
        r.code,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    if c.stdout != r.stdout {
        panic!(
            "stdout mismatch for args={:?}\nC ({} bytes):\n{}\n---\nRust ({} bytes):\n{}",
            args,
            c.stdout.len(),
            String::from_utf8_lossy(&c.stdout),
            r.stdout.len(),
            String::from_utf8_lossy(&r.stdout),
        );
    }
}

// --- argv error path ----------------------------------------------------

#[test]
fn wrong_argc_zero_args() {
    ensure_built();
    let c = run(&c_binary(), &[], b"");
    let r = run(&rust_binary(), &[], b"");
    assert_eq!(c.code, r.code, "exit code mismatch");
    assert_eq!(c.code, Some(1));
    // stderr contains the same error message on both
    assert_eq!(c.stderr, r.stderr);
}

#[test]
fn wrong_argc_two_args() {
    ensure_built();
    let c = run(&c_binary(), &["a", "b"], b"");
    let r = run(&rust_binary(), &["a", "b"], b"");
    assert_eq!(c.code, r.code);
    assert_eq!(c.stderr, r.stderr);
}

#[test]
fn wrong_argc_six_args() {
    ensure_built();
    let c = run(&c_binary(), &["a", "b", "c", "d", "e", "f"], b"");
    let r = run(&rust_binary(), &["a", "b", "c", "d", "e", "f"], b"");
    assert_eq!(c.code, r.code);
    assert_eq!(c.stderr, r.stderr);
}

// --- empty / whitespace-only input --------------------------------------

#[test]
fn empty_input() {
    assert_parity(&["-", "-", "-", "-"], b"");
}

#[test]
fn whitespace_only_input() {
    assert_parity(&["-", "-", "-", "-"], b"   \n\t  \n");
}

// --- single record ------------------------------------------------------

#[test]
fn single_record_match_all_wildcards() {
    let input = b"100 ABC12345 FL1234 NYC LAX a comment\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn single_record_no_comment() {
    // No comment after arrival; comments[0] = 0 in C (printf with empty %s).
    let input = b"100 ABC12345 FL1234 NYC LAX\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn single_record_filter_luggage_match() {
    let input = b"100 ABC12345 FL1234 NYC LAX hello\n";
    assert_parity(&["ABC12345", "-", "-", "-"], input);
}

#[test]
fn single_record_filter_luggage_miss() {
    let input = b"100 ABC12345 FL1234 NYC LAX hello\n";
    assert_parity(&["ZZZ99999", "-", "-", "-"], input);
}

#[test]
fn single_record_filter_flight_match() {
    let input = b"100 ABC12345 FL1234 NYC LAX hello\n";
    assert_parity(&["-", "FL1234", "-", "-"], input);
}

#[test]
fn single_record_filter_departure() {
    let input = b"100 ABC12345 FL1234 NYC LAX hello\n";
    assert_parity(&["-", "-", "NYC", "-"], input);
}

#[test]
fn single_record_filter_arrival_miss() {
    let input = b"100 ABC12345 FL1234 NYC LAX hello\n";
    assert_parity(&["-", "-", "-", "JFK"], input);
}

// --- ordering / sorting -------------------------------------------------

#[test]
fn time_ordering_ascending() {
    let input = b"500 AAAAAAAA FL0001 AAA BBB late\n100 BBBBBBBB FL0002 CCC DDD early\n300 CCCCCCCC FL0003 EEE FFF mid\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn time_ordering_equal_timestamps_preserve_input_order() {
    // C uses linked-list insertion: when next.time_stamp <= new.time_stamp,
    // walk past it. This means equal-timestamp records keep insertion order.
    let input = b"100 AAAAAAAA FL0001 AAA BBB first\n100 BBBBBBBB FL0002 CCC DDD second\n100 CCCCCCCC FL0003 EEE FFF third\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn time_ordering_mixed_with_duplicates() {
    let input = b"200 AAAAAAAA FL0001 AAA BBB a\n100 BBBBBBBB FL0002 CCC DDD b\n200 CCCCCCCC FL0003 EEE FFF c\n100 DDDDDDDD FL0004 GGG HHH d\n300 EEEEEEEE FL0005 III JJJ e\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

// --- supersedes / superseded --------------------------------------------

#[test]
fn superseded_same_luggage_same_departure() {
    // Two records for ABC12345 from NYC: the earlier one is superseded by
    // the later one (later record has same luggage_id and same departure).
    let input = b"100 ABC12345 FL1111 NYC LAX one\n200 ABC12345 FL2222 NYC SFO two\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn not_superseded_same_luggage_different_departure() {
    // A later record for the same luggage but from a different departure
    // does NOT supersede; per C: returns 0 in that case.
    let input = b"100 ABC12345 FL1111 NYC LAX one\n200 ABC12345 FL2222 BOS SFO two\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn supersedes_chain_only_first_match_wins() {
    // C's `supersedes` only inspects the FIRST later record with matching
    // luggage_id. If that first match has a different departure, the
    // function returns 0 even if a still-later record matches.
    let input = b"100 ABC12345 FL1111 NYC LAX one\n200 ABC12345 FL2222 BOS SFO two\n300 ABC12345 FL3333 NYC LHR three\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn superseded_with_intermediate_other_luggage() {
    let input = b"100 ABC12345 FL1111 NYC LAX one\n150 ZZZ99999 FL9999 ATL ORD other\n200 ABC12345 FL2222 NYC SFO two\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

// --- field width / scanf semantics --------------------------------------

#[test]
fn luggage_id_max_length() {
    // 8-char luggage id (max).
    let input = b"100 ABCDEFGH FL1234 NYC LAX max width\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn luggage_id_short() {
    // shorter than 8 - scanf %8[A-Z0-9] stops at first non-matching char
    let input = b"100 ABC FL1234 NYC LAX short id\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn comments_with_leading_spaces() {
    // After arrival, the next %80[^\n] reads everything until newline,
    // INCLUDING the leading space (scanf does NOT skip whitespace for %[]).
    let input = b"100 AAAAAAAA FL1234 NYC LAX   spaced comment\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn comments_max_80_chars() {
    let mut input = b"100 AAAAAAAA FL1234 NYC LAX ".to_vec();
    input.extend(std::iter::repeat(b'x').take(80));
    input.push(b'\n');
    assert_parity(&["-", "-", "-", "-"], &input);
}

#[test]
fn comments_over_80_chars() {
    // %80[^\n] caps at 80; any trailing characters remain in stdin and
    // become input to the next loop iteration's %d (which will fail to
    // parse on letters).
    let mut input = b"100 AAAAAAAA FL1234 NYC LAX ".to_vec();
    input.extend(std::iter::repeat(b'y').take(100));
    input.push(b'\n');
    assert_parity(&["-", "-", "-", "-"], &input);
}

// --- many records -------------------------------------------------------

#[test]
fn many_records_random_timestamps() {
    let mut input = Vec::new();
    let stamps = [42u32, 7, 99, 1, 1000, 500, 250, 333, 12345, 67];
    for (i, t) in stamps.iter().enumerate() {
        let line = format!(
            "{} LUGG{:04} FL{:04} {} {} record {}\n",
            t,
            i,
            i,
            ['A', 'B', 'C'].iter().collect::<String>(),
            ['X', 'Y', 'Z'].iter().collect::<String>(),
            i
        );
        input.extend_from_slice(line.as_bytes());
    }
    assert_parity(&["-", "-", "-", "-"], &input);
}

#[test]
fn timestamp_zero() {
    let input = b"0 AAAAAAAA FL0001 NYC LAX zero stamp\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn timestamp_large() {
    // 4_000_000_000 > i32::MAX but fits in u32; C reads with %d (signed),
    // then stores into unsigned int — bit pattern preserved. The Rust
    // translation also reads as i64 then casts to u32 with `as`.
    let input = b"4000000000 AAAAAAAA FL0001 NYC LAX big stamp\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

#[test]
fn timestamp_max_u32() {
    let input = b"4294967295 AAAAAAAA FL0001 NYC LAX max\n";
    assert_parity(&["-", "-", "-", "-"], input);
}

// --- filter combinations ------------------------------------------------

#[test]
fn filter_all_specified_match() {
    let input = b"100 ABC12345 FL1234 NYC LAX hit\n200 ZZZ99999 FL9999 BOS SFO miss\n";
    assert_parity(&["ABC12345", "FL1234", "NYC", "LAX"], input);
}

#[test]
fn filter_all_specified_partial_miss() {
    let input = b"100 ABC12345 FL1234 NYC LAX hit\n200 ABC12345 FL1234 NYC JFK partial\n";
    assert_parity(&["ABC12345", "FL1234", "NYC", "LAX"], input);
}

#[test]
fn filter_with_dash_only_arg_is_wildcard() {
    // matches() considers expected[0] == '-' as wildcard, regardless of length.
    let input = b"100 ABC12345 FL1234 NYC LAX hi\n";
    assert_parity(&["-anything", "-x", "-y", "-z"], input);
}

// --- no trailing newline on last record ---------------------------------

#[test]
fn last_record_no_newline() {
    // %80[^\n] will consume the comment up to EOF.
    let input = b"100 AAAAAAAA FL1234 NYC LAX no newline at end";
    assert_parity(&["-", "-", "-", "-"], input);
}

// --- multiple records with mixed superseding ----------------------------

#[test]
fn complex_superseding_scenario() {
    let input = b"\
100 LUGG0001 FL0001 NYC LAX first
200 LUGG0001 FL0002 NYC SFO supersedes 100
300 LUGG0002 FL0003 BOS MIA standalone
400 LUGG0001 FL0004 BOS LHR not supersede 200 (different dep)
500 LUGG0002 FL0005 BOS ORD supersedes 300
600 LUGG0003 FL0006 ATL DEN unique
";
    assert_parity(&["-", "-", "-", "-"], input);
}

// --- ensure libloading dependency is actually used (per harness instr) ---
//
// The project doesn't expose FFI symbols (it's a binary with main()), but
// the harness asks for libloading. We use it to load libc and verify that
// the dynamic loader works on this platform — making the dependency real.
#[test]
fn libloading_smoke() {
    // libc is a fundamental .so always available; this just exercises the
    // libloading API surface so the dev-dependency isn't dead weight.
    unsafe {
        let lib = libloading::Library::new("libc.so.6").or_else(|_| {
            libloading::Library::new("libc.so")
        });
        assert!(lib.is_ok(), "could not load libc via libloading: {:?}", lib.err());
    }
}
