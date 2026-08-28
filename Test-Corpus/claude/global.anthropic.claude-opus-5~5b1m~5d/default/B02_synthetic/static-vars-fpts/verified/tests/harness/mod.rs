//! Shared plumbing for the differential tests.
//!
//! Both programs are driven exactly the way a shell would drive them: the
//! executable is spawned as a subprocess, stdin is redirected from a file, and
//! stdout / stderr / the exit status are captured and compared byte for byte.
//! Nothing here loads the Rust crate as a library.

#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// `translation/`
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory that holds both `c_src/` and `translation/`.
fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The Rust executable under test (`cargo` builds it before running the tests).
pub fn rust_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C executable that defines correct behaviour.
///
/// Uses the CMake build tree when it is present, otherwise compiles the three
/// translation units into `target/` with `cc`.  `c_src/` is only ever read.
pub fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let cmake_built = repo_root().join("c_src/build/driver");
        if cmake_built.is_file() {
            return cmake_built;
        }

        let out_dir = manifest_dir().join("target/c_reference");
        fs::create_dir_all(&out_dir).expect("create target/c_reference");
        let out = out_dir.join("driver");

        let src = repo_root().join("c_src/src");
        let include = repo_root().join("c_src/include");
        let status = Command::new("cc")
            .arg("-O2")
            .arg("-I")
            .arg(&include)
            .arg("-o")
            .arg(&out)
            .arg(src.join("main.c"))
            .arg(src.join("analyzer.c"))
            .arg(src.join("tokenizer.c"))
            .status()
            .expect("failed to invoke cc to build the C reference");
        assert!(status.success(), "compiling the C reference failed");
        out
    })
}

/// Working directory shared by both subprocesses, populated with the data files
/// that the `2. Load text from file` cases refer to.
pub fn fixture_dir() -> &'static Path {
    static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = manifest_dir().join("target/differential-fixture");
        let data = dir.join("data");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&data).expect("create fixture data dir");
        fs::create_dir_all(dir.join("stdin")).expect("create fixture stdin dir");

        let write = |name: &str, bytes: &[u8]| {
            let path = data.join(name);
            let mut f = fs::File::create(&path).expect("create fixture file");
            f.write_all(bytes).expect("write fixture file");
        };

        write("empty.txt", b"");
        write("small.txt", b"int x = 1; // hi\n");
        write("nul.txt", b"ab\0cdef\n");
        write("nul_first.txt", b"\0abc");
        write("noeol.txt", b"abc");
        write("crlf.txt", b"a\r\nb\r\n");
        write("only_newlines.txt", &vec![b'\n'; 100]);
        write("highbytes.txt", &(1u16..256).map(|b| b as u8).collect::<Vec<u8>>());
        write("binary.bin", &(0..256).map(|b| b as u8).cycle().take(1024).collect::<Vec<u8>>());
        write("size4096.txt", &vec![b'a'; 4096]);
        write("size8000.txt", &vec![b'b'; 8000]);
        // MAX_BUFFER_SIZE boundary: 8191 loads, 8192 is rejected by
        // tokenizer_load_text, 8193 is rejected by read_file.
        write("exact8191.txt", &vec![b'a'; 8191]);
        write("exact8192.txt", &vec![b'a'; 8192]);
        write("over8192.txt", &vec![b'a'; 8193]);
        // 8192 bytes but strlen() == 0, so load_text accepts it.
        let mut nul_first_8192 = vec![0u8; 1];
        nul_first_8192.extend(std::iter::repeat(b'a').take(8191));
        write("nul_first_8192.txt", &nul_first_8192);
        let words: Vec<u8> = (0..120)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();
        write("words.txt", &words);

        write("noperm.txt", b"secret\n");
        let noperm = data.join("noperm.txt");
        let mut perms = fs::metadata(&noperm).expect("stat noperm").permissions();
        perms.set_mode(0o000);
        let _ = fs::set_permissions(&noperm, perms);

        dir
    })
}

#[derive(PartialEq, Eq)]
pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

