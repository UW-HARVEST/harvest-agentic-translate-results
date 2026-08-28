//! Differential tests for the *internal* (file-local) functions of the
//! translation unit.
//!
//! The shipped `.so` only exports `jumpnode`, and because `initialize_test_data`
//! is `static` and never called, `node_count` is permanently 0 there — so
//! `find_node_by_id`, `add_node`, `process_backward`, and the mode 0001/0002/0004
//! bodies of `jumpnode` are unreachable through the public API.
//!
//! To verify those paths anyway, two harness libraries are built:
//!   * C:    `tests/harness/c_harness.c` does `#include "lib.c"` on the pristine
//!           `c_src/src/lib.c`, making its statics reachable.
//!   * Rust: `tests/harness/rust/` does `include!("../src/lib.rs")` on the
//!           pristine translation, making its private items reachable.
//!
//! Neither `c_src/` nor the shipped crate is modified. Both harnesses are
//! loaded with `libloading` and compared through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_uchar};
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn harness_out_dir() -> PathBuf {
    let d = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("harness");
    std::fs::create_dir_all(&d).expect("create harness output dir");
    d
}

/// Optimization level for the C harness. Defaults to `-O0`, matching the
/// shipped CMake build (which sets no `CMAKE_BUILD_TYPE`, hence no `-O` flag).
/// Override with `HARNESS_C_OPT` to re-run the suite at other levels.
fn c_opt_flag() -> String {
    std::env::var("HARNESS_C_OPT").unwrap_or_else(|_| "-O0".to_string())
}

/// Cargo profile for the Rust harness. Override with `HARNESS_RUST_PROFILE`.
fn rust_profile() -> String {
    std::env::var("HARNESS_RUST_PROFILE").unwrap_or_else(|_| "release".to_string())
}

fn build_c_harness() -> PathBuf {
    let root = workspace_root();
    let opt = c_opt_flag();
    let out = harness_out_dir().join(format!("libc_internals{}.so", opt.replace('-', "_")));
    let status = std::process::Command::new("cc")
        .args(["-shared", "-fPIC"])
        .arg(&opt)
        .arg("-I")
        .arg(root.join("c_src").join("src"))
        .arg("-I")
        .arg(root.join("c_src").join("include"))
        .arg(
            root.join("translation")
                .join("tests")
                .join("harness")
                .join("c_harness.c"),
        )
        .arg("-o")
        .arg(&out)
        .arg("-lm")
        .status()
        .expect("`cc` must be available to build the C harness");
    assert!(status.success(), "C harness build failed ({opt})");
    out
}

fn build_rust_harness() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("harness")
        .join("rust");
    let profile = rust_profile();
    let target_dir = harness_out_dir().join("rust-target");

    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.arg("build");
    if profile != "dev" && profile != "debug" {
        cmd.arg("--profile").arg(&profile);
    }
    let out = cmd
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        // Avoid inheriting the outer cargo invocation's environment, which can
        // otherwise confuse the nested build.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_MAKEFLAGS")
        .output()
        .expect("nested `cargo build` for the Rust harness");
    assert!(
        out.status.success(),
        "Rust harness build failed ({profile}):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = if profile == "dev" || profile == "debug" {
        "debug"
    } else {
        &profile
    };
    let so = target_dir.join(dir).join("libjumpnode_internals.so");
    assert!(so.is_file(), "expected {}", so.display());
    so
}

/// Thin accessor over one loaded harness library.
struct Harness {
    lib: Library,
    name: &'static str,
}

macro_rules! sym {
    ($h:expr, $name:literal, $ty:ty) => {{
        #[allow(unused_unsafe)]
        let s: Symbol<$ty> = unsafe { $h.lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("{} missing `{}`: {e}", $h.name, $name));
        *s
    }};
}

impl Harness {
    fn open(path: &Path, name: &'static str) -> Harness {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Harness { lib, name }
    }

