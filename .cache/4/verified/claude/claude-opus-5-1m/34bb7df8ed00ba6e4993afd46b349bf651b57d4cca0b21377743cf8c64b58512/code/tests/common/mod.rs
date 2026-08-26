//! Shared plumbing for the differential tests.
//!
//! Both the original C library and the Rust translation are loaded as shared
//! objects with `libloading` and driven exclusively through their exported C
//! symbols — the Rust implementation is never called directly, so the
//! `#[no_mangle]` wrappers are part of what is under test.
//!
//! The public structs of `hashmap.h` / `tree.h` are re-declared here (as an
//! external consumer would) which lets the tests compare not just return values
//! but the complete internal state of both libraries, slot by slot.

#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// C ABI mirrors of c_src/include/{hashmap,tree}.h
// ---------------------------------------------------------------------------

pub const HASHMAP_INITIAL_CAPACITY: usize = 16;
pub const MAX_CHILDREN: usize = 32;
pub const MAX_DATA_LENGTH: usize = 256;

pub type TreeId = u64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HashmapEntry {
    pub key: TreeId,
    pub value: *mut c_void,
    pub occupied: c_int,
    pub deleted: c_int,
}

#[repr(C)]
pub struct Hashmap {
    pub entries: *mut HashmapEntry,
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
}

#[repr(C)]
pub struct TreeNode {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_ids: [TreeId; MAX_CHILDREN],
    pub child_count: c_int,
    pub data: [u8; MAX_DATA_LENGTH],
}

#[repr(C)]
pub struct Tree {
    pub node_map: *mut Hashmap,
    pub root_id: TreeId,
    pub has_root: c_int,
    pub node_count: usize,
}

const _: () = assert!(std::mem::size_of::<HashmapEntry>() == 24);
const _: () = assert!(std::mem::size_of::<Hashmap>() == 32);
const _: () = assert!(std::mem::size_of::<TreeNode>() == 536);
const _: () = assert!(std::mem::size_of::<Tree>() == 32);

// ---------------------------------------------------------------------------
// Loaded library: one function pointer per exported symbol
// ---------------------------------------------------------------------------

pub type FnVoidVoid = unsafe extern "C" fn();

pub struct Api {
    pub name: &'static str,
    pub hashmap_create: unsafe extern "C" fn() -> *mut Hashmap,
    pub hashmap_destroy: unsafe extern "C" fn(*mut Hashmap),
    pub hashmap_put: unsafe extern "C" fn(*mut Hashmap, TreeId, *mut c_void) -> c_int,
    pub hashmap_get: unsafe extern "C" fn(*mut Hashmap, TreeId) -> *mut c_void,
    pub hashmap_remove: unsafe extern "C" fn(*mut Hashmap, TreeId) -> *mut c_void,
    pub hashmap_contains: unsafe extern "C" fn(*mut Hashmap, TreeId) -> c_int,
    pub hashmap_size: unsafe extern "C" fn(*mut Hashmap) -> usize,
    pub hashmap_clear: unsafe extern "C" fn(*mut Hashmap),

    pub tree_create: unsafe extern "C" fn() -> *mut Tree,
    pub tree_delete: unsafe extern "C" fn(*mut Tree),
    pub tree_add_node: unsafe extern "C" fn(*mut Tree, TreeId, TreeId, *const u8) -> c_int,
    pub tree_remove_node: unsafe extern "C" fn(*mut Tree, TreeId) -> c_int,
    pub tree_get_node: unsafe extern "C" fn(*mut Tree, TreeId) -> *mut TreeNode,
    pub tree_contains: unsafe extern "C" fn(*mut Tree, TreeId) -> c_int,
    pub tree_size: unsafe extern "C" fn(*mut Tree) -> usize,
    pub tree_print: unsafe extern "C" fn(*mut Tree),
    pub tree_get_depth: unsafe extern "C" fn(*mut Tree, TreeId) -> c_int,
    pub tree_get_height: unsafe extern "C" fn(*mut Tree, TreeId) -> c_int,
    pub tree_count_descendants: unsafe extern "C" fn(*mut Tree, TreeId) -> c_int,
    pub tree_find_path: unsafe extern "C" fn(*mut Tree, TreeId, *mut TreeId, c_int) -> c_int,

    pub main: unsafe extern "C" fn() -> c_int,
    lib: &'static Library,
}

