//! Differential tests: run the C program and the Rust program as subprocesses
//! with identical stdin and require byte-identical stdout, byte-identical
//! stderr and an identical exit status (including death by signal).
//!
//! Nothing here links the Rust code as a library; both sides are driven exactly
//! the way a shell would drive them.

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// How a run ended: exit code, or the signal that killed it.
#[derive(Debug, PartialEq, Eq)]
enum Status {
    Code(i32),
    Signal(i32),
}

#[derive(Debug, PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: Status,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the compiled C program, configuring/building it on first use.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cmake = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is it installed?)");
            assert!(
                cmake.status.success(),
                "cmake configure failed:\n{}",
                String::from_utf8_lossy(&cmake.stderr)
            );
            let built = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                built.status.success(),
                "cmake --build failed:\n{}",
                String::from_utf8_lossy(&built.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
}

fn to_status(s: std::process::ExitStatus) -> Status {
    match (s.code(), s.signal()) {
        (Some(c), _) => Status::Code(c),
        (None, Some(sig)) => Status::Signal(sig),
        (None, None) => unreachable!("process neither exited nor signalled"),
    }
}

/// Run `bin` with `input` on stdin, capturing both streams.
fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        // The program consumes at most one byte, so a large input can leave the
        // writer with a broken pipe. That is expected; ignore it.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: to_status(out.status),
    }
}

/// Run `bin` with stdin redirected from `path` (a file or a directory).
fn run_with_stdin_file(bin: &Path, path: &str) -> Run {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let out = Command::new(bin)
        .stdin(Stdio::from(f))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: to_status(out.status),
    }
}

/// Run `bin` with stdin at EOF immediately (`/dev/null`).
fn run_null_stdin(bin: &Path) -> Run {
    let out = Command::new(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: to_status(out.status),
    }
}

/// Run `bin` with the read end of its stdout pipe closed before it can write
/// anything (the child blocks in `getchar()` until stdin is fed). Deterministic
/// broken-pipe scenario.
fn run_broken_stdout(bin: &Path, input: &[u8]) -> Status {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));

    // Close our end of the stdout pipe while the child is still waiting on stdin.
    drop(child.stdout.take());

    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }

    to_status(child.wait().expect("wait for child"))
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert the two programs agree on all three observable outputs.
#[track_caller]
fn assert_same(label: &str, input: &[u8]) {
    let c = run(c_bin(), input);
    let r = run(&rust_bin(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for {label}\n C: \"{}\"\nRS: \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "stderr differs for {label}\n C: \"{}\"\nRS: \"{}\"",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(c.status, r.status, "exit status differs for {label}");
}

// ---------------------------------------------------------------------------
// Input classes the C program branches on.
//
// `main` reads exactly one byte with `getchar()` and truncates it to `char`
// (signed here), so the interesting classes are:
//   * EOF (no byte available at all)             -> c == -1
//   * each of the 256 possible byte values       -> c == 0..=127 and -128..=-1
// and inside `driver`, every `<ctype.h>` classifier plus tolower/toupper is
// evaluated for that value. Bytes >= 0x80 exercise glibc's negative table
// indices; 0xff is indistinguishable from EOF after truncation.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_eof() {
    assert_same("empty stdin (EOF)", b"");
}

#[test]
fn stdin_at_eof_from_dev_null() {
    let c = run_null_stdin(c_bin());
    let r = run_null_stdin(&rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout differs for /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs for /dev/null stdin");
    assert_eq!(c.status, r.status, "status differs for /dev/null stdin");
}

#[test]
fn every_single_byte() {
    for b in 0u16..=255 {
        let byte = b as u8;
        assert_same(&format!("single byte 0x{byte:02x}"), &[byte]);
    }
}

#[test]
fn ascii_class_representatives() {
    // One case per ctype class boundary, kept explicit so a failure names the
    // class rather than a bare byte value.
    let cases: &[(&str, u8)] = &[
        ("NUL", 0x00),
        ("bell (control)", 0x07),
        ("tab (blank+space+control)", b'\t'),
        ("newline (space+control)", b'\n'),
        ("vertical tab (space)", 0x0b),
        ("form feed (space)", 0x0c),
        ("carriage return (space)", b'\r'),
        ("unit separator (last control)", 0x1f),
        ("space (print+blank, not graph)", b' '),
        ("bang (punct)", b'!'),
        ("digit 0", b'0'),
        ("digit 9", b'9'),
        ("colon (punct after digits)", b':'),
        ("upper A (xdigit)", b'A'),
        ("upper F (last xdigit)", b'F'),
        ("upper G", b'G'),
        ("upper Z", b'Z'),
        ("bracket (punct)", b'['),
        ("underscore (punct)", b'_'),
        ("backtick (punct)", b'`'),
        ("lower a (xdigit)", b'a'),
        ("lower f (last xdigit)", b'f'),
        ("lower g", b'g'),
        ("lower z", b'z'),
        ("brace (punct)", b'{'),
        ("tilde (last graph)", b'~'),
        ("DEL (control)", 0x7f),
        ("0x80 (first negative char)", 0x80),
        ("0xa0 (non-ASCII)", 0xa0),
        ("0xfe (non-ASCII)", 0xfe),
        ("0xff (aliases EOF after truncation)", 0xff),
    ];
    for (label, byte) in cases {
        assert_same(label, &[*byte]);
    }
}

#[test]
fn only_the_first_byte_is_consumed() {
    let cases: &[(&str, &[u8])] = &[
        ("two letters", b"ab"),
        ("letter then newline", b"a\n"),
        ("newline first", b"\nabc"),
        ("NUL first", b"\0abc"),
        ("space first", b" A"),
        ("0xff then more", b"\xffZ"),
        ("digits", b"1234567890"),
        ("line then line", b"A\nB\n"),
        ("whitespace run", b" \t\n\r\x0b\x0c"),
        ("utf8 multi-byte char", "é".as_bytes()),
        ("utf8 emoji", "🦀".as_bytes()),
        ("all bytes in order", &ALL_BYTES),
    ];
    for (label, input) in cases {
        assert_same(label, input);
    }
}

static ALL_BYTES: [u8; 256] = {
    let mut a = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        a[i] = i as u8;
        i += 1;
    }
    a
};

#[test]
fn large_input_only_first_byte_matters() {
    let big = vec![b'Q'; 1 << 16];
    assert_same("64 KiB of 'Q'", &big);
    let mut mixed = vec![b'\n'];
    mixed.extend(std::iter::repeat(b'x').take(1 << 16));
    assert_same("newline then 64 KiB", &mixed);
}

#[test]
fn stdin_is_a_directory_read_fails() {
    // read(2) on a directory fd fails with EISDIR, so getchar() yields EOF.
    let c = run_with_stdin_file(c_bin(), "/");
    let r = run_with_stdin_file(&rust_bin(), "/");
    assert_eq!(c.stdout, r.stdout, "stdout differs for directory stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs for directory stdin");
    assert_eq!(c.status, r.status, "status differs for directory stdin");
}

#[test]
fn closed_stdout_dies_the_same_way() {
    // The C program keeps SIGPIPE at its default disposition; the Rust runtime
    // ignores it unless the translation restores the default.
    for input in [&b"A"[..], &b""[..], &b"\xff"[..]] {
        let c = run_broken_stdout(c_bin(), input);
        let r = run_broken_stdout(&rust_bin(), input);
        assert_eq!(
            c, r,
            "status differs with closed stdout for input {:?}",
            show(input)
        );
    }
}

/// Like [`run`], but with extra argv entries and environment overrides.
fn run_with(bin: &Path, input: &[u8], args: &[&str], env: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(bin);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    {
        let mut stdin = child.stdin.take().expect("stdin pipe");
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }
    let out = child.wait_with_output().expect("wait for child");
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: to_status(out.status),
    }
}

