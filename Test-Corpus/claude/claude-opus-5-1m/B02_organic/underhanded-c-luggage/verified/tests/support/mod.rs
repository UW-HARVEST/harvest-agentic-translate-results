// Shared helpers for the differential tests.
//
// * builds the C artifacts (executable via cmake, shared library via gcc — both
//   from the pristine `c_src/`, using compiler flags only),
// * locates the Rust artifacts (executable + `libdriver.so`),
// * runs both executables with identical argv/stdin and compares
//   stdout/stderr/exit status byte-for-byte,
// * a deterministic (seeded) PRNG plus the input generators used by the
//   property-style tests.

#![allow(dead_code)]

pub mod ffi;

use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C executable produced by `c_src/CMakeLists.txt`.
pub fn c_exe() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let build = manifest_dir().join("c_src/build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let st = Command::new("cmake")
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&build)
                .status()
                .expect("run cmake");
            assert!(st.success(), "cmake configure failed");
            let st = Command::new("cmake")
                .arg("--build")
                .arg(".")
                .current_dir(&build)
                .status()
                .expect("run cmake --build");
            assert!(st.success(), "cmake build failed");
        }
        assert!(exe.exists(), "missing C executable {:?}", exe);
        exe
    })
    .as_path()
}

/// The Rust executable under test (never called in-process).
pub fn rust_exe() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// The C translation unit built as a shared library.  `c_src/` is NOT modified:
/// only compiler flags are used, `-Dmain=luggage_main` renames `main` so the
/// object can be linked into a `.so`.
pub fn c_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let dir = manifest_dir().join("cbuild");
        std::fs::create_dir_all(&dir).expect("mkdir cbuild");
        let so = dir.join("libluggage.so");
        let src = manifest_dir().join("c_src/src/luggage.c");
        let tmp = dir.join(format!("libluggage.{}.so", std::process::id()));
        if !so.exists() {
            let st = Command::new("gcc")
                .args(["-shared", "-fPIC", "-O0", "-Dmain=luggage_main", "-o"])
                .arg(&tmp)
                .arg(&src)
                .status()
                .expect("run gcc");
            assert!(st.success(), "gcc -shared failed");
            std::fs::rename(&tmp, &so).expect("rename .so");
        }
        assert!(so.exists(), "missing C shared library {:?}", so);
        so
    })
    .as_path()
}

/// The Rust `cdylib` (`libdriver.so`).
///
/// `cargo test` only builds the `rlib` flavour of the library, so if the shared
/// object is not next to the test binary it is built on demand into a separate
/// target directory (a separate directory keeps its own build lock, so this
/// cannot deadlock against the `cargo test` invocation that is running us).
pub fn rust_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/libdriver.so
        let exe = std::env::current_exe().expect("current_exe");
        let deps = exe.parent().expect("deps dir");
        let profile_dir = deps.parent().expect("profile dir");
        let candidate = profile_dir.join("libdriver.so");
        if candidate.exists() {
            return candidate;
        }
        let release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
        let out_dir = manifest_dir().join("target/ffi_so");
        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&out_dir)
            .current_dir(manifest_dir());
        // Feature selection must match the test run so that both artifacts are
        // built from the same configuration.
        if let Ok(features) = std::env::var("DIFF_TEST_FEATURES") {
            cmd.arg("--no-default-features");
            if !features.is_empty() {
                cmd.arg("--features").arg(features);
            }
        }
        if release {
            cmd.arg("--release");
        }
        let st = cmd.status().expect("run cargo build --lib");
        assert!(st.success(), "building the cdylib failed");
        let so = out_dir
            .join(if release { "release" } else { "debug" })
            .join("libdriver.so");
        assert!(so.exists(), "missing Rust shared library {:?}", so);
        so
    })
    .as_path()
}

#[derive(PartialEq, Eq)]
pub struct Run {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Run {{ code: {:?}, signal: {:?}, stdout: {}, stderr: {} }}",
            self.code,
            self.signal,
            esc(&self.stdout),
            esc(&self.stderr)
        )
    }
}

pub fn esc(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{:02x}", b)),
        }
    }
    s.push('"');
    s
}

