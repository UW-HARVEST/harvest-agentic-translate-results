//! Shared harness for the differential tests.
//!
//! Both programs are driven exactly the way a shell would drive them: a fresh
//! working directory, stdin/stdout/stderr wired to real files, no library
//! linkage of any kind. stdout, stderr, the exit status and every file left
//! behind in the working directory are then compared.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Locating and building the two executables
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust program under test.
///
/// Prefers the release binary (what `cargo build --release` produces and what
/// the C program is compared against); falls back to the binary cargo builds
/// for the test run so that `cargo test` works on a clean checkout.
pub fn rust_bin() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target").join("release").join("driver");
    if release.is_file() {
        return release;
    }
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C program, built with cmake on first use.
pub fn c_bin() -> &'static Path {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        if let Ok(p) = std::env::var("C_DRIVER") {
            return PathBuf::from(p);
        }
        let src = workspace_root().join("c_src");
        let build = src.join("build");
        let exe = build.join("driver");
        if exe.is_file() {
            return exe;
        }
        fs::create_dir_all(&build).expect("could not create c_src/build");
        let configure = Command::new("cmake")
            .arg("..")
            .current_dir(&build)
            .output()
            .expect("cmake is required to build the C reference program");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&configure.stderr)
        );
        let compile = Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .output()
            .expect("failed to run cmake --build");
        assert!(
            compile.status.success(),
            "cmake --build failed:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        assert!(exe.is_file(), "C build produced no driver executable");
        exe
    })
}

// ---------------------------------------------------------------------------
// Scratch directories (no external crates)
// ---------------------------------------------------------------------------

fn unique_dir(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!(
        "ascii-art-diff-{}-{}-{}-{}",
        std::process::id(),
        tag,
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).expect("could not create scratch directory");
    d
}

/// A file placed into the working directory before the program runs.
pub struct Fixture {
    pub name: &'static str,
    pub body: &'static [u8],
}

/// A fixture written from owned bytes (for generated content).
pub struct OwnedFixture {
    pub name: String,
    pub body: Vec<u8>,
}

