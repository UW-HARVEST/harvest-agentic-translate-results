//! Shared plumbing for the differential tests.
//!
//! Both the C reference executable and the Rust executable are driven as
//! subprocesses, exactly the way a shell would drive them: bytes on stdin,
//! bytes out of stdout/stderr, plus an exit status.  Nothing is linked in as a
//! library.

// Not every test binary uses every helper.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Result of running one of the two executables.
#[derive(PartialEq, Eq)]
pub struct Run {
    pub status: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .expect("translation/ must have a parent directory")
            .to_path_buf()
    })
}

/// Path of the Rust executable under test (built by cargo for us).
pub fn rust_binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path of the C reference executable.  If it has not been built yet, build it
/// with CMake into `target/c_build` so that nothing inside `c_src/` is touched.
pub fn c_binary() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let root = workspace_root();
        let prebuilt = root.join("c_src/build/driver");
        if prebuilt.is_file() {
            return prebuilt;
        }

        let build_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/c_build");
        let candidate = build_dir.join("driver");
        if candidate.is_file() {
            return candidate;
        }

        std::fs::create_dir_all(&build_dir).expect("cannot create the CMake build directory");
        let configure = Command::new("cmake")
            .arg("-S")
            .arg(root.join("c_src"))
            .arg("-B")
            .arg(&build_dir)
            .output()
            .expect("cmake is required to build the C reference program");
        assert!(
            configure.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&configure.stdout),
            String::from_utf8_lossy(&configure.stderr)
        );
        let build = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .output()
            .expect("cmake --build failed to start");
        assert!(
            build.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        assert!(candidate.is_file(), "the C reference program was not built");
        candidate
    })
}

/// Run one executable with `input` on stdin.
pub fn run(binary: &Path, input: &[u8]) -> Run {
    use std::os::unix::process::ExitStatusExt;

    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {}: {e}", binary.display()));

    {
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let data = input.to_vec();
        // A dedicated thread avoids dead-locking on programs that produce more
        // output than a pipe buffer holds while we are still writing.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });
    }

    let out = child.wait_with_output().expect("the child failed to run");
    Run {
        status: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn show(data: &[u8]) -> String {
    match std::str::from_utf8(data) {
        Ok(s) => s.to_string(),
        Err(_) => format!("{data:?}"),
    }
}

fn first_difference(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    let idx = (0..n).find(|&i| a[i] != b[i]).unwrap_or(n);
    let from = idx.saturating_sub(60);
    let to_a = (idx + 60).min(a.len());
    let to_b = (idx + 60).min(b.len());
    format!(
        "first difference at byte {idx}\n  C  : {}\n  Rust: {}",
        show(&a[from..to_a]),
        show(&b[from..to_b])
    )
}

/// Assert that both programs behave identically for `input`: byte-identical
/// stdout, byte-identical stderr and the same exit status.
#[track_caller]
pub fn assert_same(name: &str, input: &[u8]) {
    let c = run(c_binary(), input);
    let r = run(rust_binary(), input);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{name}] stdout differs ({} vs {} bytes)\n{}",
        c.stdout.len(),
        r.stdout.len(),
        first_difference(&c.stdout, &r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{name}] stderr differs\n  C  : {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "[{name}] exit status differs: C {:?}/{:?} vs Rust {:?}/{:?}",
        c.status,
        c.signal,
        r.status,
        r.signal
    );
}

/// A directory holding the fixture files used by the `2` (load from file) menu
/// entry.  Created once per test binary, kept under `target/` so that neither
/// `c_src/` nor the repository root is polluted.
pub fn data_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/difftest-data");
        std::fs::create_dir_all(&dir).expect("cannot create the fixture directory");

        write(&dir, "small.c", b"int main(void) { return 0; }\n");
        write(&dir, "empty.txt", b"");
        write(&dir, "one_word.txt", b"hello");
        write(&dir, "newlines.txt", b"\n\n\n\n\n");
        // MAX_BUFFER_SIZE == 8192: 8191 loads, 8192 is rejected by
        // tokenizer_load_text, 8193 is rejected by read_file.
        write(&dir, "size8191.txt", &vec![b'a'; 8191]);
        write(&dir, "size8192.txt", &vec![b'a'; 8192]);
        write(&dir, "size8193.txt", &vec![b'a'; 8193]);
        write(&dir, "size4096.txt", &vec![b'b'; 4096]);
        write(&dir, "nul_middle.bin", b"abc\0def\n");
        write(&dir, "nul_first.bin", b"\0abcdef\n");
        write(&dir, "high_bytes.bin", &(128u16..256).map(|b| b as u8).collect::<Vec<u8>>());
        write(
            &dir,
            "code.c",
            b"/* header */\nint f(int x) {\n  if (x >= 10) return x++;\n  // done\n  char *s = \"hi\\n\";\n  return 0;\n}\n",
        );
        std::fs::create_dir_all(dir.join("a_directory")).expect("cannot create the sub-directory");

        // A file that cannot be opened (unless the tests run as root, in which
        // case both programs still agree - they just take the success path).
        // A previous run may have left it mode 000, so make it writable first.
        let noperm = dir.join("no_permission.txt");
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&noperm, std::fs::Permissions::from_mode(0o600));
            write(&dir, "no_permission.txt", b"secret\n");
            let _ = std::fs::set_permissions(&noperm, std::fs::Permissions::from_mode(0o000));
        }

        dir
    })
}

fn write(dir: &Path, name: &str, data: &[u8]) {
    std::fs::write(dir.join(name), data).unwrap_or_else(|e| panic!("cannot write {name}: {e}"));
}

/// Absolute path of a fixture, as the bytes the program will receive.
pub fn data_path(name: &str) -> String {
    data_dir().join(name).to_str().expect("utf-8 path").to_string()
}
