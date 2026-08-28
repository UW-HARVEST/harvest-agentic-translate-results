//! Differential test harness.
//!
//! Both programs are driven exactly the way a shell would drive them: the
//! binary is spawned as a subprocess with a fresh working directory, the test
//! input is written to its stdin, and stdout / stderr / the exit status / the
//! files it left behind are captured and compared.  Nothing is ever called as a
//! library.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// How long a normally-terminating run is given before it is considered hung.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(10_000);

/// How long the deliberately-hanging runs are observed for.
pub const HANG_TIMEOUT: Duration = Duration::from_millis(1_500);

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Exited(i32),
    Signal(i32),
    /// Still running when the timeout expired (the C program has infinite
    /// `while (getchar() != '\n');` loops that spin forever at end of file).
    TimedOut,
}

pub struct RunResult {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// A file placed in the working directory before the program is started.
#[derive(Clone)]
pub struct Fixture {
    pub name: &'static str,
    pub content: Vec<u8>,
    pub mode: Option<u32>,
}

pub struct Case {
    pub name: String,
    pub stdin: Vec<u8>,
    pub files: Vec<Fixture>,
    pub timeout: Duration,
    /// Set for inputs that make the C program spin forever; the comparison is
    /// then done on the bytes that had reached fd 1 when the process was killed.
    pub expect_hang: bool,
}

impl Case {
    pub fn file(mut self, name: &'static str, content: impl Into<Vec<u8>>) -> Case {
        self.files.push(Fixture {
            name,
            content: content.into(),
            mode: None,
        });
        self
    }

    pub fn file_mode(mut self, name: &'static str, content: impl Into<Vec<u8>>, mode: u32) -> Case {
        self.files.push(Fixture {
            name,
            content: content.into(),
            mode: Some(mode),
        });
        self
    }

    pub fn hangs(mut self) -> Case {
        self.expect_hang = true;
        self.timeout = HANG_TIMEOUT;
        self
    }
}

/// Build a case.  `stdin` is the raw byte stream fed to the program.
pub fn case(name: &str, stdin: impl Into<Vec<u8>>) -> Case {
    Case {
        name: name.to_string(),
        stdin: stdin.into(),
        files: Vec::new(),
        timeout: DEFAULT_TIMEOUT,
        expect_hang: false,
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

/// Path of the C executable, building it with CMake on first use if needed.
pub fn c_binary() -> &'static PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let src = repo_root().join("c_src");
        let build = src.join("build");
        let bin = build.join("driver");
        if !bin.exists() {
            fs::create_dir_all(&build).expect("create c_src/build");
            let st = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .status()
                .expect("run `cmake ..` in c_src/build");
            assert!(st.success(), "cmake configure of the C program failed");
            let st = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .status()
                .expect("run `cmake --build .` in c_src/build");
            assert!(st.success(), "cmake build of the C program failed");
        }
        assert!(bin.exists(), "C binary missing at {}", bin.display());
        bin
    })
}

/// Path of the Rust executable produced by this crate.
pub fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn scratch_dir(group: &str, case: &str, which: &str) -> PathBuf {
    let sanitize = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect()
    };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("difftest")
        .join(sanitize(group))
        .join(sanitize(case))
        .join(which);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch directory");
    dir
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

fn status_of(st: std::process::ExitStatus) -> Status {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = st.signal() {
            return Status::Signal(sig);
        }
    }
    Status::Exited(st.code().unwrap_or(-1))
}

/// Run one program on one input in its own working directory.
pub fn run_one(bin: &Path, workdir: &Path, c: &Case) -> RunResult {
    for f in &c.files {
        let p = workdir.join(f.name);
        fs::write(&p, &f.content).expect("write fixture file");
        if let Some(m) = f.mode {
            set_mode(&p, m);
        }
    }

    let out_path = workdir.with_extension("stdout");
    let err_path = workdir.with_extension("stderr");
    let out_file = fs::File::create(&out_path).expect("create stdout capture file");
    let err_file = fs::File::create(&err_path).expect("create stderr capture file");

    let mut child = Command::new(bin)
        .current_dir(workdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::from(err_file))
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", bin.display()));

    {
        let mut sin = child.stdin.take().expect("stdin pipe");
        // Errors are ignored: a program may exit before reading everything.
        let _ = sin.write_all(&c.stdin);
        let _ = sin.flush();
    } // dropping the pipe gives the child end of file

    let deadline = Instant::now() + c.timeout;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(st) => break status_of(st),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Status::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    };

    let stdout = fs::read(&out_path).expect("read captured stdout");
    let stderr = fs::read(&err_path).expect("read captured stderr");

    // Collect whatever the program left in its working directory.
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(workdir).expect("read workdir") {
        let entry = entry.expect("dir entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            files.insert(name, b"<directory>".to_vec());
            continue;
        }
        // A fixture may have been made unreadable on purpose; make it readable
        // again so the comparison can look at its contents.
        set_mode(&path, 0o644);
        let content = fs::read(&path).unwrap_or_else(|_| b"<unreadable>".to_vec());
        files.insert(name, content);
    }

    RunResult {
        status,
        stdout,
        stderr,
        files,
    }
}

