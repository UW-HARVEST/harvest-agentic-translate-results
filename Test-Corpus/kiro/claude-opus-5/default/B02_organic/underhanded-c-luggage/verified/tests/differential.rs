//! Differential tests: run the original C program and the Rust translation as
//! subprocesses with the same argv and the same stdin, then require that
//! stdout, stderr and the exit status are byte-for-byte / value identical.
//!
//! The Rust code is never called as a library; both sides are driven exactly
//! the way a shell would drive them.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// locating / building the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path of the C executable, building it with CMake the first time if needed.
fn c_binary() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.is_file() {
            fs::create_dir_all(&build).expect("cannot create c_src/build");
            let configure = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .status()
                .expect("failed to run `cmake ..` -- is cmake installed?");
            assert!(configure.success(), "`cmake ..` failed in {:?}", build);
            let compile = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .status()
                .expect("failed to run `cmake --build .`");
            assert!(compile.success(), "`cmake --build .` failed in {:?}", build);
        }
        assert!(
            exe.is_file(),
            "C executable {:?} still missing after building",
            exe
        );
        exe
    })
}

// ---------------------------------------------------------------------------
// running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("stdout", &Escaped(&self.stdout))
            .field("stderr", &Escaped(&self.stderr))
            .field("code", &self.code)
            .field("signal", &self.signal)
            .finish()
    }
}

struct Escaped<'a>(&'a [u8]);

impl std::fmt::Debug for Escaped<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        for &b in self.0 {
            match b {
                b'\n' => f.write_str("\\n")?,
                b'\t' => f.write_str("\\t")?,
                b'\r' => f.write_str("\\r")?,
                b'"' => f.write_str("\\\"")?,
                b'\\' => f.write_str("\\\\")?,
                0x20..=0x7e => f.write_str(&(b as char).to_string())?,
                _ => write!(f, "\\x{:02x}", b)?,
            }
        }
        f.write_str("\"")
    }
}

/// stdin is fed from a temporary file so that arbitrarily large inputs cannot
/// deadlock against a full pipe.
fn stdin_file(input: &[u8]) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "luggage-diff-{}-{}.stdin",
        std::process::id(),
        n
    ));
    let mut f = fs::File::create(&path).expect("cannot create stdin temp file");
    f.write_all(input).expect("cannot write stdin temp file");
    f.flush().expect("cannot flush stdin temp file");
    path
}

fn run(exe: &Path, args: &[OsString], stdin_path: &Path) -> Outcome {
    let stdin = fs::File::open(stdin_path).expect("cannot reopen stdin temp file");
    let out = Command::new(exe)
        .args(args)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("cannot run {:?}: {e}", exe));
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// The single assertion used by every test: same argv, same stdin, therefore
/// same stdout, same stderr, same exit status.
#[track_caller]
fn assert_same_bytes(args: &[&[u8]], input: &[u8]) {
    let args: Vec<OsString> = args
        .iter()
        .map(|a| OsStr::from_bytes(a).to_os_string())
        .collect();
    let stdin_path = stdin_file(input);
    let c = run(c_binary(), &args, &stdin_path);
    let r = run(rust_binary(), &args, &stdin_path);
    let _ = fs::remove_file(&stdin_path);

    if c != r {
        let shown: Vec<Escaped> = args.iter().map(|a| Escaped(a.as_bytes())).collect();
        panic!(
            "divergence\n  argv : {:?}\n  stdin: {:?}\n  C    : {:#?}\n  Rust : {:#?}",
            shown,
            Escaped(input),
            c,
            r
        );
    }
}

#[track_caller]
fn assert_same(args: &[&str], input: &str) {
    let args: Vec<&[u8]> = args.iter().map(|a| a.as_bytes()).collect();
    assert_same_bytes(&args, input.as_bytes());
}

/// The four wildcard arguments used whenever the test is about stdin parsing.
const WILD: [&str; 4] = ["-", "-", "-", "-"];

#[track_caller]
fn assert_same_wild(input: &str) {
    assert_same(&WILD, input);
}

// ---------------------------------------------------------------------------
// Phase A sanity: both binaries exist and are runnable
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_are_runnable() {
    assert!(c_binary().is_file(), "C binary missing");
    assert!(rust_binary().is_file(), "Rust binary missing");
    // argc == 5 is the only accepted arity; this also proves both start up.
    assert_same(&["-", "-", "-", "-"], "");
}