/// Names of the 14 non-`static` test functions of `c_src/src/main.c`.
pub const TEST_FUNCS: [&str; 14] = [
    "test_hashmap_basic",
    "test_hashmap_collisions",
    "test_tree_creation",
    "test_tree_add_root",
    "test_tree_add_children",
    "test_tree_deep_hierarchy",
    "test_tree_remove_leaf",
    "test_tree_remove_subtree",
    "test_tree_remove_root",
    "test_tree_count_descendants",
    "test_tree_find_path",
    "test_tree_duplicate_id",
    "test_tree_max_children",
    "test_tree_complex_structure",
];

impl Api {
    pub fn load(path: &Path, name: &'static str) -> Api {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e))
        }));
        unsafe fn sym<T: Copy>(lib: &'static Library, n: &str) -> T {
            let s: Symbol<T> = lib
                .get(format!("{}\0", n).as_bytes())
                .unwrap_or_else(|e| panic!("dlsym {}: {}", n, e));
            *s
        }
        unsafe {
            Api {
                name,
                hashmap_create: sym(lib, "hashmap_create"),
                hashmap_destroy: sym(lib, "hashmap_destroy"),
                hashmap_put: sym(lib, "hashmap_put"),
                hashmap_get: sym(lib, "hashmap_get"),
                hashmap_remove: sym(lib, "hashmap_remove"),
                hashmap_contains: sym(lib, "hashmap_contains"),
                hashmap_size: sym(lib, "hashmap_size"),
                hashmap_clear: sym(lib, "hashmap_clear"),
                tree_create: sym(lib, "tree_create"),
                tree_delete: sym(lib, "tree_delete"),
                tree_add_node: sym(lib, "tree_add_node"),
                tree_remove_node: sym(lib, "tree_remove_node"),
                tree_get_node: sym(lib, "tree_get_node"),
                tree_contains: sym(lib, "tree_contains"),
                tree_size: sym(lib, "tree_size"),
                tree_print: sym(lib, "tree_print"),
                tree_get_depth: sym(lib, "tree_get_depth"),
                tree_get_height: sym(lib, "tree_get_height"),
                tree_count_descendants: sym(lib, "tree_count_descendants"),
                tree_find_path: sym(lib, "tree_find_path"),
                main: sym(lib, "main"),
                lib,
            }
        }
    }

    /// Look up one of the exported `test_*` wrappers of `main.c`.
    pub fn test_fn(&self, n: &str) -> FnVoidVoid {
        unsafe {
            let s: Symbol<FnVoidVoid> = self
                .lib
                .get(format!("{}\0", n).as_bytes())
                .unwrap_or_else(|e| panic!("dlsym {}: {}", n, e));
            *s
        }
    }
}

/// The two implementations under comparison.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` of the currently running test binary.
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/<test binary>
    exe.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Path of the C shared object, building it with `gcc` if it is not there yet.
pub fn c_so_path() -> PathBuf {
    let root = manifest_dir();
    let out = root.join("cbuild").join("libdriver_c.so");
    let srcs = [
        root.join("c_src/src/hashmap.c"),
        root.join("c_src/src/tree.c"),
        root.join("c_src/src/main.c"),
    ];
    let newest_src = srcs
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok()?.modified().ok())
        .max();
    let need = match (std::fs::metadata(&out).ok().and_then(|m| m.modified().ok()), newest_src) {
        (Some(o), Some(s)) => s > o,
        _ => true,
    };
    if need {
        std::fs::create_dir_all(root.join("cbuild")).unwrap();
        let st = Command::new("gcc")
            .arg("-shared")
            .arg("-fPIC")
            .arg("-O0")
            .arg("-g")
            .arg(format!("-I{}", root.join("c_src/include").display()))
            .arg("-o")
            .arg(&out)
            .args(&srcs)
            .status()
            .expect("run gcc");
        assert!(st.success(), "building {:?} failed", out);
    }
    out
}

