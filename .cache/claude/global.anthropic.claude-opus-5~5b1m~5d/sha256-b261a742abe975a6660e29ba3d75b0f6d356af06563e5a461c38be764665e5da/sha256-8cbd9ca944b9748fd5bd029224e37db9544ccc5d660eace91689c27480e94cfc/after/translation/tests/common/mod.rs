//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `merge_sort` only through the dynamic symbol, exactly as an
//! external consumer would.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub const ELEM_SIZE: usize = 16; // sizeof(spritebatch_sprite_t) on x86-64

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sprite {
    pub texture_id: u64,
    pub sort_bits: i32,
    // 4 bytes of tail padding; we model it explicitly so we can control it.
    pub _pad: u32,
}

pub type MergeSortFn = unsafe extern "C" fn(*mut Sprite, *mut Sprite, i32);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                best = Some(p);
            }
        }
    }
    best.unwrap_or_else(|| {
        panic!(
            "C shared library not found under {}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // `MERGE_SORT_RUST_SO` lets the harness be pointed at a specific build of
    // the cdylib. This matters: the C struct-assignment / padding behaviour the
    // translation has to reproduce is codegen-sensitive, so both the `release`
    // and the `debug` cdylib are verified (see `verify.sh`).
    if let Some(p) = std::env::var_os("MERGE_SORT_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "MERGE_SORT_RUST_SO points at a missing file: {}", p.display());
        return p;
    }
    let root = workspace_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libmerge_sort_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}. Build it with: cd translation && cargo build --release",
        root.display()
    );
}

struct Libs {
    c: Library,
    rs: Library,
    c_path: PathBuf,
    rs_path: PathBuf,
}

// SAFETY: the loaded libraries are leaked for the whole process lifetime and
// their `merge_sort` symbols are pure functions over caller-provided memory.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rs = unsafe { Library::new(&rs_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rs_path.display()));
        Libs { c, rs, c_path, rs_path }
    })
}

pub fn c_so_path() -> &'static PathBuf {
    &libs().c_path
}
pub fn rust_so_path() -> &'static PathBuf {
    &libs().rs_path
}

pub fn c_merge_sort() -> MergeSortFn {
    let l = libs();
    let s: Symbol<MergeSortFn> = unsafe { l.c.get(b"merge_sort\0") }
        .expect("C .so does not export `merge_sort`");
    *s
}

pub fn rust_merge_sort() -> MergeSortFn {
    let l = libs();
    let s: Symbol<MergeSortFn> = unsafe { l.rs.get(b"merge_sort\0") }
        .expect("Rust .so does not export `merge_sort`");
    *s
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds, fully reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform-ish in `[0, n)`; `n > 0`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
}

// ---------------------------------------------------------------------------
// Byte-level arena: we drive the FFI with raw bytes so padding is under our
// control and comparisons are truly byte-for-byte.
// ---------------------------------------------------------------------------

/// A pair of 16-byte-aligned byte arenas (`a` and `b`) that can be handed to
/// either implementation and then compared in full.
#[derive(Clone)]
pub struct Arena {
    /// backing storage, aligned to 16 bytes
    words: Vec<u64>,
    len_bytes: usize,
}

