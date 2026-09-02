//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses, feed both the same bytes on stdin, and require that stdout,
//! stderr and the exit status agree exactly.
//!
//! The Rust program is never linked as a library here. It is driven the way a
//! shell drives it, because that is how the translation is graded.
//!
//! Note that several cases below make the C reference die from SIGSEGV. That is
//! not a defect in the harness: the C overflows a `strcpy` into the global that
//! holds an array counter and then indexes with it. Those cases assert that the
//! Rust program dies from the same signal after emitting the same bytes, which
//! includes losing the same unflushed part of the stdio buffer.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Locating and building the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // tests/ lives in the crate; the C tree is a sibling of the crate.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

/// Path to the Rust binary under test, as built by cargo for this test run.
fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C reference binary, building it with cmake if it is not there
/// yet. Never skips: if the reference cannot be built, the tests must fail,
/// because a comparison against a program that did not build measures nothing.
fn c_binary() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    let bin = build.join("driver");
    if bin.exists() {
        return bin;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let conf = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        conf.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&conf.stdout),
        String::from_utf8_lossy(&conf.stderr)
    );
    let built = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output()
        .expect("run cmake --build");
    assert!(
        built.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(bin.exists(), "cmake did not produce {}", bin.display());
    bin
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status={:?} stdout={}B stderr={}B",
            self.status,
            self.stdout.len(),
            self.stderr.len()
        )
    }
}

fn run(bin: &Path, input: &[u8]) -> Outcome {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    {
        let mut sin = child.stdin.take().expect("stdin pipe");
        let data = input.to_vec();
        // Write on a thread so a program that exits early (`exit`, or a crash)
        // cannot deadlock us on a full pipe.
        std::thread::spawn(move || {
            let _ = sin.write_all(&data);
        });
    }

    let out = child.wait_with_output().expect("wait for child");
    let status = match out.status.code() {
        Some(c) => Ok(c),
        None => Err(out.status.signal().expect("exited by signal")),
    };
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// Report the first differing byte with surrounding context.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            let lo = i.saturating_sub(70);
            let hi = (i + 70).min(n);
            return format!(
                "first difference at byte {i}\n     C: {:?}\n  RUST: {:?}",
                show(&a[lo..hi.min(a.len())]),
                show(&b[lo..hi.min(b.len())]),
            );
        }
    }
    format!(
        "common prefix of {n} bytes is equal; lengths differ (C={}, RUST={})\n\
         C tail:    {:?}\n  RUST tail: {:?}",
        a.len(),
        b.len(),
        show(&a[n..]),
        show(&b[n..])
    )
}

/// The core assertion: stdout, stderr and exit status must all match.
#[track_caller]
fn assert_identical(case: &str, input: &[u8]) {
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout differs\n{}\ninput was: {:?}",
        first_diff(&c.stdout, &r.stdout),
        show(input)
    );
    assert_eq!(
        show(&c.stderr),
        show(&r.stderr),
        "[{case}] stderr differs\ninput was: {:?}",
        show(input)
    );
    assert_eq!(
        c.status, r.status,
        "[{case}] exit status differs (C={:?}, RUST={:?})\ninput was: {:?}",
        c.status,
        r.status,
        show(input)
    );
}

#[track_caller]
fn check_all(cases: &[(&str, String)]) {
    for (name, input) in cases {
        assert_identical(name, input.as_bytes());
    }
}

fn rep(c: &str, n: usize) -> String {
    c.repeat(n)
}

// ---------------------------------------------------------------------------
// Phase A: both binaries exist and run
// ---------------------------------------------------------------------------

#[test]
fn both_binaries_build_and_run() {
    let c = c_binary();
    let r = rust_binary();
    assert!(c.is_file(), "C binary missing: {}", c.display());
    assert!(r.is_file(), "Rust binary missing: {}", r.display());
    // Empty stdin: banner, one prompt, clean exit.
    assert_identical("smoke/empty-stdin", b"");
}

