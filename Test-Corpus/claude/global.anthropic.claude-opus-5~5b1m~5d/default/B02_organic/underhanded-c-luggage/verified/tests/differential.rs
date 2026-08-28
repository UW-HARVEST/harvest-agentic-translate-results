//! Differential tests: the C reference in `c_src/` and the Rust translation are
//! both executed as *subprocesses*, exactly the way a shell would run them, and
//! their stdout, stderr and exit status are compared byte for byte.
//!
//! Nothing here links against the translation as a library; the only thing under
//! test is the built `driver` executable.
//!
//! The C program is `c_src/src/luggage.c`.  Every branch it can take is covered
//! by at least one case below:
//!
//!   main()
//!     * `argc != 5`                       -> stderr message + exit 1
//!     * `argc == 5`                       -> normal run
//!     * `scanf("%d ")`  == EOF            -> break (record dropped)
//!     * `scanf("%d ")`  == 0              -> matching failure, stale time stamp
//!     * `scanf("%8[A-Z0-9] %6[A-Z0-9] ")` -> EOF / 0 / 1 / 2
//!     * `scanf("%3[A-Z] %3[A-Z]")`        -> EOF / 0 / 1 / 2
//!     * `scanf("%80[^\n]")`               -> EOF (break) / 0 (empty comment) / 1
//!     * every field width boundary (8, 6, 3, 3, 80) and one byte past it
//!   addRoutingDirectiveToList()
//!     * `next == NULL`                    -> append at the tail
//!     * `next->time_stamp > new`          -> insert before (head / middle)
//!     * otherwise                         -> recurse (equal stamps keep order)
//!   supersedes() / superseded()
//!     * `directive == NULL`               -> 0
//!     * different luggage id              -> keep walking
//!     * same luggage id, same departure   -> 1 (superseded, not printed)
//!     * same luggage id, other departure  -> 0 *and stop walking* (the quirk)
//!   matches()
//!     * `expected[0] == '-'`              -> wildcard (including "-anything")
//!     * `strcmp(expected, actual) == 0`   -> exact match (including "" vs "")
//!     * otherwise                         -> no match
//!   printMatchingDirectives()
//!     * `printf("%010u %s %s %s %s %s\n", ...)` zero padding, the double space
//!       that appears because `%80[^\n]` also captures the separating blank.

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two executables
// ---------------------------------------------------------------------------

/// The Rust executable produced by this crate.
fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

fn workspace_root() -> PathBuf {
    // `translation/` -> repository root
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// The C reference executable, built with CMake on first use.
fn c_binary() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.is_file() {
            std::fs::create_dir_all(&build).expect("cannot create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("`cmake` is required to build the C reference");
            assert!(
                configure.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&configure.stdout),
                String::from_utf8_lossy(&configure.stderr)
            );
            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("cannot run cmake --build");
            assert!(
                compile.status.success(),
                "cmake --build failed:\n{}\n{}",
                String::from_utf8_lossy(&compile.stdout),
                String::from_utf8_lossy(&compile.stderr)
            );
        }
        assert!(exe.is_file(), "C reference binary missing at {:?}", exe);
        exe
    })
}

// ---------------------------------------------------------------------------
// running a program the way a shell would
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={:?} signal={:?} stdout={} stderr={}",
            self.code,
            self.signal,
            show(&self.stdout),
            show(&self.stderr)
        )
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s.push('"');
    s
}

/// Spawns `bin` with `args` as argv[1..], feeds `stdin_data` and collects
/// everything.  stdin is written and stderr drained on helper threads so that a
/// program producing more output than a pipe buffer holds cannot deadlock.
fn run(bin: &Path, args: &[&[u8]], stdin_data: &[u8]) -> Outcome {
    let mut cmd = Command::new(bin);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {:?}: {e}", bin));

    let mut child_stdin = child.stdin.take().expect("stdin");
    let data = stdin_data.to_vec();
    let feeder = std::thread::spawn(move || {
        // A closed stdout can make the child die before it drained stdin; that
        // is a legitimate outcome, not a test failure.
        let _ = child_stdin.write_all(&data);
        let _ = child_stdin.flush();
        drop(child_stdin);
    });

    let mut child_stderr = child.stderr.take().expect("stderr");
    let drainer = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = child_stderr.read_to_end(&mut v);
        v
    });

    let mut stdout = Vec::new();
    let mut child_stdout = child.stdout.take().expect("stdout");
    let _ = child_stdout.read_to_end(&mut stdout);

    let status = child.wait().expect("wait");
    feeder.join().expect("stdin feeder panicked");
    let stderr = drainer.join().expect("stderr drainer panicked");

    Outcome {
        code: status.code(),
        signal: status.signal(),
        stdout,
        stderr,
    }
}