// ---------------------------------------------------------------------------
// argc branch: `if (argc != 5) { fprintf(stderr, ...); exit(1); }`
// ---------------------------------------------------------------------------

#[test]
fn argc_too_few() {
    assert_same(&[], "");
    assert_same(&["-"], "");
    assert_same(&["-", "-"], "");
    assert_same(&["-", "-", "-"], "");
    // stdin is irrelevant on the error path: it must not be read or echoed.
    assert_same(&["-", "-", "-"], "10 AAA111 FL01 JFK LAX ignored\n");
}

#[test]
fn argc_exactly_five() {
    assert_same(&["-", "-", "-", "-"], "10 AAA111 FL01 JFK LAX ok\n");
}

#[test]
fn argc_too_many() {
    assert_same(&["-", "-", "-", "-", "-"], "");
    assert_same(&["-", "-", "-", "-", "-", "-"], "10 AAA111 FL01 JFK LAX x\n");
}

// ---------------------------------------------------------------------------
// empty input, one item, several items
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_same_wild("");
}

#[test]
fn whitespace_only_input() {
    for input in ["\n", " ", "   ", "\t", "\n\n\n", "\r\n", "\x0b", "\x0c", " \t\r\n\x0b\x0c"] {
        assert_same_wild(input);
    }
}

#[test]
fn single_record() {
    assert_same_wild("10 AAA111 FL01 JFK LAX hello\n");
}

#[test]
fn single_record_without_comment() {
    // `%80[^\n]` fails (matching failure, not EOF) so comments stays empty and
    // the record is still recorded.
    assert_same_wild("10 AAA111 FL01 JFK LAX\n");
}

#[test]
fn single_record_without_trailing_newline_is_dropped() {
    // `%80[^\n]` hits end of file -> EOF -> break before the record is added.
    assert_same_wild("10 AAA111 FL01 JFK LAX");
}

#[test]
fn single_record_trailing_space_then_eof() {
    // Now `%80[^\n]` matches the single blank, so the record survives and the
    // comment is " " -- printed after printf's own blank, i.e. two blanks.
    assert_same_wild("10 AAA111 FL01 JFK LAX ");
}

#[test]
fn several_records() {
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX first\n\
         20 BBB222 FL02 LAX SFO second\n\
         30 CCC333 FL03 SFO ORD third\n",
    );
}

// ---------------------------------------------------------------------------
// field widths: maximum the code handles, and one past it
// ---------------------------------------------------------------------------

#[test]
fn maximum_field_widths() {
    let comment: String = "C".repeat(80);
    assert_same_wild(&format!("10 ABCDEFGH FLIGHT JFK LAX {comment}\n"));
}

#[test]
fn fields_one_character_over_the_maximum() {
    // The extra characters are left in the stream and shift every following
    // conversion.
    assert_same_wild("10 ABCDEFGHI FLIGHT JFK LAX x\n");
    assert_same_wild("10 ABCDEFGH FLIGHTX JFK LAX x\n");
    assert_same_wild("10 ABCDEFGH FLIGHT JFKX LAX x\n");
    assert_same_wild("10 ABCDEFGH FLIGHT JFK LAXX x\n");
    assert_same_wild(&format!("10 AAA111 FL01 JFK LAX {}\n", "C".repeat(81)));
    assert_same_wild(&format!("10 AAA111 FL01 JFK LAX {}\n", "C".repeat(200)));
}

#[test]
fn minimum_field_widths() {
    assert_same_wild("0 A F J L c\n");
}

#[test]
fn comment_longer_than_eighty_then_next_record() {
    let long = "C".repeat(100);
    assert_same_wild(&format!(
        "10 AAA111 FL01 JFK LAX {long}\n20 BBB222 FL02 LAX SFO second\n"
    ));
}

// ---------------------------------------------------------------------------
// truncated input: EOF at every field boundary
// ---------------------------------------------------------------------------

#[test]
fn eof_at_every_field_boundary() {
    for input in [
        "",
        "10",
        "10 ",
        "10 AAA111",
        "10 AAA111 ",
        "10 AAA111 FL01",
        "10 AAA111 FL01 ",
        "10 AAA111 FL01 JFK",
        "10 AAA111 FL01 JFK ",
        "10 AAA111 FL01 JFK LAX",
        "10 AAA111 FL01 JFK LAX ",
        "10 AAA111 FL01 JFK LAX c",
        "10 AAA111 FL01 JFK LAX c\n",
    ] {
        assert_same_wild(input);
    }
}