pub fn run_exe(exe: &Path, args: &[Vec<u8>], stdin_data: &[u8]) -> Run {
    use std::os::unix::process::ExitStatusExt;
    let mut cmd = Command::new(exe);
    for a in args {
        cmd.arg(OsStr::from_bytes(a));
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {:?}: {}", exe, e));
    {
        let mut si = child.stdin.take().expect("stdin");
        let data = stdin_data.to_vec();
        // The programs read all of stdin before writing anything, so a plain
        // blocking write is safe here.
        let _ = si.write_all(&data);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait");
    Run {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

/// Runs the C and the Rust executable with the same argv/stdin and asserts that
/// stdout, stderr and the exit status are identical.
#[track_caller]
pub fn assert_same(label: &str, args: &[Vec<u8>], stdin_data: &[u8]) {
    let c = run_exe(c_exe(), args, stdin_data);
    let r = run_exe(rust_exe(), args, stdin_data);
    if c != r {
        let pretty_args: Vec<String> = args.iter().map(|a| esc(a)).collect();
        panic!(
            "DIVERGENCE [{}]\n  argv = [{}]\n  stdin = {}\n  C    = {:?}\n  Rust = {:?}",
            label,
            pretty_args.join(", "),
            esc(stdin_data),
            c,
            r
        );
    }
}

pub fn wildcards() -> Vec<Vec<u8>> {
    vec![b"-".to_vec(), b"-".to_vec(), b"-".to_vec(), b"-".to_vec()]
}

pub fn argv(a: &[&str]) -> Vec<Vec<u8>> {
    a.iter().map(|s| s.as_bytes().to_vec()).collect()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seeds keep the tests reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xDEAD_BEEF_CAFE_F00D)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn flip(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() & 0xff) as u8
    }
    /// Random token whose length is drawn from the inclusive range `lo..=hi`.
    pub fn token(&mut self, alphabet: &[u8], lo: usize, hi: usize) -> Vec<u8> {
        let len = self.range(lo, hi);
        gen_token(self, alphabet, len)
    }
    /// `lo..=hi` random tokens of at most `max` characters each.
    pub fn pool(&mut self, lo: usize, hi: usize, alphabet: &[u8], max: usize) -> Vec<Vec<u8>> {
        let n = self.range(lo, hi);
        gen_pool(self, n, alphabet, max)
    }
}

// ---------------------------------------------------------------------------
// Input generators
// ---------------------------------------------------------------------------

pub const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const ALNUM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
pub const NOT_IN_SET: &[u8] = b"abcxyz_.!/*#%$@^&()";

pub fn gen_token(rng: &mut Rng, alphabet: &[u8], len: usize) -> Vec<u8> {
    (0..len).map(|_| *rng.pick(alphabet)).collect()
}

/// A luggage id / flight id / airport code of a random length within its limit.
pub fn gen_field(rng: &mut Rng, alphabet: &[u8], max: usize) -> Vec<u8> {
    let len = rng.range(1, max);
    gen_token(rng, alphabet, len)
}

/// Random timestamp text covering axis C of CONFIGS.md.
pub fn gen_timestamp(rng: &mut Rng) -> Vec<u8> {
    let shapes: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    match rng.pick(&shapes) {
        0 => format!("{}", rng.below(5)).into_bytes(),
        1 => format!("{}", rng.next_u64() % 1_000_000).into_bytes(),
        2 => format!("{}", rng.next_u64() % (u32::MAX as u64 + 1)).into_bytes(),
        3 => format!("+{}", rng.next_u64() % 1_000_000).into_bytes(),
        4 => format!("-{}", rng.next_u64() % 1_000_000).into_bytes(),
        5 => {
            let zeros = "0".repeat(rng.range(1, 6));
            format!("{}{}", zeros, rng.below(1000)).into_bytes()
        }
        6 => {
            let boundaries: [&str; 10] = [
                "0",
                "1",
                "2147483647",
                "2147483648",
                "4294967295",
                "4294967296",
                "9223372036854775807",
                "9223372036854775808",
                "-9223372036854775808",
                "-9223372036854775809",
            ];
            rng.pick(&boundaries).as_bytes().to_vec()
        }
        7 => {
            let n = rng.range(1, 40);
            let mut v = Vec::new();
            for _ in 0..n {
                v.push(b'0' + rng.byte() % 10);
            }
            v
        }
        8 => format!("-{}", rng.next_u64()).into_bytes(),
        9 => format!("{}", rng.next_u64()).into_bytes(),
        10 => {
            // leading whitespace before the number
            let mut v = Vec::new();
            for _ in 0..rng.range(1, 3) {
                v.push(*rng.pick(b" \t\n"));
            }
            v.extend_from_slice(format!("{}", rng.below(1000)).as_bytes());
            v
        }
        _ => format!("{}", rng.below(100)).into_bytes(),
    }
}

pub fn gen_comment(rng: &mut Rng) -> Vec<u8> {
    match rng.below(8) {
        0 => Vec::new(),
        1 => vec![b' '; rng.range(1, 3)],
        2 => {
            let mut v = vec![b' '];
            v.extend(std::iter::repeat(b'x').take(79));
            v // exactly 80 characters of comment
        }
        3 => {
            let mut v = vec![b' '];
            v.extend(std::iter::repeat(b'y').take(rng.range(85, 150)));
            v // longer than 80 => truncation + leak into the next iteration
        }
        4 => {
            let mut v = vec![b' '];
            v.extend_from_slice(b"ab\0cd ef");
            v // embedded NUL
        }
        5 => {
            let mut v = vec![b' '];
            for _ in 0..rng.range(1, 20) {
                let b = rng.byte();
                v.push(if b == b'\n' || b == 0 { b'?' } else { b });
            }
            v // arbitrary bytes (incl. >= 0x80), no newline
        }
        6 => b" \t\ttabbed comment".to_vec(),
        _ => {
            let pool: &[u8] = b"ABCabc 0123 ,.;:/-_()[]#*%\t\r";
            let n = rng.range(0, 40);
            let mut v = vec![b' '];
            v.extend((0..n).map(|_| *rng.pick(pool)));
            v
        }
    }
}

/// Whitespace run used as a field separator (axis E).
pub fn gen_sep(rng: &mut Rng) -> Vec<u8> {
    match rng.below(8) {
        0 => b" ".to_vec(),
        1 => b"  ".to_vec(),
        2 => b"   ".to_vec(),
        3 => b"\t".to_vec(),
        4 => b"\n".to_vec(),
        5 => b"\x0b".to_vec(),
        6 => b"\x0c".to_vec(),
        _ => b" \t ".to_vec(),
    }
}

pub struct RecordSpec {
    pub time_stamp: Vec<u8>,
    pub luggage_id: Vec<u8>,
    pub flight_id: Vec<u8>,
    pub departure: Vec<u8>,
    pub arrival: Vec<u8>,
    pub comment: Vec<u8>,
}

impl RecordSpec {
    pub fn render(&self, rng: &mut Rng, fancy_separators: bool) -> Vec<u8> {
        let mut v = Vec::new();
        let sep = |rng: &mut Rng| -> Vec<u8> {
            if fancy_separators {
                gen_sep(rng)
            } else {
                b" ".to_vec()
            }
        };
        v.extend_from_slice(&self.time_stamp);
        v.extend_from_slice(&sep(rng));
        v.extend_from_slice(&self.luggage_id);
        v.extend_from_slice(&sep(rng));
        v.extend_from_slice(&self.flight_id);
        v.extend_from_slice(&sep(rng));
        v.extend_from_slice(&self.departure);
        v.extend_from_slice(&sep(rng));
        v.extend_from_slice(&self.arrival);
        v.extend_from_slice(&self.comment);
        v.push(b'\n');
        v
    }
}

pub fn gen_record(rng: &mut Rng) -> RecordSpec {
    RecordSpec {
        time_stamp: gen_timestamp(rng),
        luggage_id: gen_field(rng, ALNUM, 8),
        flight_id: gen_field(rng, ALNUM, 6),
        departure: gen_field(rng, UPPER, 3),
        arrival: gen_field(rng, UPPER, 3),
        comment: gen_comment(rng),
    }
}

/// A record built from small pools so that superseding / ties / filters fire.
pub fn gen_pool_record(
    rng: &mut Rng,
    lugs: &[Vec<u8>],
    flights: &[Vec<u8>],
    airports: &[Vec<u8>],
    ts_pool: usize,
) -> RecordSpec {
    RecordSpec {
        time_stamp: format!("{}", rng.below(ts_pool)).into_bytes(),
        luggage_id: rng.pick(lugs).clone(),
        flight_id: rng.pick(flights).clone(),
        departure: rng.pick(airports).clone(),
        arrival: rng.pick(airports).clone(),
        comment: gen_comment(rng),
    }
}

pub fn gen_pool(rng: &mut Rng, n: usize, alphabet: &[u8], max: usize) -> Vec<Vec<u8>> {
    (0..n).map(|_| gen_field(rng, alphabet, max)).collect()
}

/// Random filter arguments (axis A) — mixes wildcards, values taken from the
/// stream, empty strings, `-`-prefixed strings and random literals.
pub fn gen_filters(rng: &mut Rng, words: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for _ in 0..4 {
        out.push(match rng.below(10) {
            0..=4 => b"-".to_vec(),
            5 | 6 => {
                if words.is_empty() {
                    b"-".to_vec()
                } else {
                    rng.pick(words).clone()
                }
            }
            7 => Vec::new(),
            8 => {
                let mut v = b"-".to_vec();
                v.extend(rng.token(ALNUM, 0, 3));
                v
            }
            _ => rng.token(ALNUM, 1, 4),
        });
    }
    out
}