/// Runs `bin` with stdout wired to a pipe whose read end is closed before the
/// program can write anything, i.e. the classic broken pipe.  A C program dies
/// from `SIGPIPE`; a Rust program only does so if it restored the default
/// disposition, which Rust's runtime otherwise sets to `SIG_IGN`.
fn run_with_broken_stdout(bin: &Path, args: &[&[u8]], stdin_data: &[u8]) -> (Option<i32>, Option<i32>) {
    let mut cmd = Command::new(bin);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {:?}: {e}", bin));

    // Close the read end straight away.  The program cannot have written yet:
    // it does not print anything until it has read stdin to end of file, and
    // stdin is fed only after this drop.
    drop(child.stdout.take());

    let mut child_stdin = child.stdin.take().expect("stdin");
    let _ = child_stdin.write_all(stdin_data);
    drop(child_stdin);

    let status = child.wait().expect("wait");
    (status.code(), status.signal())
}

/// Runs `bin` with file descriptors 1 and/or 2 outright closed, so that the
/// writes inside the program fail with `EBADF`.
fn run_with_closed_fds(
    bin: &Path,
    args: &[&[u8]],
    stdin_data: &[u8],
    close_stdout: bool,
    close_stderr: bool,
) -> Option<i32> {
    use std::os::unix::process::CommandExt;
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    let mut cmd = Command::new(bin);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: `close()` is async-signal-safe and touches no allocator state,
    // which is all that is required of a `pre_exec` hook.
    unsafe {
        cmd.pre_exec(move || {
            if close_stdout {
                close(1);
            }
            if close_stderr {
                close(2);
            }
            Ok(())
        });
    }
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {:?}: {e}", bin));
    let mut child_stdin = child.stdin.take().expect("stdin");
    let _ = child_stdin.write_all(stdin_data);
    drop(child_stdin);
    child.wait().expect("wait").code()
}

/// Collects mismatches so that one test reports every failing case at once.
#[derive(Default)]
struct Differ {
    checks: usize,
    failures: Vec<String>,
}

impl Differ {
    fn check(&mut self, name: &str, args: &[&[u8]], stdin_data: &[u8]) {
        self.checks += 1;
        let c = run(c_binary(), args, stdin_data);
        let r = run(rust_binary(), args, stdin_data);
        if c != r {
            let pretty_args: Vec<String> = args.iter().map(|a| show(a)).collect();
            self.failures.push(format!(
                "case {name}\n  argv  = [{}]\n  stdin = {}\n  C     : {:?}\n  Rust  : {:?}",
                pretty_args.join(", "),
                show(stdin_data),
                c,
                r
            ));
        }
    }

    /// Runs a case against a handful of argv filter sets.
    fn check_all_filters(&mut self, name: &str, stdin_data: &[u8]) {
        for (i, args) in filter_sets().iter().enumerate() {
            let refs: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
            self.check(&format!("{name}#f{i}"), &refs, stdin_data);
        }
    }

    fn finish(self) {
        if !self.failures.is_empty() {
            panic!(
                "{} of {} differential checks failed:\n\n{}",
                self.failures.len(),
                self.checks,
                self.failures.join("\n\n")
            );
        }
        assert!(self.checks > 0, "no checks were run");
        // visible with `cargo test -- --nocapture`
        eprintln!(
            "{}: {} differential checks",
            std::thread::current().name().unwrap_or("?"),
            self.checks
        );
    }
}

fn filter_sets() -> Vec<Vec<Vec<u8>>> {
    let s = |a: &str, b: &str, c: &str, d: &str| {
        vec![
            a.as_bytes().to_vec(),
            b.as_bytes().to_vec(),
            c.as_bytes().to_vec(),
            d.as_bytes().to_vec(),
        ]
    };
    vec![
        s("-", "-", "-", "-"),          // everything is a wildcard
        s("LUG00001", "-", "-", "-"),   // exact luggage id
        s("-", "FL1234", "-", "-"),     // exact flight id
        s("-", "-", "JFK", "-"),        // exact departure
        s("-", "-", "-", "LAX"),        // exact arrival
        s("LUG00001", "FL1234", "JFK", "LAX"),
        s("NOSUCHID", "-", "-", "-"),   // matches nothing
        s("", "", "", ""),              // only empty fields match
        s("-x", "-", "-", "-"),         // '-' prefix is still a wildcard
        s("-", "-", "jfk", "-"),        // case sensitive
    ]
}

const WILDCARD: &[&[u8]] = &[b"-", b"-", b"-", b"-"];

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    let c = run(c_binary(), WILDCARD, b"");
    let r = run(rust_binary(), WILDCARD, b"");
    assert_eq!(c.code, Some(0), "C reference did not run: {c:?}");
    assert_eq!(r.code, Some(0), "Rust translation did not run: {r:?}");
    assert_eq!(c, r, "empty input already differs\n C: {c:?}\n R: {r:?}");
}

// ---------------------------------------------------------------------------
// main(): the argc check
// ---------------------------------------------------------------------------

