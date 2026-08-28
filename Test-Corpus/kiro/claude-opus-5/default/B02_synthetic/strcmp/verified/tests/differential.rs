//! Differential tests: run the original C program and the Rust translation as
//! subprocesses over the same stdin and require byte-identical stdout, stderr
//! and identical termination status (exit code *or* fatal signal).
//!
//! Nothing here loads the Rust code as a library; both programs are driven the
//! way a shell would drive them (`./driver < input`).

use std::fs;
use std::io::Write;
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
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Build `c_src` with CMake once per test binary run, and return the executable.
fn c_bin() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let src = workspace_root().join("c_src");
        let build = src.join("build");
        let exe = build.join("driver");
        fs::create_dir_all(&build).expect("create c_src/build");

        let cfg = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let b = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("run cmake --build");
        assert!(
            b.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&b.stdout),
            String::from_utf8_lossy(&b.stderr)
        );

        assert!(exe.is_file(), "expected C executable at {}", exe.display());
        exe
    })
    .as_path()
}

// ---------------------------------------------------------------------------
// running one program
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `Ok(code)` for a normal exit, `Err(signal)` when killed by a signal.
    status: Result<i32, i32>,
}

fn status_str(s: &Result<i32, i32>) -> String {
    match s {
        Ok(c) => format!("exit {c}"),
        Err(sig) => format!("killed by signal {sig}"),
    }
}

/// stdin is a real file, exactly like `./driver < input`, which also avoids any
/// possibility of a pipe deadlock on large outputs.
fn stdin_file(input: &[u8]) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "driver-difftest-{}-{}.in",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let mut f = fs::File::create(&path).expect("create stdin temp file");
    f.write_all(input).expect("write stdin temp file");
    f.sync_all().ok();
    path
}

fn run(bin: &Path, stdin_path: &Path) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let f = fs::File::open(stdin_path).expect("reopen stdin temp file");
    let out = Command::new(bin)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", bin.display()));

    let status = match out.status.code() {
        Some(c) => Ok(c),
        None => Err(out.status.signal().expect("no exit code and no signal")),
    };
    Outcome {
        stdout: out.stdout,
        stderr: out.stderr,
        status,
    }
}

// ---------------------------------------------------------------------------
// comparison
// ---------------------------------------------------------------------------

fn describe_input(input: &[u8]) -> String {
    let shown: Vec<u8> = input.iter().copied().take(400).collect();
    let mut s = String::new();
    for b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(b as char),
            other => s.push_str(&format!("\\x{other:02x}")),
        }
    }
    if input.len() > 400 {
        s.push_str(&format!("... ({} bytes total)", input.len()));
    }
    s
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()));
    let lo = at.saturating_sub(60);
    format!(
        "first difference at byte {at} (C len {}, Rust len {})\n     C: {:?}\n  Rust: {:?}",
        a.len(),
        b.len(),
        String::from_utf8_lossy(&a[lo..(at + 80).min(a.len())]),
        String::from_utf8_lossy(&b[lo..(at + 80).min(b.len())]),
    )
}

/// The core assertion: same stdout, same stderr, same termination status.
fn assert_same(case: &str, input: &[u8]) {
    let path = stdin_file(input);
    let c = run(c_bin(), &path);
    let r = run(&rust_bin(), &path);
    let _ = fs::remove_file(&path);

    let mut problems: Vec<String> = Vec::new();
    if c.stdout != r.stdout {
        problems.push(format!("stdout differs: {}", first_diff(&c.stdout, &r.stdout)));
    }
    if c.stderr != r.stderr {
        problems.push(format!("stderr differs: {}", first_diff(&c.stderr, &r.stderr)));
    }
    if c.status != r.status {
        problems.push(format!(
            "status differs: C {} vs Rust {}",
            status_str(&c.status),
            status_str(&r.status)
        ));
    }
    assert!(
        problems.is_empty(),
        "case `{case}`\ninput: {}\n{}",
        describe_input(input),
        problems.join("\n")
    );
}