#[test]
fn truncation_after_a_complete_record() {
    let head = "10 AAA111 FL01 JFK LAX first\n";
    for tail in [
        "",
        "20",
        "20 ",
        "20 BBB222",
        "20 BBB222 FL02",
        "20 BBB222 FL02 LAX",
        "20 BBB222 FL02 LAX SFO",
        "20 BBB222 FL02 LAX SFO ",
        "20 BBB222 FL02 LAX SFO second",
    ] {
        assert_same_wild(&format!("{head}{tail}"));
    }
}

// ---------------------------------------------------------------------------
// `%d` conversion: matching failure, sign handling, overflow, truncation
// ---------------------------------------------------------------------------

#[test]
fn time_stamp_matching_failure() {
    // glibc pushes back only the offending character, so a sign that was
    // already consumed stays consumed.
    for input in [
        "abc\n", "-\n", "+\n", "-x\n", "+x\n", "- 5\n", "--5\n", "++5\n", "  -  \n", "!\n",
        ".5\n", "?\n", "0x10\n", "+ABC\n", "-AAA111 FL01 JFK LAX c\n",
    ] {
        assert_same_wild(input);
    }
}

#[test]
fn time_stamp_signs_and_zeroes() {
    for ts in [
        "0", "-0", "+0", "1", "+1", "-1", "007", "000000000000000000005", "0000000000",
    ] {
        assert_same_wild(&format!("{ts} AAA111 FL01 JFK LAX c\n"));
    }
}

#[test]
fn time_stamp_overflow_and_truncation() {
    // `%d` reads a signed int through an `unsigned int*`; glibc converts with
    // strtol (saturating at LONG_MIN/LONG_MAX) and truncates to int.
    for ts in [
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999999",
        "-99999999999999999999999",
        "123456789012345678901234567890123456789012345678901234567890",
    ] {
        assert_same_wild(&format!("{ts} AAA111 FL01 JFK LAX c\n"));
    }
}

// ---------------------------------------------------------------------------
// scanset conversions: rejected characters, whitespace directives
// ---------------------------------------------------------------------------

#[test]
fn scanset_rejects_lowercase_and_punctuation() {
    assert_same_wild("10 aaa111 FL01 JFK LAX x\n");
    assert_same_wild("10 AAA111 fl01 JFK LAX x\n");
    assert_same_wild("10 AAA111 FL01 jfk LAX x\n");
    assert_same_wild("10 AAA111 FL01 JFK lax x\n");
    assert_same_wild("10 AAA111 FL01 JF9 LAX x\n"); // digits rejected by %[A-Z]
    assert_same_wild("10 AAA111 FL01 JFK LA9 x\n");
    assert_same_wild("10 AA-111 FL01 JFK LAX x\n");
    assert_same_wild("10 AAA111 FL_1 JFK LAX x\n");
    assert_same_wild("10 ?? ?? ?? ?? ??\n");
}

#[test]
fn scanf_reads_across_newlines() {
    // Every whitespace directive (and the leading skip of `%d`) crosses
    // newlines, so a record may be spread over many lines.
    assert_same_wild("10\nAAA111\nFL01\nJFK\nLAX\ncomment\n");
    assert_same_wild("10\n\n\nAAA111\t\tFL01\r\nJFK\x0bLAX comment\n");
    assert_same_wild("10 AAA111 FL01 JFK\nLAX comment\n");
}

#[test]
fn newline_immediately_after_arrival_leaves_comment_empty() {
    // `%3[A-Z] %3[A-Z]` has no trailing whitespace directive, so `%80[^\n]`
    // sees the newline directly and fails without consuming it.
    assert_same_wild("10 AAA111 FL01 JFK LAX\n20 BBB222 FL02 LAX SFO\n");
}

#[test]
fn comment_keeps_the_separating_blank() {
    // Two blanks in the output: one left in the comment, one added by printf.
    assert_same_wild("10 AAA111 FL01 JFK LAX comment\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX    padded\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX\tTABBED\n");
}