#[test]
fn argc_error_path() {
    let mut d = Differ::default();
    let stdin_data = b"0000000001 LUG00001 FL1234 JFK LAX x\n";
    // argc == 1 .. 4 -> too few
    d.check("argc=1", &[], stdin_data);
    d.check("argc=2", &[b"-"], stdin_data);
    d.check("argc=3", &[b"-", b"-"], stdin_data);
    d.check("argc=4", &[b"-", b"-", b"-"], stdin_data);
    // argc == 5 -> ok
    d.check("argc=5", &[b"-", b"-", b"-", b"-"], stdin_data);
    // argc > 5 -> too many
    d.check("argc=6", &[b"-", b"-", b"-", b"-", b"-"], stdin_data);
    d.check("argc=7", &[b"-", b"-", b"-", b"-", b"-", b"-"], stdin_data);
    // the error path must not depend on stdin at all
    d.check("argc=1,empty-stdin", &[], b"");
    d.check("argc=3,garbage-stdin", &[b"a", b"b"], b"\x00\xff\x01not a record");
    d.finish();
}

// ---------------------------------------------------------------------------
// no input / no records
// ---------------------------------------------------------------------------

#[test]
fn empty_and_whitespace_only_input() {
    let mut d = Differ::default();
    d.check_all_filters("empty", b"");
    d.check_all_filters("newline", b"\n");
    d.check_all_filters("many_newlines", b"\n\n\n\n");
    d.check_all_filters("spaces", b"    ");
    d.check_all_filters("mixed_ws", b" \t\r\n\x0b\x0c \n");
    d.check_all_filters("single_space", b" ");
    d.check_all_filters("single_tab", b"\t");
    d.finish();
}

// ---------------------------------------------------------------------------
// a single record, and the comment / trailing-newline branches
// ---------------------------------------------------------------------------

#[test]
fn single_record_variants() {
    let mut d = Differ::default();
    // %80[^\n] succeeds -> the captured blank produces the double space
    d.check_all_filters("with_comment", b"0000000001 LUG00001 FL1234 JFK LAX hello\n");
    d.check_all_filters("with_comment_no_nl", b"0000000001 LUG00001 FL1234 JFK LAX hello");
    // %80[^\n] matching failure (returns 0) -> empty comment, record kept
    d.check_all_filters("no_comment", b"0000000001 LUG00001 FL1234 JFK LAX\n");
    // %80[^\n] hits EOF (returns EOF) -> break, the record is DROPPED
    d.check_all_filters("no_comment_eof", b"0000000001 LUG00001 FL1234 JFK LAX");
    // a single blank before EOF is still a successful comment conversion
    d.check_all_filters("blank_comment_eof", b"0000000001 LUG00001 FL1234 JFK LAX ");
    d.check_all_filters("blank_comment_nl", b"0000000001 LUG00001 FL1234 JFK LAX \n");
    // no separator at all between arrival and comment
    d.check_all_filters("glued_comment", b"1 LUG FL ABCDEFG\n");
    // '\r' belongs to the comment, '\n' does not
    d.check_all_filters("crlf", b"0000000001 LUG00001 FL1234 JFK LAX c\r\n");
    d.check_all_filters("crlf_no_comment", b"0000000001 LUG00001 FL1234 JFK LAX\r\n");
    d.finish();
}

// ---------------------------------------------------------------------------
// field widths: the maximum the code handles, and one byte past it
// ---------------------------------------------------------------------------