// ---------------------------------------------------------------------------
// Phase B: the inputs the C branches on
// ---------------------------------------------------------------------------

#[test]
fn empty_and_whitespace_only_input() {
    check_all(&[
        ("empty", String::new()),
        ("single newline", "\n".into()),
        ("three blank lines", "\n\n\n".into()),
        ("spaces only", "   \n".into()),
        ("tabs only", "\t\t\n".into()),
        ("mixed blanks", " \t \n\t\n \n".into()),
        // A blank line leaves `command` uninitialised in the C. In the
        // reference build that stale buffer reads as empty, so the line is a
        // no-op -- even directly after a real command.
        ("blank after command", "whoami\n\nstatus\n\n".into()),
        ("blank between commands", "status\n\nwhoami\n\nlistusers\n\n".into()),
        ("no trailing newline", "status".into()),
        ("only prompt then EOF", "\n".into()),
    ]);
}

#[test]
fn user_management_branches() {
    check_all(&[
        // arg-count guard, duplicate, default and explicit permission level
        ("adduser no args", "adduser\n".into()),
        ("adduser one arg", "adduser alice\n".into()),
        ("adduser ok default level", "adduser alice pw\n".into()),
        ("adduser duplicate", "adduser alice pw\nadduser alice other\n".into()),
        ("adduser explicit level", "adduser bob pw 5\n".into()),
        // atoi paths: non-numeric, negative, overflow saturation, truncation
        ("adduser level non-numeric", "adduser c pw abc\n".into()),
        ("adduser level negative", "adduser c pw -3\n".into()),
        ("adduser level huge", "adduser c pw 99999999999999999999\n".into()),
        ("adduser level 2^31", "adduser c pw 2147483648\n".into()),
        ("adduser level -2^31-1", "adduser c pw -2147483649\n".into()),
        ("adduser level leading space", "adduser c pw +7\n".into()),
        // login / logout / whoami
        ("login no args", "login\n".into()),
        ("login one arg", "login alice\n".into()),
        ("login unknown user", "login nobody pw\n".into()),
        ("login wrong password", "adduser a p\nlogin a bad\n".into()),
        ("login ok", "adduser a p\nlogin a p\nwhoami\n".into()),
        ("login twice", "adduser a p\nlogin a p\nlogin a p\n".into()),
        ("login while logged in", "adduser a p\nadduser b q\nlogin a p\nlogin b q\n".into()),
        ("logout when none", "logout\n".into()),
        ("logout ok", "adduser a p\nlogin a p\nlogout\nlogout\n".into()),
        ("whoami not logged in", "whoami\n".into()),
        ("relogin after logout", "adduser a p\nlogin a p\nlogout\nlogin a p\nwhoami\n".into()),
        // listusers: empty, populated, logged-in marker, and the `users` alias
        ("listusers empty", "listusers\n".into()),
        ("users alias empty", "users\n".into()),
        (
            "listusers with marker",
            "adduser a p 1\nadduser b q 2\nlogin a p\nlistusers\nusers\n".into(),
        ),
    ]);
}

#[test]
fn max_users_boundary() {
    // MAX_USERS is 10; the 11th and 12th must be refused.
    let mut s = String::new();
    for i in 1..=12 {
        s += &format!("adduser u{i} p{i} {i}\n");
    }
    s += "listusers\nstatus\n";
    check_all(&[("12 users, MAX_USERS=10", s)]);
}