/// Path of the Rust `cdylib`, rebuilt when it is older than the sources.
///
/// `cargo test` does *not* necessarily re-emit a `cdylib` that no test links
/// against, so a stale `libdriver.so` would silently make every differential
/// test compare the C library against an old translation.  The freshness check
/// (and, if needed, a nested `cargo build`) removes that failure mode.
pub fn rust_so_path() -> PathBuf {
    let profile_dir = target_profile_dir();
    let so = profile_dir.join("libdriver.so");
    let root = manifest_dir();

    let newest_src = || -> Option<std::time::SystemTime> {
        let mut newest: Option<std::time::SystemTime> = None;
        let mut files: Vec<PathBuf> = vec![root.join("Cargo.toml")];
        if let Ok(rd) = std::fs::read_dir(root.join("src")) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    files.push(p);
                }
            }
        }
        for f in files {
            if let Ok(t) = std::fs::metadata(&f).and_then(|m| m.modified()) {
                newest = Some(match newest {
                    Some(n) if n >= t => n,
                    _ => t,
                });
            }
        }
        newest
    };

    let stale = || -> bool {
        match (
            std::fs::metadata(&so).and_then(|m| m.modified()).ok(),
            newest_src(),
        ) {
            (Some(o), Some(s)) => s > o,
            (None, _) => true,
            _ => false,
        }
    };

    if stale() {
        let release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
        let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.current_dir(&root).arg("build").arg("--offline");
        if release {
            cmd.arg("--release");
        }
        let st = cmd.status();
        if !matches!(st, Ok(ref s) if s.success()) {
            // retry without --offline in case the registry cache is cold
            let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
            cmd.current_dir(&root).arg("build");
            if release {
                cmd.arg("--release");
            }
            let _ = cmd.status();
        }
    }

    assert!(so.exists(), "{:?} not found - run `cargo build`", so);
    assert!(
        !stale(),
        "{:?} is older than src/*.rs - run `cargo build` before `cargo test`",
        so
    );
    so
}

pub fn load_pair() -> Pair {
    Pair {
        c: Api::load(&c_so_path(), "C"),
        r: Api::load(&rust_so_path(), "Rust"),
    }
}