impl Arena {
    pub fn new(len_bytes: usize) -> Self {
        let words = vec![0u64; len_bytes.div_ceil(8).max(1)];
        Arena { words, len_bytes }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.len_bytes) }
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.words.as_mut_ptr() as *mut u8, self.len_bytes)
        }
    }
    pub fn ptr(&mut self) -> *mut Sprite {
        self.words.as_mut_ptr() as *mut Sprite
    }
    /// Pointer to element `idx` (may be past the logical end — used for guards).
    pub fn ptr_at(&mut self, idx: usize) -> *mut Sprite {
        unsafe { (self.words.as_mut_ptr() as *mut u8).add(idx * ELEM_SIZE) as *mut Sprite }
    }
    pub fn fill_random(&mut self, rng: &mut Rng) {
        let n = self.len_bytes;
        let b = self.bytes_mut();
        let mut i = 0;
        while i < n {
            let v = rng.next_u64().to_le_bytes();
            let take = (n - i).min(8);
            b[i..i + take].copy_from_slice(&v[..take]);
            i += take;
        }
    }
    pub fn write_sprites(&mut self, sprites: &[Sprite]) {
        assert!(sprites.len() * ELEM_SIZE <= self.len_bytes);
        unsafe {
            std::ptr::copy_nonoverlapping(
                sprites.as_ptr() as *const u8,
                self.bytes_mut().as_mut_ptr(),
                sprites.len() * ELEM_SIZE,
            );
        }
    }
    /// Overwrite the sprite fields at `idx` while leaving the padding bytes
    /// exactly as they are (used by the padding-sensitivity test).
    pub fn write_fields(&mut self, idx: usize, texture_id: u64, sort_bits: i32) {
        let off = idx * ELEM_SIZE;
        let b = self.bytes_mut();
        b[off..off + 8].copy_from_slice(&texture_id.to_le_bytes());
        b[off + 8..off + 12].copy_from_slice(&sort_bits.to_le_bytes());
    }
}

/// Run `merge_sort` in both libraries on identical copies of `(a, b)` and
/// assert the resulting arenas are byte-identical.
///
/// `size` is passed through verbatim (it may be any `i32`).
#[track_caller]
pub fn diff_run(label: &str, a: &Arena, b: &Arena, size: i32) {
    let (mut ac, mut bc) = (a.clone(), b.clone());
    let (mut ar, mut br) = (a.clone(), b.clone());

    unsafe { c_merge_sort()(ac.ptr(), bc.ptr(), size) };
    unsafe { rust_merge_sort()(ar.ptr(), br.ptr(), size) };

    cmp_arena(label, "a", &ac, &ar);
    cmp_arena(label, "b", &bc, &br);
}

/// Same as [`diff_run`] but both pointers are the SAME arena (aliased case).
#[track_caller]
pub fn diff_run_aliased(label: &str, a: &Arena, size: i32) {
    let mut ac = a.clone();
    let mut ar = a.clone();
    unsafe {
        let p = ac.ptr();
        c_merge_sort()(p, p, size);
    }
    unsafe {
        let p = ar.ptr();
        rust_merge_sort()(p, p, size);
    }
    cmp_arena(label, "aliased", &ac, &ar);
}

/// Both `a` and `b` are windows into a single arena, `elem_gap` elements apart.
#[track_caller]
pub fn diff_run_windows(label: &str, arena: &Arena, a_idx: usize, b_idx: usize, size: i32) {
    let mut c_arena = arena.clone();
    let mut r_arena = arena.clone();
    unsafe { c_merge_sort()(c_arena.ptr_at(a_idx), c_arena.ptr_at(b_idx), size) };
    unsafe { rust_merge_sort()(r_arena.ptr_at(a_idx), r_arena.ptr_at(b_idx), size) };
    cmp_arena(label, "window-arena", &c_arena, &r_arena);
}

#[track_caller]
pub fn cmp_arena(label: &str, which: &str, c: &Arena, r: &Arena) {
    let cb = c.bytes();
    let rb = r.bytes();
    assert_eq!(cb.len(), rb.len(), "{label}: arena `{which}` length mismatch");
    if cb != rb {
        let first = cb.iter().zip(rb).position(|(x, y)| x != y).unwrap();
        panic!(
            "{label}: arena `{which}` diverges at byte {first} (element {}, offset {} in element)\n\
             C   : {:02x?}\n\
             Rust: {:02x?}",
            first / ELEM_SIZE,
            first % ELEM_SIZE,
            &cb[first.saturating_sub(16)..(first + 16).min(cb.len())],
            &rb[first.saturating_sub(16)..(first + 16).min(rb.len())],
        );
    }
}

// ---------------------------------------------------------------------------
// Fatal-signal differential harness (for the crashing ERRORS.md rows).
// ---------------------------------------------------------------------------

/// Outcome of running a closure in a forked child.
#[derive(Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Exited(i32),
    Signaled(i32),
}

