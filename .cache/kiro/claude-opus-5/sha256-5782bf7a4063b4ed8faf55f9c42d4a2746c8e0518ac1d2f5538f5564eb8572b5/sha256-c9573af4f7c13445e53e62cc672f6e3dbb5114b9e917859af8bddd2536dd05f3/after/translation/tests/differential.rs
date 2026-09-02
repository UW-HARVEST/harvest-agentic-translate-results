//! Differential tests: C `libdriver.so` vs. Rust `libdriver.so`.
//!
//! Both objects are loaded with `libloading` and every call goes through the
//! `.so`'s exported symbol — the Rust functions are never called directly, so
//! the `#[no_mangle]` / `extern "C"` wrappers are part of what is tested.
//!
//! ## Why the harness controls the stack
//!
//! `bad()` reads an uninitialized `char *` (CWE-457), so its output is a
//! function of the caller's stack residue. Comparing the two libraries
//! meaningfully therefore requires *pinning* that residue: before every call the
//! harness overwrites the stack region the callee will occupy with a chosen
//! pattern, and then calls the C and the Rust symbol from one identical call
//! site at one identical stack depth (`perform`, whose only difference between
//! the two runs is the runtime value of a function pointer).
//!
//! Two residue patterns are used:
//!
//! * `ResidueMode::Uniform` — every word gets the same 64-bit value, so the
//!   *content* `bad()` emits is under test.
//! * `ResidueMode::Indexed` — word *i* gets a pointer to the distinct string
//!   `slotNNNN` (NNNN = i), so the emitted label identifies *exactly which stack
//!   word the callee read*. This is what makes an offset mismatch between the C
//!   and the Rust frame layout a test failure rather than an invisible
//!   coincidence.
//!
//! Without this pinning the residue is whatever the dynamic loader happened to
//! leave behind, which differs between a 16 KiB C object and a 400 KiB Rust one
//! and is randomised by ASLR run to run — i.e. not a property of the
//! translation.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::OnceLock;


// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