#[test]
fn file_management_branches() {
    check_all(&[
        // every command gated on being logged in
        ("createfile not logged in", "createfile f\n".into()),
        ("writefile not logged in", "writefile f c\n".into()),
        ("deletefile not logged in", "deletefile f\n".into()),
        // readfile and listfiles are NOT gated on login -- note the asymmetry
        ("readfile not logged in", "readfile f\n".into()),
        ("listfiles not logged in", "listfiles\n".into()),
        // arg-count guards, checked only after the login guard
        ("createfile no args", "adduser a p\nlogin a p\ncreatefile\n".into()),
        ("writefile one arg", "adduser a p\nlogin a p\nwritefile f\n".into()),
        ("deletefile no args", "adduser a p\nlogin a p\ndeletefile\n".into()),
        ("readfile no args", "readfile\n".into()),
        // create with and without content, duplicate name
        (
            "createfile with/without content",
            "adduser a p\nlogin a p\ncreatefile f1\nreadfile f1\ncreatefile f2 hello\nreadfile f2\n".into(),
        ),
        (
            "createfile duplicate",
            "adduser a p\nlogin a p\ncreatefile f1 x\ncreatefile f1 y\nreadfile f1\n".into(),
        ),
        ("readfile missing", "adduser a p\nlogin a p\nreadfile nope\n".into()),
        (
            "writefile then read",
            "adduser a p\nlogin a p\ncreatefile f x\nwritefile f newer\nreadfile f\n".into(),
        ),
        ("writefile missing", "adduser a p\nlogin a p\nwritefile nope x\n".into()),
        ("deletefile missing", "adduser a p\nlogin a p\ndeletefile nope\n".into()),
        ("listfiles empty", "listfiles\n".into()),
        // the shift-down loop in deletefile, deleting from the middle
        (
            "delete middle shifts",
            "adduser a p\nlogin a p\ncreatefile f1 a\ncreatefile f2 b\ncreatefile f3 c\n\
             listfiles\ndeletefile f2\nlistfiles\nreadfile f3\ndeletefile f1\nlistfiles\n"
                .into(),
        ),
        // aliases route to the same handlers
        (
            "aliases touch/cat/write/rm/ls",
            "adduser a p\nlogin a p\ntouch f hello\ncat f\nwrite f bye\ncat f\nls\nrm f\nls\n".into(),
        ),
    ]);
}

#[test]
fn file_permission_branches() {
    // writefile allows the owner or permission_level >= 5;
    // deletefile allows the owner or permission_level >= 9.
    // Level 5 can therefore write but not delete -- exercise both sides.
    let setup = "adduser owner opw 1\nadduser low lpw 0\nadduser mid mpw 5\nadduser high hpw 9\n\
                 login owner opw\ncreatefile shared original\nlogout\n";
    check_all(&[
        (
            "level 0 denied write and delete",
            format!("{setup}login low lpw\nwritefile shared x\ndeletefile shared\nreadfile shared\n"),
        ),
        (
            "level 5 may write, not delete",
            format!("{setup}login mid mpw\nwritefile shared midwrote\ndeletefile shared\nreadfile shared\n"),
        ),
        (
            "level 9 may write and delete",
            format!("{setup}login high hpw\nwritefile shared hiwrote\nreadfile shared\ndeletefile shared\nlistfiles\n"),
        ),
        (
            "owner may write and delete",
            format!("{setup}login owner opw\nwritefile shared mine\ndeletefile shared\nlistfiles\n"),
        ),
    ]);
}

#[test]
fn max_files_boundary() {
    // MAX_FILES is 20; the 21st and 22nd must be refused.
    let mut s = String::from("adduser a p 1\nlogin a p\n");
    for i in 1..=22 {
        s += &format!("createfile f{i} c{i}\n");
    }
    s += "listfiles\nstatus\n";
    check_all(&[("22 files, MAX_FILES=20", s)]);
}

#[test]
fn variable_branches() {
    check_all(&[
        ("set no args", "set\n".into()),
        ("set one arg", "set x\n".into()),
        ("set new", "set x 1\nlistvars\n".into()),
        ("set existing updates", "set x 1\nset x 2\nget x\n".into()),
        ("get no args", "get\n".into()),
        ("get missing", "get nope\n".into()),
        ("unset no args", "unset\n".into()),
        ("unset missing", "unset nope\n".into()),
        ("listvars empty", "listvars\n".into()),
        ("vars alias empty", "vars\n".into()),
        (
            "unset middle shifts",
            "set a 1\nset b 2\nset c 3\nlistvars\nunset b\nlistvars\nget c\nunset a\nlistvars\nvars\n"
                .into(),
        ),
    ]);
}

