//! Shared differential-testing harness.
//!
//! BOTH libraries are loaded as shared objects with `libloading` and every call
//! goes through `dlsym`. No Rust function is ever called directly, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const MAX_NODES: usize = 50;
pub const NODE_BYTES: usize = 52;
pub const TABLE_BYTES: usize = MAX_NODES * NODE_BYTES; // 2600

// ---------------------------------------------------------------------------
// C signatures
// ---------------------------------------------------------------------------

pub type Op4 = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
type FindFn = unsafe extern "C" fn(i32) -> *mut u8;
type AddFn = unsafe extern "C" fn(i32, i32, i32, *const c_char) -> i32;
type SumFn = unsafe extern "C" fn(i32) -> i32;
type ParseFn = unsafe extern "C" fn(*const c_char) -> i32;
type GetOpFn = unsafe extern "C" fn(i32) -> Op4;
type InrefFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

pub const OP_NAMES: [&str; 5] = [
    "add_op",
    "multiply_op",
    "subtract_op",
    "divide_op",
    "modulo_op",
];

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn find_so_in(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|e| e == "so").unwrap_or(false) && p.is_file() {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The C shared object produced by `c_src/CMakeLists.txt`. Its file name is
/// derived from the name of the directory that *contains* `c_src`, so it is
/// discovered by globbing rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("manifest has a parent");
    let mut candidates = find_so_in(&root.join("c_src").join("build"));
    if candidates.is_empty() {
        // some generators drop the artifact one level deeper
        for sub in ["Debug", "Release", "lib"] {
            candidates = find_so_in(&root.join("c_src").join("build").join(sub));
            if !candidates.is_empty() {
                break;
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "no C .so found under {:?}/c_src/build — build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        root
    );
    candidates.remove(0)
}

/// The Rust `cdylib`. Derived from the running test executable so that the
/// profile *and* the feature combination always match the harness.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // <target>/<profile>/deps/<test-bin>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe has target/<profile>/deps layout");
    let direct = profile_dir.join("libinreftree_lib.so");
    if direct.is_file() {
        return direct;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for prof in ["release", "debug"] {
        let p = manifest.join("target").join(prof).join("libinreftree_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libinreftree_lib.so not found (looked in {:?}); run `cargo build` first",
        profile_dir
    );
}

// ---------------------------------------------------------------------------
// One loaded library
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: &'static Library,
    pub add_op: Op4,
    pub multiply_op: Op4,
    pub subtract_op: Op4,
    pub divide_op: Op4,
    pub modulo_op: Op4,
    find_node_by_id: FindFn,
    add_tree_node: AddFn,
    calculate_tree_sum: SumFn,
    parse_operation: ParseFn,
    get_operation_func: GetOpFn,
    inreftree: InrefFn,
    pub node_table: *mut u8,
    pub node_count: *mut i32,
    pub op_addrs: [usize; 5],
}

impl Lib {
    fn load(path: PathBuf, name: &'static str) -> Lib {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("dlopen {:?}: {e}", path))
        }));
        unsafe fn fnsym<T: Copy>(lib: &'static Library, s: &[u8]) -> T {
            let sym: Symbol<T> = lib
                .get(s)
                .unwrap_or_else(|e| panic!("dlsym {}: {e}", String::from_utf8_lossy(s)));
            *sym
        }
        unsafe {
            let add_op: Op4 = fnsym(lib, b"add_op\0");
            let multiply_op: Op4 = fnsym(lib, b"multiply_op\0");
            let subtract_op: Op4 = fnsym(lib, b"subtract_op\0");
            let divide_op: Op4 = fnsym(lib, b"divide_op\0");
            let modulo_op: Op4 = fnsym(lib, b"modulo_op\0");
            let table: *mut u8 = fnsym(lib, b"node_table\0");
            let count: *mut i32 = fnsym(lib, b"node_count\0");
            Lib {
                name,
                path,
                _lib: lib,
                add_op,
                multiply_op,
                subtract_op,
                divide_op,
                modulo_op,
                find_node_by_id: fnsym(lib, b"find_node_by_id\0"),
                add_tree_node: fnsym(lib, b"add_tree_node\0"),
                calculate_tree_sum: fnsym(lib, b"calculate_tree_sum\0"),
                parse_operation: fnsym(lib, b"parse_operation\0"),
                get_operation_func: fnsym(lib, b"get_operation_func\0"),
                inreftree: fnsym(lib, b"inreftree\0"),
                node_table: table,
                node_count: count,
                op_addrs: [
                    add_op as usize,
                    multiply_op as usize,
                    subtract_op as usize,
                    divide_op as usize,
                    modulo_op as usize,
                ],
            }
        }
    }

    // -- global state ------------------------------------------------------

    pub fn count(&self) -> i32 {
        unsafe { std::ptr::read_volatile(self.node_count) }
    }
    pub fn set_count(&self, v: i32) {
        unsafe { std::ptr::write_volatile(self.node_count, v) }
    }
    pub fn table(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.node_table, TABLE_BYTES).to_vec() }
    }
    pub fn set_table(&self, bytes: &[u8]) {
        assert_eq!(bytes.len(), TABLE_BYTES);
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.node_table, TABLE_BYTES) }
    }
    /// Zero the whole array and reset the counter — puts the library back into
    /// its freshly-`dlopen`ed (`.bss`) state.
    pub fn reset(&self) {
        unsafe { std::ptr::write_bytes(self.node_table, 0, TABLE_BYTES) };
        self.set_count(0);
    }
    /// Raw 52 bytes of one slot.
    pub fn node_bytes(&self, idx: usize) -> Vec<u8> {
        let t = self.table();
        t[idx * NODE_BYTES..(idx + 1) * NODE_BYTES].to_vec()
    }
    /// Full observable state: (node_count, all 2600 table bytes).
    pub fn state(&self) -> (i32, Vec<u8>) {
        (self.count(), self.table())
    }

    /// Overwrite the decoded fields of a slot (used to build trees the public
    /// API cannot express, e.g. dangling child ids).
    pub fn poke_node(
        &self,
        idx: usize,
        id: i32,
        value: i32,
        parent: i32,
        left: i32,
        right: i32,
        label: &[u8],
    ) {
        let mut slot = [0u8; NODE_BYTES];
        slot[0..4].copy_from_slice(&id.to_ne_bytes());
        slot[4..8].copy_from_slice(&value.to_ne_bytes());
        slot[8..12].copy_from_slice(&parent.to_ne_bytes());
        slot[12..16].copy_from_slice(&left.to_ne_bytes());
        slot[16..20].copy_from_slice(&right.to_ne_bytes());
        let n = label.len().min(31);
        slot[20..20 + n].copy_from_slice(&label[..n]);
        unsafe {
            std::ptr::copy_nonoverlapping(
                slot.as_ptr(),
                self.node_table.add(idx * NODE_BYTES),
                NODE_BYTES,
            )
        };
    }

    /// Decode a slot: `(id, value, parent_id, left_child_id, right_child_id)`.
    pub fn decode(&self, idx: usize) -> (i32, i32, i32, i32, i32) {
        let b = self.node_bytes(idx);
        let g = |o: usize| i32::from_ne_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        (g(0), g(4), g(8), g(12), g(16))
    }

    /// Pure-Rust replica of `find_node_by_id`, working off the raw bytes, used
    /// only to *predict* whether `calculate_tree_sum` would terminate.
    fn find_index(&self, id: i32) -> Option<usize> {
        let count = self.count();
        if count <= 0 {
            return None;
        }
        (0..count.min(MAX_NODES as i32) as usize).find(|&i| self.decode(i).0 == id)
    }

    /// Would `calculate_tree_sum(id)` terminate?
    ///
    /// `node_table` can hold cycles — reachable through the public API by
    /// inserting a duplicate id whose parent is that same id, and present in
    /// every freshly-zeroed slot (`id == 0`, `left_child_id == 0`), which points
    /// at itself. The C recurses forever and blows the stack; so does the Rust.
    /// Tests use this to steer clear of that shared, untestable UB.
    pub fn sum_terminates(&self, id: i32) -> bool {
        fn walk(l: &Lib, id: i32, budget: &mut i32, depth: u32) -> bool {
            if *budget <= 0 || depth > 200 {
                return false;
            }
            *budget -= 1;
            let Some(i) = l.find_index(id) else {
                return true;
            };
            let (_, _, _, left, right) = l.decode(i);
            if left != -1 && !walk(l, left, budget, depth + 1) {
                return false;
            }
            if right != -1 && !walk(l, right, budget, depth + 1) {
                return false;
            }
            true
        }
        let mut budget = 20_000;
        walk(self, id, &mut budget, 0)
    }

    // -- exported functions ------------------------------------------------

    /// `find_node_by_id`, normalised to a byte offset from this library's own
    /// `node_table` base (absolute addresses obviously differ between the two
    /// independently mapped objects).
    pub fn find(&self, id: i32) -> Option<isize> {
        let p = unsafe { (self.find_node_by_id)(id) };
        if p.is_null() {
            None
        } else {
            Some(p as isize - self.node_table as isize)
        }
    }
    pub fn add(&self, id: i32, value: i32, parent: i32, label: &[u8]) -> i32 {
        let mut c = label.to_vec();
        c.push(0);
        unsafe { (self.add_tree_node)(id, value, parent, c.as_ptr() as *const c_char) }
    }
    /// `add_tree_node` with a caller-supplied raw pointer (for the NULL case).
    pub unsafe fn add_raw(&self, id: i32, value: i32, parent: i32, label: *const c_char) -> i32 {
        (self.add_tree_node)(id, value, parent, label)
    }
    pub fn sum(&self, id: i32) -> i32 {
        unsafe { (self.calculate_tree_sum)(id) }
    }
    pub fn parse(&self, s: &[u8]) -> i32 {
        let mut c = s.to_vec();
        c.push(0);
        unsafe { (self.parse_operation)(c.as_ptr() as *const c_char) }
    }
    pub unsafe fn parse_raw(&self, s: *const c_char) -> i32 {
        (self.parse_operation)(s)
    }
    /// `get_operation_func`, normalised to the index of the matching exported
    /// `*_op` symbol (`usize::MAX` if it is none of them).
    pub fn op_index(&self, op: i32) -> usize {
        let f = unsafe { (self.get_operation_func)(op) };
        let a = f as usize;
        self.op_addrs
            .iter()
            .position(|&x| x == a)
            .unwrap_or(usize::MAX)
    }
    /// Call the function returned by `get_operation_func`.
    pub fn call_op(&self, op: i32, a: i32, b: i32, u1: i32, u2: i32) -> i32 {
        let f = unsafe { (self.get_operation_func)(op) };
        unsafe { f(a, b, u1, u2) }
    }
    pub fn inreftree(&self, a: i32, b: i32, c: i32, d: i32) -> i32 {
        unsafe { (self.inreftree)(a, b, c, d) }
    }
}