pub enum Prep {
    /// Create an empty subdirectory.
    Dir(&'static str),
    /// Create a file with the given contents.
    File(&'static str, &'static [u8]),
    /// Create a file with generated contents.
    Owned(String, Vec<u8>),
    /// Create a file and make it read-only.
    ReadOnlyFile(&'static str, &'static [u8]),
}

fn apply(dir: &Path, preps: &[Prep]) {
    for p in preps {
        match p {
            Prep::Dir(name) => {
                fs::create_dir_all(dir.join(name)).expect("mkdir fixture");
            }
            Prep::File(name, body) => {
                if let Some(parent) = Path::new(name).parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(dir.join(parent)).expect("mkdir -p fixture");
                    }
                }
                fs::write(dir.join(name), body).expect("write fixture");
            }
            Prep::Owned(name, body) => {
                fs::write(dir.join(name), body).expect("write fixture");
            }
            Prep::ReadOnlyFile(name, body) => {
                let p = dir.join(name);
                fs::write(&p, body).expect("write fixture");
                let mut perm = fs::metadata(&p).expect("stat fixture").permissions();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    perm.set_mode(0o400);
                }
                #[cfg(not(unix))]
                {
                    perm.set_readonly(true);
                }
                fs::set_permissions(&p, perm).expect("chmod fixture");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

pub struct Outcome {
    /// `Some(code)` for a normal exit, `None` when killed by a signal.
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Every regular file left in the working directory, minus the three
    /// redirection files the harness itself creates.
    pub files: BTreeMap<String, Vec<u8>>,
    /// True when the program was still running when the time limit expired.
    pub timed_out: bool,
}

const LIMIT: std::time::Duration = std::time::Duration::from_secs(20);

/// How long to wait before concluding that a program is spinning. Both programs
/// enter the spin immediately, so this only has to be long enough to rule out a
/// slow start.
const SPIN_LIMIT: std::time::Duration = std::time::Duration::from_secs(4);

/// Runs `exe` with `input` on stdin in a fresh working directory prepared with
/// `preps`. stdio is redirected to files, so neither program can deadlock on a
/// full pipe and both see a fully buffered stdout (as glibc gives a C program
/// whose stdout is not a terminal).
pub fn run(exe: &Path, input: &[u8], preps: &[Prep], tag: &str) -> Outcome {
    run_with_limit(exe, input, preps, tag, LIMIT)
}

pub fn run_with_limit(
    exe: &Path,
    input: &[u8],
    preps: &[Prep],
    tag: &str,
    limit: std::time::Duration,
) -> Outcome {
    let dir = unique_dir(tag);
    apply(&dir, preps);

    let stdin_path = dir.join(".harness.stdin");
    let stdout_path = dir.join(".harness.stdout");
    let stderr_path = dir.join(".harness.stderr");
    fs::write(&stdin_path, input).expect("write stdin file");

    let mut child = Command::new(exe)
        .current_dir(&dir)
        .stdin(Stdio::from(
            fs::File::open(&stdin_path).expect("open stdin file"),
        ))
        .stdout(Stdio::from(
            fs::File::create(&stdout_path).expect("create stdout file"),
        ))
        .stderr(Stdio::from(
            fs::File::create(&stderr_path).expect("create stderr file"),
        ))
        .spawn()
        .unwrap_or_else(|e| panic!("could not spawn {}: {e}", exe.display()));

    let start = std::time::Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break Some(s),
            None => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    };

    let stdout = fs::read(&stdout_path).unwrap_or_default();
    let stderr = fs::read(&stderr_path).unwrap_or_default();

    let mut files = BTreeMap::new();
    collect(&dir, &dir, &mut files);
    files.remove(".harness.stdin");
    files.remove(".harness.stdout");
    files.remove(".harness.stderr");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.as_ref().and_then(|s| s.signal())
    };
    #[cfg(not(unix))]
    let signal = None;

    let out = Outcome {
        code: status.as_ref().and_then(|s| s.code()),
        signal,
        stdout,
        stderr,
        files,
        timed_out,
    };

    // Make the scratch tree removable even when a fixture was read-only.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let _ = fs::set_permissions(e.path(), fs::Permissions::from_mode(0o700));
            }
        }
    }
    let _ = fs::remove_dir_all(&dir);
    out
}