#[test]
fn max_variables_boundary() {
    // MAX_VARIABLES is 20; the 21st and 22nd must be refused.
    let mut s = String::new();
    for i in 1..=22 {
        s += &format!("set v{i} val{i}\n");
    }
    s += "listvars\nstatus\n";
    check_all(&[("22 variables, MAX_VARIABLES=20", s)]);
}

#[test]
fn string_comparison_branches() {
    check_all(&[
        // compare: guard, equal, less, greater. The printed value is glibc's
        // difference of the first differing bytes as unsigned char.
        ("compare no args", "compare\n".into()),
        ("compare one arg", "compare a\n".into()),
        ("compare equal", "compare abc abc\n".into()),
        ("compare less", "compare abc abd\n".into()),
        ("compare greater", "compare abd abc\n".into()),
        ("compare prefix shorter", "compare ab abc\n".into()),
        ("compare prefix longer", "compare abc ab\n".into()),
        ("compare wide byte gap", "compare ~ A\ncompare A ~\n".into()),
        ("cmp alias", "cmp a b\n".into()),
        // compareN: guard, n == 0, in-range, beyond both strings, and the
        // negative n that sign-extends into size_t while printing negative.
        ("compareN no args", "compareN\n".into()),
        ("compareN two args", "compareN a b\n".into()),
        ("compareN n=0 equal", "compareN abc abd 0\n".into()),
        ("compareN n within", "compareN abc abd 2\n".into()),
        ("compareN n at diff", "compareN abc abd 3\n".into()),
        ("compareN reversed", "compareN abd abc 3\n".into()),
        ("compareN n beyond", "compareN abc abd 99999\n".into()),
        ("compareN n negative", "compareN a b -1\ncompareN abc abc -5\n".into()),
        ("compareN n non-numeric", "compareN a b abc\n".into()),
        ("compareN n 2^31", "compareN a b 2147483648\n".into()),
        ("cmpn alias", "cmpn ab ac 1\n".into()),
        // startswith
        ("startswith no args", "startswith\n".into()),
        ("startswith one arg", "startswith a\n".into()),
        ("startswith yes", "startswith hello he\n".into()),
        ("startswith no", "startswith hello xx\n".into()),
        ("startswith prefix longer", "startswith he hello\n".into()),
        ("startswith identical", "startswith abc abc\n".into()),
        // match: exact, substring, none, and several strings at once
        ("match no args", "match\n".into()),
        ("match one arg", "match a\n".into()),
        ("match exact", "match abc abc\n".into()),
        ("match contains", "match abc xabcx\n".into()),
        ("match none", "match abc zzz\n".into()),
        ("match mixed", "match a abc bcd a xay zzz\n".into()),
    ]);
}

#[test]
fn system_command_branches() {
    check_all(&[
        ("debug query", "debug\n".into()),
        ("debug on", "debug on\ndebug\n".into()),
        ("debug off", "debug on\ndebug off\ndebug\n".into()),
        ("debug bad arg", "debug bogus\n".into()),
        ("verbose query", "verbose\n".into()),
        ("verbose on", "verbose on\nverbose\n".into()),
        ("verbose off", "verbose on\nverbose off\nverbose\n".into()),
        ("verbose bad arg", "verbose bogus\n".into()),
        // debug prefixes each dispatched command; verbose echoes the raw line.
        ("debug echoes command", "debug on\nstatus\nwhoami\nbogus a b\n".into()),
        ("verbose echoes line", "verbose on\nfoo   bar\n\nstatus\n".into()),
        ("debug and verbose", "debug on\nverbose on\nstatus\nfoo\n\n".into()),
        ("status empty", "status\n".into()),
        ("help", "help\n".into()),
        ("question-mark alias", "?\n".into()),
        ("help twice", "help\n?\n".into()),
    ]);
}