// ---------------------------------------------------------------------------
// The pair, plus a process-wide lock (the libraries have mutable globals and
// `dlopen` of the same path returns the same mapping, so tests must not run
// concurrently against them).
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Lib::load(c_so_path(), "C"),
        r: Lib::load(rust_so_path(), "Rust"),
    })
}

/// Acquire the harness. Returns the guard (keep it alive for the whole test)
/// and the library pair, with both libraries reset to pristine `.bss` state.
pub fn harness() -> (MutexGuard<'static, ()>, &'static Pair) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = pair();
    p.c.reset();
    p.r.reset();
    (g, p)
}

impl Pair {
    pub fn reset(&self) {
        self.c.reset();
        self.r.reset();
    }

    /// Assert that the two libraries' full observable global state matches.
    pub fn assert_state(&self, ctx: &str) {
        let (cc, ct) = self.c.state();
        let (rc, rt) = self.r.state();
        assert_eq!(cc, rc, "node_count mismatch [{ctx}]: C={cc} Rust={rc}");
        if ct != rt {
            let first = (0..TABLE_BYTES).find(|&i| ct[i] != rt[i]).unwrap();
            let node = first / NODE_BYTES;
            panic!(
                "node_table mismatch [{ctx}]: first differing byte {first} \
                 (node {node}, offset {}): C=0x{:02x} Rust=0x{:02x}\n C node: {:?}\n R node: {:?}",
                first % NODE_BYTES,
                ct[first],
                rt[first],
                &ct[node * NODE_BYTES..(node + 1) * NODE_BYTES],
                &rt[node * NODE_BYTES..(node + 1) * NODE_BYTES],
            );
        }
    }