/// Convenience: build stdin from command lines, each terminated by '\n'.
fn lines(cmds: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for c in cmds {
        v.extend_from_slice(c.as_bytes());
        v.push(b'\n');
    }
    v
}

fn rep(c: char, n: usize) -> String {
    std::iter::repeat(c).take(n).collect()
}

// ===========================================================================
// Phase A sanity: both programs exist and are runnable
// ===========================================================================

#[test]
fn both_binaries_run() {
    let path = stdin_file(b"");
    let c = run(c_bin(), &path);
    let r = run(&rust_bin(), &path);
    let _ = fs::remove_file(&path);
    assert_eq!(c.status, Ok(0), "C program should exit 0 on empty stdin");
    assert_eq!(r.status, Ok(0), "Rust program should exit 0 on empty stdin");
    assert!(
        c.stdout.starts_with(b"|---"),
        "C program should print the banner"
    );
    assert_eq!(c.stdout, r.stdout);
    assert_eq!(c.stderr, r.stderr);
}

// ===========================================================================
// Phase B: the input classes the C program branches on
// ===========================================================================

#[test]
fn empty_and_whitespace_input() {
    // `fgets` returns NULL straight away -> banner, one prompt, exit 0.
    assert_same("empty stdin", b"");
    // A single empty line: `strlen(command) == 0` -> process_command returns.
    assert_same("one newline", b"\n");
    assert_same("blank lines", b"\n   \n\t\t\n \t \n");
    // No trailing newline: fgets still returns the partial line.
    assert_same("no trailing newline", b"status");
    assert_same("single item", b"help\n");
    // Only separators, so strtok finds no first token at all.
    assert_same("separators only", b" \t \t \n");
}

#[test]
fn help_and_unknown_and_exit() {
    assert_same("help", &lines(&["help"]));
    assert_same("help alias ?", &lines(&["?"]));
    assert_same("unknown command", &lines(&["nosuchthing"]));
    // exit(0) mid-stream: the rest of stdin must be ignored.
    assert_same("exit", &lines(&["exit", "listusers"]));
    assert_same("quit", &lines(&["quit", "listusers"]));
    assert_same("exit with args", &lines(&["exit now please"]));
}

#[test]
fn every_command_with_too_few_arguments() {
    // Each of these takes the `arg_count < N` usage branch.
    assert_same(
        "usage branches",
        &lines(&[
            "adduser",
            "adduser onlyname",
            "login",
            "login onlyname",
            "logout",
            "whoami",
            "listusers",
            "users",
            "createfile",
            "readfile",
            "writefile",
            "writefile onlyname",
            "deletefile",
            "listfiles",
            "ls",
            "set",
            "set onlyname",
            "get",
            "unset",
            "listvars",
            "vars",
            "compare",
            "compare one",
            "compareN",
            "compareN a b",
            "startswith",
            "startswith a",
            "match",
            "match onlypattern",
            "debug",
            "verbose",
            "status",
        ]),
    );
}

#[test]
fn command_aliases() {
    assert_same(
        "aliases",
        &lines(&[
            "adduser a p 9",
            "login a p",
            "touch f body",
            "cat f",
            "write f other",
            "cat f",
            "ls",
            "vars",
            "users",
            "cmp a a",
            "cmpn ab ac 1",
            "rm f",
            "ls",
        ]),
    );
}

#[test]
fn strncmp_suggestion_ladder() {
    // Every `strncmp(command, "...", n)` fallback, plus prefixes one byte too
    // short to reach them.
    assert_same(
        "suggestions",
        &lines(&[
            "add", "addx", "adduse", "log", "logx", "logi", "list", "listx", "listuser",
            "create", "createx", "createfil", "read", "readx", "readfil", "write", "writex",
            "writefil", "delete", "deletex", "deletefil", "ad", "lo", "lis", "creat", "rea",
            "writ", "delet", "l", "w", "d", "c", "r", "a",
        ]),
    );
}