/// Fork and run `f` in the child. Returns how the child terminated.
pub fn run_in_child<F: FnOnce()>(f: F) -> ChildOutcome {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: make sure a fault really kills us rather than being caught.
            libc::signal(libc::SIGSEGV, libc::SIG_DFL);
            libc::signal(libc::SIGBUS, libc::SIG_DFL);
            libc::signal(libc::SIGABRT, libc::SIG_DFL);
            f();
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            ChildOutcome::Signaled(libc::WTERMSIG(status))
        } else {
            ChildOutcome::Exited(libc::WEXITSTATUS(status))
        }
    }
}

/// Assert the C and Rust implementations terminate the same way for an input
/// that is expected to be fatal (or at least is UB-adjacent).
#[track_caller]
pub fn diff_fatal(label: &str, a: *mut Sprite, b: *mut Sprite, size: i32) {
    let cf = c_merge_sort();
    let rf = rust_merge_sort();
    let c_out = run_in_child(move || unsafe { cf(a, b, size) });
    let r_out = run_in_child(move || unsafe { rf(a, b, size) });
    assert_eq!(
        c_out, r_out,
        "{label}: C and Rust terminated differently (C={c_out:?}, Rust={r_out:?})"
    );
}

// ---------------------------------------------------------------------------
// Shared-memory differential harness.
//
// For inputs that may kill the process (huge / negative `size`, null pointers)
// we cannot inspect memory afterwards from the parent when using ordinary heap
// buffers. `MAP_SHARED` anonymous mappings make the child's writes visible to
// the parent, so we can compare BOTH the termination outcome AND every byte the
// two implementations wrote before dying.
// ---------------------------------------------------------------------------

pub struct SharedArena {
    ptr: *mut u8,
    map_len: usize,
    len: usize,
}

impl SharedArena {
    pub fn new(len_bytes: usize) -> Self {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
        let map_len = len_bytes.max(1).next_multiple_of(page);
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(ptr != libc::MAP_FAILED, "mmap failed");
        SharedArena { ptr: ptr as *mut u8, map_len, len: len_bytes }
    }
    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
    pub fn sprites(&mut self) -> *mut Sprite {
        self.ptr as *mut Sprite
    }
    pub fn fill_random(&mut self, rng: &mut Rng) {
        let n = self.len;
        let b = self.bytes_mut();
        let mut i = 0;
        while i < n {
            let v = rng.next_u64().to_le_bytes();
            let take = (n - i).min(8);
            b[i..i + take].copy_from_slice(&v[..take]);
            i += take;
        }
    }
    pub fn copy_from(&mut self, other: &SharedArena) {
        assert_eq!(self.len, other.len);
        let src = other.bytes().to_vec();
        self.bytes_mut().copy_from_slice(&src);
    }
}

impl Drop for SharedArena {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.map_len);
        }
    }
}

/// Run `merge_sort(a, b, size)` in a forked child for BOTH implementations and
/// assert that
///  * both children terminated the same way (same signal, or same exit code),
///  * every byte of both `a` and `b` matches afterwards.
///
/// Both implementations are handed the **very same two addresses**: the C child
/// runs first, its result is snapshotted out of the `MAP_SHARED` mappings, the
/// mappings are reset to the seed contents, and then the Rust child runs on the
/// identical addresses. This matters for the pathological `size` values: glibc's
/// `memcpy` picks its strategy partly from `dst - src`, so handing the two
/// implementations *different* mappings would compare two different glibc code
/// paths rather than the two translations.
///
/// `elems` is the number of `spritebatch_sprite_t` slots to allocate.
#[track_caller]
pub fn diff_shared(label: &str, elems: usize, size: i32, rng: &mut Rng) {
    let bytes = elems.max(1) * ELEM_SIZE;
    let mut a = SharedArena::new(bytes);
    let mut b = SharedArena::new(bytes);
    a.fill_random(rng);
    b.fill_random(rng);
    let seed_a = a.bytes().to_vec();
    let seed_b = b.bytes().to_vec();

    let cf = c_merge_sort();
    let rf = rust_merge_sort();
    let (pa, pb) = (a.sprites(), b.sprites());

    let c_out = run_in_child(move || unsafe { cf(pa, pb, size) });
    let after_a_c = a.bytes().to_vec();
    let after_b_c = b.bytes().to_vec();

    a.bytes_mut().copy_from_slice(&seed_a);
    b.bytes_mut().copy_from_slice(&seed_b);
    let r_out = run_in_child(move || unsafe { rf(pa, pb, size) });
    let after_a_r = a.bytes().to_vec();
    let after_b_r = b.bytes().to_vec();

    assert_eq!(
        c_out, r_out,
        "{label}: termination differs (C={c_out:?}, Rust={r_out:?})"
    );
    assert_shared_eq(label, "a", &after_a_c, &after_a_r);
    assert_shared_eq(label, "b", &after_b_c, &after_b_r);
}