    fn reset(&self) {
        unsafe { sym!(self, "h_reset", unsafe extern "C" fn())() }
    }
    fn node_count(&self) -> c_int {
        unsafe { sym!(self, "h_node_count", unsafe extern "C" fn() -> c_int)() }
    }
    fn set_node_count(&self, n: c_int) {
        unsafe { sym!(self, "h_set_node_count", unsafe extern "C" fn(c_int))(n) }
    }
    fn find_node_by_id(&self, id: c_int) -> c_int {
        unsafe { sym!(self, "h_find_node_by_id", unsafe extern "C" fn(c_int) -> c_int)(id) }
    }
    fn add_node(&self, id: c_int, parent: c_int, value: c_double) -> c_int {
        unsafe {
            sym!(
                self,
                "h_add_node",
                unsafe extern "C" fn(c_int, c_int, c_double) -> c_int
            )(id, parent, value)
        }
    }
    fn process_backward(&self, array: &mut [c_int], size: usize, start_offset: c_int) -> c_int {
        unsafe {
            sym!(
                self,
                "h_process_backward",
                unsafe extern "C" fn(*mut c_int, usize, c_int) -> c_int
            )(array.as_mut_ptr(), size, start_offset)
        }
    }
    fn compute_size_metric(&self, bytes: &[u8]) -> c_int {
        assert_eq!(bytes.last(), Some(&0), "input must be NUL-terminated");
        unsafe {
            sym!(
                self,
                "h_compute_size_metric",
                unsafe extern "C" fn(*const c_char) -> c_int
            )(bytes.as_ptr() as *const c_char)
        }
    }
    fn safe_double_to_int(&self, v: c_double) -> c_int {
        unsafe {
            sym!(
                self,
                "h_safe_double_to_int",
                unsafe extern "C" fn(c_double) -> c_int
            )(v)
        }
    }
    fn initialize_test_data(&self) {
        unsafe { sym!(self, "h_initialize_test_data", unsafe extern "C" fn())() }
    }
    fn get_node(&self, index: c_int) -> Option<(c_int, c_int, c_double, [c_int; 4])> {
        let mut id: c_int = 0;
        let mut parent: c_int = 0;
        let mut value: c_double = 0.0;
        let mut data: [c_int; 4] = [0; 4];
        let rc = unsafe {
            sym!(
                self,
                "h_get_node",
                unsafe extern "C" fn(
                    c_int,
                    *mut c_int,
                    *mut c_int,
                    *mut c_double,
                    *mut c_int,
                ) -> c_int
            )(index, &mut id, &mut parent, &mut value, data.as_mut_ptr())
        };
        if rc == 0 {
            Some((id, parent, value, data))
        } else {
            None
        }
    }
    fn node_bytes(&self, index: c_int) -> Option<Vec<u8>> {
        let n = self.sizeof_node();
        let mut buf = vec![0u8; n];
        let rc = unsafe {
            sym!(
                self,
                "h_node_bytes",
                unsafe extern "C" fn(c_int, *mut c_uchar, usize) -> c_int
            )(index, buf.as_mut_ptr(), buf.len())
        };
        if rc >= 0 {
            buf.truncate(rc as usize);
            Some(buf)
        } else {
            None
        }
    }
    fn sizeof_node(&self) -> usize {
        unsafe { sym!(self, "h_sizeof_node", unsafe extern "C" fn() -> usize)() }
    }
    fn constant(&self, name: &'static str) -> c_int {
        let s: Symbol<unsafe extern "C" fn() -> c_int> =
            unsafe { self.lib.get(format!("{name}\0").as_bytes()) }
                .unwrap_or_else(|e| panic!("{} missing `{name}`: {e}", self.name));
        unsafe { s() }
    }
    fn jumpnode(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe {
            sym!(
                self,
                "jumpnode",
                unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
            )(a, b, c, d)
        }
    }
}

/// Both harnesses hold process-wide mutable state (`node_storage` /
/// `node_count`). `dlopen` on the same path returns a single refcounted handle,
/// so every test in this binary shares that state; the pair is loaded once and
/// handed out under a mutex so parallel test threads cannot interleave
/// mutations of it.
static HARNESSES: std::sync::OnceLock<std::sync::Mutex<(Harness, Harness)>> =
    std::sync::OnceLock::new();