#[test]
fn user_management_paths() {
    assert_same("no users registered", &lines(&["listusers"]));
    assert_same(
        "add / duplicate / list / login / logout",
        &lines(&[
            "adduser alice pw1",
            "adduser bob pw2 7",
            "adduser alice other",
            "listusers",
            "whoami",
            "login alice pw1",
            "whoami",
            "listusers",
            "login bob pw2",
            "logout",
            "logout",
            "login bob pw2",
            "whoami",
            "logout",
        ]),
    );
    assert_same(
        "login failure paths",
        &lines(&[
            "login ghost pw",
            "adduser alice pw",
            "login alice wrong",
            "login alice pw",
            "login alice pw",
        ]),
    );
}

#[test]
fn adduser_permission_level_atoi() {
    // `atoi` on the optional third argument, including overflow/truncation.
    assert_same(
        "atoi levels",
        &lines(&[
            "adduser a p",
            "adduser b p 0",
            "adduser c p -3",
            "adduser d p +5",
            "adduser e p 12abc",
            "adduser f p abc",
            "adduser g p 2147483647",
            "adduser h p 2147483648",
            "adduser i p 4294967296",
            "adduser j p 99999999999999999999",
            "listusers",
            "status",
        ]),
    );
    assert_same(
        "atoi negative saturation",
        &lines(&["adduser a p -99999999999999999999", "listusers"]),
    );
}

#[test]
fn maximum_users_reached() {
    let mut cmds: Vec<String> = (1..=11).map(|i| format!("adduser u{i} p{i} {i}")).collect();
    cmds.push("listusers".into());
    cmds.push("status".into());
    let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    assert_same("MAX_USERS", &lines(&refs));
}

#[test]
fn file_management_paths() {
    assert_same(
        "must be logged in",
        &lines(&[
            "createfile f",
            "writefile f x",
            "deletefile f",
            "readfile f",
            "listfiles",
        ]),
    );
    assert_same(
        "create / duplicate / read / write / delete",
        &lines(&[
            "adduser a p",
            "login a p",
            "listfiles",
            "createfile f1",
            "createfile f1",
            "createfile f2 body",
            "listfiles",
            "readfile f1",
            "readfile f2",
            "readfile missing",
            "writefile f1 new",
            "readfile f1",
            "writefile missing x",
            "deletefile f1",
            "listfiles",
            "deletefile f1",
        ]),
    );
    assert_same(
        "delete shifts the array down",
        &lines(&[
            "adduser a p",
            "login a p",
            "createfile f1 one",
            "createfile f2 two",
            "createfile f3 three",
            "deletefile f2",
            "listfiles",
            "readfile f3",
            "deletefile f3",
            "deletefile f1",
            "listfiles",
        ]),
    );
}

#[test]
fn file_permission_boundaries() {
    // writefile needs owner or level >= 5; deletefile needs owner or level >= 9.
    for (level, name) in [(4, "level 4"), (5, "level 5"), (8, "level 8"), (9, "level 9")] {
        let other = format!("adduser other p {level}");
        assert_same(
            name,
            &lines(&[
                "adduser owner p 1",
                &other,
                "login owner p",
                "createfile f mine",
                "logout",
                "login other p",
                "writefile f theirs",
                "readfile f",
                "deletefile f",
                "listfiles",
            ]),
        );
    }
}

#[test]
fn maximum_files_reached() {
    let mut cmds: Vec<String> = vec!["adduser a p 9".into(), "login a p".into()];
    cmds.extend((1..=21).map(|i| format!("createfile f{i} c{i}")));
    cmds.push("listfiles".into());
    cmds.push("status".into());
    let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    assert_same("MAX_FILES", &lines(&refs));
}

#[test]
fn variable_paths() {
    assert_same(
        "set / update / get / unset",
        &lines(&[
            "listvars",
            "get missing",
            "unset missing",
            "set x 1",
            "set x 2",
            "get x",
            "listvars",
            "unset x",
            "listvars",
            "get x",
        ]),
    );
    assert_same(
        "unset shifts the array down",
        &lines(&[
            "set a 1", "set b 2", "set c 3", "unset b", "listvars", "unset c", "unset a",
            "listvars",
        ]),
    );
}