#[track_caller]
fn assert_shared_eq(label: &str, which: &str, cb: &[u8], rb: &[u8]) {
    if cb != rb {
        let first = cb.iter().zip(rb).position(|(x, y)| x != y).unwrap();
        panic!(
            "{label}: shared arena `{which}` diverges at byte {first} \
             (element {}, offset {})\nC   : {:02x?}\nRust: {:02x?}",
            first / ELEM_SIZE,
            first % ELEM_SIZE,
            &cb[first..(first + 16).min(cb.len())],
            &rb[first..(first + 16).min(rb.len())],
        );
    }
}


// ---------------------------------------------------------------------------
// Guarded arena + fault-address capture.
//
// A negative or very large `size` makes the C compute an astronomically large
// byte count and hand it to `memcpy`, which then runs off the end of the buffer.
// Whether and where that faults normally depends on what happens to be mapped
// after the buffer — i.e. on the process address-space layout, which necessarily
// differs between the C child and the Rust child (different `.so`s are loaded).
// That makes a naive "did it crash?" comparison meaningless; see ERRORS.md.
//
// A `GuardedArena` removes the dependency: a small read/write window is placed
// inside a large `PROT_NONE` reservation, so the very first out-of-window access
// faults, immediately, at an address that is a pure function of the arguments
// `merge_sort` passed to `memcpy`. The child installs a `SIGSEGV`/`SIGBUS`
// handler that records `si_addr` into shared memory, so the differential
// assertion compares the EXACT faulting address, not just "both died".
// ---------------------------------------------------------------------------

/// Reservation size around the writable window. Large enough to dwarf any
/// nearby-mapping variability, small enough that faulting is instant.
pub const GUARD_RESERVE: usize = 256 << 20; // 256 MiB
/// Writable window size (one page = 256 sprites).
pub const GUARD_WINDOW: usize = 4096;

/// Shared cell the child's signal handler writes its `si_addr` into.
static mut FAULT_CELL: *mut u64 = std::ptr::null_mut();

extern "C" fn fault_handler(
    _sig: libc::c_int,
    info: *mut libc::siginfo_t,
    _ctx: *mut libc::c_void,
) {
    unsafe {
        let cell = FAULT_CELL;
        if !cell.is_null() && !info.is_null() {
            // `si_addr` is the first field of the union on Linux; libc exposes
            // it via `si_addr()`.
            *cell = (*info).si_addr() as u64;
        }
        libc::_exit(42);
    }
}

pub struct GuardedArena {
    base: *mut u8,
    window: *mut u8,
    cell: *mut u64,
}