#[test]
fn comment_may_contain_anything_but_newline() {
    assert_same_wild("10 AAA111 FL01 JFK LAX a\rb\tc  d!?%s%n\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX %010u %s\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX -leading dash\n");
}

#[test]
fn garbage_between_valid_records() {
    assert_same_wild("10 AAA111 FL01 JFK LAX ok\n!!!\n20 BBB222 FL02 LAX SFO ok2\n");
    assert_same_wild("!!!\n10 AAA111 FL01 JFK LAX ok\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX ok\n!!!\n");
    assert_same_wild("###\n$$$\n%%%\n");
}

#[test]
fn buffers_carry_over_between_iterations() {
    // The C buffers are reused stack slots: a failed conversion leaves the
    // previous record's value in place, and it is copied into the new record.
    assert_same_wild("10 AAA111 FL01 JFK LAX first\n20 ??? ??? ??? ??? second\n");
    assert_same_wild("10 ABCDEFGH FLIGHT JFK LAX first\n20 !\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX first\n20 BBB222 !\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX first\n20 BBB222 FL02 !\n");
    assert_same_wild("10 AAA111 FL01 JFK LAX first\n20 BBB222 FL02 LAX !\n");
}

#[test]
fn first_iteration_reads_uninitialised_buffers() {
    // Nothing is assigned to luggage_id/flight_id/departure/arrival before the
    // first `strcpy`, so the C program copies the stack residue. See ERRORS.md.
    assert_same_wild("abc\n");
    assert_same_wild("10 ?? ??\n");
    assert_same_wild("10 AAA111 ??\n");
    assert_same_wild("10 AAA111 FL01 ??\n");
    assert_same_wild("10 AAA111 FL01 JFK ??\n");
}

#[test]
fn nul_and_high_bytes_in_input() {
    assert_same_bytes(
        &[b"-", b"-", b"-", b"-"],
        b"10 AAA111 FL01 JFK LAX co\x00mment\n",
    );
    assert_same_bytes(&[b"-", b"-", b"-", b"-"], b"\x00\n10 AAA111 FL01 JFK LAX c\n");
    assert_same_bytes(
        &[b"-", b"-", b"-", b"-"],
        b"10 AAA111 FL01 JFK LAX \xff\xfe\x80 high\n",
    );
    assert_same_bytes(&[b"-", b"-", b"-", b"-"], b"\xff\xff\xff\n");
}

// ---------------------------------------------------------------------------
// ordering: addRoutingDirectiveToList
// ---------------------------------------------------------------------------

#[test]
fn records_are_sorted_by_time_stamp() {
    assert_same_wild(
        "30 CCC333 FL03 AAA BBB third\n\
         10 AAA111 FL01 JFK LAX first\n\
         20 BBB222 FL02 LAX SFO second\n",
    );
}

#[test]
fn equal_time_stamps_keep_input_order() {
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX a\n\
         10 BBB222 FL02 LAX SFO b\n\
         10 CCC333 FL03 SFO ORD c\n",
    );
}

#[test]
fn descending_and_interleaved_time_stamps() {
    assert_same_wild(
        "50 AAA111 FL01 JFK LAX e\n\
         40 BBB222 FL02 LAX SFO d\n\
         30 CCC333 FL03 SFO ORD c\n\
         20 DDD444 FL04 ORD DEN b\n\
         10 EEE555 FL05 DEN JFK a\n",
    );
    assert_same_wild(
        "30 AAA111 FL01 JFK LAX c\n\
         10 BBB222 FL02 LAX SFO a\n\
         50 CCC333 FL03 SFO ORD e\n\
         20 DDD444 FL04 ORD DEN b\n\
         40 EEE555 FL05 DEN JFK d\n",
    );
}

#[test]
fn ordering_uses_unsigned_comparison_after_truncation() {
    // -1 becomes 4294967295, so it sorts last even though it was written first.
    assert_same_wild(
        "-1 AAA111 FL01 JFK LAX minusone\n\
         10 BBB222 FL02 LAX SFO ten\n",
    );
}

// ---------------------------------------------------------------------------
// supersedes / superseded
// ---------------------------------------------------------------------------

#[test]
fn later_directive_with_same_id_and_departure_supersedes() {
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX old\n\
         20 AAA111 FL02 JFK SFO new\n",
    );
}