#[test]
fn maximum_variables_reached() {
    let mut cmds: Vec<String> = (1..=21).map(|i| format!("set v{i} x{i}")).collect();
    cmds.push("listvars".into());
    cmds.push("status".into());
    // Updating an existing variable must still work once the table is full.
    cmds.push("set v1 updated".into());
    cmds.push("get v1".into());
    let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    assert_same("MAX_VARIABLES", &lines(&refs));
}

#[test]
fn strcmp_return_values() {
    // glibc returns the difference of the first differing bytes, not -1/0/1.
    assert_same(
        "compare",
        &lines(&[
            "compare abc abc",
            "compare abc abd",
            "compare abd abc",
            "compare abc abcd",
            "compare abcd abc",
            "compare a Z",
            "compare Z a",
            "compare ~ !",
            "compare A a",
        ]),
    );
    assert_same(
        "compare with high bytes",
        b"compare \xff a\ncompare a \xff\ncompare \xff \xfe\n",
    );
}

#[test]
fn strncmp_return_values_and_n_conversion() {
    // `n` is an int fed to a size_t parameter: negative values sign extend to
    // an enormous length, and are still printed back as a negative int.
    assert_same(
        "compareN",
        &lines(&[
            "compareN abc abd 0",
            "compareN abc abd 1",
            "compareN abc abd 2",
            "compareN abc abd 3",
            "compareN abc abd 99",
            "compareN abc abd -1",
            "compareN abc abc -1",
            "compareN abc abc -99999999999",
            "compareN abc abd notanumber",
            "compareN a b 2147483647",
            "compareN abc abcdef 99",
            "compareN abcdef abc 99",
        ]),
    );
}

#[test]
fn startswith_and_match() {
    assert_same(
        "startswith",
        &lines(&[
            "startswith hello hell",
            "startswith hell hello",
            "startswith abc abc",
            "startswith abc x",
            "startswith a aa",
        ]),
    );
    assert_same(
        "match",
        &lines(&[
            "match ab ab",
            "match ab xaby",
            "match ab zz",
            "match ab ab xaby zz",
            "match aaa a",
            "match a aaa",
        ]),
    );
    // MAX_ARGS is 10, so trailing arguments beyond the tenth are dropped.
    assert_same(
        "match with more than MAX_ARGS tokens",
        &lines(&["match p p a b c d e f g h i j k l m"]),
    );
}

#[test]
fn debug_and_verbose_modes() {
    assert_same(
        "modes",
        &lines(&[
            "debug",
            "debug on",
            "status",
            "debug on",
            "debug off",
            "debug maybe",
            "verbose",
            "verbose on",
            "hello there",
            "verbose off",
            "verbose maybe",
            "status",
        ]),
    );
    // With verbose on, the blank-line path still prints the [VERBOSE] line even
    // though process_command returns immediately.
    assert_same(
        "debug + verbose over blank input",
        &lines(&["debug on", "verbose on", "", "   ", "status", "junk", "exit"]),
    );
}

#[test]
fn tokenisation() {
    // strtok splits on ' ' and '\t' only and skips runs of them.
    assert_same(
        "leading/trailing/multiple separators",
        b"  \t adduser \t alice \t pw \t 3 \t \nlistusers\n set\tk\tv\nlistvars\n",
    );
    // A token longer than MAX_COMMAND - 1 is truncated to 63 bytes.
    let mut v = Vec::new();
    v.extend_from_slice(b"set ");
    v.extend_from_slice(rep('n', 63).as_bytes());
    v.extend_from_slice(b" ");
    v.extend_from_slice(rep('v', 63).as_bytes());
    v.push(b'\n');
    v.extend_from_slice(b"listvars\n");
    v.extend_from_slice(b"get ");
    v.extend_from_slice(rep('n', 63).as_bytes());
    v.push(b'\n');
    assert_same("63 byte tokens", &v);

    let mut v = Vec::new();
    v.extend_from_slice(b"set ");
    v.extend_from_slice(rep('n', 64).as_bytes());
    v.extend_from_slice(b" value\nlistvars\n");
    assert_same("64 byte token truncated", &v);

    let mut v = Vec::new();
    v.extend_from_slice(b"compare ");
    v.extend_from_slice(rep('a', 70).as_bytes());
    v.push(b' ');
    v.extend_from_slice(rep('a', 70).as_bytes());
    v.push(b'\n');
    assert_same("70 byte tokens truncated to 63", &v);
}

