//! Differential test harness.
//!
//! Every case runs the ORIGINAL C executable and the translated Rust
//! executable as subprocesses, feeds them identical stdin, and compares
//! stdout, stderr, the exit status and any files the program created in its
//! working directory.
//!
//! The Rust code is never linked as a library: the binary is driven exactly the
//! way a shell would drive it.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Generous wall clock cap so a runaway process can never wedge the suite.
/// `timeout(1)` reports status 124 when it fires, which is itself compared
/// between the two programs (the C code has code paths that spin forever at
/// EOF, and the translation must spin too).
const DEFAULT_TIMEOUT: &str = "20";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation/ should have a parent directory")
        .to_path_buf()
}

// ---------------------------------------------------------------------------
// Building / locating the two executables
// ---------------------------------------------------------------------------

/// Configures and builds `c_src` with CMake, out of tree, so nothing inside
/// `c_src/` is touched. Returns the path to the resulting `driver` executable.
pub fn c_binary() -> &'static PathBuf {
    static C: OnceLock<PathBuf> = OnceLock::new();
    C.get_or_init(|| {
        let src = repo_root().join("c_src");
        assert!(
            src.join("CMakeLists.txt").is_file(),
            "expected {}/CMakeLists.txt",
            src.display()
        );
        let build = manifest_dir().join("target").join("c_build");
        std::fs::create_dir_all(&build).expect("create c build dir");

        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .output()
            .expect("`cmake` must be installed to run the differential tests");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("run cmake --build");
        assert!(
            bld.status.success(),
            "cmake --build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        let bin = build.join("driver");
        assert!(bin.is_file(), "C executable missing at {}", bin.display());
        bin
    })
}

/// Every Rust executable available for comparison.
///
/// `CARGO_BIN_EXE_driver` is the binary cargo built for this test run (debug
/// when invoked as plain `cargo test`). If a `--release` build is also present
/// it is compared as well, so the artifact a grader runs is covered too. No
/// nested `cargo build` is attempted: that would deadlock on the target lock.
pub fn rust_binaries() -> &'static Vec<(String, PathBuf)> {
    static R: OnceLock<Vec<(String, PathBuf)>> = OnceLock::new();
    R.get_or_init(|| {
        let mut v = Vec::new();
        let primary = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
        assert!(
            primary.is_file(),
            "Rust executable missing at {}",
            primary.display()
        );
        let primary_label = primary
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "cargo".to_string());
        v.push((primary_label.clone(), primary.clone()));

        for profile in ["release", "debug"] {
            if profile == primary_label {
                continue;
            }
            let alt = manifest_dir().join("target").join(profile).join("driver");
            if alt.is_file() {
                v.push((profile.to_string(), alt));
            }
        }
        v
    })
}

// ---------------------------------------------------------------------------
// `%p` output
// ---------------------------------------------------------------------------

/// How `%p` output is compared. See `translation/ERRORS.md`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PtrMode {
    /// ASLR could be disabled for the C child, so raw addresses match exactly.
    Exact,
    /// ASLR could not be disabled, so hex addresses are canonicalised on both
    /// sides before comparison.
    Normalized,
}

struct CLaunch {
    prefix: Vec<String>,
    mode: PtrMode,
}

/// stdin that makes the C program print two `%p` values and exit cleanly.
const PTR_PROBE: &[u8] = b"9\n0\n1\n12\n";

fn c_launch() -> &'static CLaunch {
    static L: OnceLock<CLaunch> = OnceLock::new();
    L.get_or_init(|| {
        let candidate = vec!["setarch".to_string(), "--addr-no-randomize".to_string()];

        let works = Command::new(&candidate[0])
            .arg(&candidate[1])
            .arg("true")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if works {
            // Confirm it actually pins the heap: two runs must agree.
            let a = raw_run(&candidate, c_binary(), PTR_PROBE);
            let b = raw_run(&candidate, c_binary(), PTR_PROBE);
            if a == b && a.iter().any(|w| w.windows(6).any(|s| s == b"(ptr: ")) {
                return CLaunch {
                    prefix: candidate,
                    mode: PtrMode::Exact,
                };
            }
        }
        CLaunch {
            prefix: Vec::new(),
            mode: PtrMode::Normalized,
        }
    })
}

pub fn ptr_mode() -> PtrMode {
    c_launch().mode
}

/// Minimal runner used only by the `%p` probe above.
fn raw_run(prefix: &[String], bin: &Path, stdin: &[u8]) -> Vec<Vec<u8>> {
    let mut cmd = build_command(prefix, bin, DEFAULT_TIMEOUT);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn probe");
    let mut sink = child.stdin.take().unwrap();
    let data = stdin.to_vec();
    let t = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
    });
    let out = child.wait_with_output().expect("probe wait");
    let _ = t.join();
    vec![out.stdout, out.stderr]
}