#[test]
fn exit_paths() {
    check_all(&[
        // exit(0) from inside process_command: the trailing commands must not run
        ("exit", "exit\nstatus\n".into()),
        ("quit", "quit\nstatus\n".into()),
        ("exit with extra args", "exit now please\n".into()),
        ("exit after work", "adduser a p\nlogin a p\ncreatefile f x\nexit\nlistfiles\n".into()),
        // EOF from fgets is the other way out
        ("EOF ends loop", "status\n".into()),
    ]);
}

#[test]
fn unknown_and_prefix_suggestion_branches() {
    // Reached only after every strcmp fails, in source order. Note that a
    // prefix test can shadow a longer one, so order matters.
    check_all(&[
        ("add prefix", "add\naddx\nadduse\n".into()),
        ("log prefix", "log\nlogx\nlogi\n".into()),
        ("list prefix", "list\nlistx\nlistu\n".into()),
        ("create prefix", "create\ncreatex\ncreatefil\n".into()),
        ("read prefix", "read\nreadx\nreadfil\n".into()),
        ("write prefix", "write2\nwritex\nwritefil\n".into()),
        ("delete prefix", "delete\ndeletex\ndeletefil\n".into()),
        ("unknown", "bogus\nxyz\nzzz\n".into()),
        // one char short of each prefix, so the suggestion must NOT fire
        ("shorter than prefix", "ad\nlo\nlis\ncreat\nrea\nwrit\ndelet\n".into()),
        ("case sensitivity", "ADDUSER\nHelp\nSTATUS\n".into()),
    ]);
}

#[test]
fn tokenizing_branches() {
    check_all(&[
        // strtok on " \t" collapses runs and ignores leading/trailing delimiters
        ("multiple spaces", "compare    a     b\n".into()),
        ("leading whitespace", "   status\n".into()),
        ("trailing whitespace", "status   \n".into()),
        ("tab separated", "set\t\tx\t\ty\nget x\n".into()),
        ("mixed separators", " \t compare \t a \t b \t \n".into()),
        // MAX_ARGS is 10, so tokens past the tenth argument are dropped
        ("exactly 10 args", "match p a b c d e f g h i\n".into()),
        ("more than 10 args", "match p a b c d e f g h i j k l m n o\n".into()),
        // MAX_COMMAND is 64, so strncpy truncates each token to 63 bytes
        ("63-byte token", format!("compare {} {}\n", rep("z", 63), rep("z", 63))),
        ("64-byte token truncated", format!("compare {} {}\n", rep("z", 64), rep("z", 63))),
        ("70-byte tokens", format!("compare {} {}\n", rep("z", 70), rep("z", 70))),
        ("long command name", format!("{}\n", rep("q", 80))),
    ]);
}

#[test]
fn input_length_and_line_ending_branches() {
    check_all(&[
        // fgets caps a line at MAX_INPUT-1 = 255 bytes and leaves the rest to
        // be read as the next line, so one long line becomes several commands.
        ("254-byte line", format!("compare {} b\n", rep("a", 246))),
        ("255-byte line", format!("compare {} b\n", rep("a", 247))),
        ("256-byte line", format!("compare {} b\n", rep("a", 248))),
        ("300-byte line", format!("compare {} b\n", rep("a", 300))),
        ("600-byte line", format!("{}\nstatus\n", rep("x", 600))),
        ("long line splits into commands", format!("{}status\n", rep(" ", 260))),
        // strcspn only strips the first \n, so a \r survives into the token
        ("CRLF endings", "status\r\nwhoami\r\n".into()),
        ("bare CR", "status\rwhoami\n".into()),
    ]);
}