fn harnesses() -> std::sync::MutexGuard<'static, (Harness, Harness)> {
    HARNESSES
        .get_or_init(|| {
            let c = Harness::open(&build_c_harness(), "C harness");
            let r = Harness::open(&build_rust_harness(), "Rust harness");
            std::sync::Mutex::new((c, r))
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Level 0: compile-time constants and struct layout
// ---------------------------------------------------------------------------

#[test]
fn constants_and_layout_match() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    assert_eq!(c.sizeof_node(), r.sizeof_node(), "sizeof(Node)");
    for name in [
        "h_status_ok",
        "h_status_warning",
        "h_status_error",
        "h_status_critical",
        "h_max_nodes",
    ] {
        assert_eq!(c.constant(name), r.constant(name), "{name}");
    }
}

// ---------------------------------------------------------------------------
// Level 1: leaf helpers
// ---------------------------------------------------------------------------

#[test]
fn safe_double_to_int_matches() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    let mut probes: Vec<f64> = vec![
        0.0,
        -0.0,
        0.5,
        -0.5,
        0.9999999,
        -0.9999999,
        1.0,
        -1.0,
        1.5,
        -1.5,
        2.5,
        -2.5,
        100.5,
        50.25,
        75.75,
        25.125,
        30.875,
        40.0625,
        12.5,
        2147483646.0,
        2147483646.5,
        2147483647.0,
        2147483647.5,
        2147483648.0,
        4e9,
        1e18,
        -2147483647.0,
        -2147483647.5,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
        -4e9,
        -1e18,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        // NaN passes both clamp comparisons untouched and reaches C's
        // `(int)value` cast. Quiet, negative, and signalling payloads.
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff0_0000_0000_0001),
        f64::from_bits(0xfff0_0000_0000_0001),
        f64::from_bits(0x7ff8_dead_beef_0000),
    ];
    // Dense sweep near the clamping boundaries and around zero.
    for i in -2000i64..=2000 {
        probes.push(i as f64);
        probes.push(i as f64 + 0.5);
        probes.push(2147483647.0 + i as f64 / 8.0);
        probes.push(-2147483648.0 + i as f64 / 8.0);
    }

    for &v in &probes {
        let cv = c.safe_double_to_int(v);
        let rv = r.safe_double_to_int(v);
        assert_eq!(cv, rv, "safe_double_to_int({v:?}) C={cv} Rust={rv}");
    }
}

#[test]
fn compute_size_metric_matches() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    let mut cases: Vec<Vec<u8>> = Vec::new();
    // Every length from 0 to 300.
    for len in 0..=300usize {
        let mut s = vec![b'x'; len];
        s.push(0);
        cases.push(s);
    }
    // Embedded NUL: strlen must stop early.
    cases.push(b"abc\0def\0".to_vec());
    cases.push(b"\0ignored\0".to_vec());
    // High bytes, to confirm `char` signedness does not affect the scan.
    cases.push(vec![0xff, 0x80, 0x7f, 0x01, 0]);
    cases.push(b"Node_1_Depth_2\0".to_vec());
    cases.push(b"Node_-2147483648_Depth_-2147483648\0".to_vec());
    // A length long enough that metric*2 overflows int, exercising the
    // wrapping arithmetic. 2^30 bytes is too large to allocate here, so use
    // the largest practical case plus targeted lengths.
    for len in [1000usize, 4096, 65535, 65536] {
        let mut s = vec![b'a'; len];
        s.push(0);
        cases.push(s);
    }

    for case in &cases {
        let cv = c.compute_size_metric(case);
        let rv = r.compute_size_metric(case);
        assert_eq!(
            cv,
            rv,
            "compute_size_metric(len={}) C={cv} Rust={rv}",
            case.len() - 1
        );
    }
}

