//! Differential tests: C `libdriver.so` vs Rust `libdriver.so`.
//!
//! Both libraries are loaded with `libloading` and driven **only** through
//! their exported `driver` symbol, exactly as an external C caller would, so
//! the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `driver` returns `void` and its entire observable behaviour is what it
//! writes to `stdout`. The harness therefore redirects file descriptor 1 to a
//! temporary file, invokes one implementation, flushes libc's streams, and
//! reads back exactly the bytes that call produced. C and Rust byte streams
//! must be identical.

use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

type DriverFn = unsafe extern "C" fn(c_int, c_int);

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, which is how we force
    /// the block-buffered redirected `stdout` out to the capture file.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built: {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    // Allows the same suite to be pointed at a different build profile /
    // feature configuration of the Rust cdylib.
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO does not exist: {}", p.display());
        return p;
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not built under {} (run `cargo build --release`)",
        base.display()
    );
}

struct Libs {
    c: Library,
    rust: Library,
}

// Symbols borrow from the `Library`, so keep the libraries alive for the whole
// process in a `OnceLock`.
fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");
        Libs { c, rust }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C .so exports `driver`") }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe {
        libs()
            .rust
            .get(b"driver\0")
            .expect("Rust .so exports `driver`")
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

struct Capture {
    saved_stdout: c_int,
    // `writer` owns the fd that is dup2'd onto 1; it must outlive the redirect.
    _writer: File,
    reader: File,
    path: PathBuf,
}

impl Capture {
    fn new(tag: &str) -> Capture {
        let path = std::env::temp_dir().join(format!(
            "driver_diff_{}_{}_{:?}.out",
            std::process::id(),
            tag,
            std::thread::current().id()
        ));
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let writer = File::create(&path).expect("create capture file");
        let reader = File::open(&path).expect("open capture file for reading");
        let saved_stdout = unsafe { dup(1) };
        assert!(saved_stdout >= 0, "dup(1) failed");
        assert!(
            unsafe { dup2(writer.as_raw_fd(), 1) } >= 0,
            "dup2 onto stdout failed"
        );
        Capture {
            saved_stdout,
            _writer: writer,
            reader,
            path,
        }
    }