#[test]
fn high_byte_and_nul_branches() {
    // strcmp compares as unsigned char, so bytes >= 0x80 sort above ASCII.
    // Non-UTF-8 bytes must survive to stdout unchanged.
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("0xff vs a", b"compare \xff a\n".to_vec()),
        ("a vs 0xff", b"compare a \xff\n".to_vec()),
        ("0x80 vs 0x7f", b"compare \x80 \x7f\n".to_vec()),
        ("utf8 bytes", b"compare \xc3\xa9 A\n".to_vec()),
        ("high byte prefix", b"startswith \xffabc \xff\n".to_vec()),
        ("high bytes in name", b"adduser \xff\xfe pw\nlistusers\n".to_vec()),
        ("match on high bytes", b"match \xff x\xffy zzz\n".to_vec()),
        // a NUL mid-line terminates the C string, so the tail is dropped
        ("embedded NUL", b"set\0x y\nstatus\n".to_vec()),
        ("NUL after command", b"status\0extra\n".to_vec()),
    ];
    for (name, input) in cases {
        assert_identical(name, &input);
    }
}

// ---------------------------------------------------------------------------
// Phase C: the buffer-overflow paths
//
// `strcpy` copies up-to-63-byte tokens into 32-byte struct fields. These cases
// pin down the resulting field-to-field, record-to-record and
// array-to-neighbouring-global corruption.
// ---------------------------------------------------------------------------

#[test]
fn overflow_within_a_record() {
    check_all(&[
        // A 40-byte username fills name[32] and runs into password[32]; the
        // password write then lands after it, so the stored name reads back as
        // the first 32 bytes of the username followed by the password.
        (
            "username overflows into password",
            format!("adduser {} secret 7\nlistusers\nstatus\n", rep("A", 40)),
        ),
        ("username exactly 31", format!("adduser {} pw 1\nlistusers\n", rep("A", 31))),
        ("username exactly 32", format!("adduser {} pw 1\nlistusers\n", rep("A", 32))),
        ("username exactly 33", format!("adduser {} pw 1\nlistusers\n", rep("A", 33))),
        // A 48-byte password runs past password[32] into permission_level,
        // which is then assigned afterwards and truncates the stored password.
        (
            "password overflows into permission_level",
            format!(
                "adduser bob {} 3\nlistusers\nlogin bob {}\nwhoami\n",
                rep("P", 48),
                rep("P", 48)
            ),
        ),
        ("password exactly 31", format!("adduser b {} 3\nlogin b {}\n", rep("P", 31), rep("P", 31))),
        ("password exactly 32", format!("adduser b {} 3\nlogin b {}\n", rep("P", 32), rep("P", 32))),
        // Logging in under the mangled name that listusers reports
        (
            "login under mangled name",
            format!(
                "adduser {} secret 9\nlistusers\nlogin {}secret secret\nwhoami\n",
                rep("A", 40),
                rep("A", 32)
            ),
        ),
        // A 63-byte variable name fills name[32] and runs into value[128]
        (
            "variable name overflows into value",
            format!("set {} myvalue\nlistvars\nget {}\n", rep("N", 63), rep("N", 63)),
        ),
        (
            "variable value 63 bytes",
            format!("set v {}\nlistvars\nget v\n", rep("V", 63)),
        ),
    ]);
}

#[test]
fn overflow_across_records() {
    let mangled = format!("{}secret", rep("A", 32));
    check_all(&[
        // A 63-byte password on user 0 runs past its own record into user 1's
        // name field, before user 1 is created.
        (
            "password spills into next user record",
            format!("adduser bob {} 4\nadduser carol cpw 6\nlistusers\nstatus\n", rep("P", 63)),
        ),
        // The owner copied into a file is current_user->name, which is itself
        // over-long here, so owner[32] at offset 576 of a 612-byte record
        // overruns permissions and the following record's filename.
        (
            "over-long owner spills across file records",
            format!(
                "adduser {} secret 9\nlogin {} secret\ncreatefile f1 c1\ncreatefile f2 c2\n\
                 listfiles\nreadfile f1\nreadfile f2\nstatus\ndeletefile f1\nlistfiles\n",
                rep("A", 40),
                mangled
            ),
        ),
        // The same, but ending in the shift-down loop so corrupted records get
        // memmoved over one another.
        (
            "delete shifts corrupted records",
            format!(
                "adduser {} secret 9\nlogin {} secret\ncreatefile f1 c1\ncreatefile f2 c2\n\
                 createfile f3 c3\ndeletefile f2\nlistfiles\nreadfile f3\n",
                rep("A", 40),
                mangled
            ),
        ),
    ]);
}