fn is_hex(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
}

/// Replace every `%p` address with a token derived from the order in which the
/// address first appears.
///
/// The C program prints raw `malloc` addresses, which no independent process can
/// reproduce byte for byte.  Mapping them to `<ptrN>` keeps everything the
/// output actually encodes: how many distinct objects there are, and exactly
/// which printed addresses are equal to which others.
pub fn normalize_ptrs(data: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len());
    let mut seen: Vec<Vec<u8>> = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        if data[i] == b'0' && i + 2 < data.len() && data[i + 1] == b'x' && is_hex(data[i + 2]) {
            let mut j = i + 2;
            while j < data.len() && is_hex(data[j]) {
                j += 1;
            }
            let tok = data[i..j].to_vec();
            let idx = match seen.iter().position(|t| *t == tok) {
                Some(p) => p,
                None => {
                    seen.push(tok);
                    seen.len() - 1
                }
            };
            out.extend_from_slice(format!("<ptr{idx}>").as_bytes());
            i = j;
        } else {
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

fn show(data: &[u8]) -> String {
    String::from_utf8_lossy(data).into_owned()
}

fn first_diff(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    (0..n).find(|&i| a[i] != b[i]).unwrap_or(n)
}

fn context(data: &[u8], at: usize) -> String {
    let start = at.saturating_sub(80);
    let end = (at + 120).min(data.len());
    format!(
        "…{}‹HERE›{}…  (total {} bytes)",
        show(&data[start..at]),
        show(&data[at..end]),
        data.len()
    )
}

fn compare_stream(kind: &str, case: &str, c: &[u8], r: &[u8], normalize: bool) {
    let (cv, rv) = if normalize {
        (normalize_ptrs(c), normalize_ptrs(r))
    } else {
        (c.to_vec(), r.to_vec())
    };
    if cv != rv {
        let at = first_diff(&cv, &rv);
        panic!(
            "[{case}] {kind} differs at byte {at}\n  C   : {}\n  RUST: {}",
            context(&cv, at),
            context(&rv, at)
        );
    }
}

/// Run one case through both programs and require identical observable results.
pub fn check(group: &str, c: &Case) {
    let cdir = scratch_dir(group, &c.name, "c");
    let rdir = scratch_dir(group, &c.name, "rust");

    let cres = run_one(c_binary(), &cdir, c);
    let rres = run_one(&rust_binary(), &rdir, c);

    assert_eq!(
        cres.status, rres.status,
        "[{}] exit status differs (C={:?}, Rust={:?})",
        c.name, cres.status, rres.status
    );

    if c.expect_hang {
        assert_eq!(
            cres.status,
            Status::TimedOut,
            "[{}] was expected to hang but the C program terminated",
            c.name
        );
        // These cases are chosen so that no `%p` address is printed, which makes
        // the surviving prefix comparable byte for byte.
        compare_stream("stdout", &c.name, &cres.stdout, &rres.stdout, false);
        compare_stream("stderr", &c.name, &cres.stderr, &rres.stderr, false);
    } else {
        compare_stream("stdout", &c.name, &cres.stdout, &rres.stdout, true);
        compare_stream("stderr", &c.name, &cres.stderr, &rres.stderr, true);
    }

    let cnames: Vec<&String> = cres.files.keys().collect();
    let rnames: Vec<&String> = rres.files.keys().collect();
    assert_eq!(
        cnames, rnames,
        "[{}] the set of files in the working directory differs",
        c.name
    );
    for (name, cdata) in &cres.files {
        let rdata = &rres.files[name];
        if cdata != rdata {
            let at = first_diff(cdata, rdata);
            panic!(
                "[{}] file '{name}' differs at byte {at}\n  C   : {}\n  RUST: {}",
                c.name,
                context(cdata, at),
                context(rdata, at)
            );
        }
    }
}

pub fn check_all(group: &str, cases: Vec<Case>) {
    assert!(!cases.is_empty(), "group {group} has no cases");
    for c in &cases {
        check(group, c);
    }
}