#[test]
fn fgets_chunking_at_max_input() {
    // MAX_INPUT is 256, so fgets consumes at most 255 bytes per call and a
    // longer line is split into several iterations, each with its own prompt.
    for n in [253usize, 254, 255, 256, 257, 300, 511, 512, 513] {
        let mut v = Vec::new();
        v.extend_from_slice(rep('x', n).as_bytes());
        v.extend_from_slice(b"\nstatus\n");
        assert_same(&format!("line of {n} x's"), &v);
    }
    let mut v = Vec::new();
    v.extend_from_slice(b"set k ");
    v.extend_from_slice(rep('v', 300).as_bytes());
    v.extend_from_slice(b"\nlistvars\n");
    assert_same("long set line split across fgets calls", &v);

    // A line whose split boundary lands between tokens: the tail becomes its
    // own command.
    let mut v = Vec::new();
    v.extend_from_slice(b"status");
    v.extend_from_slice(rep(' ', 250).as_bytes());
    v.extend_from_slice(b"help\n");
    assert_same("split between tokens", &v);
}

#[test]
fn embedded_nuls_and_carriage_returns() {
    // strcspn stops at the first '\n' *or* the terminating NUL, so a NUL in the
    // middle of a line truncates the command.
    assert_same(
        "embedded NUL",
        b"sta\x00tus\nset a\x00b c\nlistvars\nstatus\n",
    );
    assert_same("NUL first", b"\x00status\nstatus\n");
    // '\r' is not a separator and not stripped, so it stays part of the token.
    assert_same("CRLF", b"status\r\nhelp\r\ncompare a\r b\r\n");
}

#[test]
fn non_utf8_bytes_pass_through() {
    assert_same(
        "high bytes",
        b"compare \xff\xfe \x01\x02\nset \xc3\x28 \xff\nlistvars\nmatch \xff \xff\xff\n\
          adduser \xe9\xe9 \xf0 3\nlistusers\nlogin \xe9\xe9 \xf0\nwhoami\n",
    );
}

#[test]
fn status_reflects_all_counters() {
    assert_same(
        "status",
        &lines(&[
            "status",
            "adduser a p 3",
            "login a p",
            "createfile f x",
            "set v 1",
            "debug on",
            "verbose on",
            "status",
            "logout",
            "status",
        ]),
    );
}

// ===========================================================================
// Phase C: paths reached only through the C code's fixed-size-buffer overflows
// ===========================================================================

#[test]
fn name_field_overflow_spills_into_password() {
    // `strcpy` of a token up to 63 bytes into `char name[32]`: the stored name
    // reads as its first 32 bytes followed by whatever `password` holds.
    for n in [31usize, 32, 33, 39, 40, 47, 63] {
        let name = rep('N', n);
        assert_same(
            &format!("username of {n} bytes"),
            &lines(&[
                &format!("adduser {name} pw 4"),
                "listusers",
                &format!("login {name} pw"),
                "whoami",
                "status",
                &format!("login {name}pw pw"),
                "whoami",
                "logout",
            ]),
        );
    }
    // Both fields long at once.
    let name = rep('N', 63);
    let pass = rep('P', 63);
    assert_same(
        "63 byte name and password",
        &lines(&[
            &format!("adduser {name} {pass} 4"),
            "listusers",
            &format!("login {name} {pass}"),
            "whoami",
            "status",
        ]),
    );
}