    /// `calculate_tree_sum(id)` is only comparable when the recursion is finite
    /// in BOTH libraries (their tables are always in lock-step, so this really
    /// is one predicate).
    pub fn sum_is_comparable(&self, id: i32) -> bool {
        let c = self.c.sum_terminates(id);
        assert_eq!(
            c,
            self.r.sum_terminates(id),
            "the two tables disagree about termination for sum({id})"
        );
        c
    }

    /// `diff` for `calculate_tree_sum`, skipping the cyclic (unbounded
    /// recursion) cases that neither library can survive.
    pub fn diff_sum(&self, ctx: &str, id: i32) {
        if self.sum_is_comparable(id) {
            self.diff(ctx, |l| l.sum(id));
        }
    }

    /// Run the same closure against both libraries, compare its result and the
    /// resulting global state.
    pub fn diff<T: PartialEq + std::fmt::Debug>(&self, ctx: &str, f: impl Fn(&Lib) -> T) {
        let cv = f(&self.c);
        let rv = f(&self.r);
        assert_eq!(cv, rv, "return-value mismatch [{ctx}]");
        self.assert_state(ctx);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — no external dependency, fixed seed.
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
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// A value biased towards "interesting" magnitudes.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 12] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            -3,
            4,
            -4,
            i32::MAX,
            i32::MIN,
            i32::MIN + 1,
        ];
        match self.below(4) {
            0 => SPECIAL[self.below(SPECIAL.len())],
            1 => self.range_i32(-16, 16),
            2 => self.range_i32(-100_000, 100_000),
            _ => self.next_i32(),
        }
    }
    pub fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len).map(|_| alphabet[self.below(alphabet.len())]).collect()
    }
}