fn build_command(prefix: &[String], bin: &Path, timeout: &str) -> Command {
    // `timeout` guarantees the suite terminates; its own exit status (124 on
    // expiry) participates in the comparison.
    let mut cmd = Command::new("timeout");
    cmd.arg(timeout);
    for p in prefix {
        cmd.arg(p);
    }
    cmd.arg(bin);
    cmd
}

/// Replace every `0x`-prefixed hex run with a fixed token.
fn normalize_ptrs(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'0' && i + 1 < bytes.len() && bytes[i + 1] == b'x' {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > i + 2 {
                out.extend_from_slice(b"0xPTR");
                i = j;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn for_compare(bytes: &[u8]) -> Vec<u8> {
    match ptr_mode() {
        PtrMode::Exact => bytes.to_vec(),
        PtrMode::Normalized => normalize_ptrs(bytes),
    }
}

// ---------------------------------------------------------------------------
// Running one program
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StdinMode {
    /// `prog < file` -- a seekable regular file.
    File,
    /// `cat file | prog` -- a pipe, which can deliver short reads.
    Pipe,
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: String,
    files: BTreeMap<String, FileEntry>,
}

#[derive(PartialEq, Eq, Debug)]
enum FileEntry {
    Dir,
    File(Vec<u8>),
    Other,
}

fn describe_status(st: std::process::ExitStatus) -> String {
    match (st.code(), st.signal()) {
        (Some(124), _) => "timed-out(124)".to_string(),
        (Some(c), _) => format!("exit={c}"),
        (None, Some(s)) => format!("signal={s}"),
        (None, None) => "unknown".to_string(),
    }
}

fn snapshot(dir: &Path) -> BTreeMap<String, FileEntry> {
    let mut map = BTreeMap::new();
    fn walk(base: &Path, dir: &Path, map: &mut BTreeMap<String, FileEntry>) {
        let rd = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .to_string();
            let md = match std::fs::symlink_metadata(&p) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_dir() {
                map.insert(rel, FileEntry::Dir);
                walk(base, &p, map);
            } else if md.is_file() {
                let mut buf = Vec::new();
                match std::fs::File::open(&p).and_then(|mut f| f.read_to_end(&mut buf)) {
                    Ok(_) => {
                        map.insert(rel, FileEntry::File(buf));
                    }
                    Err(_) => {
                        map.insert(rel, FileEntry::Other);
                    }
                }
            } else {
                map.insert(rel, FileEntry::Other);
            }
        }
    }
    walk(dir, dir, &mut map);
    map
}

fn run_one(
    prefix: &[String],
    bin: &Path,
    cwd: &Path,
    stdin_file: &Path,
    stdin_mode: StdinMode,
    timeout: &str,
    merged: bool,
) -> Outcome {
    let mut cmd = build_command(prefix, bin, timeout);
    cmd.current_dir(cwd);

    match stdin_mode {
        StdinMode::File => {
            let f = std::fs::File::open(stdin_file).expect("open stdin file");
            cmd.stdin(Stdio::from(f));
        }
        StdinMode::Pipe => {
            cmd.stdin(Stdio::piped());
        }
    }

    // Capture files live beside the working directory so they never show up in
    // the directory snapshot.
    let out_path = cwd.with_extension("stdout");
    let err_path = cwd.with_extension("stderr");

    if merged {
        let f = std::fs::File::create(&out_path).expect("create merged capture");
        let g = f.try_clone().expect("clone merged capture");
        cmd.stdout(Stdio::from(f));
        cmd.stderr(Stdio::from(g));
    } else {
        cmd.stdout(Stdio::from(
            std::fs::File::create(&out_path).expect("create stdout capture"),
        ));
        cmd.stderr(Stdio::from(
            std::fs::File::create(&err_path).expect("create stderr capture"),
        ));
    }

    let mut child = cmd.spawn().unwrap_or_else(|e| {
        panic!(
            "failed to spawn {} ({e}); is `timeout` from coreutils available?",
            bin.display()
        )
    });

    let writer = if stdin_mode == StdinMode::Pipe {
        let mut sink = child.stdin.take().expect("piped stdin");
        let data = std::fs::read(stdin_file).expect("read stdin file");
        Some(std::thread::spawn(move || {
            // A closed stdin (the child exited early) is normal, not an error.
            let _ = sink.write_all(&data);
        }))
    } else {
        None
    };

    let status = child.wait().expect("wait for child");
    if let Some(w) = writer {
        let _ = w.join();
    }

    let stdout = std::fs::read(&out_path).unwrap_or_default();
    let stderr = if merged {
        Vec::new()
    } else {
        std::fs::read(&err_path).unwrap_or_default()
    };

    Outcome {
        stdout,
        stderr,
        status: describe_status(status),
        files: snapshot(cwd),
    }
}

// ---------------------------------------------------------------------------
// Case description
// ---------------------------------------------------------------------------

pub struct Case {
    name: String,
    stdin: Vec<u8>,
    seed_files: Vec<(String, Vec<u8>)>,
    seed_dirs: Vec<String>,
    timeout: String,
    stdin_mode: StdinMode,
    merged: bool,
}

impl Case {
    pub fn new(name: &str, stdin: impl AsRef<[u8]>) -> Case {
        Case {
            name: name.to_string(),
            stdin: stdin.as_ref().to_vec(),
            seed_files: Vec::new(),
            seed_dirs: Vec::new(),
            timeout: DEFAULT_TIMEOUT.to_string(),
            stdin_mode: StdinMode::File,
            merged: false,
        }
    }

    /// Pre-create a file in the working directory before the program runs.
    pub fn seed(mut self, name: &str, content: impl AsRef<[u8]>) -> Case {
        self.seed_files
            .push((name.to_string(), content.as_ref().to_vec()));
        self
    }

    /// Pre-create a directory in the working directory.
    pub fn seed_dir(mut self, name: &str) -> Case {
        self.seed_dirs.push(name.to_string());
        self
    }

    pub fn timeout(mut self, secs: u32) -> Case {
        self.timeout = secs.to_string();
        self
    }

    /// Deliver stdin through a pipe rather than a regular file.
    pub fn piped_stdin(mut self) -> Case {
        self.stdin_mode = StdinMode::Pipe;
        self
    }

    /// Send stdout and stderr to the same descriptor (`prog > f 2>&1`), which
    /// makes the comparison sensitive to C's buffering rules: stdout is block
    /// buffered on a pipe or file, stderr is unbuffered.
    pub fn merged_streams(mut self) -> Case {
        self.merged = true;
        self
    }

    fn prepare(&self, dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).expect("create case dir");
        for d in &self.seed_dirs {
            std::fs::create_dir_all(dir.join(d)).expect("create seed dir");
        }
        for (n, c) in &self.seed_files {
            let p = dir.join(n);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&p, c).expect("write seed file");
        }
    }

    /// Run both programs and assert stdout, stderr, exit status and produced
    /// files are identical.
    pub fn assert_matches(self) {
        let root = manifest_dir().join("target").join("difftest");
        let safe: String = self
            .name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();

        let stdin_path = root.join(format!("{safe}.stdin"));
        std::fs::create_dir_all(&root).expect("create difftest root");
        std::fs::write(&stdin_path, &self.stdin).expect("write stdin");

        let c_dir = root.join(format!("{safe}.c"));
        self.prepare(&c_dir);
        let c_out = run_one(
            &c_launch().prefix,
            c_binary(),
            &c_dir,
            &stdin_path,
            self.stdin_mode,
            &self.timeout,
            self.merged,
        );

        for (label, rust_bin) in rust_binaries() {
            let r_dir = root.join(format!("{safe}.rust-{label}"));
            self.prepare(&r_dir);
            let r_out = run_one(
                &[],
                rust_bin,
                &r_dir,
                &stdin_path,
                self.stdin_mode,
                &self.timeout,
                self.merged,
            );

            let ctx = format!("case `{}` [rust: {label}]", self.name);

            compare_stream(&ctx, "stdout", &c_out.stdout, &r_out.stdout);
            compare_stream(&ctx, "stderr", &c_out.stderr, &r_out.stderr);
            assert_eq!(
                c_out.status, r_out.status,
                "{ctx}: exit status differs (C={} Rust={})",
                c_out.status, r_out.status
            );
            compare_files(&ctx, &c_out.files, &r_out.files);
        }
    }
}

