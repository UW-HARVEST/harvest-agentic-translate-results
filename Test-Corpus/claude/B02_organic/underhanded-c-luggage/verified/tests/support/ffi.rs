// Shared scaffolding for the shared-library (FFI) differential tests: the
// C-layout node struct, the exported function signatures, `dlopen` helpers,
// randomized node generators and the file-descriptor based stdout capture used
// for `printMatchingDirectives`.

#![allow(dead_code)]

use super::{c_so, rust_so, Rng, ALNUM};
use libloading::os::unix::{Library as UnixLibrary, Symbol as UnixSymbol};
use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub const LUG: usize = 8;
pub const FLT: usize = 6;
pub const AIR: usize = 3;
pub const COM: usize = 80;

/// Byte-for-byte mirror of the C `RoutingDirective` declared in
/// `c_src/src/luggage.c` (size 120, offsets 0/4/13/20/24/28/112).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    pub time_stamp: u32,
    pub luggage_id: [u8; LUG + 1],
    pub flight_id: [u8; FLT + 1],
    pub departure: [u8; AIR + 1],
    pub arrival: [u8; AIR + 1],
    pub comments: [u8; COM + 1],
    pub next: *mut Node,
}

impl Node {
    pub fn zeroed() -> Node {
        Node {
            time_stamp: 0,
            luggage_id: [0; LUG + 1],
            flight_id: [0; FLT + 1],
            departure: [0; AIR + 1],
            arrival: [0; AIR + 1],
            comments: [0; COM + 1],
            next: std::ptr::null_mut(),
        }
    }
}

pub type FnAdd = unsafe extern "C" fn(*mut Node, *mut Node);
pub type FnSupersedes = unsafe extern "C" fn(*mut Node, *const c_char, *const c_char) -> c_int;
pub type FnSuperseded = unsafe extern "C" fn(*mut Node) -> c_int;
pub type FnMatches = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnPrint = unsafe extern "C" fn(
    *mut Node,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
);

pub fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e))
}

pub fn libs() -> (Library, Library) {
    (load(c_so()), load(rust_so()))
}

pub fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: libloading::Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("dlsym {}: {}", String::from_utf8_lossy(name), e));
        *s
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// NUL-terminated C string from bytes.
pub fn cs(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// Writes `src` into a fixed size C char array: the value, a NUL terminator and
/// random garbage in the remaining bytes (everything past the NUL must be
/// ignored by both implementations).
pub fn set_field(dst: &mut [u8], src: &[u8], rng: &mut Rng) {
    let max = dst.len() - 1;
    let n = src.len().min(max);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
    for b in dst.iter_mut().skip(n + 1) {
        *b = match rng.below(4) {
            0 => 0,
            1 => *rng.pick(ALNUM),
            2 => rng.byte(),
            _ => b'x',
        };
    }
}

/// Random field content: mixes the shapes the parser can produce with shapes it
/// cannot (lower case, punctuation, spaces, bytes >= 0x80, empty).
pub fn gen_ffi_field(rng: &mut Rng, max: usize) -> Vec<u8> {
    let pool: &[u8] = b"ABCXYZ012abcxyz -_.#\xff\x80\x01";
    let len = match rng.below(6) {
        0 => 0,
        1 => 1,
        2 => max,
        _ => rng.below(max + 1),
    };
    (0..len).map(|_| *rng.pick(pool)).collect()
}

pub fn gen_node(rng: &mut Rng, lugs: &[Vec<u8>], deps: &[Vec<u8>], ts_pool: usize) -> Node {
    let mut n = Node::zeroed();
    n.time_stamp = match rng.below(4) {
        0 => rng.below(ts_pool.max(1)) as u32,
        1 => rng.next_u64() as u32,
        2 => *rng.pick(&[0u32, 1, u32::MAX, u32::MAX - 1, 2147483648, 2147483647]),
        _ => rng.below(1000) as u32,
    };
    let lug = rng.pick(lugs).clone();
    let dep = rng.pick(deps).clone();
    let flt = gen_ffi_field(rng, FLT);
    let arr = gen_ffi_field(rng, AIR);
    let com = gen_ffi_field(rng, COM);
    set_field(&mut n.luggage_id, &lug, rng);
    set_field(&mut n.flight_id, &flt, rng);
    set_field(&mut n.departure, &dep, rng);
    set_field(&mut n.arrival, &arr, rng);
    set_field(&mut n.comments, &com, rng);
    n
}

/// Links `nodes[0..n]` into a chain in array order.  Returns the head pointer.
pub fn link(nodes: &mut Vec<Node>) -> *mut Node {
    if nodes.is_empty() {
        return std::ptr::null_mut();
    }
    let base = nodes.as_mut_ptr();
    let len = nodes.len();
    for i in 0..len {
        nodes[i].next = if i + 1 < len {
            unsafe { base.add(i + 1) }
        } else {
            std::ptr::null_mut()
        };
    }
    base
}

/// Walks a C chain and reports the visited nodes as indices into `base`
/// (`extra` is the address of a node that lives outside the array).
pub unsafe fn chain_indices(head: *mut Node, base: *const Node, len: usize, extra: *const Node) -> Vec<i64> {
    let mut out = Vec::new();
    let mut p = head;
    let mut guard = 0;
    while !p.is_null() {
        let idx = if p as *const Node == extra {
            -1i64
        } else {
            let off = (p as usize).wrapping_sub(base as usize) / std::mem::size_of::<Node>();
            assert!(off < len, "pointer outside the node array");
            off as i64
        };
        out.push(idx);
        p = (*p).next;
        guard += 1;
        assert!(guard < 10_000, "cycle in the chain");
    }
    out
}

// --- stdout capture (needed for printMatchingDirectives) -------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: AtomicUsize = AtomicUsize::new(0);

pub struct Libc {
    _this: UnixLibrary,
    dup: UnixSymbol<unsafe extern "C" fn(c_int) -> c_int>,
    dup2: UnixSymbol<unsafe extern "C" fn(c_int, c_int) -> c_int>,
    close: UnixSymbol<unsafe extern "C" fn(c_int) -> c_int>,
    fflush: UnixSymbol<unsafe extern "C" fn(*mut c_void) -> c_int>,
}

pub fn libc() -> Libc {
    let this = UnixLibrary::this();
    unsafe {
        let dup = this.get(b"dup\0").expect("dlsym dup");
        let dup2 = this.get(b"dup2\0").expect("dlsym dup2");
        let close = this.get(b"close\0").expect("dlsym close");
        let fflush = this.get(b"fflush\0").expect("dlsym fflush");
        Libc {
            _this: this,
            dup,
            dup2,
            close,
            fflush,
        }
    }
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// everything that was written (after flushing the C stdio buffers).
pub fn capture_stdout(libc: &Libc, f: impl FnOnce()) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap();
    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("luggage_cap_{}_{}.out", std::process::id(), seq));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();
    unsafe {
        (libc.fflush)(std::ptr::null_mut());
        let saved = (libc.dup)(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!((libc.dup2)(fd, 1) >= 0, "dup2 failed");
        f();
        (libc.fflush)(std::ptr::null_mut());
        assert!((libc.dup2)(saved, 1) >= 0, "restore dup2 failed");
        (libc.close)(saved);
    }
    drop(file);
    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