fn collect(root: &Path, dir: &Path, into: &mut BTreeMap<String, Vec<u8>>) {
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for e in rd.flatten() {
        let p = e.path();
        let rel = p
            .strip_prefix(root)
            .unwrap_or(&p)
            .to_string_lossy()
            .into_owned();
        match e.file_type() {
            Ok(t) if t.is_dir() => collect(root, &p, into),
            Ok(t) if t.is_file() => {
                into.insert(rel, fs::read(&p).unwrap_or_default());
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Pointer canonicalisation
// ---------------------------------------------------------------------------

/// The C program prints heap addresses with `printf("%p")`. Those differ from
/// one run of the *C program itself* to the next, because the heap is placed
/// randomly, so no byte-exact comparison of raw addresses is possible for any
/// implementation. What is observable and must agree is the *identity relation*
/// the addresses encode: which printed pointers are the same and which differ.
///
/// Every `0x<hex>` run is therefore replaced by a token numbered in order of
/// first appearance. Two outputs compare equal only if they agree byte for byte
/// everywhere else and print the same pattern of equal/distinct pointers in the
/// same positions.
pub fn canonicalize_pointers(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut seen: Vec<&[u8]> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'0' && i + 1 < bytes.len() && bytes[i + 1] == b'x' {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                let tok = &bytes[i..j];
                let idx = match seen.iter().position(|s| *s == tok) {
                    Some(k) => k,
                    None => {
                        seen.push(tok);
                        seen.len() - 1
                    }
                };
                out.extend_from_slice(format!("<ptr{idx}>").as_bytes());
                i = j;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Every raw address the program printed, in order of appearance.
pub fn raw_pointers(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut found = Vec::new();
    let mut i = 0usize;
    while i + 2 < bytes.len() {
        if bytes[i] == b'0' && bytes[i + 1] == b'x' {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                found.push(bytes[i..j].to_vec());
                i = j;
                continue;
            }
        }
        i += 1;
    }
    found
}

// ---------------------------------------------------------------------------
// The assertion used by every test
// ---------------------------------------------------------------------------

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn first_difference(a: &[u8], b: &[u8]) -> String {
    let al: Vec<&[u8]> = a.split(|&c| c == b'\n').collect();
    let bl: Vec<&[u8]> = b.split(|&c| c == b'\n').collect();
    for i in 0..al.len().max(bl.len()) {
        let x = al.get(i).copied().unwrap_or(b"<no such line>");
        let y = bl.get(i).copied().unwrap_or(b"<no such line>");
        if x != y {
            return format!(
                "first difference at line {}:\n     C: {:?}\n  Rust: {:?}",
                i + 1,
                show(x),
                show(y)
            );
        }
    }
    "streams differ only in trailing bytes".to_string()
}

pub fn assert_same(name: &str, input: &[u8], preps: &[Prep]) {
    let c = run(c_bin(), input, preps, &format!("c-{name}"));
    let r = run(&rust_bin(), input, preps, &format!("r-{name}"));

    assert!(
        !c.timed_out && !r.timed_out,
        "[{name}] a program did not finish within the time limit \
         (C timed out: {}, Rust timed out: {}) - use assert_both_hang for inputs \
         that make the C program spin",
        c.timed_out,
        r.timed_out
    );

    let cs = canonicalize_pointers(&c.stdout);
    let rs = canonicalize_pointers(&r.stdout);
    assert!(
        cs == rs,
        "[{name}] stdout differs\ninput: {:?}\n{}",
        show(input),
        first_difference(&cs, &rs)
    );

    let ce = canonicalize_pointers(&c.stderr);
    let re = canonicalize_pointers(&r.stderr);
    assert!(
        ce == re,
        "[{name}] stderr differs\ninput: {:?}\n     C: {:?}\n  Rust: {:?}",
        show(input),
        show(&c.stderr),
        show(&r.stderr)
    );

    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{name}] exit status differs (code, signal) for input {:?}",
        show(input)
    );

    assert_eq!(
        c.files.keys().collect::<Vec<_>>(),
        r.files.keys().collect::<Vec<_>>(),
        "[{name}] the two programs left different files behind"
    );
    for (k, cv) in &c.files {
        let rv = &r.files[k];
        assert!(
            cv == rv,
            "[{name}] file {k:?} differs\n     C: {:?}\n  Rust: {:?}",
            show(cv),
            show(rv)
        );
    }

    // Anything printed with %p must look like glibc's %p in both programs.
    for p in raw_pointers(&c.stdout).iter().chain(raw_pointers(&r.stdout).iter()) {
        assert!(
            p.len() > 2 && p[2..].iter().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "[{name}] pointer {:?} is not formatted the way glibc's %p is",
            show(p)
        );
    }
}

/// For inputs where the C program's `while (getchar() != '\n');` spins forever
/// at end of file: both programs must spin, and neither may have flushed
/// anything.
pub fn assert_both_hang(name: &str, input: &[u8]) {
    let c = run_with_limit(c_bin(), input, &[], &format!("c-{name}"), SPIN_LIMIT);
    let r = run_with_limit(&rust_bin(), input, &[], &format!("r-{name}"), SPIN_LIMIT);
    assert!(
        c.timed_out,
        "[{name}] expected the C program to spin at EOF, but it exited: {:?}",
        c.code
    );
    assert!(
        r.timed_out,
        "[{name}] the C program spins at EOF but the Rust program exited: {:?}",
        r.code
    );
    assert_eq!(
        canonicalize_pointers(&c.stdout),
        canonicalize_pointers(&r.stdout),
        "[{name}] the two programs flushed different output before being killed"
    );
}