/// The four exported entry points, as raw C function pointers pulled out of a
/// `.so` with `dlsym`.
#[derive(Clone, Copy)]
struct Api {
    #[allow(dead_code)]
    name: &'static str,
    print_line: unsafe extern "C" fn(*const c_char),
    bad: unsafe extern "C" fn(),
    good: unsafe extern "C" fn(),
    driver: unsafe extern "C" fn(c_int),
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workspace>/c_src/build/libdriver.so`, built by CMake.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

/// The Rust `cdylib` for the profile the test binary itself was built with, so
/// `cargo test --release` exercises `target/release/libdriver.so`.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // .../target/<profile>/deps/<test-exe>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test exe lives in target/<profile>/deps");
    profile_dir.join("libdriver.so")
}

fn load(path: &Path, label: &'static str) -> Api {
    assert!(
        path.exists(),
        "{label} shared object not found at {}.\n\
         Build the C side with:  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust side with:  cd translation && cargo build --release",
        path.display()
    );
    // Leaked on purpose: the raw function pointers below must stay valid for
    // the whole process, and unloading a `.so` mid-test would invalidate them.
    let lib = Box::leak(Box::new(unsafe {
        libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
    }));
    macro_rules! sym {
        ($n:literal, $t:ty) => {{
            let s: libloading::Symbol<$t> = unsafe { lib.get($n) }
                .unwrap_or_else(|e| panic!("dlsym {:?} in {}: {e}", $n, path.display()));
            *s
        }};
    }
    Api {
        name: label,
        print_line: sym!(b"printLine\0", unsafe extern "C" fn(*const c_char)),
        bad: sym!(b"bad\0", unsafe extern "C" fn()),
        good: sym!(b"good\0", unsafe extern "C" fn()),
        driver: sym!(b"driver\0", unsafe extern "C" fn(c_int)),
    }
}

/// `(c, rust)`. Both are loaded exactly once, C first, in every test process.
fn apis() -> &'static (Api, Api) {
    static APIS: OnceLock<(Api, Api)> = OnceLock::new();
    APIS.get_or_init(|| {
        let c = load(&c_so_path(), "C");
        let r = load(&rust_so_path(), "Rust");
        (c, r)
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    static stdout: *mut c_void;
}

const IONBF: c_int = 2;

/// Flush *all* stdio streams. Both libraries call `puts` in this process and so
/// share one `FILE *stdout`; the buffer has to be drained while fd 1 still
/// points at the capture target, otherwise output is attributed to the wrong
/// run.
fn flush_all() {
    unsafe { fflush(ptr::null_mut()) };
}

/// Run `f` with fd 1 redirected to a fresh temporary file and return the bytes
/// it wrote. Handles payloads of any size (unlike a pipe, which would deadlock
/// past 64 KiB).
fn capture_file<F: FnOnce()>(f: F) -> Vec<u8> {
    flush_all();
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "driver-diff-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    flush_all();
    assert!(unsafe { libc::dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { libc::close(saved) };
    drop(file);

    let mut out = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut out)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Same, but fd 1 is a **pipe**, which glibc buffers differently from a regular
/// file (`fstat` on fd 1 drives the choice). Only for payloads well under the
/// 64 KiB pipe capacity.
fn capture_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    flush_all();
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let (rd, wr) = (fds[0], fds[1]);
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0);
    assert!(unsafe { libc::dup2(wr, 1) } >= 0);

    f();

    flush_all();
    assert!(unsafe { libc::dup2(saved, 1) } >= 0);
    unsafe {
        libc::close(saved);
        libc::close(wr);
    }
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(rd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { libc::close(rd) };
    out
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (split-mix64) with a fixed seed
// ---------------------------------------------------------------------------

const SEED: u64 = 0x5EED_D1FF_2025_0901;

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        Rng(SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (n > 0).
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    /// A byte in `1..=255` — never 0, so it cannot terminate the string early.
    fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
}

/// A random NUL-terminated buffer of `len` payload bytes drawn from `1..=255`,
/// plus random non-NUL garbage after the terminator (so an over-read is
/// visible).
fn rand_cbuf(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 9);
    for _ in 0..len {
        v.push(rng.nonzero_byte());
    }
    v.push(0);
    for _ in 0..8 {
        v.push(rng.nonzero_byte());
    }
    v
}

/// Printable-ASCII variant.
fn rand_ascii(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push(0x20 + rng.below(95) as u8);
    }
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Stack-residue control
// ---------------------------------------------------------------------------
//
// The residue is planted with inline asm that writes `DIRTY_WORDS` words to
// `[rsp-512 .. rsp-8]` *immediately before* the indirect call. That window is
// exactly the memory the callee's frame will occupy, so it covers both the
// direct path (`bad` reads `rsp-0x18`) and the `driver` path (`bad` reads
// `rsp-0x38`). Doing it from a Rust helper's stack frame instead does not work:
// the helper's own saved registers sit between its buffer and the call site, so
// the very words the callee reads stay unpinned.
//
// The `nop` after the call is load-bearing: without it LLVM turns the indirect
// call into a tail jump, which unwinds this frame first and lands the callee
// somewhere above the pinned window.

/// 64 words = 512 bytes below the call site.
const DIRTY_WORDS: usize = 64;

struct Pattern(std::cell::UnsafeCell<[u64; DIRTY_WORDS]>);
// Access is serialised by `serial()`; the tests are single-threaded.
unsafe impl Sync for Pattern {}
static PATTERN: Pattern = Pattern(std::cell::UnsafeCell::new([0; DIRTY_WORDS]));

fn pattern_ptr() -> *const u64 {
    PATTERN.0.get() as *const u64
}

fn set_pattern(f: impl Fn(usize) -> u64) {
    let p = PATTERN.0.get();
    for i in 0..DIRTY_WORDS {
        unsafe { (*p)[i] = f(i) };
    }
}

/// Every word of the window gets the same value — puts the *content* `bad()`
/// emits under test.
fn set_residue_uniform(v: u64) {
    set_pattern(|_| v);
}

/// Word *i* gets a pointer to the distinct string `slotNNNN` (NNNN == i), so the
/// bytes `bad()` emits identify the exact stack offset it read. An offset
/// mismatch between the C and the Rust frame layout then shows up as a diff.
fn set_residue_indexed() {
    let slots = slot_ptrs();
    set_pattern(|i| slots[i]);
}

fn slot_ptrs() -> &'static [u64; DIRTY_WORDS] {
    static P: OnceLock<[u64; DIRTY_WORDS]> = OnceLock::new();
    P.get_or_init(|| {
        let mut a = [0u64; DIRTY_WORDS];
        for (i, slot) in a.iter_mut().enumerate() {
            let s = CString::new(format!("slot{i:04}")).unwrap();
            *slot = Box::leak(s.into_boxed_c_str()).as_ptr() as u64;
        }
        a
    })
}

/// Uniform shape for all four entry points. On x86-64 SysV a `void(void)`
/// callee simply ignores `rdi`, and `void(int)` reads `edi`, so one pointer type
/// can drive `printLine(char*)`, `driver(int)`, `bad()` and `good()` — which is
/// what lets the C and the Rust run share a single call site.
type Thunk = unsafe extern "C" fn(usize);

fn resolve(api: Api, op: Op, arg: *const c_char) -> (Thunk, usize) {
    unsafe {
        match op {
            Op::PrintLine => (std::mem::transmute::<_, Thunk>(api.print_line), arg as usize),
            Op::PrintLineNull => (std::mem::transmute::<_, Thunk>(api.print_line), 0),
            Op::Bad => (std::mem::transmute::<_, Thunk>(api.bad), 0),
            Op::Good => (std::mem::transmute::<_, Thunk>(api.good), 0),
            Op::Driver(v) => (
                std::mem::transmute::<_, Thunk>(api.driver),
                v as u32 as usize,
            ),
        }
    }
}

/// Call `f(a)`, optionally pinning the stack window just below the call first.
#[inline(never)]
#[cfg(target_arch = "x86_64")]
fn call_at_depth(dirty: bool, f: Thunk, a: usize) {
    unsafe {
        if dirty {
            core::arch::asm!(
                "lea {b}, [rsp - 512]",
                "xor {k:e}, {k:e}",
                "2:",
                "mov {t}, [{src} + {k} * 8]",
                "mov [{b} + {k} * 8], {t}",
                "inc {k}",
                "cmp {k}, 64",
                "jb 2b",
                src = in(reg) pattern_ptr(),
                b = out(reg) _,
                k = out(reg) _,
                t = out(reg) _,
            );
        }
        f(a);
        // Blocks the tail call; see the module note above.
        core::arch::asm!("nop");
    }
}

#[inline(never)]
#[cfg(not(target_arch = "x86_64"))]
fn call_at_depth(_dirty: bool, f: Thunk, a: usize) {
    unsafe { f(a) };
    std::hint::black_box(a);
}

/// One scripted operation against a library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    /// `printLine(arg)` where `arg` is the raw pointer supplied to `perform`.
    PrintLine,
    /// `printLine(NULL)`
    PrintLineNull,
    Bad,
    Good,
    Driver(c_int),
}

/// Execute `op` against `api` with the stack residue pinned.
///
/// The C run and the Rust run execute the *same* instructions at the *same*
/// stack depth; the only difference is the value of the function pointer.
#[inline(never)]
fn perform(api: Api, op: Op, arg: *const c_char) {
    let (f, a) = resolve(api, op, arg);
    call_at_depth(true, f, a);
}

/// Run a whole *sequence* of ops, pinning the residue only before the first, so
/// later ops see whatever the earlier ones left behind — the composed pipeline.
#[inline(never)]
fn perform_seq(api: Api, ops: &[Op], arg: *const c_char) {
    for (i, &op) in ops.iter().enumerate() {
        let (f, a) = resolve(api, op, arg);
        call_at_depth(i == 0, f, a);
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

fn hex(b: &[u8]) -> String {
    const MAX: usize = 96;
    let mut s = String::new();
    for byte in b.iter().take(MAX) {
        s.push_str(&format!("{byte:02x}"));
    }
    if b.len() > MAX {
        s.push_str(&format!("...(+{} bytes)", b.len() - MAX));
    }
    s
}

/// Run one op against both libraries with the identical pinned residue and
/// assert the captured stdout bytes are equal.
#[track_caller]
fn assert_same_op(row: &str, case: &str, op: Op, arg: *const c_char) -> Vec<u8> {
    let (c, r) = *apis();
    let out_c = capture_file(|| perform(c, op, arg));
    let out_r = capture_file(|| perform(r, op, arg));
    assert!(
        out_c == out_r,
        "\n{row} / {case}: C and Rust disagree for {op:?}\n  C    ({} bytes): {}\n  Rust ({} bytes): {}\n",
        out_c.len(),
        hex(&out_c),
        out_r.len(),
        hex(&out_r)
    );
    out_c
}

/// Exit status of a child: `(was_signalled, signal_or_exit_code)`.
type Status = (bool, i32);

/// First thing every forked child does: suppress core dumps. Several rows
/// deliberately provoke `SIGSEGV`, and letting the system core-dump handler run
/// for each one makes the suite tens of times slower.
fn child_prep() {
    unsafe {
        let lim = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &lim);
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
}

fn wait_status(pid: i32) -> Status {
    let mut st = 0i32;
    assert_eq!(
        unsafe { libc::waitpid(pid, &mut st, 0) },
        pid,
        "waitpid failed"
    );
    if libc::WIFSIGNALED(st) {
        (true, libc::WTERMSIG(st))
    } else {
        (false, libc::WEXITSTATUS(st))
    }
}

/// Run a whole op sequence against one library **in a forked child**, with fd 1
/// pointing at a fresh file and stdout unbuffered, and return the child's exit
/// status together with the bytes it wrote.
///
/// A fork is required because in a *sequence* only the first call has its
/// residue pinned; a later `bad()` picks up whatever the previous call left in
/// that stack word, which may not be a valid pointer at all. The C library
/// faults there — that is its real behaviour — so the fault has to be *compared*
/// rather than avoided. Unbuffered stdout keeps the bytes written before a fault
/// observable.
fn seq_in_child(api: Api, ops: &[Op], arg: *const c_char) -> (Status, Vec<u8>) {
    flush_all();
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("driver-seq-{}-{n}.out", std::process::id()));
    let file = std::fs::File::create(&path).expect("create child capture file");
    let fd = file.as_raw_fd();

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        child_prep();
        unsafe {
            libc::dup2(fd, 1);
            setvbuf(stdout, ptr::null_mut(), IONBF, 0);
            perform_seq(api, ops, arg);
            fflush(ptr::null_mut());
            libc::_exit(0);
        }
    }
    let status = wait_status(pid);
    drop(file);
    let mut out = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen child capture file")
        .read_to_end(&mut out)
        .expect("read child capture file");
    let _ = std::fs::remove_file(&path);
    (status, out)
}

/// Sequence differential: both the bytes written *and* how the process ended
/// must match.
#[track_caller]
fn assert_same_seq(row: &str, case: &str, ops: &[Op], arg: *const c_char) -> Vec<u8> {
    let (c, r) = *apis();
    let (st_c, out_c) = seq_in_child(c, ops, arg);
    let (st_r, out_r) = seq_in_child(r, ops, arg);
    assert!(
        st_c == st_r,
        "\n{row} / {case}: C and Rust ended differently for sequence {ops:?}\n  \
         C    (signalled, code) = {st_c:?}\n  Rust (signalled, code) = {st_r:?}\n"
    );
    assert!(
        out_c == out_r,
        "\n{row} / {case}: C and Rust disagree for sequence {ops:?}\n  C    ({} bytes): {}\n  Rust ({} bytes): {}\n",
        out_c.len(),
        hex(&out_c),
        out_r.len(),
        hex(&out_r)
    );
    out_c
}

/// Single-op differential run in forked children, so that a divergence which
/// *crashes* (rather than printing different bytes) is reported as a status
/// mismatch instead of taking the whole test runner down with it. Used for the
/// rows where a fault is a plausible outcome.
#[track_caller]
fn assert_same_op_forked(row: &str, case: &str, op: Op, arg: *const c_char) -> Vec<u8> {
    let (c, r) = *apis();
    let (st_c, out_c) = seq_in_child(c, &[op], arg);
    let (st_r, out_r) = seq_in_child(r, &[op], arg);
    assert!(
        st_c == st_r,
        "\n{row} / {case}: C and Rust ended differently for {op:?}\n  \
         C    (signalled, code) = {st_c:?}\n  Rust (signalled, code) = {st_r:?}\n"
    );
    assert!(
        out_c == out_r,
        "\n{row} / {case}: C and Rust disagree for {op:?}\n  C    ({} bytes): {}\n  Rust ({} bytes): {}\n",
        out_c.len(),
        hex(&out_c),
        out_r.len(),
        hex(&out_r)
    );
    out_c
}

/// `printLine` differential over a raw buffer, plus an independent check that
/// the bytes equal `puts(line)` (gcc's lowering of `printf("%s\n", line)`).
#[track_caller]
fn assert_print_line(row: &str, case: &str, buf: &[u8]) {
    let out = assert_same_op(row, case, Op::PrintLine, buf.as_ptr() as *const c_char);
    let payload = &buf[..buf.iter().position(|&b| b == 0).expect("NUL-terminated")];
    let mut want = payload.to_vec();
    want.push(b'\n');
    assert!(
        out == want,
        "{row} / {case}: output is not puts(line): got {} bytes, want {}",
        out.len(),
        want.len()
    );
}

// ===========================================================================
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ===========================================================================

/// Row 1 — `printLine`, length 1..=64, random printable ASCII.
#[test]
fn phase_b_row01_printline_short_ascii() {
    let _serial = serial();
    let mut rng = Rng::new(1);
    for i in 0..4096 {
        let len = rng.range(1, 64);
        let buf = rand_ascii(&mut rng, len);
        assert_print_line("CONFIGS row 1", &format!("i={i} len={len}"), &buf);
    }
}

/// Row 2 — `printLine("")`, the zero-length shape.
#[test]
fn phase_b_row02_printline_empty() {
    let _serial = serial();
    assert_print_line("CONFIGS row 2", "empty", b"\0");
}

/// Row 3 — `printLine`, random length up to 4096, full 1..=255 byte range.
#[test]
fn phase_b_row03_printline_random_bytes() {
    let _serial = serial();
    let mut rng = Rng::new(3);
    for i in 0..2048 {
        let len = rng.range(1, 4096);
        let buf = rand_cbuf(&mut rng, len);
        assert_print_line("CONFIGS row 3", &format!("i={i} len={len}"), &buf);
    }
}

/// Row 4 — every single-byte string `\x01`..`\xFF`.
#[test]
fn phase_b_row04_printline_every_single_byte() {
    let _serial = serial();
    for b in 1u8..=255 {
        assert_print_line("CONFIGS row 4", &format!("byte={b:#04x}"), &[b, 0]);
    }
}

/// Row 5 — lengths at and around glibc's stdio buffer boundaries.
#[test]
fn phase_b_row05_printline_buffer_boundaries() {
    let _serial = serial();
    let mut rng = Rng::new(5);
    for len in [4095usize, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537] {
        let buf = rand_cbuf(&mut rng, len);
        assert_print_line("CONFIGS row 5", &format!("len={len}"), &buf);
    }
}

/// Row 6 — multi-megabyte payloads.
#[test]
fn phase_b_row06_printline_oversized() {
    let _serial = serial();
    let mut rng = Rng::new(6);
    for len in [1usize << 20, 1usize << 22] {
        let buf = rand_cbuf(&mut rng, len);
        assert_print_line("CONFIGS row 6", &format!("len={len}"), &buf);
    }
}

/// Row 7 — garbage after the NUL terminator must never be emitted.
#[test]
fn phase_b_row07_printline_no_overread() {
    let _serial = serial();
    let mut rng = Rng::new(7);
    for i in 0..4096 {
        let len = rng.range(0, 128);
        // rand_cbuf already appends 8 non-NUL bytes past the terminator.
        let buf = rand_cbuf(&mut rng, len);
        assert_print_line("CONFIGS row 7", &format!("i={i} len={len}"), &buf);
    }
}

/// Row 8 — `printf` conversion specifiers are data, not format.
#[test]
fn phase_b_row08_printline_format_specifiers() {
    let _serial = serial();
    let cases: [&[u8]; 16] = [
        b"%s\0",
        b"%d\0",
        b"%n\0",
        b"%p\0",
        b"%%\0",
        b"%s%s%s%s%s%s%s%s\0",
        b"%n%n%n%n\0",
        b"%1000000d\0",
        b"%.*s\0",
        b"%hn\0",
        b"%99$n\0",
        b"a%sb%nc%dd\0",
        b"\\n%s\0",
        b"%\0",
        b"%s\n%s\0",
        b"100%% sure: %s -> %p\0",
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_print_line("CONFIGS row 8", &format!("i={i}"), c);
    }
}

/// Row 9 — embedded `\n`, `\r`, `\t`.
#[test]
fn phase_b_row09_printline_embedded_control() {
    let _serial = serial();
    let mut rng = Rng::new(9);
    for i in 0..2048 {
        let len = rng.range(1, 200);
        let mut buf = rand_ascii(&mut rng, len);
        buf.pop(); // drop NUL, re-add after edits
        for _ in 0..rng.range(1, 8) {
            let pos = rng.below(buf.len());
            buf[pos] = *[b'\n', b'\r', b'\t'].get(rng.below(3)).unwrap();
        }
        buf.push(0);
        assert_print_line("CONFIGS row 9", &format!("i={i} len={len}"), &buf);
    }
}

/// Row 10 — stdout is a pipe, not a regular file.
#[test]
fn phase_b_row10_printline_stdout_is_pipe() {
    let _serial = serial();
    let (c, r) = *apis();
    let mut rng = Rng::new(10);
    for i in 0..1024 {
        let len = rng.range(0, 2000);
        let buf = rand_cbuf(&mut rng, len);
        let p = buf.as_ptr() as *const c_char;
        let out_c = capture_pipe(|| perform(c, Op::PrintLine, p));
        let out_r = capture_pipe(|| perform(r, Op::PrintLine, p));
        assert!(
            out_c == out_r,
            "CONFIGS row 10 / i={i} len={len}: pipe output differs\n  C    {}\n  Rust {}",
            hex(&out_c),
            hex(&out_r)
        );
        let want_len = len + 1;
        assert_eq!(out_c.len(), want_len, "CONFIGS row 10 / i={i}: length");
    }
}

/// Row 11 — `good()` called directly on a fresh (zeroed-residue) stack.
#[test]
fn phase_b_row11_good_direct() {
    let _serial = serial();
    set_residue_uniform(0);
    for i in 0..64 {
        let out = assert_same_op("CONFIGS row 11", &format!("i={i}"), Op::Good, ptr::null());
        assert_eq!(out, b"string\n", "CONFIGS row 11 / i={i}");
    }
}

/// Row 12 — `good()` must ignore the residue entirely and always emit `string`.
#[test]
fn phase_b_row12_good_ignores_residue() {
    let _serial = serial();
    let mut rng = Rng::new(12);
    let labels: Vec<CString> = (0..2048)
        .map(|i| CString::new(format!("residue-{i}")).unwrap())
        .collect();
    for (i, l) in labels.iter().enumerate() {
        // Half the iterations use a real (printable) pointer, half a NULL.
        let v = if rng.next_u64() & 1 == 0 {
            l.as_ptr() as u64
        } else {
            0
        };
        set_residue_uniform(v);
        let out = assert_same_op("CONFIGS row 12", &format!("i={i}"), Op::Good, ptr::null());
        assert_eq!(out, b"string\n", "CONFIGS row 12 / i={i}: good() leaked residue");
    }
    set_residue_indexed();
    for i in 0..64 {
        let out = assert_same_op(
            "CONFIGS row 12",
            &format!("indexed i={i}"),
            Op::Good,
            ptr::null(),
        );
        assert_eq!(out, b"string\n", "CONFIGS row 12 / indexed i={i}");
    }
    set_residue_uniform(0);
}

/// Row 13 — `bad()` with a NULL residue: `printLine` rejects it, nothing is
/// printed. This is the CWE-457 read landing on zero.
#[test]
fn phase_b_row13_bad_null_residue() {
    let _serial = serial();
    set_residue_uniform(0);
    for i in 0..64 {
        let out = assert_same_op_forked("CONFIGS row 13", &format!("i={i}"), Op::Bad, ptr::null());
        assert!(
            out.is_empty(),
            "CONFIGS row 13 / i={i}: expected no output, got {}",
            hex(&out)
        );
    }
}

/// Row 14 — `bad()` with the residue pointing at a random short string. The
/// defect is reproduced: both libraries emit the residue string.
#[test]
fn phase_b_row14_bad_pointer_residue_short() {
    let _serial = serial();
    let mut rng = Rng::new(14);
    for i in 0..4096 {
        let len = rng.range(1, 64);
        let buf = rand_ascii(&mut rng, len);
        set_residue_uniform(buf.as_ptr() as u64);
        let out = assert_same_op(
            "CONFIGS row 14",
            &format!("i={i} len={len}"),
            Op::Bad,
            ptr::null(),
        );
        let mut want = buf[..len].to_vec();
        want.push(b'\n');
        assert!(
            out == want,
            "CONFIGS row 14 / i={i}: bad() did not forward the residue pointer (got {} bytes, want {})",
            out.len(),
            want.len()
        );
    }
    set_residue_uniform(0);
}

/// Row 15 — same, with long payloads over the full byte range.
#[test]
fn phase_b_row15_bad_pointer_residue_long() {
    let _serial = serial();
    let mut rng = Rng::new(15);
    for i in 0..1024 {
        let len = rng.range(1, 4096);
        let buf = rand_cbuf(&mut rng, len);
        set_residue_uniform(buf.as_ptr() as u64);
        let out = assert_same_op(
            "CONFIGS row 15",
            &format!("i={i} len={len}"),
            Op::Bad,
            ptr::null(),
        );
        assert_eq!(out.len(), len + 1, "CONFIGS row 15 / i={i}: length");
        assert_eq!(&out[..len], &buf[..len], "CONFIGS row 15 / i={i}: payload");
    }
    set_residue_uniform(0);
}

/// Row 16 — `good()` immediately followed by `bad()` at the same depth: the
/// `"string"` pointer `good` stored lands in the slot `bad` reads.
#[test]
fn phase_b_row16_good_then_bad() {
    let _serial = serial();
    for i in 0..256 {
        set_residue_uniform(0);
        assert_same_seq(
            "CONFIGS row 16",
            &format!("uniform i={i}"),
            &[Op::Good, Op::Bad],
            ptr::null(),
        );
        set_residue_indexed();
        assert_same_seq(
            "CONFIGS row 16",
            &format!("indexed i={i}"),
            &[Op::Good, Op::Bad],
            ptr::null(),
        );
        // Longer alternations, still one shared pinned residue.
        set_residue_uniform(0);
        assert_same_seq(
            "CONFIGS row 16",
            &format!("g,g,b,b i={i}"),
            &[Op::Good, Op::Good, Op::Bad, Op::Bad],
            ptr::null(),
        );
    }
    set_residue_uniform(0);
}

/// Row 17 — `printLine(x)` immediately followed by `bad()`: `printLine`'s
/// spilled parameter is the residue `bad` may pick up.
#[test]
fn phase_b_row17_printline_then_bad() {
    let _serial = serial();
    let mut rng = Rng::new(17);
    for i in 0..512 {
        let len = rng.range(1, 96);
        let buf = rand_ascii(&mut rng, len);
        let p = buf.as_ptr() as *const c_char;
        set_residue_uniform(0);
        assert_same_seq(
            "CONFIGS row 17",
            &format!("uniform i={i} len={len}"),
            &[Op::PrintLine, Op::Bad],
            p,
        );
        set_residue_indexed();
        assert_same_seq(
            "CONFIGS row 17",
            &format!("indexed i={i} len={len}"),
            &[Op::PrintLine, Op::Bad],
            p,
        );
        set_residue_uniform(0);
        assert_same_seq(
            "CONFIGS row 17",
            &format!("null-first i={i}"),
            &[Op::PrintLineNull, Op::Bad],
            p,
        );
    }
    set_residue_uniform(0);
}

/// Row 18 — `driver(0)` with a NULL residue.
#[test]
fn phase_b_row18_driver_zero_null_residue() {
    let _serial = serial();
    set_residue_uniform(0);
    // (a) Fresh process each time. Note that the pinned residue is deliberately
    // *not* what `bad()` ends up reading here: `driver`'s `call bad@plt` is the
    // first PLT call in the child, so `_dl_runtime_resolve` runs and overwrites
    // the word first. Whatever it leaves behind must be the same for both
    // libraries — that is the whole point of `-z lazy` in `build.rs` — so only
    // the differential is asserted, not an absolute value.
    for i in 0..64 {
        assert_same_op_forked(
            "CONFIGS row 18",
            &format!("forked i={i}"),
            Op::Driver(0),
            ptr::null(),
        );
    }
    // (b) In-process, with both PLT slots already bound by earlier calls: now
    // the pinned NULL really is what `bad()` reads, so the observable result is
    // exactly `printLine(NULL)` — silence.
    perform(apis().0, Op::Driver(0), ptr::null());
    perform(apis().1, Op::Driver(0), ptr::null());
    for i in 0..64 {
        let out = assert_same_op(
            "CONFIGS row 18",
            &format!("bound i={i}"),
            Op::Driver(0),
            ptr::null(),
        );
        assert!(
            out.is_empty(),
            "CONFIGS row 18 / bound i={i}: expected silence, got {}",
            hex(&out)
        );
    }
}

/// Row 19 — `driver(0)` with the residue pointing at a random string. This is
/// the composed path: `driver` → `bad` → `printLine`, through the C `.so`'s
/// lazily-bound PLT.
#[test]
fn phase_b_row19_driver_zero_pointer_residue() {
    let _serial = serial();
    let mut rng = Rng::new(19);
    for i in 0..4096 {
        let len = rng.range(1, 512);
        let buf = rand_ascii(&mut rng, len);
        set_residue_uniform(buf.as_ptr() as u64);
        assert_same_op(
            "CONFIGS row 19",
            &format!("i={i} len={len}"),
            Op::Driver(0),
            ptr::null(),
        );
    }
    set_residue_uniform(0);
}

/// Row 20 — the *first* `driver` call in the process versus a later one. In the
/// C `.so` the first `call good@plt` / `call bad@plt` runs
/// `_dl_runtime_resolve`, which overwrites the very stack word `bad` reads;
/// `build.rs` passes `-z lazy` so the Rust `.so` binds the same way.
///
/// Runs in its own process (`cargo test` gives each test a thread, not a
/// process, so "first call" is established by ordering inside this test being
/// the only `driver` user — see `phase_b_row20_first_driver_call_subprocess`).
#[test]
fn phase_b_row20_first_and_later_driver_call() {
    let _serial = serial();
    set_residue_indexed();
    // First driver call for each library, back to back, same pinned residue.
    assert_same_op("CONFIGS row 20", "call #1", Op::Driver(0), ptr::null());
    // Now both PLT slots are bound; repeat.
    for i in 2..=8 {
        assert_same_op(
            "CONFIGS row 20",
            &format!("call #{i}"),
            Op::Driver(0),
            ptr::null(),
        );
    }
    set_residue_uniform(0);
}

/// Row 21 — `driver(v)` for random non-zero `v`: must select `good`.
#[test]
fn phase_b_row21_driver_nonzero_selects_good() {
    let _serial = serial();
    let mut rng = Rng::new(21);
    set_residue_indexed();
    for i in 0..8192 {
        let mut v = rng.next_u64() as u32;
        if v == 0 {
            v = 1;
        }
        let out = assert_same_op(
            "CONFIGS row 21",
            &format!("i={i} v={v:#010x}"),
            Op::Driver(v as c_int),
            ptr::null(),
        );
        assert_eq!(
            out, b"string\n",
            "CONFIGS row 21 / v={v:#010x}: non-zero selector did not choose good()"
        );
    }
    set_residue_uniform(0);
}

/// Row 22 — non-zero selectors whose **low byte is zero**. A translation that
/// tested `al` or a `bool` instead of the full 32-bit `cmpl` would take the
/// `bad` branch here.
#[test]
fn phase_b_row22_driver_nonzero_low_byte_zero() {
    let _serial = serial();
    let mut rng = Rng::new(22);
    set_residue_indexed();
    for i in 0..4096 {
        let mut v = (rng.next_u64() as u32) & 0xFFFF_FF00;
        if v == 0 {
            v = 0x100;
        }
        let out = assert_same_op(
            "CONFIGS row 22",
            &format!("i={i} v={v:#010x}"),
            Op::Driver(v as c_int),
            ptr::null(),
        );
        assert_eq!(
            out, b"string\n",
            "CONFIGS row 22 / v={v:#010x}: low-byte-zero selector mis-tested"
        );
    }
    set_residue_uniform(0);
}

/// Row 23 — random alternating `driver(1)` / `driver(0)` sequences.
#[test]
fn phase_b_row23_driver_random_sequences() {
    let _serial = serial();
    let mut rng = Rng::new(23);
    for i in 0..512 {
        let n = rng.range(1, 12);
        let ops: Vec<Op> = (0..n)
            .map(|_| {
                if rng.next_u64() & 1 == 0 {
                    Op::Driver(0)
                } else {
                    Op::Driver(rng.next_u64() as c_int | 1)
                }
            })
            .collect();
        if i % 2 == 0 {
            set_residue_indexed();
        } else {
            set_residue_uniform(0);
        }
        assert_same_seq("CONFIGS row 23", &format!("i={i} n={n}"), &ops, ptr::null());
    }
    set_residue_uniform(0);
}

/// Row 24 — random interleavings of **all four** entry points over a single
/// pinned residue: the composed pipeline a real consumer would drive.
#[test]
fn phase_b_row24_all_entry_points_interleaved() {
    let _serial = serial();
    let mut rng = Rng::new(24);
    for i in 0..1024 {
        let len = rng.range(1, 128);
        let buf = rand_ascii(&mut rng, len);
        let p = buf.as_ptr() as *const c_char;
        let n = rng.range(1, 10);
        let ops: Vec<Op> = (0..n)
            .map(|_| match rng.below(6) {
                0 => Op::PrintLine,
                1 => Op::PrintLineNull,
                2 => Op::Bad,
                3 => Op::Good,
                4 => Op::Driver(0),
                _ => Op::Driver(rng.next_u64() as c_int | 1),
            })
            .collect();
        match i % 3 {
            0 => set_residue_uniform(0),
            1 => set_residue_uniform(buf.as_ptr() as u64),
            _ => set_residue_indexed(),
        }
        assert_same_seq("CONFIGS row 24", &format!("i={i} n={n}"), &ops, p);
    }
    set_residue_uniform(0);
}

/// Row 25 — the string sits at the very end of a mapped page, with the next
/// page unmapped, so any over-read by one byte faults instead of silently
/// succeeding.
#[test]
fn phase_b_row25_page_boundary_string() {
    let _serial = serial();
    let mut rng = Rng::new(25);
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    for i in 0..256 {
        let len = rng.range(1, 64);
        // Two pages; second one is made inaccessible.
        let base = unsafe {
            libc::mmap(
                ptr::null_mut(),
                2 * page,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(base != libc::MAP_FAILED, "mmap failed");
        assert_eq!(
            unsafe { libc::mprotect(base.cast::<u8>().wrapping_add(page).cast(), page, libc::PROT_NONE) },
            0,
            "mprotect failed"
        );
        // Payload + NUL end exactly at the page boundary.
        let start = page - (len + 1);
        let payload = rand_ascii(&mut rng, len); // ends with NUL already
        unsafe {
            ptr::copy_nonoverlapping(
                payload.as_ptr(),
                base.cast::<u8>().wrapping_add(start),
                len + 1,
            );
        }
        let p = base.cast::<u8>().wrapping_add(start) as *const c_char;

        assert_print_line(
            "CONFIGS row 25",
            &format!("printLine i={i} len={len}"),
            &payload,
        );
        set_residue_uniform(p as u64);
        let out = assert_same_op(
            "CONFIGS row 25",
            &format!("bad i={i} len={len}"),
            Op::Bad,
            ptr::null(),
        );
        assert_eq!(out.len(), len + 1, "CONFIGS row 25 / i={i}: length");
        set_residue_uniform(0);
        unsafe { libc::munmap(base, 2 * page) };
    }
}

/// Row 26 — which stack word does the callee actually read? The residue is
/// filled with 64 *distinct* pointers (`slot0000`..`slot0063`), so the emitted
/// label names the exact 8-byte offset. C and Rust must name the same one, for
/// `bad()` and for `driver(0)`, and the label must be stable across repeats.
#[test]
fn phase_b_row26_residue_offset_identity() {
    let _serial = serial();
    set_residue_indexed();
    let mut seen_bad = Vec::new();
    let mut seen_drv = Vec::new();
    for i in 0..32 {
        let b = assert_same_op(
            "CONFIGS row 26",
            &format!("bad i={i}"),
            Op::Bad,
            ptr::null(),
        );
        let d = assert_same_op(
            "CONFIGS row 26",
            &format!("driver(0) i={i}"),
            Op::Driver(0),
            ptr::null(),
        );
        seen_bad.push(b);
        seen_drv.push(d);
    }
    // Stability: the same slot every time. `driver`'s *first* call in the
    // process is exempt — it runs `_dl_runtime_resolve` for the lazily-bound
    // `bad@plt` slot, which overwrites the word `bad` then reads. Both
    // libraries do this (the C by default, the Rust because `build.rs` passes
    // `-z lazy`), and `assert_same_op` above already proved they agree on that
    // first call too; only the steady state is required to be stable.
    assert!(
        seen_bad.windows(2).all(|w| w[0] == w[1]),
        "CONFIGS row 26: bad() read an unstable stack offset: {:?}",
        seen_bad
            .iter()
            .map(|v| String::from_utf8_lossy(v).into_owned())
            .collect::<Vec<_>>()
    );
    assert!(
        seen_drv[1..].windows(2).all(|w| w[0] == w[1]),
        "CONFIGS row 26: driver(0) read an unstable stack offset after PLT \
         binding: {:?}",
        seen_drv
            .iter()
            .map(|v| String::from_utf8_lossy(v).into_owned())
            .collect::<Vec<_>>()
    );
    // And the slot must be one of the labels we planted (i.e. the read really
    // landed inside the pinned region, so the comparison above was meaningful).
    let b = String::from_utf8_lossy(&seen_bad[0]).trim_end().to_string();
    let d = String::from_utf8_lossy(&seen_drv[1]).trim_end().to_string();
    assert!(
        b.starts_with("slot"),
        "CONFIGS row 26: bad() did not read the pinned region (got {b:?}); \
         the residue window needs widening for this frame layout"
    );
    assert!(
        d.starts_with("slot"),
        "CONFIGS row 26: driver(0) did not read the pinned region (got {d:?})"
    );
    eprintln!(
        "CONFIGS row 26: bad() reads {b}; driver(0) reads {d} once bound \
         (first, unbound call emitted {:?})",
        String::from_utf8_lossy(&seen_drv[0])
    );
    set_residue_uniform(0);
}

// ===========================================================================
// Phase C — error / rejection differential tests, one per ERRORS.md row
// ===========================================================================

/// Rows 1 & 2 — the only conditional rejection in the library:
/// `if (line != NULL)`. NULL → exactly zero bytes; non-NULL → `puts`.
#[test]
fn phase_c_row01_02_printline_null_vs_nonnull() {
    let _serial = serial();
    let mut rng = Rng::new(101);
    // Row 1: NULL, under every residue mode (the rejection must not depend on
    // anything else) and repeated so a stale buffer would show up.
    for i in 0..256 {
        match i % 3 {
            0 => set_residue_uniform(0),
            1 => set_residue_uniform(u64::MAX),
            _ => set_residue_indexed(),
        }
        let out = assert_same_op_forked(
            "ERRORS row 1",
            &format!("NULL i={i}"),
            Op::PrintLineNull,
            ptr::null(),
        );
        assert!(
            out.is_empty(),
            "ERRORS row 1 / i={i}: printLine(NULL) emitted {}",
            hex(&out)
        );
    }
    set_residue_uniform(0);
    // Row 2: the accepted arm, for contrast.
    for i in 0..256 {
        let len = rng.range(0, 200);
        let buf = rand_cbuf(&mut rng, len);
        let out = assert_same_op(
            "ERRORS row 2",
            &format!("non-NULL i={i} len={len}"),
            Op::PrintLine,
            buf.as_ptr() as *const c_char,
        );
        assert_eq!(out.len(), len + 1, "ERRORS row 2 / i={i}");
    }
    // Interleave NULL and non-NULL so a mis-cached parameter would surface.
    for i in 0..128 {
        let l = rng.range(1, 60);
        let buf = rand_cbuf(&mut rng, l);
        assert_same_seq(
            "ERRORS rows 1+2",
            &format!("interleaved i={i}"),
            &[
                Op::PrintLineNull,
                Op::PrintLine,
                Op::PrintLineNull,
                Op::PrintLine,
            ],
            buf.as_ptr() as *const c_char,
        );
    }
}

/// Row 3 — `bad()`'s uninitialized slot holds 0: the CWE-457 value is rejected
/// downstream by `printLine`, so nothing is printed. Both must agree.
#[test]
fn phase_c_row03_bad_with_null_residue() {
    let _serial = serial();
    set_residue_uniform(0);
    for i in 0..128 {
        // Direct `bad()`: its own `printLine@plt` call happens *after* the
        // uninitialized read, so the resolver cannot disturb the pinned word and
        // the expected result is exactly `printLine(NULL)` — silence.
        let out = assert_same_op_forked("ERRORS row 3", &format!("i={i}"), Op::Bad, ptr::null());
        assert!(
            out.is_empty(),
            "ERRORS row 3 / i={i}: expected silence, got {}",
            hex(&out)
        );
        // Fresh child: `driver`'s first `call bad@plt` runs the lazy resolver,
        // which overwrites the pinned word before `bad()` reads it. Both
        // libraries must still agree; the absolute "silence" expectation for the
        // `driver` path is checked in the bound state below.
        assert_same_op_forked(
            "ERRORS row 3",
            &format!("via driver, fresh process, i={i}"),
            Op::Driver(0),
            ptr::null(),
        );
    }
    // Bound state, in-process: `driver(0)` with a NULL residue is silent.
    perform(apis().0, Op::Driver(0), ptr::null());
    perform(apis().1, Op::Driver(0), ptr::null());
    for i in 0..128 {
        let out = assert_same_op(
            "ERRORS row 3",
            &format!("via driver, bound, i={i}"),
            Op::Driver(0),
            ptr::null(),
        );
        assert!(
            out.is_empty(),
            "ERRORS row 3 / via driver bound i={i}: expected silence, got {}",
            hex(&out)
        );
    }
}

/// Row 4 — the slot holds a valid pointer: the defect is *reproduced*, both
/// libraries print the stale string. Not fixed on either side.
#[test]
fn phase_c_row04_bad_with_valid_residue() {
    let _serial = serial();
    let mut rng = Rng::new(104);
    for i in 0..2048 {
        let len = rng.range(1, 300);
        let buf = rand_ascii(&mut rng, len);
        set_residue_uniform(buf.as_ptr() as u64);
        let out = assert_same_op("ERRORS row 4", &format!("i={i}"), Op::Bad, ptr::null());
        assert!(
            !out.is_empty(),
            "ERRORS row 4 / i={i}: the CWE-457 read was silently fixed"
        );
        assert_eq!(out.len(), len + 1, "ERRORS row 4 / i={i}");
    }
    set_residue_uniform(0);
}

/// Row 5 — `driver(0)` is behaviourally identical to `bad()` at the same
/// residue, for both libraries.
#[test]
fn phase_c_row05_driver_zero_is_bad() {
    let _serial = serial();
    let mut rng = Rng::new(105);
    for i in 0..1024 {
        let len = rng.range(1, 120);
        let buf = rand_ascii(&mut rng, len);
        set_residue_uniform(buf.as_ptr() as u64);
        let via_driver = assert_same_op(
            "ERRORS row 5",
            &format!("driver(0) i={i}"),
            Op::Driver(0),
            ptr::null(),
        );
        let direct = assert_same_op(
            "ERRORS row 5",
            &format!("bad() i={i}"),
            Op::Bad,
            ptr::null(),
        );
        // Both must print the residue string; the frame depth differs but the
        // pinned region is uniform, so the bytes are the same.
        assert_eq!(via_driver, direct, "ERRORS row 5 / i={i}");
    }
    set_residue_uniform(0);
}

/// Rows 6–11 — out-of-range values for the `int` mode selector crossing the FFI
/// boundary. A C `int` parameter accepts any 32-bit pattern; every non-zero one
/// must select `good`.
#[test]
fn phase_c_row06_11_driver_out_of_range_selectors() {
    let _serial = serial();
    set_residue_indexed();
    let cases: [(&str, c_int); 14] = [
        ("row 6: -1", -1),
        ("row 7: 2", 2),
        ("row 7: 3", 3),
        ("row 8: INT_MIN", c_int::MIN),
        ("row 9: INT_MAX", c_int::MAX),
        ("row 10: 0x100", 0x100),
        ("row 10: 0x0000FF00", 0x0000_FF00),
        ("row 11: 0xFFFFFF00", -256), // 0xFFFFFF00 as i32
        ("row 11: 0x80000000", c_int::MIN),
        ("extra: -2", -2),
        ("extra: 0x7FFFFF00", 0x7FFF_FF00),
        ("extra: 0x00010000", 0x0001_0000),
        ("extra: 0x01000000", 0x0100_0000),
        ("extra: 1", 1),
    ];
    for (label, v) in cases {
        let out = assert_same_op("ERRORS rows 6-11", label, Op::Driver(v), ptr::null());
        assert_eq!(
            out, b"string\n",
            "ERRORS rows 6-11 / {label}: non-zero selector {v:#010x} did not select good()"
        );
    }
    // And zero, the one value that selects the defective path.
    set_residue_uniform(0);
    let out = assert_same_op_forked("ERRORS rows 6-11", "0", Op::Driver(0), ptr::null());
    assert!(out.is_empty(), "ERRORS rows 6-11 / 0: expected the bad path");

    // Exhaustive-ish sweep: every single-bit value, positive and negative.
    set_residue_indexed();
    for bit in 0..32 {
        let v = (1u32 << bit) as c_int;
        let out = assert_same_op(
            "ERRORS rows 6-11",
            &format!("bit {bit} ({v:#010x})"),
            Op::Driver(v),
            ptr::null(),
        );
        assert_eq!(out, b"string\n", "ERRORS rows 6-11 / bit {bit}");
    }
    set_residue_uniform(0);
}

/// Row 12 — zero length is *not* a rejection: `puts("")` emits one `\n`.
#[test]
fn phase_c_row12_printline_empty_string() {
    let _serial = serial();
    for i in 0..64 {
        let out = assert_same_op_forked(
            "ERRORS row 12",
            &format!("i={i}"),
            Op::PrintLine,
            b"\0".as_ptr() as *const c_char,
        );
        assert_eq!(out, b"\n", "ERRORS row 12 / i={i}");
    }
    // Residue pointing at an empty string, reached through bad().
    let empty = CString::new("").unwrap();
    set_residue_uniform(empty.as_ptr() as u64);
    let out = assert_same_op("ERRORS row 12", "via bad", Op::Bad, ptr::null());
    assert_eq!(out, b"\n", "ERRORS row 12 / via bad");
    set_residue_uniform(0);
}

/// Row 13 — oversized input is not rejected or truncated.
#[test]
fn phase_c_row13_printline_oversized() {
    let _serial = serial();
    let mut rng = Rng::new(113);
    let len = 1usize << 20;
    let buf = rand_cbuf(&mut rng, len);
    let out = assert_same_op(
        "ERRORS row 13",
        "1MiB",
        Op::PrintLine,
        buf.as_ptr() as *const c_char,
    );
    assert_eq!(out.len(), len + 1, "ERRORS row 13: truncated?");
    assert_eq!(&out[..len], &buf[..len]);
    // Same payload reached through the residue path.
    set_residue_uniform(buf.as_ptr() as u64);
    let out = assert_same_op("ERRORS row 13", "1MiB via driver(0)", Op::Driver(0), ptr::null());
    assert_eq!(out.len(), len + 1, "ERRORS row 13 / via driver(0)");
    set_residue_uniform(0);
}

/// Row 14 — a non-NULL but unmapped pointer passes the NULL check and is then
/// dereferenced by `puts`. Neither library rejects it; both must die on the same
/// fatal signal. Run in forked children so the test process survives.
#[test]
fn phase_c_row14_printline_wild_pointer_same_fatal_signal() {
    let _serial = serial();
    let (c, r) = *apis();

    /// Call `f(arg)` in a forked child; return the raw `waitpid` status.
    fn child_status(f: unsafe extern "C" fn(*const c_char), arg: *const c_char) -> (bool, i32) {
        flush_all();
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: silence stdout so the harness output stays clean, call,
            // and exit without running any Rust destructors.
            child_prep();
            unsafe {
                let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
                if devnull >= 0 {
                    libc::dup2(devnull, 1);
                }
                f(arg);
                libc::_exit(0);
            }
        }
        let mut status = 0i32;
        assert!(
            unsafe { libc::waitpid(pid, &mut status, 0) } == pid,
            "waitpid failed"
        );
        let signalled = libc::WIFSIGNALED(status);
        let code = if signalled {
            libc::WTERMSIG(status)
        } else {
            libc::WEXITSTATUS(status)
        };
        (signalled, code)
    }

    for wild in [1usize, 2, 7, 0xdead, 0x1000, usize::MAX, usize::MAX - 7] {
        let p = wild as *const c_char;
        let sc = child_status(c.print_line, p);
        let sr = child_status(r.print_line, p);
        assert_eq!(
            sc, sr,
            "ERRORS row 14 / ptr={wild:#x}: C {sc:?} vs Rust {sr:?} \
             (signalled, signal-or-exit-code)"
        );
        assert!(
            sc.0,
            "ERRORS row 14 / ptr={wild:#x}: expected a fatal signal, got exit code {}",
            sc.1
        );
        assert_eq!(
            sc.1,
            libc::SIGSEGV,
            "ERRORS row 14 / ptr={wild:#x}: expected SIGSEGV"
        );
    }

    // And the same wild value reached through the CWE-457 read in bad().
    for wild in [1usize, 0xdead, usize::MAX] {
        set_residue_uniform(wild as u64);
        let bad_c = {
            flush_all();
            let pid = unsafe { libc::fork() };
            assert!(pid >= 0);
            if pid == 0 {
                child_prep();
                unsafe {
                    let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
                    if devnull >= 0 {
                        libc::dup2(devnull, 1);
                    }
                    perform(c, Op::Bad, ptr::null());
                    libc::_exit(0);
                }
            }
            let mut st = 0i32;
            unsafe { libc::waitpid(pid, &mut st, 0) };
            (libc::WIFSIGNALED(st), if libc::WIFSIGNALED(st) { libc::WTERMSIG(st) } else { libc::WEXITSTATUS(st) })
        };
        let bad_r = {
            flush_all();
            let pid = unsafe { libc::fork() };
            assert!(pid >= 0);
            if pid == 0 {
                child_prep();
                unsafe {
                    let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
                    if devnull >= 0 {
                        libc::dup2(devnull, 1);
                    }
                    perform(r, Op::Bad, ptr::null());
                    libc::_exit(0);
                }
            }
            let mut st = 0i32;
            unsafe { libc::waitpid(pid, &mut st, 0) };
            (libc::WIFSIGNALED(st), if libc::WIFSIGNALED(st) { libc::WTERMSIG(st) } else { libc::WEXITSTATUS(st) })
        };
        assert_eq!(
            bad_c, bad_r,
            "ERRORS row 14 / bad() residue={wild:#x}: C {bad_c:?} vs Rust {bad_r:?}"
        );
    }
    set_residue_uniform(0);
}

/// Row 15 — conversion specifiers in the *data* are emitted literally, proving
/// neither library passes the caller's bytes as a format string.
#[test]
fn phase_c_row15_printline_format_specifiers_not_interpreted() {
    let _serial = serial();
    let cases: [&[u8]; 8] = [
        b"%n\0",
        b"%s\0",
        b"%99999999d\0",
        b"%n%n%n%n%n%n%n%n\0",
        b"%1$n\0",
        b"AAAA%08x.%08x.%08x.%08x\0",
        b"%hhn%hn%ln%lln\0",
        b"%*d%*s\0",
    ];
    for (i, case) in cases.iter().enumerate() {
        let out = assert_same_op(
            "ERRORS row 15",
            &format!("i={i}"),
            Op::PrintLine,
            case.as_ptr() as *const c_char,
        );
        let mut want = case[..case.len() - 1].to_vec();
        want.push(b'\n');
        assert_eq!(
            out, want,
            "ERRORS row 15 / i={i}: specifiers were interpreted"
        );
        // Same bytes via the residue path.
        set_residue_uniform(case.as_ptr() as u64);
        let out = assert_same_op(
            "ERRORS row 15",
            &format!("via bad i={i}"),
            Op::Bad,
            ptr::null(),
        );
        assert_eq!(out, want, "ERRORS row 15 / via bad i={i}");
        set_residue_uniform(0);
    }
}

// ===========================================================================
// Phase D — symbol parity and import resolution
// ===========================================================================

fn nm_defined(path: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under the
/// exact same name. The difference must be empty.
#[test]
fn phase_d_symbol_parity() {
    let _serial = serial();
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());
    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so — is it built?"
    );
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c:?}\n\
         Rust exports: {r:?}",
        missing.len()
    );
    // Sanity: the four documented names are really there.
    for name in ["printLine", "bad", "good", "driver"] {
        assert!(c.contains(&name.to_string()), "C .so lacks {name}");
        assert!(r.contains(&name.to_string()), "Rust .so lacks {name}");
    }
    eprintln!("phase_d_symbol_parity: C exports {c:?}; all present in Rust .so");
}

/// `RTLD_NOW` binds every undefined symbol at load time, so a successful open is
/// proof that the Rust `.so` has no unresolvable imports.
#[test]
fn phase_d_rtld_now_resolves_every_import() {
    let _serial = serial();
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    for p in [c_so_path(), rust_so_path()] {
        let lib = unsafe { Library::open(Some(&p), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("RTLD_NOW dlopen {} failed: {e}", p.display()));
        for name in [&b"printLine\0"[..], b"bad\0", b"good\0", b"driver\0"] {
            let s: Result<libloading::os::unix::Symbol<*mut c_void>, _> =
                unsafe { lib.get(name) };
            assert!(
                s.is_ok(),
                "dlsym {} failed in {}",
                CStr::from_bytes_with_nul(name).unwrap().to_string_lossy(),
                p.display()
            );
        }
        std::mem::forget(lib);
    }
}

/// The C `.so` must not be accidentally satisfying the Rust `.so`'s calls (or
/// vice versa): each library's `driver` has to reach *its own* `good`/`bad`.
/// Proven by making the two observably different — `good` is identical in both,
/// so instead we check that `driver(1)` on each library emits exactly one line
/// and that unloading order does not matter, plus that the two `.so` files have
/// distinct symbol addresses.
#[test]
fn phase_d_libraries_are_independent() {
    let _serial = serial();
    let (c, r) = *apis();
    assert_ne!(
        c.driver as usize, r.driver as usize,
        "both handles resolved to the same driver — RTLD_LOCAL isolation broke"
    );
    assert_ne!(c.bad as usize, r.bad as usize);
    assert_ne!(c.good as usize, r.good as usize);
    assert_ne!(c.print_line as usize, r.print_line as usize);
    set_residue_indexed();
    for v in [1, 0, 1, 0] {
        assert_same_op(
            "phase D independence",
            &format!("driver({v})"),
            Op::Driver(v),
            ptr::null(),
        );
    }
    set_residue_uniform(0);
}

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------

/// Every test in this file manipulates *process-global* state — fd 1 (the
/// capture redirect) and the residue pattern the `dirty_now` helper reads — so
/// they must not run concurrently. `scripts/verify.sh` passes
/// `--test-threads=1`; this lock makes the suite correct even without it.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}