fn compare_stream(ctx: &str, which: &str, c: &[u8], r: &[u8]) {
    let cc = for_compare(c);
    let rr = for_compare(r);
    if cc == rr {
        return;
    }
    let at = cc
        .iter()
        .zip(rr.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(cc.len().min(rr.len()));
    panic!(
        "{ctx}: {which} differs at byte {at} (C {} bytes, Rust {} bytes)\n\
         --- C ---\n{}\n--- Rust ---\n{}\n--- C (around {at}) ---\n{:?}\n--- Rust (around {at}) ---\n{:?}",
        c.len(),
        r.len(),
        String::from_utf8_lossy(&cc),
        String::from_utf8_lossy(&rr),
        String::from_utf8_lossy(&cc[at.saturating_sub(40)..(at + 40).min(cc.len())]),
        String::from_utf8_lossy(&rr[at.saturating_sub(40)..(at + 40).min(rr.len())]),
    );
}

fn compare_files(
    ctx: &str,
    c: &BTreeMap<String, FileEntry>,
    r: &BTreeMap<String, FileEntry>,
) {
    let ck: Vec<&String> = c.keys().collect();
    let rk: Vec<&String> = r.keys().collect();
    assert_eq!(
        ck, rk,
        "{ctx}: the set of files in the working directory differs"
    );
    for k in ck {
        let cv = &c[k];
        let rv = &r[k];
        match (cv, rv) {
            (FileEntry::File(a), FileEntry::File(b)) => assert!(
                a == b,
                "{ctx}: contents of `{k}` differ\n--- C ---\n{}\n--- Rust ---\n{}",
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b)
            ),
            _ => assert!(cv == rv, "{ctx}: `{k}` differs ({cv:?} vs {rv:?})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared fixtures for the scene_load tests
// ---------------------------------------------------------------------------

/// Attach the standard set of saved-scene fixtures used by the option 8 tests.
pub fn with_scene_files(mut case: Case) -> Case {
    let mut overfull = b"Full\n55\n".to_vec();
    for _ in 0..55 {
        overfull.extend_from_slice(b"0\n");
    }
    let mut fifty = b"Fifty\n50\n".to_vec();
    for i in 0..50u32 {
        fifty.extend_from_slice(format!("{}\n", i % 10).as_bytes());
    }
    let mut fifty_rev = b"FiftyRev\n50\n".to_vec();
    for i in (0..50u32).rev() {
        fifty_rev.extend_from_slice(format!("{}\n", i % 10).as_bytes());
    }

    let files: Vec<(&str, Vec<u8>)> = vec![
        ("good.txt", b"Loaded\n3\n0\n7\n9\n".to_vec()),
        ("empty.txt", Vec::new()),
        ("nameonly.txt", b"JustAName\n".to_vec()),
        ("name_nonl.txt", b"NoNewline".to_vec()),
        ("badcount.txt", b"X\nnotanumber\n".to_vec()),
        ("count_neg.txt", b"Neg\n-4\n".to_vec()),
        ("count_zero.txt", b"Zero\n0\n".to_vec()),
        ("short.txt", b"Short\n5\n0\n1\n".to_vec()),
        ("badtype.txt", b"BadType\n3\n0\n99\n1\n".to_vec()),
        ("negtype.txt", b"NegType\n3\n0\n-1\n1\n".to_vec()),
        ("overfull.txt", overfull),
        (
            "longname.txt",
            b"0123456789012345678901234567890123456789012345678901234567890123456789XYZ\n1\n0\n"
                .to_vec(),
        ),
        (
            "name63plus.txt",
            b"012345678901234567890123456789012345678901234567890123456789012345\n1\n0\n".to_vec(),
        ),
        ("ws.txt", b"WS\n   2  \n  0   \n   3\n".to_vec()),
        ("inline.txt", b"Inline\n2 0 1\n".to_vec()),
        ("trailing.txt", b"Trail\n2\n0\n1\nEXTRAJUNK\n".to_vec()),
        ("count_huge.txt", b"Huge\n2000000000\n0\n".to_vec()),
        ("type_ov.txt", b"TypeOv\n1\n4294967296\n".to_vec()),
        ("count_ov.txt", b"CountOv\n4294967296\n0\n".to_vec()),
        ("crlf.txt", b"CrLf\r\n2\r\n0\r\n1\r\n".to_vec()),
        ("plus.txt", b"Plus\n+2\n+0\n+9\n".to_vec()),
        ("nul.txt", b"Na\x00me\n1\n0\n".to_vec()),
        ("tabs.txt", b"Tabs\n\t2\t\n\t0\t\n\t1\t\n".to_vec()),
        ("high.txt", b"\xff\xfe\xfd\n1\n0\n".to_vec()),
        ("nonl_end.txt", b"NoTrail\n1\n0".to_vec()),
        ("zeros.txt", b"Zeros\n0000000002\n0000000000\n00000009\n".to_vec()),
        ("emptyname.txt", b"\n2\n0\n1\n".to_vec()),
        ("fifty.txt", fifty),
        ("fiftyrev.txt", fifty_rev),
    ];
    for (n, c) in files {
        case = case.seed(n, c);
    }
    case.seed_dir("adir")
}