#[test]
fn overflow_into_adjacent_globals_without_crashing() {
    // `users` ends exactly where `user_count` begins. A 40-byte password on the
    // tenth user puts its NUL terminator on user_count's low byte, zeroing the
    // count; the following field assignments and the increment then operate on
    // index 0. This corrupts state without faulting.
    let mut s = String::new();
    for i in 1..=9 {
        s += &format!("adduser u{i} p{i} {i}\n");
    }
    s += &format!("adduser last {} 5\n", rep("P", 40));
    s += "listusers\nstatus\nwhoami\n";
    check_all(&[("40-byte password zeroes user_count", s)]);
}

#[test]
fn overflow_that_crashes_the_reference() {
    // These make the C reference die from SIGSEGV, and the Rust program must
    // die the same way after emitting the same bytes.
    //
    // `users` is immediately followed by `user_count`, then `current_user`,
    // then `files`; `files` is immediately followed by `file_count`. So an
    // overflowing strcpy can overwrite the counter that the very next
    // statement uses as an array subscript, producing a wild store.

    // Case 1: 63-byte password on the tenth user overwrites user_count with
    // 0x50505050, and the next line assigns users[0x50505050].permission_level.
    let mut users_case = String::new();
    for i in 1..=9 {
        users_case += &format!("adduser u{i} p{i} {i}\n");
    }
    users_case += &format!("adduser last {} 5\n", rep("P", 63));
    users_case += "listusers\nstatus\n";

    // Case 2: with an over-long owner, creating the twentieth file overruns
    // files[] into file_count, and the next line assigns
    // files[<clobbered>].permissions.
    let mangled = format!("{}secret", rep("A", 32));
    let mut files_case = format!(
        "adduser {} secret 9\nlogin {} secret\n",
        rep("A", 40),
        mangled
    );
    for i in 1..=20 {
        files_case += &format!("createfile f{i} c{i}\n");
    }
    files_case += "listfiles\nstatus\n";

    // Case 3: the same crash, but with enough output beforehand that glibc has
    // already flushed whole 4096-byte blocks. This pins down how much of the
    // stream survives: the flushed blocks appear, the rest is lost with the
    // process.
    let mut partial_flush = format!(
        "adduser {} secret 9\nlogin {} secret\n",
        rep("A", 40),
        mangled
    );
    for i in 1..=19 {
        partial_flush += &format!("createfile f{i} c{i}\n");
    }
    for _ in 0..5 {
        partial_flush += "help\n";
    }
    partial_flush += "createfile f20 c20\nlistfiles\n";

    check_all(&[
        ("63-byte password clobbers user_count", users_case),
        ("over-long owner clobbers file_count", files_case),
        ("crash after partial 4096-byte flushes", partial_flush),
    ]);
}

#[test]
fn crashing_cases_really_do_die_from_a_signal() {
    // Guards the test above: if the reference stopped crashing (different
    // compiler, different .bss layout) these assertions would fire rather than
    // letting the comparison pass trivially.
    let mangled = format!("{}secret", rep("A", 32));
    let mut input = format!(
        "adduser {} secret 9\nlogin {} secret\n",
        rep("A", 40),
        mangled
    );
    for i in 1..=20 {
        input += &format!("createfile f{i} c{i}\n");
    }
    input += "listfiles\nstatus\n";

    let c = run(&c_binary(), input.as_bytes());
    let r = run(&rust_binary(), input.as_bytes());
    assert_eq!(
        c.status,
        Err(libc_sigsegv()),
        "expected the C reference to die from SIGSEGV, got {:?}",
        c.status
    );
    assert_eq!(c.status, r.status, "Rust must die from the same signal");
    assert_eq!(c.stdout, r.stdout, "bytes emitted before the crash must match");
    assert_eq!(c.stderr, r.stderr);
}

