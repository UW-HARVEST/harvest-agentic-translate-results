//! Differential test harness.
//!
//! This module is compiled into every integration test binary, and not every
//! binary uses every helper, hence the blanket `dead_code` allowance.
//!
//! Both programs are driven exactly the way a shell drives them: spawn the
//! executable, pipe the input into its stdin, read stdout and stderr, wait for
//! the exit status.  Nothing is loaded as a library, and nothing about the
//! Rust program's internals is inspected.

#![allow(dead_code)]

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Once;

pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Normal exit code, or `None` when killed by a signal.
    pub code: Option<i32>,
    /// Terminating signal, or `None` on a normal exit.
    pub signal: Option<i32>,
}

impl Run {
    fn status_desc(&self) -> String {
        match (self.code, self.signal) {
            (Some(c), _) => format!("exited with code {c}"),
            (_, Some(s)) => format!("killed by signal {s}"),
            _ => "unknown status".to_string(),
        }
    }
}

/// The repository root, i.e. the directory holding `c_src/` and `translation/`.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// The Rust executable under test, as built by cargo for this integration test.
pub fn rust_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

/// The C executable, built via the supplied CMakeLists.txt if not present yet.
pub fn c_binary() -> PathBuf {
    static BUILD: Once = Once::new();
    let path = repo_root().join("c_src").join("build").join("driver");
    BUILD.call_once(|| {
        if !path.exists() {
            build_c();
        }
    });
    assert!(
        path.exists(),
        "the C reference binary is missing at {}.\n\
         Build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .",
        path.display()
    );
    path
}

fn build_c() {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    std::fs::create_dir_all(&build).expect("cannot create c_src/build");

    let configure = Command::new("cmake")
        .arg("..")
        .current_dir(&build)
        .status()
        .expect("failed to run `cmake ..` - is cmake installed?");
    assert!(configure.success(), "`cmake ..` failed in c_src/build");

    let compile = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .status()
        .expect("failed to run `cmake --build .`");
    assert!(compile.success(), "`cmake --build .` failed in c_src/build");
}

/// Run `bin` with `input` on stdin and collect everything observable.
pub fn run(bin: &Path, input: &[u8]) -> Run {
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Keep ctime()/localtime() deterministic between the two runs.
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", bin.display()));

    // Feed stdin from a helper thread: the child may fill the stdout pipe
    // before it has consumed all of stdin, which would deadlock a
    // write-then-read sequence on this thread.
    let mut sink = child.stdin.take().expect("stdin was piped");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || {
        // A broken pipe is expected whenever the program exits early (`exit`
        // command, SIGSEGV), so failures here are deliberately ignored.
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("cannot wait for {}: {e}", bin.display()));
    let _ = writer.join();

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

/// Like [`run`], but with stdout redirected to a real file instead of a pipe.
/// glibc sizes its stdio buffer from `st_blksize`, which can differ between a
/// pipe and a regular file, and the buffer contents are lost when the process
/// dies - so the two cases are worth checking separately.
pub fn run_stdout_to_file(bin: &Path, input: &[u8], out_path: &Path) -> (Run, Vec<u8>) {
    let file = std::fs::File::create(out_path)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", out_path.display()));

    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(file))
        .stderr(Stdio::piped())
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", bin.display()));

    let mut sink = child.stdin.take().expect("stdin was piped");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
        let _ = sink.flush();
    });

    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("cannot wait for {}: {e}", bin.display()));
    let _ = writer.join();

    let written = std::fs::read(out_path).unwrap_or_default();
    (
        Run {
            stdout: Vec::new(),
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
        },
        written,
    )
}

/// When `DIFFTEST_DUMP_INPUTS` names a directory, every input the suite feeds
/// to the programs is saved there.  That makes it possible to replay the whole
/// corpus through a coverage-instrumented build of the C reference and check
/// which of its branches the suite actually reaches.
pub fn dump_input(input: &[u8]) {
    let Ok(dir) = std::env::var("DIFFTEST_DUMP_INPUTS") else {
        return;
    };
    let dir = PathBuf::from(dir);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    // Content-addressed (FNV-1a) so repeated inputs collapse and parallel test
    // threads never collide.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in input {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    let _ = std::fs::write(dir.join(format!("{h:016x}.in")), input);
}

/// Render bytes readably, with non-printables escaped, for failure reports.
fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n\n"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

/// Index and content of the first differing byte.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            let lo = i.saturating_sub(48);
            return format!(
                "first difference at byte {i}: C has {:?} (0x{:02x}), Rust has {:?} (0x{:02x})\n\
                 C    ...{}\n\
                 Rust ...{}",
                a[i] as char,
                a[i],
                b[i] as char,
                b[i],
                show(&a[lo..(i + 16).min(a.len())]),
                show(&b[lo..(i + 16).min(b.len())]),
            );
        }
    }
    format!(
        "common prefix of {n} bytes is equal; lengths differ (C {} vs Rust {})",
        a.len(),
        b.len()
    )
}

/// Compare the two programs for `input`: stdout byte for byte, stderr byte for
/// byte, and the exit status (including death by signal).  `Ok(())` when they
/// agree, otherwise a report of the first thing that differed.
pub fn check(case: &str, input: &[u8]) -> Result<(), String> {
    dump_input(input);
    let c = run(&c_binary(), input);
    let r = run(&rust_binary(), input);

    if c.stdout != r.stdout {
        return Err(format!(
            "[{case}] stdout differs ({} C bytes vs {} Rust bytes)\n{}\n\
             ---- input ----\n{}\n---- C stdout ----\n{}\n---- Rust stdout ----\n{}",
            c.stdout.len(),
            r.stdout.len(),
            first_diff(&c.stdout, &r.stdout),
            show(input),
            show(&c.stdout),
            show(&r.stdout),
        ));
    }
    if c.stderr != r.stderr {
        return Err(format!(
            "[{case}] stderr differs\n{}\n---- input ----\n{}\n\
             ---- C stderr ----\n{}\n---- Rust stderr ----\n{}",
            first_diff(&c.stderr, &r.stderr),
            show(input),
            show(&c.stderr),
            show(&r.stderr),
        ));
    }
    if (c.code, c.signal) != (r.code, r.signal) {
        return Err(format!(
            "[{case}] exit status differs: C {} but Rust {}\n---- input ----\n{}",
            c.status_desc(),
            r.status_desc(),
            show(input),
        ));
    }
    Ok(())
}

/// Assert that both programs behave identically for `input`.
#[track_caller]
pub fn assert_same(case: &str, input: &[u8]) {
    if let Err(e) = check(case, input) {
        panic!("{e}");
    }
}

/// Assert a whole batch, reporting every failing case rather than only the
/// first one.
#[track_caller]
pub fn assert_all(cases: &[(&str, Vec<u8>)]) {
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(name, input)| check(name, input).err())
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} case(s) mismatched:\n\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n\n========================================\n\n")
    );
}

/// Build a script from lines, each terminated by `\n`.
pub fn script(lines: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for l in lines {
        v.extend_from_slice(l.as_bytes());
        v.push(b'\n');
    }
    v
}