#[test]
fn process_backward_matches() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    // The C function does no bounds checking on `start_offset`, so the buffer
    // is padded on both sides and the pointer handed in points into the middle.
    // Every offset exercised below therefore stays inside one allocation, which
    // keeps the comparison well-defined and identical for both libraries.
    const PAD: usize = 64;
    const LEN: usize = 20;

    for pattern in 0..6 {
        let mut buf = vec![0i32; PAD + LEN + PAD];
        for (i, slot) in buf.iter_mut().enumerate() {
            *slot = match pattern {
                0 => i as i32,
                1 => -(i as i32),
                2 => (i as i32) * 7,
                3 => i32::MAX - i as i32,
                4 => i32::MIN + i as i32,
                _ => ((i as i32) << 24) ^ 0x5a5a_5a5a,
            };
        }

        for size in 0..=LEN {
            for start_offset in -(PAD as i32)..=((LEN + PAD) as i32) {
                let mut cb = buf.clone();
                let mut rb = buf.clone();
                let cv = c.process_backward(&mut cb[PAD..], size, start_offset);
                let rv = r.process_backward(&mut rb[PAD..], size, start_offset);
                assert_eq!(
                    cv, rv,
                    "process_backward(pattern={pattern}, size={size}, start={start_offset}) \
                     C={cv} Rust={rv}"
                );
                assert_eq!(cb, rb, "process_backward must not mutate the buffer");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: node storage
// ---------------------------------------------------------------------------

#[test]
fn add_node_and_find_node_by_id_match() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    c.reset();
    r.reset();
    assert_eq!(c.node_count(), 0);
    assert_eq!(r.node_count(), 0);

    // Fill past MAX_NODES to exercise the STATUS_ERROR path.
    let max = c.constant("h_max_nodes");
    for i in 0..(max + 5) {
        let id = if i % 7 == 3 { i / 2 } else { i }; // duplicate ids on purpose
        let parent = if i == 0 { -1 } else { i - 1 };
        let value = (i as f64) * 1.25 - 3.5;

        let cv = c.add_node(id, parent, value);
        let rv = r.add_node(id, parent, value);
        assert_eq!(cv, rv, "add_node({id}, {parent}, {value}) at i={i}");
        assert_eq!(c.node_count(), r.node_count(), "node_count after i={i}");
    }

    // Storage contents, field-by-field and byte-for-byte (padding included).
    for idx in 0..max {
        assert_eq!(c.get_node(idx), r.get_node(idx), "node[{idx}] fields");
        assert_eq!(c.node_bytes(idx), r.node_bytes(idx), "node[{idx}] bytes");
    }
    assert_eq!(c.get_node(-1), r.get_node(-1));
    assert_eq!(c.get_node(max), r.get_node(max));

    // Lookups, including ids that are absent and duplicated.
    for id in -20..(max + 20) {
        assert_eq!(c.find_node_by_id(id), r.find_node_by_id(id), "find({id})");
    }

    // With node_count forced to 0, every lookup must miss.
    c.set_node_count(0);
    r.set_node_count(0);
    for id in -5..10 {
        assert_eq!(c.find_node_by_id(id), r.find_node_by_id(id), "empty find({id})");
    }

    // Negative node_count: the C loop condition `i < node_count` is false
    // immediately, so this is well-defined and must also miss.
    c.set_node_count(-3);
    r.set_node_count(-3);
    for id in -5..10 {
        assert_eq!(
            c.find_node_by_id(id),
            r.find_node_by_id(id),
            "negative-count find({id})"
        );
    }
}

#[test]
fn initialize_test_data_matches() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    c.initialize_test_data();
    r.initialize_test_data();

    assert_eq!(c.node_count(), r.node_count(), "node_count");
    assert_eq!(c.node_count(), 7, "sanity: seven nodes are added");

    for idx in 0..c.constant("h_max_nodes") {
        assert_eq!(c.get_node(idx), r.get_node(idx), "node[{idx}] fields");
        assert_eq!(c.node_bytes(idx), r.node_bytes(idx), "node[{idx}] bytes");
    }

    // Idempotent: calling it twice must reset rather than append.
    c.initialize_test_data();
    r.initialize_test_data();
    assert_eq!(c.node_count(), r.node_count());
    assert_eq!(c.node_count(), 7);
}

// ---------------------------------------------------------------------------
// Level 3: jumpnode with populated storage (unreachable in the shipped .so)
// ---------------------------------------------------------------------------

#[test]
fn jumpnode_matches_with_populated_storage() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    c.initialize_test_data();
    r.initialize_test_data();

    let node_ids: Vec<c_int> = (-2..12).collect();
    // `depth` is kept >= 0 for mode 0002: a negative `start_offset` makes the C
    // `process_backward` read below `temp_array`, i.e. off the end of a local
    // object, which has no defined value to compare against. Every other mode
    // is swept with negative depths too.
    let depths_any: Vec<c_int> = vec![
        i32::MIN,
        -1000,
        -20,
        -16,
        -5,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        15,
        16,
        17,
        20,
        21,
        100,
        1000,
        i32::MAX,
    ];
    let depths_nonneg: Vec<c_int> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 25, 100,
        1000, i32::MAX,
    ];
    let flags: Vec<c_int> = vec![0, 1, -1, 2, 7, 0o177, 0o200, 0o377, 12345, -12345, i32::MIN, i32::MAX];

    for mode in [0o0, 0o1, 0o2, 0o3, 0o4, 0o5, 0o200, -1, i32::MIN, i32::MAX] {
        let depths: &Vec<c_int> = if mode == 0o2 { &depths_nonneg } else { &depths_any };
        for &node_id in &node_ids {
            for &depth in depths {
                for &f in &flags {
                    let cv = c.jumpnode(mode, node_id, depth, f);
                    let rv = r.jumpnode(mode, node_id, depth, f);
                    assert_eq!(
                        cv, rv,
                        "jumpnode({mode}, {node_id}, {depth}, {f}) C={cv} Rust={rv}"
                    );
                }
            }
        }
    }
}