// ---------------------------------------------------------------------------
// State snapshots
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SlotSnap {
    pub slot: usize,
    pub key: TreeId,
    pub occupied: c_int,
    pub deleted: c_int,
    /// Comparable stand-in for the raw `void *` (see the two `snap_map*` fns).
    pub value: u64,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MapSnap {
    pub capacity: usize,
    pub size: usize,
    pub deleted_count: usize,
    pub slots: Vec<SlotSnap>,
}

/// Snapshot of a `hashmap_t` whose values are opaque tokens created by the test
/// (identical in both libraries), so the raw pointer bits are compared too.
pub unsafe fn snap_map_raw(m: *mut Hashmap) -> Option<MapSnap> {
    if m.is_null() {
        return None;
    }
    let mut slots = Vec::new();
    for i in 0..(*m).capacity {
        let e = (*m).entries.add(i);
        slots.push(SlotSnap {
            slot: i,
            key: (*e).key,
            occupied: (*e).occupied,
            deleted: (*e).deleted,
            value: (*e).value as u64,
        });
    }
    Some(MapSnap {
        capacity: (*m).capacity,
        size: (*m).size,
        deleted_count: (*m).deleted_count,
        slots,
    })
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct NodeSnap {
    pub id: TreeId,
    pub parent_id: TreeId,
    pub child_count: c_int,
    /// Only `child_ids[0..child_count]`: the rest is left uninitialised by the
    /// C `malloc`, in both implementations.
    pub child_ids: Vec<TreeId>,
    /// The NUL-terminated contents of `data` — the only part `tree_add_node`
    /// guarantees to have written when `data == NULL`.
    pub data: Vec<u8>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TreeSlotSnap {
    pub slot: usize,
    pub key: TreeId,
    pub occupied: c_int,
    pub deleted: c_int,
    pub value_null: bool,
    /// Present for live slots only (a tombstoned slot points at freed memory).
    pub node: Option<NodeSnap>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct TreeSnap {
    pub root_id: TreeId,
    pub has_root: c_int,
    pub node_count: usize,
    pub capacity: usize,
    pub map_size: usize,
    pub deleted_count: usize,
    pub slots: Vec<TreeSlotSnap>,
}

pub unsafe fn snap_node(n: *mut TreeNode) -> NodeSnap {
    let cc = (*n).child_count;
    let mut child_ids = Vec::new();
    if cc > 0 {
        for i in 0..(cc as usize).min(MAX_CHILDREN) {
            child_ids.push((*n).child_ids[i]);
        }
    }
    NodeSnap {
        id: (*n).id,
        parent_id: (*n).parent_id,
        child_count: cc,
        child_ids,
        data: cstr_bytes(&(*n).data),
    }
}

pub unsafe fn snap_tree(t: *mut Tree) -> Option<TreeSnap> {
    if t.is_null() {
        return None;
    }
    let m = (*t).node_map;
    let mut slots = Vec::new();
    for i in 0..(*m).capacity {
        let e = (*m).entries.add(i);
        let live = (*e).occupied != 0 && (*e).deleted == 0;
        slots.push(TreeSlotSnap {
            slot: i,
            key: (*e).key,
            occupied: (*e).occupied,
            deleted: (*e).deleted,
            value_null: (*e).value.is_null(),
            node: if live && !(*e).value.is_null() {
                Some(snap_node((*e).value as *mut TreeNode))
            } else {
                None
            },
        });
    }
    Some(TreeSnap {
        root_id: (*t).root_id,
        has_root: (*t).has_root,
        node_count: (*t).node_count,
        capacity: (*m).capacity,
        map_size: (*m).size,
        deleted_count: (*m).deleted_count,
        slots,
    })
}

pub fn cstr_bytes(buf: &[u8]) -> Vec<u8> {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    buf[..end].to_vec()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn usize_below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A key drawn from a mix of tiny, huge and random values.
    pub fn key(&mut self) -> TreeId {
        match self.below(6) {
            0 => 0,
            1 => 1,
            2 => u64::MAX,
            3 => u64::MAX - 1,
            4 => 1u64 << 63,
            _ => self.next_u64(),
        }
    }
}

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

// ---------------------------------------------------------------------------
// Tiny harness (the test binaries run with `harness = false` so that stdout /
// stderr redirection and the shared library state stay single-threaded)
// ---------------------------------------------------------------------------

pub struct Row {
    pub name: String,
    pub errs: Vec<String>,
    pub checks: usize,
}

impl Row {
    pub fn eq<T: PartialEq + std::fmt::Debug>(&mut self, what: &str, c: T, r: T) {
        self.checks += 1;
        if c != r {
            if self.errs.len() < 6 {
                self.errs
                    .push(format!("{}\n        C   = {:?}\n        Rust= {:?}", what, c, r));
            } else if self.errs.len() == 6 {
                self.errs.push("... (more divergences suppressed)".into());
            }
        }
    }
    pub fn ok(&mut self, what: &str, cond: bool) {
        self.checks += 1;
        if !cond {
            if self.errs.len() < 6 {
                self.errs.push(format!("{} (condition failed)", what));
            }
        }
    }
}

pub struct Harness {
    pub failed: Vec<String>,
    pub passed: usize,
    pub checks: usize,
    pub title: &'static str,
}

impl Harness {
    pub fn new(title: &'static str) -> Harness {
        println!("=== {} ===", title);
        Harness {
            failed: Vec::new(),
            passed: 0,
            checks: 0,
            title,
        }
    }

    pub fn row(&mut self, name: &str, f: impl FnOnce(&mut Row)) {
        let mut row = Row {
            name: name.to_string(),
            errs: Vec::new(),
            checks: 0,
        };
        f(&mut row);
        self.checks += row.checks;
        if row.errs.is_empty() {
            self.passed += 1;
            println!("  [PASS] {:<10} ({} checks)", name, row.checks);
        } else {
            self.failed.push(name.to_string());
            println!("  [FAIL] {:<10} ({} checks)", name, row.checks);
            for e in &row.errs {
                println!("      - {}", e);
            }
        }
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    pub fn finish(self) {
        println!(
            "--- {}: {} rows passed, {} rows failed, {} individual comparisons ---",
            self.title,
            self.passed,
            self.failed.len(),
            self.checks
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
        if !self.failed.is_empty() {
            eprintln!("FAILED rows: {:?}", self.failed);
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

pub struct Capture {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
    /// `Some(code)` for a normal exit, `None` when the child was killed.
    pub exit: Option<c_int>,
    pub signal: Option<c_int>,
}

fn tmp_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("cdiff_{}_{}", std::process::id(), tag));
    p
}

unsafe fn open_trunc(p: &Path) -> c_int {
    let s = format!("{}\0", p.display());
    let fd = libc::open(
        s.as_ptr() as *const i8,
        libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
        0o600,
    );
    assert!(fd >= 0, "open {:?}", p);
    fd
}

/// Run `f` in this process with `stdout`/`stderr` redirected into temporary
/// files; side effects on the libraries' state are preserved.
pub unsafe fn capture_inproc(f: impl FnOnce()) -> (Vec<u8>, Vec<u8>) {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    libc::fflush(std::ptr::null_mut());

    let op = tmp_path("out");
    let ep = tmp_path("err");
    let ofd = open_trunc(&op);
    let efd = open_trunc(&ep);
    let saved_out = libc::dup(1);
    let saved_err = libc::dup(2);
    assert!(saved_out >= 0 && saved_err >= 0);
    libc::dup2(ofd, 1);
    libc::dup2(efd, 2);

    f();

    libc::fflush(std::ptr::null_mut());
    libc::dup2(saved_out, 1);
    libc::dup2(saved_err, 2);
    libc::close(saved_out);
    libc::close(saved_err);
    libc::close(ofd);
    libc::close(efd);

    let out = std::fs::read(&op).unwrap_or_default();
    let err = std::fs::read(&ep).unwrap_or_default();
    let _ = std::fs::remove_file(&op);
    let _ = std::fs::remove_file(&ep);
    (out, err)
}

/// Run `f` in a forked child with `stdout`/`stderr` captured, so that an
/// `assert()`/`abort()` inside the library is observable instead of fatal.
pub unsafe fn capture_fork(f: impl FnOnce()) -> Capture {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    libc::fflush(std::ptr::null_mut());

    let op = tmp_path("fout");
    let ep = tmp_path("ferr");
    let ofd = open_trunc(&op);
    let efd = open_trunc(&ep);

    let pid = libc::fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        libc::dup2(ofd, 1);
        libc::dup2(efd, 2);
        f();
        libc::fflush(std::ptr::null_mut());
        libc::_exit(0);
    }

    let mut status: c_int = 0;
    libc::waitpid(pid, &mut status, 0);
    libc::close(ofd);
    libc::close(efd);

    let out = std::fs::read(&op).unwrap_or_default();
    let err = std::fs::read(&ep).unwrap_or_default();
    let _ = std::fs::remove_file(&op);
    let _ = std::fs::remove_file(&ep);

    let exited = libc::WIFEXITED(status);
    Capture {
        out,
        err,
        exit: if exited {
            Some(libc::WEXITSTATUS(status))
        } else {
            None
        },
        signal: if libc::WIFSIGNALED(status) {
            Some(libc::WTERMSIG(status))
        } else {
            None
        },
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A NUL-terminated buffer usable as `const char *`.
pub fn cstring(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

/// An opaque, non-dereferenced `void *` token, identical for both libraries.
pub fn token(i: u64) -> *mut c_void {
    (0x1000 + i * 8) as *mut c_void
}

// ---------------------------------------------------------------------------
// Structured comparisons (element-wise, so a divergence report stays readable)
// ---------------------------------------------------------------------------

impl Row {
    /// Compare two operation logs entry by entry.
    pub fn eq_logs(&mut self, what: &str, c: &[String], r: &[String]) {
        self.checks += 1;
        if c.len() != r.len() {
            self.errs.push(format!(
                "{}: log length C={} Rust={}",
                what,
                c.len(),
                r.len()
            ));
        }
        let mut shown = 0;
        for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
            self.checks += 1;
            if a != b {
                if shown < 6 {
                    self.errs
                        .push(format!("{}[{}]: C={:?} Rust={:?}", what, i, a, b));
                    shown += 1;
                } else if shown == 6 {
                    self.errs.push(format!("{}: ... more divergences", what));
                    shown += 1;
                }
            }
        }
    }

    pub fn eq_map(&mut self, what: &str, c: &Option<MapSnap>, r: &Option<MapSnap>) {
        match (c, r) {
            (None, None) => {
                self.checks += 1;
            }
            (Some(a), Some(b)) => {
                self.eq(&format!("{}.capacity", what), a.capacity, b.capacity);
                self.eq(&format!("{}.size", what), a.size, b.size);
                self.eq(
                    &format!("{}.deleted_count", what),
                    a.deleted_count,
                    b.deleted_count,
                );
                self.eq(&format!("{}.slots.len", what), a.slots.len(), b.slots.len());
                let mut shown = 0;
                for (x, y) in a.slots.iter().zip(b.slots.iter()) {
                    self.checks += 1;
                    if x != y && shown < 6 {
                        self.errs
                            .push(format!("{}.slot[{}]: C={:?} Rust={:?}", what, x.slot, x, y));
                        shown += 1;
                    }
                }
            }
            _ => {
                self.checks += 1;
                self.errs.push(format!(
                    "{}: one side is NULL (C={:?} Rust={:?})",
                    what,
                    c.is_some(),
                    r.is_some()
                ));
            }
        }
    }

    pub fn eq_tree(&mut self, what: &str, c: &Option<TreeSnap>, r: &Option<TreeSnap>) {
        match (c, r) {
            (None, None) => {
                self.checks += 1;
            }
            (Some(a), Some(b)) => {
                self.eq(&format!("{}.root_id", what), a.root_id, b.root_id);
                self.eq(&format!("{}.has_root", what), a.has_root, b.has_root);
                self.eq(&format!("{}.node_count", what), a.node_count, b.node_count);
                self.eq(&format!("{}.map.capacity", what), a.capacity, b.capacity);
                self.eq(&format!("{}.map.size", what), a.map_size, b.map_size);
                self.eq(
                    &format!("{}.map.deleted_count", what),
                    a.deleted_count,
                    b.deleted_count,
                );
                self.eq(&format!("{}.slots.len", what), a.slots.len(), b.slots.len());
                let mut shown = 0;
                for (x, y) in a.slots.iter().zip(b.slots.iter()) {
                    self.checks += 1;
                    if x != y && shown < 6 {
                        self.errs
                            .push(format!("{}.slot[{}]:\n        C   ={:?}\n        Rust={:?}", what, x.slot, x, y));
                        shown += 1;
                    }
                }
            }
            _ => {
                self.checks += 1;
                self.errs.push(format!(
                    "{}: one side is NULL (C={:?} Rust={:?})",
                    what,
                    c.is_some(),
                    r.is_some()
                ));
            }
        }
    }

    /// Compare captured output byte-for-byte.
    pub fn eq_bytes(&mut self, what: &str, c: &[u8], r: &[u8]) {
        self.checks += 1;
        if c != r {
            let pos = c.iter().zip(r.iter()).position(|(a, b)| a != b);
            self.errs.push(format!(
                "{}: {} bytes vs {} bytes, first difference at {:?}\n        C   ={:?}\n        Rust={:?}",
                what,
                c.len(),
                r.len(),
                pos,
                String::from_utf8_lossy(&c[..c.len().min(400)]),
                String::from_utf8_lossy(&r[..r.len().min(400)]),
            ));
        }
    }
}

/// Run the same closure against both libraries.
pub unsafe fn both<T>(p: &Pair, f: impl Fn(&Api) -> T) -> (T, T) {
    let c = f(&p.c);
    let r = f(&p.r);
    (c, r)
}

/// Long-lived redirection of `stdout`/`stderr` into files, with incremental
/// reads — used by the fuzz rows, which need the output of every single call.
pub struct Redirect {
    saved_out: c_int,
    saved_err: c_int,
    ofd: c_int,
    efd: c_int,
    oread: std::fs::File,
    eread: std::fs::File,
    opath: PathBuf,
    epath: PathBuf,
}

impl Redirect {
    pub unsafe fn start(tag: &str) -> Redirect {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        libc::fflush(std::ptr::null_mut());
        let opath = tmp_path(&format!("r{}out", tag));
        let epath = tmp_path(&format!("r{}err", tag));
        let ofd = open_trunc(&opath);
        let efd = open_trunc(&epath);
        let oread = std::fs::File::open(&opath).unwrap();
        let eread = std::fs::File::open(&epath).unwrap();
        let saved_out = libc::dup(1);
        let saved_err = libc::dup(2);
        libc::dup2(ofd, 1);
        libc::dup2(efd, 2);
        Redirect {
            saved_out,
            saved_err,
            ofd,
            efd,
            oread,
            eread,
            opath,
            epath,
        }
    }

    /// Everything written since the previous `take()`.
    pub unsafe fn take(&mut self) -> (Vec<u8>, Vec<u8>) {
        use std::io::Read;
        libc::fflush(std::ptr::null_mut());
        let mut o = Vec::new();
        let mut e = Vec::new();
        let _ = self.oread.read_to_end(&mut o);
        let _ = self.eread.read_to_end(&mut e);
        (o, e)
    }

    pub unsafe fn stop(self) {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(self.saved_out, 1);
        libc::dup2(self.saved_err, 2);
        libc::close(self.saved_out);
        libc::close(self.saved_err);
        libc::close(self.ofd);
        libc::close(self.efd);
        let _ = std::fs::remove_file(&self.opath);
        let _ = std::fs::remove_file(&self.epath);
    }
}

/// Cheap order-sensitive digest of a map snapshot.
pub fn digest_map(s: &Option<MapSnap>) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut f = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x100000001b3);
    };
    match s {
        None => f(0xdead),
        Some(m) => {
            f(m.capacity as u64);
            f(m.size as u64);
            f(m.deleted_count as u64);
            for s in &m.slots {
                f(s.key);
                f(s.occupied as u64);
                f(s.deleted as u64);
                f(s.value);
            }
        }
    }
    h
}

/// Like [`capture_inproc`] but forwards the closure's return value.
pub unsafe fn capture_ret<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>, Vec<u8>) {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    libc::fflush(std::ptr::null_mut());

    let op = tmp_path("rout");
    let ep = tmp_path("rerr");
    let ofd = open_trunc(&op);
    let efd = open_trunc(&ep);
    let saved_out = libc::dup(1);
    let saved_err = libc::dup(2);
    libc::dup2(ofd, 1);
    libc::dup2(efd, 2);

    let v = f();

    libc::fflush(std::ptr::null_mut());
    libc::dup2(saved_out, 1);
    libc::dup2(saved_err, 2);
    libc::close(saved_out);
    libc::close(saved_err);
    libc::close(ofd);
    libc::close(efd);

    let out = std::fs::read(&op).unwrap_or_default();
    let err = std::fs::read(&ep).unwrap_or_default();
    let _ = std::fs::remove_file(&op);
    let _ = std::fs::remove_file(&ep);
    (v, out, err)
}

/// Build (once) the `LD_PRELOAD` shim that makes the n-th `malloc`/`calloc`
/// after `failalloc_arm(n)` return NULL, so the allocation-failure branches of
/// the C code become reachable.
pub fn failalloc_so_path() -> PathBuf {
    let dir = manifest_dir().join("cbuild");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("failalloc.c");
    let out = dir.join("libfailalloc.so");
    let code = br#"/* LD_PRELOAD allocation-failure injector (test scaffolding only). */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>

static long armed = -1;                 /* -1: disarmed, n>0: fail the n-th call */
static void *(*real_malloc)(size_t);
static void *(*real_calloc)(size_t, size_t);
static void *(*real_realloc)(void *, size_t);

__attribute__((constructor)) static void init(void) {
    real_malloc  = dlsym(RTLD_NEXT, "malloc");
    real_calloc  = dlsym(RTLD_NEXT, "calloc");
    real_realloc = dlsym(RTLD_NEXT, "realloc");
}

void failalloc_arm(long n)   { armed = n; }
void failalloc_disarm(void)  { armed = -1; }
long failalloc_state(void)   { return armed; }

static int consume(void) {
    if (armed > 0) {
        if (--armed == 0) { armed = -1; return 1; }
    }
    return 0;
}

void *malloc(size_t n) {
    if (!real_malloc) init();
    if (consume()) return NULL;
    return real_malloc(n);
}
void *calloc(size_t a, size_t b) {
    if (!real_calloc) init();
    if (consume()) return NULL;
    return real_calloc(a, b);
}
void *realloc(void *p, size_t n) {
    if (!real_realloc) init();
    return real_realloc(p, n);
}
"#;
    let need = match (std::fs::read(&src), std::fs::metadata(&out)) {
        (Ok(existing), Ok(_)) => existing != code.to_vec(),
        _ => true,
    };
    if need {
        std::fs::write(&src, code).unwrap();
        let st = Command::new("gcc")
            .args(["-shared", "-fPIC", "-O0", "-o"])
            .arg(&out)
            .arg(&src)
            .arg("-ldl")
            .status()
            .expect("gcc");
        assert!(st.success(), "building {:?} failed", out);
    }
    out
}