fn next_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Run `bin` with `input` on stdin, in the fixture directory, and capture
/// everything the process produced.
pub fn run(bin: &Path, input: &[u8], tag: &str) -> Run {
    let dir = fixture_dir();
    let stdin_path = dir.join("stdin").join(format!("{}-{}", tag, next_id()));
    {
        let mut f = fs::File::create(&stdin_path).expect("create stdin file");
        f.write_all(input).expect("write stdin file");
    }
    let stdin_file = fs::File::open(&stdin_path).expect("open stdin file");

    let child = Command::new(bin)
        .current_dir(dir)
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {}", bin.display(), e));

    let out = child.wait_with_output().expect("wait for child");
    let _ = fs::remove_file(&stdin_path);

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s
}

fn clip(bytes: &[u8]) -> String {
    const LIMIT: usize = 400;
    if bytes.len() <= LIMIT {
        show(bytes)
    } else {
        format!("{} ... <{} bytes total>", show(&bytes[..LIMIT]), bytes.len())
    }
}

/// First position at which the two buffers differ, with context.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let at = a
        .iter()
        .zip(b.iter())
        .position(|(x, y)| x != y)
        .unwrap_or_else(|| a.len().min(b.len()));
    let lo = at.saturating_sub(60);
    format!(
        "first difference at byte {}\n     C: {}\n  rust: {}",
        at,
        clip(&a[lo..(lo + 160).min(a.len())]),
        clip(&b[lo..(lo + 160).min(b.len())]),
    )
}

/// Assert that the C and the Rust program agree on stdout, stderr and status.
pub fn assert_same(name: &str, input: &[u8]) {
    let c = run(c_bin(), input, name);
    let r = run(&rust_bin(), input, name);

    let mut problems = Vec::new();
    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout differs ({} C bytes vs {} rust bytes)\n  {}",
            c.stdout.len(),
            r.stdout.len(),
            first_diff(&c.stdout, &r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        problems.push(format!(
            "stderr differs\n     C: {}\n  rust: {}",
            clip(&c.stderr),
            clip(&r.stderr)
        ));
    }
    if c.code != r.code || c.signal != r.signal {
        problems.push(format!(
            "exit status differs: C code={:?} signal={:?}, rust code={:?} signal={:?}",
            c.code, c.signal, r.code, r.signal
        ));
    }

    assert!(
        problems.is_empty(),
        "case `{}` diverged\ninput: {}\n{}",
        name,
        clip(input),
        problems.join("\n")
    );
}

/// Run a whole table of cases and report every failure at once.
pub fn assert_all(cases: &[(&str, Vec<u8>)]) {
    let mut failures = Vec::new();
    for (name, input) in cases {
        let c = run(c_bin(), input, name);
        let r = run(&rust_bin(), input, name);
        if c != r {
            let mut why = Vec::new();
            if c.stdout != r.stdout {
                why.push(format!(
                    "stdout ({} vs {} bytes): {}",
                    c.stdout.len(),
                    r.stdout.len(),
                    first_diff(&c.stdout, &r.stdout)
                ));
            }
            if c.stderr != r.stderr {
                why.push(format!(
                    "stderr: C={} rust={}",
                    clip(&c.stderr),
                    clip(&r.stderr)
                ));
            }
            if c.code != r.code || c.signal != r.signal {
                why.push(format!(
                    "status: C code={:?} sig={:?} rust code={:?} sig={:?}",
                    c.code, c.signal, r.code, r.signal
                ));
            }
            failures.push(format!(
                "--- {}\ninput: {}\n{}",
                name,
                clip(input),
                why.join("\n")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases diverged:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// small helpers for building inputs
// ---------------------------------------------------------------------------

pub fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

pub fn rep(pattern: &[u8], n: usize) -> Vec<u8> {
    pattern.repeat(n)
}

/// `1\n<text>\n\n7\n`: analyse `text`, then exit.
pub fn analyze(text: &[u8]) -> Vec<u8> {
    cat(&[b"1\n", text, b"\n\n7\n"])
}

/// `6\n<text>\n\n7\n`: tokenize `text` interactively, then exit.
pub fn interactive(text: &[u8]) -> Vec<u8> {
    cat(&[b"6\n", text, b"\n\n7\n"])
}

/// Deterministic xorshift64* generator, so the "fuzz" test is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}