/// Mode 0004 sums the last three nodes backwards and is gated on
/// `node_count > 2`; mode 0001 walks parents up to `depth`. Sweep the node
/// count across that boundary and across custom parent topologies.
#[test]
fn jumpnode_matches_across_node_counts_and_topologies() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    for count in 0..=8 {
        c.reset();
        r.reset();
        for i in 0..count {
            // Mix of self-parent, root, forward and backward references, so the
            // parent walk in mode 0001 sees cycles, dead ends, and roots.
            let parent = match i % 4 {
                0 => -1,
                1 => i,     // self-cycle
                2 => i - 1, // normal
                _ => 999,   // dangling
            };
            let id = i + 1;
            let value = (i as f64) * 13.375 - 7.25;
            assert_eq!(c.add_node(id, parent, value), r.add_node(id, parent, value));
        }
        assert_eq!(c.node_count(), r.node_count(), "count={count}");

        for mode in [0o1, 0o2, 0o3, 0o4] {
            for node_id in -1..=(count + 2) {
                for depth in [0, 1, 2, 3, 4, 8, 16, 17, 50] {
                    for f in [0, 1, -1, 0o177, 255] {
                        let cv = c.jumpnode(mode, node_id, depth, f);
                        let rv = r.jumpnode(mode, node_id, depth, f);
                        assert_eq!(
                            cv, rv,
                            "count={count} jumpnode({mode}, {node_id}, {depth}, {f}) \
                             C={cv} Rust={rv}"
                        );
                    }
                }
            }
        }
    }
}

/// Mode 0004 takes `sqrt` of each `data[i]` and mode 0001 accumulates
/// `value * 1.5`; feed values that stress rounding, clamping, and the
/// infinity/NaN paths. A mixed-sign infinity chain makes mode 0001 evaluate
/// `inf + (-inf) * 1.5`, producing a NaN that flows into
/// `safe_double_to_int` — the one place where C's `(int)` cast and Rust's `as`
/// disagree unless handled explicitly.
#[test]
fn jumpnode_floating_point_paths_match() {
    let guard = harnesses();
    let (c, r) = (&guard.0, &guard.1);

    let values: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.1,
        1e-300,
        1e300,
        -1e300,
        1.5,
        2.5,
        100.5,
        1431655764.0,
        1431655765.0,
        2147483646.9,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];

    for &v in &values {
        c.reset();
        r.reset();
        // A three-node chain so mode 0001 walks parents and mode 0004's
        // `node_count > 2` branch is taken.
        for (id, parent) in [(1, -1), (2, 1), (3, 2)] {
            assert_eq!(c.add_node(id, parent, v), r.add_node(id, parent, v));
        }

        for mode in [0o1, 0o4] {
            for node_id in 1..=3 {
                for depth in [0, 1, 2, 3, 10, -1, i32::MAX, i32::MIN] {
                    let cv = c.jumpnode(mode, node_id, depth, 0);
                    let rv = r.jumpnode(mode, node_id, depth, 0);
                    assert_eq!(
                        cv, rv,
                        "value={v:?} jumpnode({mode}, {node_id}, {depth}, 0) C={cv} Rust={rv}"
                    );
                }
            }
        }
    }

    // Mixed-sign values, including chains that generate NaN mid-accumulation.
    let chains: [[f64; 3]; 7] = [
        [f64::INFINITY, f64::NEG_INFINITY, 1.0],
        [f64::NEG_INFINITY, f64::INFINITY, 1.0],
        [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY],
        [f64::NAN, 1.0, 2.0],
        [1.0, f64::NAN, 2.0],
        [f64::MAX, f64::MAX, f64::MAX],
        [-f64::MAX, -f64::MAX, -f64::MAX],
    ];
    for chain in chains {
        c.reset();
        r.reset();
        for (i, &v) in chain.iter().enumerate() {
            let id = i as c_int + 1;
            let parent = if i == 0 { -1 } else { i as c_int };
            assert_eq!(c.add_node(id, parent, v), r.add_node(id, parent, v));
        }
        for mode in [0o1, 0o4] {
            for node_id in 1..=3 {
                for depth in [0, 1, 2, 3, 4, 10] {
                    let cv = c.jumpnode(mode, node_id, depth, 0);
                    let rv = r.jumpnode(mode, node_id, depth, 0);
                    assert_eq!(
                        cv, rv,
                        "chain={chain:?} jumpnode({mode}, {node_id}, {depth}, 0) C={cv} Rust={rv}"
                    );
                }
            }
        }
    }
}