    /// Runs `f` with stdout captured and returns exactly the bytes it emitted.
    fn run(&mut self, f: impl FnOnce()) -> Vec<u8> {
        f();
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let mut out = Vec::new();
        // The reader has its own file offset, so this returns only the bytes
        // written since the previous `run`.
        self.reader
            .read_to_end(&mut out)
            .expect("read captured stdout");
        out
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved_stdout, 1);
            close(self.saved_stdout);
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Calls `driver(x, y)` in both libraries and returns `(c_bytes, rust_bytes)`.
fn both(cap: &mut Capture, x: c_int, y: c_int) -> (Vec<u8>, Vec<u8>) {
    let c = c_driver();
    let r = rust_driver();
    let c_out = cap.run(|| unsafe { c(x, y) });
    let r_out = cap.run(|| unsafe { r(x, y) });
    (c_out, r_out)
}

fn show(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    if s.len() <= 400 {
        format!("{:?}", s)
    } else {
        format!(
            "{:?}…{:?} (len {})",
            &s[..200],
            &s[s.len() - 200..],
            b.len()
        )
    }
}

/// Asserts byte-for-byte equality of the two implementations for one input.
fn assert_same(cap: &mut Capture, row: &str, x: c_int, y: c_int) -> Vec<u8> {
    let (c_out, r_out) = both(cap, x, y);
    if c_out != r_out {
        // Restore stdout before panicking so the message is visible.
        let msg = format!(
            "DIVERGENCE [{}] driver({}, {}):\n  C   ({} bytes): {}\n  Rust({} bytes): {}\n  first differing byte at {}",
            row,
            x,
            y,
            c_out.len(),
            show(&c_out),
            r_out.len(),
            show(&r_out),
            c_out
                .iter()
                .zip(r_out.iter())
                .position(|(a, b)| a != b)
                .map(|i| i.to_string())
                .unwrap_or_else(|| format!("length only ({} vs {})", c_out.len(), r_out.len())),
        );
        panic!("{}", msg);
    }
    c_out
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible)
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5DEECE66D;

struct Lcg(u64);

impl Lcg {
    fn new(stream: u64) -> Lcg {
        Lcg(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mut z = self.0;
        z ^= z >> 33;
        z = z.wrapping_mul(0xff51afd7ed558ccd);
        z ^= z >> 33;
        z
    }
    /// Uniform in `[lo, hi]` (inclusive), valid across the whole `i32` range.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

// ---------------------------------------------------------------------------
// Phase B — valid-path differential tests, one block per CONFIGS.md row
// ---------------------------------------------------------------------------

/// `(row label, iterations, generator)` for every row of `CONFIGS.md`.
#[allow(clippy::type_complexity)]
fn configs_rows() -> Vec<(&'static str, usize, Box<dyn Fn(&mut Lcg) -> (i32, i32)>)> {
    vec![
        ("row 1: x==0,y==0 (empty)", 1, Box::new(|_: &mut Lcg| (0, 0))),
        (
            "row 2: x in [INT_MIN,-1], y==0",
            128,
            Box::new(|r: &mut Lcg| (r.range(i32::MIN, -1), 0)),
        ),
        (
            "row 3: x==0, y in [INT_MIN,-1]",
            128,
            Box::new(|r: &mut Lcg| (0, r.range(i32::MIN, -1))),
        ),
        (
            "row 4: x,y in [INT_MIN,-1]",
            128,
            Box::new(|r: &mut Lcg| (r.range(i32::MIN, -1), r.range(i32::MIN, -1))),
        ),
        ("row 5: x==0,y==1", 1, Box::new(|_: &mut Lcg| (0, 1))),
        (
            "row 6: x==0, y in [2,4]",
            32,
            Box::new(|r: &mut Lcg| (0, r.range(2, 4))),
        ),
        (
            "row 7: x==0, y in [5,4096]",
            128,
            Box::new(|r: &mut Lcg| (0, r.range(5, 4096))),
        ),
        (
            "row 8: x in [-4096,-1], y in [1,4096]",
            64,
            Box::new(|r: &mut Lcg| (r.range(-4096, -1), r.range(1, 4096))),
        ),
        (
            "row 9: x==INT_MIN, y in [1,512]",
            64,
            Box::new(|r: &mut Lcg| (i32::MIN, r.range(1, 512))),
        ),
        ("row 10: x==1,y==0", 1, Box::new(|_: &mut Lcg| (1, 0))),
        ("row 11: x==1,y==1", 1, Box::new(|_: &mut Lcg| (1, 1))),
        ("row 12: x==1,y==3", 1, Box::new(|_: &mut Lcg| (1, 3))),
        ("row 13: x==1,y==4 (forward goto)", 1, Box::new(|_: &mut Lcg| (1, 4))),
        ("row 14: x==1,y==5", 1, Box::new(|_: &mut Lcg| (1, 5))),
        (
            "row 15: x==1, y in [6,4096]",
            128,
            Box::new(|r: &mut Lcg| (1, r.range(6, 4096))),
        ),
        ("row 16: x==2,y==0", 1, Box::new(|_: &mut Lcg| (2, 0))),
        ("row 17: x==2,y==4", 1, Box::new(|_: &mut Lcg| (2, 4))),
        (
            "row 18: x==2, y in [1,4096]",
            128,
            Box::new(|r: &mut Lcg| (2, r.range(1, 4096))),
        ),
        ("row 19: x==3,y==0", 1, Box::new(|_: &mut Lcg| (3, 0))),
        ("row 20: x==3,y==4", 1, Box::new(|_: &mut Lcg| (3, 4))),
        (
            "row 21: x==3, y in [1,4096]",
            128,
            Box::new(|r: &mut Lcg| (3, r.range(1, 4096))),
        ),
        ("row 22: x==4,y==4", 1, Box::new(|_: &mut Lcg| (4, 4))),
        (
            "row 23: x in [4,64], y==0",
            61,
            Box::new(|r: &mut Lcg| (r.range(4, 64), 0)),
        ),
        (
            "row 24: x in [4,64], y in [1,64]",
            1024,
            Box::new(|r: &mut Lcg| (r.range(4, 64), r.range(1, 64))),
        ),
        (
            "row 25: x,y in [500,4096]",
            32,
            Box::new(|r: &mut Lcg| (r.range(500, 4096), r.range(500, 4096))),
        ),
        (
            "row 26: x in [4,4096], y in [1,3]",
            64,
            Box::new(|r: &mut Lcg| (r.range(4, 4096), r.range(1, 3))),
        ),
        (
            "row 27: x in [1,3], y in [4,4096]",
            64,
            Box::new(|r: &mut Lcg| (r.range(1, 3), r.range(4, 4096))),
        ),
        (
            "row 28: broad sweep x,y in [-8,40]",
            2048,
            Box::new(|r: &mut Lcg| (r.range(-8, 40), r.range(-8, 40))),
        ),
        ("row 29: x==4096,y==4096", 1, Box::new(|_: &mut Lcg| (4096, 4096))),
        (
            "row 30: single-axis large",
            2,
            Box::new(|r: &mut Lcg| {
                if r.next_u64() & 1 == 0 {
                    (0, 4096)
                } else {
                    (4096, 0)
                }
            }),
        ),
    ]
}

/// Inputs on which the C never returns (`ERRORS.md` rows 12–13) must never be
/// fed to the libraries. Guard every generated input with this predicate.
fn terminates(x: i32, y: i32) -> bool {
    // Guard rejects immediately -> always terminates.
    if !(x > 0 || y > 0) {
        return true;
    }
    // Accepted and y < 0: `y == 0` is never reached, so `y--` runs forever.
    y >= 0
}

fn phase_b_configuration_surface() {
    let mut cap = Capture::new("phase_b");
    let mut total = 0usize;
    for (i, (label, iters, gen)) in configs_rows().into_iter().enumerate() {
        let mut rng = Lcg::new(i as u64 + 1);
        for k in 0..iters {
            // Some rows (e.g. the broad sweep) span the non-terminating region
            // `x > 0 && y < 0` (ERRORS.md rows 12-13). Re-draw past it: the C
            // never returns there, so it cannot be differentially executed.
            let mut drawn = None;
            for _ in 0..64 {
                let (x, y) = gen(&mut rng);
                if terminates(x, y) {
                    drawn = Some((x, y));
                    break;
                }
            }
            let (x, y) = match drawn {
                Some(p) => p,
                None => continue,
            };
            assert_same(&mut cap, &format!("{} iter {}", label, k), x, y);
            total += 1;
        }
    }
    drop(cap);
    println!("phase B: {} differential invocations matched", total);
    assert!(total >= 2000, "expected a substantial sweep, got {}", total);
}

/// Exhaustive small grid — every `(x, y)` pair in `[-4, 40]^2` that terminates.
/// This is the cross-product of all value classes in `CONFIGS.md` at full
/// density, so no class interaction can be missed.
fn phase_b_exhaustive_small_grid() {
    let mut cap = Capture::new("grid");
    let mut n = 0;
    for x in -4..=40 {
        for y in -4..=40 {
            if !terminates(x, y) {
                continue;
            }
            assert_same(&mut cap, &format!("grid({},{})", x, y), x, y);
            n += 1;
        }
    }
    drop(cap);
    println!("phase B grid: {} pairs matched", n);
    assert!(n > 1800, "grid too small: {}", n);
}

/// Independent oracle for the C control flow, written straight from
/// `c_src/src/driver.c`. This catches the case where *both* `.so`s agree
/// because the Rust and the harness share a misunderstanding — it pins the
/// expected byte stream to the C source text itself.
fn oracle(mut x: i32, mut y: i32) -> Vec<u8> {
    let mut out = Vec::new();
    // Program counter over the C statements, so the `goto`s are literal jumps.
    #[derive(PartialEq)]
    enum Pc {
        Cond,
        Head,
        Label1,
        Label2,
    }
    let mut pc = Pc::Cond;
    loop {
        match pc {
            Pc::Cond => {
                if x > 0 || y > 0 {
                    pc = Pc::Head;
                } else {
                    return out;
                }
            }
            Pc::Head => {
                out.extend_from_slice(b"loop\n");
                if x == 1 && y == 4 {
                    pc = Pc::Label2;
                } else {
                    pc = Pc::Label1;
                }
            }
            Pc::Label1 => {
                if x > 0 {
                    out.extend_from_slice(b"x\n");
                    x -= 1;
                }
                pc = Pc::Label2;
            }
            Pc::Label2 => {
                if y == 0 {
                    pc = Pc::Cond; // `continue`
                    continue;
                }
                out.extend_from_slice(b"y\n");
                y -= 1;
                pc = if x < 3 { Pc::Label1 } else { Pc::Cond };
            }
        }
    }
}

fn phase_b_matches_independent_oracle() {
    let mut cap = Capture::new("oracle");
    let mut rng = Lcg::new(0xABCD);
    let mut checked = 0;
    // Fixed interesting points plus a randomized sweep.
    let mut cases: Vec<(i32, i32)> = vec![
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (1, 3),
        (1, 4),
        (1, 5),
        (2, 4),
        (3, 4),
        (4, 4),
        (3, 0),
        (5, 7),
        (i32::MIN, 3),
        (-1, 0),
        (0, -1),
        (-1, -1),
        (i32::MIN, i32::MIN),
        (0, i32::MIN),
        (i32::MIN, 0),
    ];
    for _ in 0..400 {
        cases.push((rng.range(-6, 30), rng.range(-6, 30)));
    }
    for (x, y) in cases {
        if !terminates(x, y) {
            continue;
        }
        let got = assert_same(&mut cap, &format!("oracle({},{})", x, y), x, y);
        let want = oracle(x, y);
        assert_eq!(
            got,
            want,
            "both .so agree but disagree with the C-source oracle for driver({}, {})",
            x,
            y
        );
        checked += 1;
    }
    drop(cap);
    println!("phase B oracle: {} cases matched C, Rust and oracle", checked);
    assert!(checked > 300);
}

// ---------------------------------------------------------------------------
// Phase C — error / rejection surface (ERRORS.md rows 1–11)
// ---------------------------------------------------------------------------

fn phase_c_error_surface_rows_1_through_11() {
    let mut cap = Capture::new("phase_c");

    // Rows 1-6, 10: the loop-entry guard rejects. The C emits *nothing at all*;
    // that empty byte stream is the rejection signal (the function is `void`,
    // so there is no code to compare). Assert both are byte-identical AND
    // that the shared result is genuinely the rejection, not merely "equal".
    let rejecting: &[(&str, i32, i32)] = &[
        ("row 1: x==0,y==0", 0, 0),
        ("row 2: x==0,y==-1", 0, -1),
        ("row 3: x==-1,y==0", -1, 0),
        ("row 4: x==-1,y==-1", -1, -1),
        ("row 5: x==INT_MIN,y==INT_MIN", i32::MIN, i32::MIN),
        ("row 6a: x==INT_MIN,y==0", i32::MIN, 0),
        ("row 6b: x==0,y==INT_MIN", 0, i32::MIN),
        ("row 10: x==0,y==INT_MIN", 0, i32::MIN),
        ("row 10b: x==-7,y==INT_MIN", -7, i32::MIN),
    ];
    for (label, x, y) in rejecting {
        let out = assert_same(&mut cap, label, *x, *y);
        assert!(
            out.is_empty(),
            "{}: expected the guard to reject with zero output, got {}",
            label,
            show(&out)
        );
        assert_eq!(out, oracle(*x, *y), "{}: oracle mismatch", label);
    }

    // Row 7: one step past the rejecting range on x only. y stays 0 forever so
    // the `if (y == 0) continue;` branch is taken on every iteration and no
    // "y" line is ever produced.
    let out = assert_same(&mut cap, "row 7: x==1,y==0", 1, 0);
    assert_eq!(out, b"loop\nx\n", "row 7 byte stream");
    assert!(!out.windows(2).any(|w| w == b"y\n"), "row 7 must never print y");

    // Row 8: one step past on y only. x <= 0 so `if (x > 0)` at label1 never
    // fires and no "x" line is ever produced.
    let out = assert_same(&mut cap, "row 8: x==0,y==1", 0, 1);
    assert_eq!(out, b"loop\ny\n", "row 8 byte stream");
    assert!(!out.windows(2).any(|w| w == b"x\n"), "row 8 must never print x");

    // Row 9: extreme negative x with a live y. `x--` is unreachable so there is
    // no underflow, and `x < 3` keeps the back-edge to label1 hot: exactly one
    // "loop" followed by y copies of "y".
    for y in [1, 2, 3, 4, 5, 17, 512] {
        let out = assert_same(&mut cap, &format!("row 9: x==INT_MIN,y=={}", y), i32::MIN, y);
        let mut want = b"loop\n".to_vec();
        for _ in 0..y {
            want.extend_from_slice(b"y\n");
        }
        assert_eq!(out, want, "row 9 (y={}) byte stream", y);
    }

    // Row 11: extreme-but-terminating magnitudes on the accepted side.
    for (x, y) in [(0, 4096), (4096, 0), (4096, 4096)] {
        let out = assert_same(&mut cap, &format!("row 11: ({},{})", x, y), x, y);
        assert_eq!(out, oracle(x, y), "row 11 ({},{}) oracle mismatch", x, y);
        assert!(!out.is_empty());
    }

    drop(cap);
    println!("phase C: ERRORS.md rows 1-11 all matched");
}

/// Generic C-API boundary sweep required by Phase C, adapted to this API's
/// actual shape: no pointers, no lengths, no enums — two by-value `int`s. So
/// the boundary set is the extremes of `int` plus one step either side of every
/// constant the C compares against (0, 1, 3, 4), on both parameters.
fn phase_c_generic_int_boundaries() {
    let mut cap = Capture::new("bounds");

    let interesting: [i32; 14] = [
        i32::MIN,
        i32::MIN + 1,
        -4097,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        4096,
        i32::MAX, // only used where the guard rejects or y == 0 is impossible
    ];

    let mut n = 0;
    let mut skipped_nonterminating = 0;
    let mut skipped_impractical = 0;
    for &x in &interesting {
        for &y in &interesting {
            if !terminates(x, y) {
                // ERRORS.md rows 12-13: C never returns. Cannot be executed.
                skipped_nonterminating += 1;
                continue;
            }
            // ERRORS.md row 14: terminates, but emits O(2^31) lines.
            if x > 100_000 || y > 100_000 {
                skipped_impractical += 1;
                continue;
            }
            let out = assert_same(&mut cap, &format!("bound({},{})", x, y), x, y);
            assert_eq!(out, oracle(x, y), "bound({},{}) oracle mismatch", x, y);
            n += 1;
        }
    }
    drop(cap);
    println!(
        "phase C boundaries: {} executed, {} non-terminating (ERRORS rows 12-13), {} impractical (row 14)",
        n, skipped_nonterminating, skipped_impractical
    );
    assert!(n > 100, "boundary sweep too small: {}", n);
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity asserted from inside the test, via dlsym
// ---------------------------------------------------------------------------

fn phase_d_symbol_parity() {
    // Every symbol the C .so exports must resolve in the Rust .so under the
    // exact same name. The C .so exports exactly one non-libc symbol.
    let c_syms = nm_defined(&c_so_path());
    let rust_syms = nm_defined(&rust_so_path());
    assert!(
        c_syms.contains(&"driver".to_string()),
        "C .so must export `driver`, got {:?}",
        c_syms
    );
    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {:?}",
        missing
    );

    // And prove they are callable through dlsym, not just present in the table.
    let _ = c_driver();
    let _ = rust_driver();
}

/// Defined, non-libc global symbols of a shared object, via `nm -D
/// --defined-only`. Returns an empty-tolerant list; if `nm` is unavailable the
/// test falls back to the dlsym check above.
fn nm_defined(path: &Path) -> Vec<String> {
    let out = match std::process::Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        _ => return vec!["driver".to_string()],
    };
    String::from_utf8_lossy(&out)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Global text/data only; skip Rust/libc runtime plumbing.
            if !matches!(kind, "T" | "D" | "B" | "R" | "W" | "V") {
                return None;
            }
            if name.starts_with('_') || name.starts_with("rust_") {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

/// Documents (and enforces) the non-terminating region so the exclusion in
/// `ERRORS.md` rows 12–13 is not a silent hand-wave: the predicate the harness
/// uses must agree with the C source's structure.
fn phase_c_nontermination_predicate_is_sound() {
    // Accepted by the guard and y < 0 => `y == 0` is unreachable (y only ever
    // decreases), so `continue` never fires and the loop cannot exit.
    for x in 1..50 {
        for y in [-1, -2, -1000, i32::MIN] {
            assert!(!terminates(x, y), "({}, {}) should be non-terminating", x, y);
        }
    }
    // Rejected by the guard => returns before touching y.
    for x in [0, -1, i32::MIN] {
        for y in [0, -1, i32::MIN] {
            assert!(terminates(x, y), "({}, {}) is rejected by the guard", x, y);
        }
    }
    // y >= 0 always terminates: x only decrements while x > 0 (so x >= 0 is
    // invariant and bounded below), and y strictly decreases whenever y != 0.
    for x in [-1, 0, 1, 2, 3, 4, 100, i32::MIN] {
        for y in [0, 1, 4, 5, 100] {
            assert!(terminates(x, y), "({}, {}) should terminate", x, y);
        }
    }
}

// A dummy use to keep the `c_char` import meaningful for readers of the FFI
// section above (the loaded symbols use `c_int` only).
const _: Option<*const c_char> = None;

// ---------------------------------------------------------------------------
// Single entry point.
//
// All phases run inside ONE `#[test]`. `driver`'s only observable behaviour is
// what it writes to file descriptor 1, so the harness has to redirect fd 1 —
// and fd 1 is process-wide. `cargo test` writes its own "test ... ok" lines to
// fd 1 from the harness thread whenever *any* test finishes, which would be
// captured as if the library had emitted it. Running every phase sequentially
// in a single test is what makes the capture window exclusively ours.
// ---------------------------------------------------------------------------

#[test]
fn differential_all_phases() {
    // Phase D first: no point diffing behaviour if the export surface differs.
    phase_d_symbol_parity();
    println!("== phase D: symbol parity OK ==");

    phase_c_nontermination_predicate_is_sound();
    println!("== non-termination predicate sound ==");

    // Phase B — valid paths.
    phase_b_configuration_surface();
    phase_b_exhaustive_small_grid();
    phase_b_matches_independent_oracle();

    // Phase C — error / rejection surface.
    phase_c_error_surface_rows_1_through_11();
    phase_c_generic_int_boundaries();

    println!("== all phases passed ==");
}