#[test]
fn later_directive_with_same_id_but_other_departure_does_not_supersede() {
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX old\n\
         20 AAA111 FL02 SFO ORD new\n",
    );
}

#[test]
fn supersedes_only_consults_the_first_later_match() {
    // The middle directive departs elsewhere, so it -- and only it -- decides
    // for the first directive: the third one is never reached.
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX a\n\
         20 AAA111 FL02 SFO ORD b\n\
         30 AAA111 FL03 JFK DEN c\n",
    );
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX a\n\
         20 AAA111 FL02 JFK ORD b\n\
         30 AAA111 FL03 SFO DEN c\n",
    );
}

#[test]
fn different_luggage_ids_are_skipped_while_searching() {
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX a\n\
         20 BBB222 FL02 JFK ORD b\n\
         30 CCC333 FL03 JFK DEN c\n\
         40 AAA111 FL04 JFK SEA d\n",
    );
}

#[test]
fn superseded_chain_of_four() {
    assert_same_wild(
        "10 AAA111 FL01 JFK LAX a\n\
         20 AAA111 FL02 JFK ORD b\n\
         30 AAA111 FL03 JFK DEN c\n\
         40 AAA111 FL04 JFK SEA d\n",
    );
}

#[test]
fn supersession_follows_sorted_order_not_input_order() {
    assert_same_wild(
        "20 AAA111 FL02 JFK SFO later\n\
         10 AAA111 FL01 JFK LAX earlier\n",
    );
}

#[test]
fn empty_luggage_ids_compare_equal() {
    // A failed `%8[A-Z0-9]` can leave an id equal to the previous record's, or
    // empty on the very first record.
    assert_same_wild("10 ?? ?? JFK LAX a\n20 ?? ?? JFK LAX b\n");
}

// ---------------------------------------------------------------------------
// matches(): the '-' wildcard and exact comparison
// ---------------------------------------------------------------------------

#[test]
fn each_filter_position_can_be_exact() {
    let input = "10 AAA111 FL01 JFK LAX a\n20 BBB222 FL02 LAX SFO b\n";
    assert_same(&["AAA111", "-", "-", "-"], input);
    assert_same(&["-", "FL01", "-", "-"], input);
    assert_same(&["-", "-", "JFK", "-"], input);
    assert_same(&["-", "-", "-", "LAX"], input);
    assert_same(&["AAA111", "FL01", "JFK", "LAX"], input);
    assert_same(&["BBB222", "FL02", "LAX", "SFO"], input);
}

#[test]
fn filters_that_match_nothing() {
    let input = "10 AAA111 FL01 JFK LAX a\n20 BBB222 FL02 LAX SFO b\n";
    assert_same(&["ZZZ999", "-", "-", "-"], input);
    assert_same(&["AAA111", "FL02", "-", "-"], input);
    assert_same(&["AAA111", "-", "LAX", "-"], input);
    assert_same(&["aaa111", "-", "-", "-"], input);
    assert_same(&["AAA1110", "-", "-", "-"], input);
    assert_same(&["AAA11", "-", "-", "-"], input);
}

#[test]
fn only_the_first_character_of_a_filter_makes_it_a_wildcard() {
    let input = "10 AAA111 FL01 JFK LAX a\n20 BBB222 FL02 LAX SFO b\n";
    assert_same(&["-anything", "-", "-", "-"], input);
    assert_same(&["-", "-nonsense", "--", "-zzz"], input);
    // A dash that is not first is just a character to compare.
    assert_same(&["A-A111", "-", "-", "-"], input);
}

#[test]
fn empty_filter_strings_match_only_empty_fields() {
    let input = "10 AAA111 FL01 JFK LAX a\n";
    assert_same(&["", "", "", ""], input);
    assert_same(&["", "-", "-", "-"], input);
    // A record whose comment-only fields failed to convert has empty fields.
    assert_same(&["", "", "", ""], "10 ?? ?? ?? ?? x\n20 ?? ?? ?? ?? y\n");
}

#[test]
fn filters_with_non_utf8_bytes() {
    let input = b"10 AAA111 FL01 JFK LAX a\n";
    assert_same_bytes(&[b"\xff\xfe", b"-", b"-", b"-"], input);
    assert_same_bytes(&[b"-", b"\x80", b"-", b"-"], input);
    // 0x03 is the residue the C program leaves in luggage_id; make sure the
    // exact-match path can select such a record too.
    assert_same_bytes(&[b"\x03", b"", b"", b""], b"abc\n");
}