fn libc_sigsegv() -> i32 {
    11
}

// ---------------------------------------------------------------------------
// Long mixed sessions
// ---------------------------------------------------------------------------

#[test]
fn long_mixed_session() {
    let session = "\
adduser alice apw 1
adduser bob bpw 5
adduser carol cpw 9
listusers
login alice apw
whoami
createfile notes hello
createfile data 12345
listfiles
readfile notes
writefile notes goodbye
readfile notes
set greeting hi
set greeting hello
get greeting
listvars
compare alice bob
compareN alice alicia 4
startswith alice ali
match a alice bob carol
status
logout
login bob bpw
writefile notes bobwrote
readfile notes
deletefile notes
listfiles
logout
login carol cpw
deletefile data
listfiles
unset greeting
listvars
debug on
status
verbose on
help
verbose off
debug off
status
exit
";
    check_all(&[("long mixed session", session.into())]);
}

#[test]
fn every_command_name_once() {
    // Dispatch every literal the C compares against, including all aliases,
    // so no routing branch is left unexecuted.
    let names = [
        "adduser", "login", "logout", "whoami", "listusers", "users", "createfile", "touch",
        "readfile", "cat", "writefile", "write", "deletefile", "rm", "listfiles", "ls", "set",
        "get", "unset", "listvars", "vars", "compare", "cmp", "compareN", "cmpn", "startswith",
        "match", "debug", "verbose", "status", "help", "?",
    ];
    // No arguments: every handler takes its usage/guard path.
    let bare: String = names.iter().map(|n| format!("{n}\n")).collect();
    // With arguments, after logging in, so the handlers take their real paths.
    let mut armed = String::from("adduser a p 9\nlogin a p\ncreatefile seed body\nset k v\n");
    for n in &names {
        armed += &format!("{n} seed body 2\n");
    }
    check_all(&[
        ("every command, no args", bare),
        ("every command, with args", armed),
    ]);
}

// ---------------------------------------------------------------------------
// `time` is the one nondeterministic command
// ---------------------------------------------------------------------------

#[test]
fn time_command_matches_except_for_the_clock() {
    // `printf("Current time: %s", ctime(&now))` cannot be compared byte for
    // byte across two runs, so compare everything else and check that the
    // timestamp itself has ctime's fixed 25-character shape.
    let input = b"status\ntime\nstatus\n";
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    assert_eq!(c.status, r.status, "exit status differs");
    assert_eq!(c.stderr, r.stderr, "stderr differs");

    let strip = |out: &[u8]| -> (Vec<u8>, String) {
        let s = String::from_utf8_lossy(out).into_owned();
        let marker = "Current time: ";
        let at = s.find(marker).expect("`time` printed no timestamp");
        let rest = &s[at + marker.len()..];
        let nl = rest.find('\n').expect("ctime output ends in a newline");
        let stamp = rest[..nl].to_string();
        let mut redacted = String::new();
        redacted.push_str(&s[..at + marker.len()]);
        redacted.push_str("<TIME>");
        redacted.push_str(&rest[nl..]);
        (redacted.into_bytes(), stamp)
    };

    let (c_red, c_stamp) = strip(&c.stdout);
    let (r_red, r_stamp) = strip(&r.stdout);
    assert_eq!(
        String::from_utf8_lossy(&c_red),
        String::from_utf8_lossy(&r_red),
        "output around the timestamp differs"
    );
    // ctime yields "Www Mmm dd hh:mm:ss yyyy" -- 24 characters plus the \n we
    // already split off.
    for (who, stamp) in [("C", &c_stamp), ("RUST", &r_stamp)] {
        assert_eq!(stamp.len(), 24, "{who} timestamp has odd length: {stamp:?}");
        assert_eq!(
            stamp.chars().filter(|c| *c == ':').count(),
            2,
            "{who} timestamp is not a ctime string: {stamp:?}"
        );
    }
}