impl GuardedArena {
    pub fn new() -> Option<Self> {
        unsafe {
            // PROT_NONE private reservation.
            let base = libc::mmap(
                std::ptr::null_mut(),
                GUARD_RESERVE,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            );
            if base == libc::MAP_FAILED {
                return None;
            }
            let base = base as *mut u8;
            // Writable, process-shared window in the middle of the reservation.
            let window_off = GUARD_RESERVE / 2;
            let window = libc::mmap(
                base.add(window_off) as *mut libc::c_void,
                GUARD_WINDOW,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
                -1,
                0,
            );
            if window == libc::MAP_FAILED {
                libc::munmap(base as *mut libc::c_void, GUARD_RESERVE);
                return None;
            }
            // Shared one-word cell for the fault address.
            let cell = libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            if cell == libc::MAP_FAILED {
                libc::munmap(base as *mut libc::c_void, GUARD_RESERVE);
                return None;
            }
            Some(GuardedArena {
                base,
                window: window as *mut u8,
                cell: cell as *mut u64,
            })
        }
    }
    pub fn window(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.window, GUARD_WINDOW) }
    }
    pub fn window_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.window, GUARD_WINDOW) }
    }
    pub fn at(&self, idx: usize) -> *mut Sprite {
        unsafe { self.window.add(idx * ELEM_SIZE) as *mut Sprite }
    }
    pub fn fill_random(&mut self, rng: &mut Rng) {
        let w = self.window_mut();
        let n = w.len();
        let mut i = 0;
        while i < n {
            let v = rng.next_u64().to_le_bytes();
            let take = (n - i).min(8);
            w[i..i + take].copy_from_slice(&v[..take]);
            i += take;
        }
    }
    fn fault_addr(&self) -> u64 {
        unsafe { *self.cell }
    }
    fn reset_fault(&self) {
        unsafe { *self.cell = u64::MAX }
    }
    /// Run `f` in a child with the fault handler installed. Returns
    /// `(outcome, si_addr)`; `si_addr` is `u64::MAX` when no fault occurred.
    fn run_capturing<F: FnOnce()>(&self, f: F) -> (ChildOutcome, u64) {
        self.reset_fault();
        let cell = self.cell;
        let out = run_in_child(move || unsafe {
            FAULT_CELL = cell;
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = fault_handler as *const () as usize;
            sa.sa_flags = libc::SA_SIGINFO | libc::SA_NODEFER;
            libc::sigemptyset(&mut sa.sa_mask);
            libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGBUS, &sa, std::ptr::null_mut());
            f();
        });
        (out, self.fault_addr())
    }
}

impl Drop for GuardedArena {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, GUARD_RESERVE);
            libc::munmap(self.cell as *mut libc::c_void, 4096);
        }
    }
}

/// Differential test for a pathological `size` inside a [`GuardedArena`].
///
/// `a` sits at element 0 of the window, `b` at element `b_idx`. The C child runs
/// first (its window contents and fault address are snapshotted), the window is
/// restored, then the Rust child runs on the *identical* addresses. All three
/// observables are compared: termination outcome, faulting address, and every
/// byte of the window.
///
/// Returns `false` if the reservation could not be made (test then skips).
#[track_caller]
pub fn diff_guarded(label: &str, size: i32, b_idx: usize, rng: &mut Rng) -> bool {
    let Some(mut g) = GuardedArena::new() else { return false };
    g.fill_random(rng);
    let seed = g.window().to_vec();

    let cf = c_merge_sort();
    let rf = rust_merge_sort();
    let pa = g.at(0);
    let pb = g.at(b_idx);

    let (c_out, c_addr) = g.run_capturing(move || unsafe { cf(pa, pb, size) });
    let after_c = g.window().to_vec();

    g.window_mut().copy_from_slice(&seed);
    let (r_out, r_addr) = g.run_capturing(move || unsafe { rf(pa, pb, size) });
    let after_r = g.window().to_vec();

    assert_eq!(
        c_out, r_out,
        "{label}: termination differs (C={c_out:?}, Rust={r_out:?})"
    );
    assert_eq!(
        c_addr, r_addr,
        "{label}: fault address differs (C=0x{c_addr:x}, Rust=0x{r_addr:x}) \
         -> the two implementations handed different arguments to memcpy"
    );
    assert_shared_eq(label, "guarded-window", &after_c, &after_r);
    true
}

/// Debug/introspection helper used to prove the guarded rows are not vacuous:
/// returns `(c_outcome, c_fault_addr, r_outcome, r_fault_addr)`.
pub fn guarded_detail(
    g: &GuardedArena,
    size: i32,
    b_idx: usize,
    _rng: &mut Rng,
) -> (ChildOutcome, u64, ChildOutcome, u64) {
    let cf = c_merge_sort();
    let rf = rust_merge_sort();
    let pa = g.at(0);
    let pb = g.at(b_idx);
    let (c_out, c_addr) = g.run_capturing(move || unsafe { cf(pa, pb, size) });
    let (r_out, r_addr) = g.run_capturing(move || unsafe { rf(pa, pb, size) });
    (c_out, c_addr, r_out, r_addr)
}