#[test]
fn field_width_boundaries() {
    let mut d = Differ::default();
    // luggage id: 1, 8 (max), 9, 16 characters
    d.check_all_filters("lug1", b"1 A FL1234 JFK LAX c\n");
    d.check_all_filters("lug8", b"1 ABCDEFGH FL1234 JFK LAX c\n");
    d.check_all_filters("lug9", b"1 ABCDEFGHI FL1234 JFK LAX c\n");
    d.check_all_filters("lug16", b"1 ABCDEFGHIJKLMNOP QRS TUV c\n");
    // flight id: 1, 6 (max), 7
    d.check_all_filters("fl1", b"1 LUG00001 F JFK LAX c\n");
    d.check_all_filters("fl6", b"1 LUG00001 ABCDEF JFK LAX c\n");
    d.check_all_filters("fl7", b"1 LUG00001 ABCDEFG JFK LAX c\n");
    // airports: 1, 3 (max), 4, 6
    d.check_all_filters("ap1", b"1 LUG00001 FL1234 A B c\n");
    d.check_all_filters("ap4", b"1 LUG00001 FL1234 JFKK LAXX c\n");
    d.check_all_filters("ap6", b"1 LUG00001 FL1234 ABCDEF GHIJKL c\n");
    // comments: 79, 80 (max), 81, 200 characters
    for n in [79usize, 80, 81, 200] {
        let mut v = b"1 LUG00001 FL1234 JFK LAX".to_vec();
        v.extend(std::iter::repeat(b'z').take(n));
        v.push(b'\n');
        d.check_all_filters(&format!("comment{n}"), &v);
    }
    // 80 comment characters *after* the separating blank
    let mut v = b"1 LUG00001 FL1234 JFK LAX ".to_vec();
    v.extend(std::iter::repeat(b'y').take(80));
    v.extend_from_slice(b"TAIL\n2 LUG2 FL2 BOS SFO next\n");
    d.check_all_filters("comment_overflow_spills", &v);
    // every field simultaneously at its maximum
    d.check_all_filters(
        "all_max",
        b"4294967295 ABCDEFGH ABCDEF ABC DEF 12345678901234567890\n",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// "%d" into an unsigned int: signedness, truncation and overflow
// ---------------------------------------------------------------------------

#[test]
fn timestamp_conversions() {
    let mut d = Differ::default();
    let numbers: &[&[u8]] = &[
        b"0",
        b"1",
        b"9",
        b"0000000042",
        b"00000000000000000000005",
        b"999999999",
        b"1000000000",
        b"2147483647",
        b"2147483648",
        b"4294967294",
        b"4294967295",
        b"4294967296",
        b"12345678901",
        b"9223372036854775807",  // LONG_MAX
        b"9223372036854775808",  // saturates at LONG_MAX
        b"18446744073709551616", // saturates too
        b"99999999999999999999",
        b"-0",
        b"-1",
        b"-5",
        b"-2147483648",
        b"-2147483649",
        b"-4294967295",
        b"-9223372036854775808", // LONG_MIN
        b"-9223372036854775809", // saturates at LONG_MIN
        b"-99999999999999999999",
        b"+0",
        b"+7",
        b"+2147483648",
    ];
    for n in numbers {
        let mut v = n.to_vec();
        v.extend_from_slice(b" LUG00001 FL1234 JFK LAX c\n");
        d.check(&format!("ts={}", show(n)), WILDCARD, &v);
    }
    // 5000 digits: glibc saturates, then truncates to int
    let mut v = vec![b'9'; 5000];
    v.extend_from_slice(b" LUG00001 FL1234 JFK LAX c\n");
    d.check("ts=9x5000", WILDCARD, &v);

    // %d matching failures: the conversion stores nothing and the rest of the
    // format string is abandoned
    let non_numbers: &[&[u8]] = &[
        b"-", b"+", b"--5", b"+-5", b"-x", b"abc", b"0x1A", b".5", b"1.5", b"1e5", b"5,000", b"#1",
    ];
    for n in non_numbers {
        for tail in [
            &b" LUG00001 FL1234 JFK LAX c\n"[..],
            &b"\nLUG00001 FL1234 JFK LAX c\n"[..],
            &b""[..],
        ] {
            let mut v = n.to_vec();
            v.extend_from_slice(tail);
            d.check(&format!("bad_ts={}", show(n)), WILDCARD, &v);
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// every early `break` out of the read loop, and every matching failure
// ---------------------------------------------------------------------------

#[test]
fn scan_failure_and_break_paths() {
    let mut d = Differ::default();
    let good = b"0000000009 LUGZZZZZ FL9999 ORD MIA prev\n";
    // Each fragment stops at a different point in the record; prefixing a
    // complete record also exercises the stale-buffer reuse of the C code.
    let fragments: &[(&str, &[u8])] = &[
        ("ts_eof", b"1"),
        ("ts_then_space_eof", b"1 "),
        ("ts_then_nl_eof", b"1\n"),
        ("lug_eof", b"1 LUG00001"),
        ("lug_then_space_eof", b"1 LUG00001 "),
        ("lug_fail_lower", b"1 lug00001 FL1234 JFK LAX c\n"),
        ("lug_fail_dash", b"1 - FL1234 JFK LAX c\n"),
        ("flight_eof", b"1 LUG00001 FL1234"),
        ("flight_fail", b"1 LUG00001 --- JFK LAX c\n"),
        ("flight_fail_lower", b"1 LUG00001 fl1234 JFK LAX c\n"),
        ("dep_eof", b"1 LUG00001 FL1234 JFK"),
        ("dep_fail_digit", b"1 LUG00001 FL1234 123 LAX c\n"),
        ("dep_fail_lower", b"1 LUG00001 FL1234 jfk LAX c\n"),
        ("arr_eof", b"1 LUG00001 FL1234 JFK "),
        ("arr_fail_digit", b"1 LUG00001 FL1234 JFK 12 c\n"),
        ("arr_fail_lower", b"1 LUG00001 FL1234 JFK lax c\n"),
        ("comments_eof", b"1 LUG00001 FL1234 JFK LAX"),
        ("comments_fail", b"1 LUG00001 FL1234 JFK LAX\n"),
        ("all_lower", b"1 lug fl jfk lax c\n"),
        ("nothing_matches", b"abc\n"),
        ("nothing_matches_no_nl", b"abc"),
        ("only_letters", b"A B C D E\n"),
        ("only_letters_no_nl", b"A B C D E"),
    ];
    for (name, frag) in fragments {
        d.check_all_filters(&format!("alone/{name}"), frag);
        let mut with_prev = good.to_vec();
        with_prev.extend_from_slice(frag);
        d.check_all_filters(&format!("after_record/{name}"), &with_prev);
        // and followed by a well-formed record, so the loop keeps going
        let mut then_more = frag.to_vec();
        then_more.extend_from_slice(b"\n0000000002 LUG00002 FL5678 BOS SFO after\n");
        d.check_all_filters(&format!("then_record/{name}"), &then_more);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// the C buffers live in main()'s frame, so a failed conversion leaves the
// previous record's characters behind
// ---------------------------------------------------------------------------

#[test]
fn stale_buffer_reuse() {
    let mut d = Differ::default();
    // record 2 fails on the luggage id -> keeps record 1's ids
    d.check_all_filters(
        "stale_ids",
        b"0000000001 LUG00001 FL1234 JFK LAX first\n0000000002 lower FL5678 BOS SFO second\n",
    );
    // record 2 fails on the airports -> keeps record 1's airports
    d.check_all_filters(
        "stale_airports",
        b"0000000001 LUG00001 FL1234 JFK LAX first\n0000000002 LUG00002 FL5678 12 34 second\n",
    );
    // record 2 has no numeric time stamp -> keeps record 1's time stamp
    d.check_all_filters(
        "stale_timestamp",
        b"0000000077 LUG00001 FL1234 JFK LAX first\nLUG00002 FL5678 BOS SFO second\n",
    );
    // a longer id followed by a shorter one: the C NUL terminator shortens the
    // visible string
    d.check_all_filters(
        "shrinking_ids",
        b"1 ABCDEFGH ABCDEF JFK LAX long\n2 XY ZW BOS SFO short\n",
    );
    // comments are explicitly reset every iteration, so they never go stale
    d.check_all_filters(
        "comments_reset",
        b"1 LUG00001 FL1234 JFK LAX has a comment\n2 LUG00002 FL5678 BOS SFO\n",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// addRoutingDirectiveToList(): the sorted insertion
// ---------------------------------------------------------------------------

#[test]
fn insertion_order() {
    let mut d = Differ::default();
    // strictly increasing -> every insert walks to the tail (next == NULL)
    d.check_all_filters(
        "increasing",
        b"1 LUGA FL1 AAA BBB a\n2 LUGB FL2 CCC DDD b\n3 LUGC FL3 EEE FFF c\n",
    );
    // strictly decreasing -> every insert lands at the head
    d.check_all_filters(
        "decreasing",
        b"3 LUGC FL3 EEE FFF c\n2 LUGB FL2 CCC DDD b\n1 LUGA FL1 AAA BBB a\n",
    );
    // insert into the middle
    d.check_all_filters(
        "middle",
        b"1 LUGA FL1 AAA BBB a\n9 LUGC FL3 EEE FFF c\n5 LUGB FL2 CCC DDD b\n",
    );
    // equal time stamps keep input order
    d.check_all_filters(
        "equal_stamps",
        b"5 LUGA FL1 AAA BBB a\n5 LUGB FL2 CCC DDD b\n5 LUGC FL3 EEE FFF c\n",
    );
    // zero time stamp compared against the sentinel (whose stamp is 0)
    d.check_all_filters(
        "zero_stamps",
        b"0 LUGA FL1 AAA BBB a\n0 LUGB FL2 CCC DDD b\n1 LUGC FL3 EEE FFF c\n0 LUGD FL4 GGG HHH d\n",
    );
    // wrapped time stamps sort by their unsigned value
    d.check_all_filters(
        "wrapped_stamps",
        b"-1 LUGA FL1 AAA BBB neg\n1 LUGB FL2 CCC DDD one\n2147483648 LUGC FL3 EEE FFF big\n",
    );
    // fully identical records
    d.check_all_filters(
        "identical",
        b"9 LUG00001 FL0001 JFK LAX same\n9 LUG00001 FL0001 JFK LAX same\n9 LUG00001 FL0001 JFK LAX same\n",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// supersedes() / superseded()
// ---------------------------------------------------------------------------

#[test]
fn supersession_rules() {
    let mut d = Differ::default();
    // later directive, same luggage, same departure -> the earlier one is hidden
    d.check_all_filters(
        "superseded",
        b"1 LUG00001 FL0001 JFK LAX v1\n2 LUG00001 FL0002 JFK BOS v2\n",
    );
    // later directive, same luggage, different departure -> nothing is hidden
    d.check_all_filters(
        "not_superseded",
        b"1 LUG00001 FL0001 JFK LAX v1\n2 LUG00001 FL0002 BOS SFO v2\n",
    );
    // the quirk: the walk stops at the first later directive for the same
    // luggage, so the third record here cannot hide the first
    d.check_all_filters(
        "stops_at_first_match",
        b"1 LUG00001 FL0001 JFK LAX v1\n2 LUG00001 FL0002 BOS SFO v2\n3 LUG00001 FL0003 JFK ORD v3\n",
    );
    // directives for other luggage are skipped while walking
    d.check_all_filters(
        "skips_other_luggage",
        b"1 LUG00001 FL0001 JFK LAX v1\n2 LUG00002 FL0002 JFK BOS other\n3 LUG00001 FL0003 JFK ORD v3\n",
    );
    // only *later* directives count, so the last one is never superseded
    d.check_all_filters(
        "chain",
        b"1 LUGX FL1 AAA BBB one\n2 LUGX FL2 AAA CCC two\n3 LUGX FL3 AAA DDD three\n",
    );
    // equal time stamps: order of insertion decides who supersedes whom
    d.check_all_filters(
        "equal_stamp_supersede",
        b"7 LUGX FL1 AAA BBB one\n7 LUGX FL2 AAA CCC two\n",
    );
    // empty luggage ids (stale/never-written buffers) compare equal to each other
    d.check_all_filters("empty_ids_supersede", b"1 abc\n2 def\n3 ghi\n");
    d.finish();
}

// ---------------------------------------------------------------------------
// matches() and the argv filters
// ---------------------------------------------------------------------------

#[test]
fn filters_and_wildcards() {
    let mut d = Differ::default();
    let input = b"1 LUG00001 FL1234 JFK LAX one\n\
                  2 LUG00002 FL5678 BOS SFO two\n\
                  3 LUG00003 FL1234 JFK SFO three\n";
    let arg_cases: &[[&[u8]; 4]] = &[
        [b"-", b"-", b"-", b"-"],
        [b"LUG00001", b"-", b"-", b"-"],
        [b"LUG00002", b"FL5678", b"BOS", b"SFO"],
        [b"LUG00002", b"FL5678", b"BOS", b"JFK"], // one field mismatches
        [b"-", b"FL1234", b"-", b"-"],
        [b"-", b"-", b"JFK", b"-"],
        [b"-", b"-", b"-", b"SFO"],
        [b"-anything", b"-", b"-", b"-"], // '-' prefix wildcard
        [b"--", b"-!", b"-0", b"-\xff"],  // still wildcards
        [b"", b"", b"", b""],             // only empty fields match
        [b"", b"-", b"-", b"-"],
        [b"lug00001", b"-", b"-", b"-"], // case sensitive
        [b"LUG0000", b"-", b"-", b"-"],  // prefix is not a match
        [b"LUG000011", b"-", b"-", b"-"],
        [b"\xff\xfe", b"-", b"-", b"-"],       // non-UTF-8 argv
        [b"-", b"-", b"-", b"\xc3\x28"],       // invalid UTF-8 argv
        [b"LUG00001", b"FL1234", b"JFK", b"LAX"],
    ];
    for (i, args) in arg_cases.iter().enumerate() {
        d.check(&format!("filter{i}"), &args[..], input);
    }
    // a record whose fields are the empty string, so that "" can actually match
    d.check("empty_field_match", &[b"", b"", b"", b""], b"5 comment only\n");
    d.check("empty_field_wildcard", WILDCARD, b"5 comment only\n");
    d.finish();
}

// ---------------------------------------------------------------------------
// scanf reads across newlines
// ---------------------------------------------------------------------------

#[test]
fn scanf_crosses_line_boundaries() {
    let mut d = Differ::default();
    d.check_all_filters(
        "one_field_per_line",
        b"0000000001\nLUG00001\nFL1234\nJFK\nLAX\n",
    );
    d.check_all_filters(
        "two_records_one_line",
        b"1 LUG00001 FL1234 JFK LAX one 2 LUG00002 FL5678 BOS SFO two\n",
    );
    d.check_all_filters(
        "record_split_after_arrival",
        b"1 LUG00001 FL1234 JFK\nLAX comment\n",
    );
    d.check_all_filters(
        "blank_lines_between",
        b"\n\n1 LUG00001 FL1234 JFK LAX one\n\n\n2 LUG00002 FL5678 BOS SFO two\n\n",
    );
    d.check_all_filters(
        "tabs_as_separators",
        b"1\tLUG00001\tFL1234\tJFK\tLAX\tcomment\n",
    );
    d.check_all_filters(
        "odd_whitespace",
        b"1\x0bLUG00001\x0cFL1234\rJFK \tLAX x\n",
    );
    d.check_all_filters(
        "lots_of_spaces",
        b"   1     LUG00001     FL1234     JFK     LAX     spaced   \n",
    );
    d.check_all_filters(
        "no_final_newline_multi",
        b"1 LUG00001 FL1234 JFK LAX one\n2 LUG00002 FL5678 BOS SFO two",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// bytes that are not text
// ---------------------------------------------------------------------------

#[test]
fn binary_input() {
    let mut d = Differ::default();
    // a NUL inside a comment terminates the C string, the rest is invisible
    d.check_all_filters(
        "nul_in_comment",
        b"1 LUG00001 FL1234 JFK LAX ab\x00cd\n2 LUG00002 FL5678 BOS SFO after\n",
    );
    d.check_all_filters(
        "leading_nul_comment",
        b"1 LUG00001 FL1234 JFK LAX\x00hidden\n",
    );
    d.check_all_filters("nul_before_record", b"\x001 LUG00001 FL1234 JFK LAX c\n");
    d.check_all_filters("only_nul", b"\x00");
    d.check_all_filters("nul_run", b"\x00\x00\x00\n\x00");
    d.check_all_filters(
        "high_bytes",
        b"1 LUG00001 FL1234 JFK LAX \x80\xff\xfe\x01\x7f end\n",
    );
    d.check_all_filters(
        "utf8_comment",
        "1 LUG00001 FL1234 JFK LAX caf\u{e9} \u{fc}n\u{ef}c\u{f8}d\u{e9} \u{2708}\n".as_bytes(),
    );
    d.check_all_filters(
        "del_and_controls",
        b"1 LUG00001 FL1234 JFK LAX \x07\x08\x1b[31m\x7f c\n",
    );
    d.finish();
}

// ---------------------------------------------------------------------------
// larger inputs
// ---------------------------------------------------------------------------

#[test]
fn many_records() {
    let mut d = Differ::default();

    // 40 records, five luggage ids, mixed airports: plenty of supersession
    let mut small = Vec::new();
    for i in 0..40u32 {
        small.extend_from_slice(
            format!(
                "{:010} LUG{:05} FL{:04} {} {} comment {}\n",
                (i * 7) % 23,
                i % 5,
                i,
                if i % 2 == 0 { "JFK" } else { "BOS" },
                if i % 3 == 0 { "LAX" } else { "SFO" },
                i
            )
            .as_bytes(),
        );
    }
    d.check_all_filters("mixed40", &small);

    // 2000 records inserted in ascending order (the C recursion walks the whole
    // list every time) -- output larger than a pipe buffer
    let mut ascending = Vec::new();
    for i in 0..2000u32 {
        ascending.extend_from_slice(
            format!("{} LUG{:05} FL{:04} JFK LAX c{}\n", i, i, i % 9999, i).as_bytes(),
        );
    }
    d.check("ascending2000", WILDCARD, &ascending);
    d.check("ascending2000/exact", &[b"LUG01999", b"-", b"-", b"-"], &ascending);

    // 2000 records inserted in descending order (always at the head)
    let mut descending = Vec::new();
    for i in (0..2000u32).rev() {
        descending.extend_from_slice(
            format!("{} LUG{:05} FL{:04} BOS SFO c{}\n", i, i, i % 9999, i).as_bytes(),
        );
    }
    d.check("descending2000", WILDCARD, &descending);

    // 500 records that all share one luggage id and one departure: every record
    // but the last is superseded
    let mut shared = Vec::new();
    for i in 0..500u32 {
        shared.extend_from_slice(format!("{} SAMEID00 FL0001 JFK LAX c{}\n", i, i).as_bytes());
    }
    d.check("shared500", WILDCARD, &shared);
    d.finish();
}

// ---------------------------------------------------------------------------
// randomized differential fuzzing (deterministic seed)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

#[test]
fn fuzz_structured_records() {
    let mut d = Differ::default();
    let mut rng = Rng(0x1234_5678_9abc_def1);

    let stamps: &[&[u8]] = &[
        b"0",
        b"1",
        b"5",
        b"0000000007",
        b"42",
        b"-3",
        b"-1",
        b"4294967295",
        b"2147483648",
        b"99999999999999999999",
        b"12345678901",
        b"-",
        b"+",
        b"x",
        b"",
    ];
    let luggage: &[&[u8]] = &[
        b"LUG00001",
        b"LUG00002",
        b"A",
        b"9",
        b"ABCDEFGH",
        b"ABCDEFGHIJ",
        b"lower",
        b"",
        b"-",
    ];
    let flights: &[&[u8]] = &[b"FL0001", b"FL0002", b"F", b"ABCDEF", b"ABCDEFGH", b"fl", b""];
    let airports: &[&[u8]] = &[b"JFK", b"BOS", b"LAX", b"SFO", b"A", b"AB", b"ABCD", b"1A", b""];
    let comments: &[&[u8]] = &[
        b"",
        b" hello",
        b" a b c",
        b"   ",
        b"\x00zzz",
        b" \xff\x01",
        b" 1 LUG00003 FL0003 ORD MIA nested",
    ];
    let long_comment: Vec<u8> = {
        let mut v = vec![b' '];
        v.extend(std::iter::repeat(b'q').take(95));
        v
    };
    let separators: &[&[u8]] = &[b" ", b"  ", b"\t", b"\n", b" \n ", b"\r\n", b"", b"\x0b"];
    let filters = filter_sets();

    for _ in 0..500 {
        let records = rng.below(6);
        let mut data = Vec::new();
        for _ in 0..records {
            for field in [
                *rng.pick(stamps),
                *rng.pick(luggage),
                *rng.pick(flights),
                *rng.pick(airports),
                *rng.pick(airports),
            ] {
                data.extend_from_slice(field);
                data.extend_from_slice(*rng.pick(separators));
            }
            if rng.below(10) == 0 {
                data.extend_from_slice(&long_comment);
            } else {
                data.extend_from_slice(*rng.pick(comments));
            }
            if rng.below(10) != 0 {
                data.push(b'\n');
            }
        }
        let args = rng.pick(&filters).clone();
        let refs: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
        d.check("fuzz_structured", &refs, &data);
    }
    d.finish();
}

#[test]
fn fuzz_random_bytes() {
    let mut d = Differ::default();
    let mut rng = Rng(0x0bad_c0ff_eeba_d5eeu64);

    // an alphabet that keeps hitting the interesting scanf transitions
    let mut alphabet: Vec<u8> = Vec::new();
    alphabet.extend_from_slice(b"0123456789");
    alphabet.extend_from_slice(b"ABCDEFGHIJKLX");
    alphabet.extend_from_slice(b"abcx");
    alphabet.extend_from_slice(b"-+ \t\n\n\r\x00\xff\x80");
    let filters = filter_sets();

    for _ in 0..500 {
        let len = rng.below(70);
        let mut data = Vec::with_capacity(len);
        for _ in 0..len {
            data.push(*rng.pick(&alphabet));
        }
        let args = rng.pick(&filters).clone();
        let refs: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
        d.check("fuzz_bytes", &refs, &data);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// the environment the program is run in
// ---------------------------------------------------------------------------

#[test]
fn broken_stdout_kills_both_programs_the_same_way() {
    let inputs: &[&[u8]] = &[
        b"1 LUG00001 FL1234 JFK LAX one\n2 LUG00002 FL5678 BOS SFO two\n",
        b"1 LUG00001 FL1234 JFK LAX only\n",
    ];
    // more output than a pipe buffer holds, so the failing write happens while
    // still printing rather than at exit
    let mut big = Vec::new();
    for i in 0..3000u32 {
        big.extend_from_slice(format!("{} LUG{:05} FL0001 JFK LAX c{}\n", i, i, i).as_bytes());
    }

    let mut cases: Vec<&[u8]> = inputs.to_vec();
    cases.push(&big);

    for (i, data) in cases.iter().enumerate() {
        // repeated, because a difference here could otherwise be a fluke
        for round in 0..3 {
            let c = run_with_broken_stdout(c_binary(), WILDCARD, data);
            let r = run_with_broken_stdout(rust_binary(), WILDCARD, data);
            assert_eq!(
                c, r,
                "broken stdout case {i} round {round}: C (code, signal) = {c:?}, Rust = {r:?}"
            );
        }
    }
}

#[test]
fn closed_descriptors_are_survived_identically() {
    let record = b"1 LUG00001 FL1234 JFK LAX one\n2 LUG00002 FL5678 BOS SFO two\n";
    // stdout closed: every printf fails, the program still exits 0
    assert_eq!(
        run_with_closed_fds(c_binary(), WILDCARD, record, true, false),
        run_with_closed_fds(rust_binary(), WILDCARD, record, true, false),
        "exit status differs when stdout is closed"
    );
    // stderr closed on the argc error path: the message is lost, exit is still 1
    assert_eq!(
        run_with_closed_fds(c_binary(), &[b"-"], record, false, true),
        run_with_closed_fds(rust_binary(), &[b"-"], record, false, true),
        "exit status differs when stderr is closed on the argc error path"
    );
    // both closed
    assert_eq!(
        run_with_closed_fds(c_binary(), WILDCARD, record, true, true),
        run_with_closed_fds(rust_binary(), WILDCARD, record, true, true),
        "exit status differs when both stdout and stderr are closed"
    );
}

#[test]
fn empty_stdin_from_dev_null() {
    // /dev/null gives an immediate end of file, just like an empty pipe
    let spawn = |bin: &Path| {
        let out = Command::new(bin)
            .args(["-", "-", "-", "-"])
            .stdin(Stdio::null())
            .output()
            .expect("spawn");
        (out.status.code(), out.stdout, out.stderr)
    };
    assert_eq!(spawn(c_binary()), spawn(rust_binary()));
}