#[test]
fn locale_environment_is_overridden_by_setlocale() {
    // `driver` calls setlocale(LC_ALL, "C"), so the classification must not
    // follow the ambient locale even for bytes that differ between locales.
    let locales = [
        "C",
        "C.UTF-8",
        "POSIX",
        "en_US.UTF-8",
        "tr_TR.UTF-8",
        "de_DE.UTF-8",
        "not-a-locale",
        "",
    ];
    for loc in locales {
        for byte in [b'A', b'a', b'I', b'i', 0xdf, 0xe9, 0xff] {
            let env = [("LC_ALL", loc), ("LANG", loc), ("LC_CTYPE", loc)];
            let c = run_with(c_bin(), &[byte], &[], &env);
            let r = run_with(&rust_bin(), &[byte], &[], &env);
            let label = format!("locale {loc:?} byte 0x{byte:02x}");
            assert_eq!(c.stdout, r.stdout, "stdout differs for {label}");
            assert_eq!(c.stderr, r.stderr, "stderr differs for {label}");
            assert_eq!(c.status, r.status, "status differs for {label}");
        }
    }
}

#[test]
fn argv_is_ignored() {
    // `int main()` takes no parameters; extra arguments must change nothing.
    let arg_sets: &[&[&str]] = &[&[], &["ignored"], &["-h"], &["--help"], &["a", "b", "c"]];
    for args in arg_sets {
        for input in [&b"A"[..], &b""[..]] {
            let c = run_with(c_bin(), input, args, &[]);
            let r = run_with(&rust_bin(), input, args, &[]);
            let label = format!("args {args:?} input {:?}", show(input));
            assert_eq!(c.stdout, r.stdout, "stdout differs for {label}");
            assert_eq!(c.stderr, r.stderr, "stderr differs for {label}");
            assert_eq!(c.status, r.status, "status differs for {label}");
        }
    }
}