#[test]
fn last_user_slot_overflow_clobbers_globals() {
    // users[9].password starts 40 bytes before `user_count`, so a password of
    // 40 bytes zeroes its low byte, 41 bytes stores a character into it, and 42
    // or more produce an index far outside the writable pages.
    for n in 0..=63usize {
        let mut cmds: Vec<String> = (1..=9).map(|i| format!("adduser u{i} p{i} {i}")).collect();
        cmds.push(format!("adduser LAST {} 5", rep('P', n)));
        cmds.push("listusers".into());
        cmds.push("status".into());
        cmds.push("whoami".into());
        cmds.push("listfiles".into());
        cmds.push("listvars".into());
        let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
        assert_same(&format!("users[9].password of {n} bytes"), &lines(&refs));
    }
}

#[test]
fn last_user_slot_name_overflow() {
    for n in 28..=63usize {
        let mut cmds: Vec<String> = (1..=9).map(|i| format!("adduser u{i} p{i} {i}")).collect();
        cmds.push(format!("adduser {} pw 5", rep('N', n)));
        cmds.push("listusers".into());
        cmds.push("status".into());
        cmds.push("whoami".into());
        let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
        assert_same(&format!("users[9].name of {n} bytes"), &lines(&refs));
    }
}

#[test]
fn continuing_after_a_clobbered_user_count() {
    for n in [40usize, 41] {
        let mut cmds: Vec<String> = (1..=9).map(|i| format!("adduser u{i} p{i} {i}")).collect();
        cmds.push(format!("adduser LAST {} 5", rep('P', n)));
        cmds.extend(
            [
                "listusers",
                "adduser fresh np 2",
                "listusers",
                "login u1 p1",
                "whoami",
                "createfile z body",
                "listfiles",
                "readfile z",
                "set k v",
                "listvars",
                "status",
                "logout",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
        let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
        assert_same(&format!("keep going after {n} byte clobber"), &lines(&refs));
    }
}

#[test]
fn clobbering_counters_with_high_bytes() {
    // 0xff bytes drive the clobbered counter negative instead of positive.
    for n in [39usize, 40, 41, 44, 48] {
        let mut input = Vec::new();
        for i in 1..=9 {
            input.extend_from_slice(format!("adduser u{i} p{i} {i}\n").as_bytes());
        }
        input.extend_from_slice(b"adduser LAST ");
        input.extend(std::iter::repeat(0xffu8).take(n));
        input.extend_from_slice(b" 5\n");
        input.extend_from_slice(b"listusers\nstatus\nwhoami\nlistfiles\n");
        assert_same(&format!("0xff password of {n} bytes"), &input);
    }
}

#[test]
fn owner_field_overflow_clobbers_file_count() {
    // `files[i].owner` is a char[32] filled by strcpy from current_user->name.
    // A name that already overflowed is longer than 31 bytes, so the copy runs
    // past `owner`; in the last slot it reaches `file_count` and beyond.
    for k in 1..=31usize {
        let pass = rep('P', k);
        let stored = format!("{}{}", rep('N', 32), pass); // what strcmp matches
        let mut cmds: Vec<String> = vec![
            format!("adduser {} {pass} 9", rep('N', 32)),
            format!("login {stored} {pass}"),
            "whoami".into(),
        ];
        cmds.extend((1..=20).map(|i| format!("createfile f{i}")));
        cmds.extend(
            ["listfiles", "status", "listvars", "readfile f20", "get x"]
                .iter()
                .map(|s| s.to_string()),
        );
        let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
        assert_same(&format!("owner name of {} bytes, full table", 32 + k), &lines(&refs));

        // Same owner length but only a couple of files, so the overflow lands
        // inside `files` instead of past its end.
        let refs2: Vec<String> = vec![
            format!("adduser {} {pass} 9", rep('N', 32)),
            format!("login {stored} {pass}"),
            "createfile a A".into(),
            "createfile b B".into(),
            "listfiles".into(),
            "readfile a".into(),
            "writefile a Z".into(),
            "deletefile a".into(),
            "listfiles".into(),
            "status".into(),
            "listvars".into(),
        ];
        let refs2: Vec<&str> = refs2.iter().map(|s| s.as_str()).collect();
        assert_same(
            &format!("owner name of {} bytes, two files", 32 + k),
            &lines(&refs2),
        );
    }
}

#[test]
fn variable_field_overflow_stays_inside_the_table() {
    // variable_t is name[32] + value[128], so a 63 byte name spills into
    // `value` but can never leave the table.
    let name = rep('N', 63);
    let value = rep('V', 63);
    let mut cmds: Vec<String> = (1..=19).map(|i| format!("set v{i} x{i}")).collect();
    cmds.push(format!("set {name} {value}"));
    cmds.push("listvars".into());
    cmds.push(format!("get {name}"));
    cmds.push(format!("set {name} other"));
    cmds.push("listvars".into());
    cmds.push(format!("unset {name}"));
    cmds.push("listvars".into());
    cmds.push("status".into());
    let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
    assert_same("last variable slot overflow", &lines(&refs));
}

#[test]
fn output_buffered_before_a_fatal_fault_matches() {
    // When the C program dies of SIGSEGV, whatever is still sitting in its
    // 4096 byte stdout buffer is lost. The amount that survives therefore
    // depends on how much was printed first.
    for reps in [0usize, 1, 3, 7, 10, 30] {
        let mut cmds: Vec<String> = (1..=9).map(|i| format!("adduser u{i} p{i} {i}")).collect();
        cmds.extend(std::iter::repeat("help".to_string()).take(reps));
        cmds.push(format!("adduser LAST {} 5", rep('P', 48)));
        cmds.push("status".into());
        let refs: Vec<&str> = cmds.iter().map(|s| s.as_str()).collect();
        assert_same(&format!("fault after {reps} help calls"), &lines(&refs));
    }
}

#[test]
fn long_mixed_session() {
    assert_same(
        "mixed session",
        &lines(&[
            "debug on",
            "verbose on",
            "adduser root root 9",
            "adduser guest guest 1",
            "login root root",
            "createfile a A",
            "createfile b B",
            "set v 1",
            "listusers",
            "listfiles",
            "listvars",
            "compare a b",
            "compareN a b 1",
            "startswith abc ab",
            "match a a b",
            "status",
            "whoami",
            "writefile a AA",
            "readfile a",
            "deletefile b",
            "logout",
            "login guest guest",
            "deletefile a",
            "whoami",
            "debug off",
            "verbose off",
            "help",
            "status",
            "exit",
        ]),
    );
}

// ===========================================================================
// `time` is the one command whose output depends on the clock
// ===========================================================================

#[test]
fn time_command_matches_modulo_the_clock() {
    // `ctime` renders the current second, so compare with the timestamp masked
    // and separately check that both render the same 24 character ctime shape.
    let path = stdin_file(b"time\ntime\nstatus\n");
    let c = run(c_bin(), &path);
    let r = run(&rust_bin(), &path);
    let _ = fs::remove_file(&path);

    assert_eq!(c.status, r.status, "time: termination status");
    assert_eq!(c.stderr, r.stderr, "time: stderr");

    fn mask(out: &[u8]) -> Vec<u8> {
        let needle = b"Current time: ";
        let mut v: Vec<u8> = Vec::new();
        let mut i = 0;
        while i < out.len() {
            if out[i..].starts_with(needle) {
                v.extend_from_slice(needle);
                i += needle.len();
                // ctime() is "Www Mmm dd hh:mm:ss yyyy\n" -- 25 bytes.
                let end = out[i..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|p| i + p + 1)
                    .unwrap_or(out.len());
                assert_eq!(end - i, 25, "unexpected ctime length in {:?}", &out[i..end]);
                v.extend_from_slice(b"<CTIME>\n");
                i = end;
            } else {
                v.push(out[i]);
                i += 1;
            }
        }
        v
    }

    assert_eq!(
        mask(&c.stdout),
        mask(&r.stdout),
        "time: stdout differs outside the timestamp"
    );
    assert_eq!(
        c.stdout.len(),
        r.stdout.len(),
        "time: stdout lengths differ"
    );
}