#[test]
fn filter_interacts_with_supersession() {
    // A superseded record is dropped before the filters are consulted.
    let input = "10 AAA111 FL01 JFK LAX old\n20 AAA111 FL02 JFK SFO new\n";
    assert_same(&["AAA111", "FL01", "JFK", "LAX"], input);
    assert_same(&["AAA111", "FL02", "JFK", "SFO"], input);
}

// ---------------------------------------------------------------------------
// larger inputs
// ---------------------------------------------------------------------------

#[test]
fn many_records_ascending() {
    let mut input = String::new();
    for i in 0..500 {
        input.push_str(&format!("{} A{:04} F{:04} JFK LAX comment {}\n", i + 1, i, i, i));
    }
    assert_same_wild(&input);
}

#[test]
fn many_records_descending_with_repeated_ids() {
    let mut input = String::new();
    for i in (0..500).rev() {
        input.push_str(&format!(
            "{} A{:02} F{:02} {} {} c{}\n",
            i + 1,
            i % 17,
            i % 7,
            ["JFK", "LAX", "SFO"][i % 3],
            ["ORD", "DEN", "SEA"][i % 3],
            i
        ));
    }
    assert_same_wild(&input);
    assert_same(&["A05", "-", "-", "-"], &input);
    assert_same(&["-", "-", "JFK", "-"], &input);
}

// ---------------------------------------------------------------------------
// deterministic sweep over generated inputs (no external crates)
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 16
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() % xs.len() as u64) as usize]
    }
}

#[test]
fn generated_token_soup() {
    const TOKENS: [&str; 22] = [
        "10", "0", "-1", "+7", "4294967296", "99999999999999999999", "-", "+", "A", "Z", "9",
        "AAA111", "ABCDEFGHI", "FL01", "FLIGHTX", "JFK", "LAX", "?", "!", ".", "a", "zz",
    ];
    const SEPS: [&str; 6] = [" ", " ", "\n", "\t", "  ", ""];
    const FILTERS: [&str; 7] = ["-", "", "AAA111", "FL01", "JFK", "LAX", "-x"];

    let mut rng = Lcg(0x5eed);
    for _ in 0..300 {
        let mut input = String::new();
        let n = rng.next() % 14;
        for _ in 0..n {
            input.push_str(rng.pick(&TOKENS));
            input.push_str(rng.pick(&SEPS));
        }
        let args = [
            *rng.pick(&FILTERS),
            *rng.pick(&FILTERS),
            *rng.pick(&FILTERS),
            *rng.pick(&FILTERS),
        ];
        assert_same(&args, &input);
    }
}

#[test]
fn generated_records() {
    const TS: [&str; 9] = [
        "1", "2", "10", "10", "0", "-1", "4294967295", "2147483648", "007",
    ];
    const IDS: [&str; 7] = ["A", "AAA111", "ABCDEFGH", "ABCDEFGHIJ", "", "??", "a1"];
    const FLIGHTS: [&str; 6] = ["F", "FL01", "FLIGHT", "FLIGHTX", "", "!"];
    const PORTS: [&str; 7] = ["JFK", "LAX", "SFO", "AB", "ABCD", "", "ab"];
    const COMMENTS: [&str; 5] = ["", "c", "a comment", "  spaced  ", "%s%d"];
    const FILTERS: [&str; 8] = ["-", "", "AAA111", "A", "FL01", "JFK", "LAX", "-zzz"];

    let mut rng = Lcg(0xc0ffee);
    for _ in 0..300 {
        let mut input = String::new();
        let n = rng.next() % 6;
        for _ in 0..n {
            input.push_str(&format!(
                "{} {} {} {} {} {}",
                rng.pick(&TS),
                rng.pick(&IDS),
                rng.pick(&FLIGHTS),
                rng.pick(&PORTS),
                rng.pick(&PORTS),
                rng.pick(&COMMENTS),
            ));
            input.push_str(rng.pick(&["\n", "\n", "\n", ""]));
        }
        let args = [
            *rng.pick(&FILTERS),
            *rng.pick(&FILTERS),
            *rng.pick(&FILTERS),
            *rng.pick(&FILTERS),
        ];
        assert_same(&args, &input);
    }
}
