//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared libraries with `libloading` and
//! driven exclusively through their exported C symbols — the Rust code is never
//! called directly, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]

use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libc::{c_char, c_int, c_void};
use libloading::{Library, Symbol};

// ============================================================================
// The two libraries
// ============================================================================

pub struct Libs {
    pub c: Library,
    pub rs: Library,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Path of the Rust `cdylib`. `cargo test` does not itself emit the `cdylib`
/// artifact, so `./run_all.sh` (or a plain `cargo build`) must have produced it.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test executable location")
        .to_path_buf();
    let candidate = profile_dir.join("libdriver.so");
    assert!(
        candidate.exists(),
        "{} not found -- build the cdylib first: `cargo build --offline` (or run ./run_all.sh)",
        candidate.display()
    );
    candidate
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = PathBuf::from(env!("C_SO_PATH"));
        assert!(
            c_path.exists(),
            "C shared library missing: {}",
            c_path.display()
        );
        let rs_path = rust_so_path();
        unsafe {
            let c = Library::new(&c_path).expect("dlopen C library");
            let rs = Library::new(&rs_path).expect("dlopen Rust library");
            Libs {
                c,
                rs,
                c_path,
                rs_path,
            }
        }
    })
}

pub fn c_lib() -> &'static Library {
    &libs().c
}

pub fn rs_lib() -> &'static Library {
    &libs().rs
}

/// Path of the C reference executable built by `build.rs`.
pub fn c_exe() -> PathBuf {
    PathBuf::from(env!("C_EXE_PATH"))
}

// ============================================================================
// stdout capture (file descriptor 1, shared by both libraries)
// ============================================================================

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Redirecting file descriptor 1 is process-global, so the harness must run one
/// test at a time — otherwise a concurrent test's output, or the harness' own
/// progress line, lands inside a capture. `.cargo/config.toml` sets
/// `RUST_TEST_THREADS = "1"`, so a plain `cargo test` is already serialized; this
/// guard makes a wrong invocation fail loudly instead of flakily.
fn assert_serialized() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let env_ok = std::env::var("RUST_TEST_THREADS")
            .map(|v| v.trim() == "1")
            .unwrap_or(false);
        let args: Vec<String> = std::env::args().collect();
        let arg_ok = args.iter().enumerate().any(|(i, a)| {
            a == "--test-threads=1"
                || (a == "--test-threads" && args.get(i + 1).map(String::as_str) == Some("1"))
        });
        assert!(
            env_ok || arg_ok,
            "these differential tests capture file descriptor 1 and must run \
             serialized; re-run with `--test-threads=1` or \
             `RUST_TEST_THREADS=1` (which .cargo/config.toml sets by default, \
             and ./run_all.sh passes explicitly)"
        );
    });
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// everything that was written. C's `stdout` is flushed before and after, so the
/// buffered `printf` output of the C library is included.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert_serialized();

    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let mut tmp = tempfile();
    let tmp_fd = as_fd(&tmp);

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(tmp_fd, 1) } >= 0, "dup2 failed");

    f();

    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };
    assert!(unsafe { libc::dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { libc::close(saved) };

    let mut buf = Vec::new();
    tmp.seek(std::io::SeekFrom::Start(0)).expect("seek");
    tmp.read_to_end(&mut buf).expect("read capture");
    buf
}

fn tempfile() -> std::fs::File {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(dir).join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create temp capture file");
    let _ = std::fs::remove_file(&path); // keep it anonymous
    file
}

fn as_fd(f: &std::fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    f.as_raw_fd()
}

/// Captures the output of the same operation from both libraries and asserts the
/// byte streams are identical.
pub fn assert_same_output<FC, FR>(what: &str, c_call: FC, rs_call: FR)
where
    FC: FnOnce(),
    FR: FnOnce(),
{
    let c_out = capture(c_call);
    let rs_out = capture(rs_call);
    if c_out != rs_out {
        panic!(
            "output mismatch in {what}\n--- C ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}\n--- first difference ---\n{}",
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            rs_out.len(),
            String::from_utf8_lossy(&rs_out),
            first_diff(&c_out, &rs_out),
        );
    }
}

pub fn first_diff(a: &[u8], b: &[u8]) -> String {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            let lo = i.saturating_sub(30);
            return format!(
                "byte {i}: C {:?} ({:#04x}) vs Rust {:?} ({:#04x})\ncontext C:    {:?}\ncontext Rust: {:?}",
                a[i] as char, a[i], b[i] as char, b[i],
                String::from_utf8_lossy(&a[lo..(i + 30).min(a.len())]),
                String::from_utf8_lossy(&b[lo..(i + 30).min(b.len())]),
            );
        }
    }
    format!("common prefix of {n} bytes, lengths {} vs {}", a.len(), b.len())
}

// ============================================================================
// Deterministic PRNG (SplitMix64)
// ============================================================================

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }

    /// Small signed integer, the shape the demos use.
    pub fn small_i32(&mut self) -> i32 {
        (self.next_u64() % 2001) as i32 - 1000
    }

    /// A double drawn from a mixture of "money-like" values, exact ties, and
    /// wholly random bit patterns (so NaN, inf and subnormals show up).
    pub fn f64_any(&mut self) -> f64 {
        match self.below(10) {
            0 => f64::from_bits(self.next_u64()),
            1 => SPECIAL_DOUBLES[self.below(SPECIAL_DOUBLES.len())],
            2 => -(self.below(1_000_000) as f64) / 100.0,
            3 => (self.below(1_000_000) as f64) / 8.0, // exact binary ties
            4 => (self.below(1 << 20) as f64) * 1e300,
            5 => (self.below(1 << 20) as f64) * 1e-300,
            _ => (self.below(100_000_000) as f64) / 100.0,
        }
    }

    /// "Money-like" positive double, as the demos use.
    pub fn price(&mut self) -> f64 {
        (self.below(200_000) as f64) / 100.0
    }

    pub fn ascii_string(&mut self, max_len: usize) -> Vec<u8> {
        let len = self.below(max_len + 1);
        (0..len)
            .map(|_| {
                let choice = self.below(16);
                match choice {
                    0 => b' ',
                    1 => b'.',
                    _ => b'A' + (self.below(26) as u8),
                }
            })
            .collect()
    }

    /// Arbitrary bytes (no NUL, since these become C strings).
    pub fn byte_string(&mut self, max_len: usize) -> Vec<u8> {
        let len = self.below(max_len + 1);
        (0..len)
            .map(|_| {
                let b = (self.next_u64() % 255) as u8;
                b.wrapping_add(1) // 1..=255, never NUL
            })
            .collect()
    }
}

pub const SPECIAL_DOUBLES: [f64; 24] = [
    0.0,
    -0.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
    -f64::NAN,
    f64::MIN_POSITIVE,
    5e-324,
    f64::MAX,
    f64::MIN,
    1.0,
    -1.0,
    0.5,
    0.125,
    0.375,
    0.005,
    1.005,
    2.675,
    0.015,
    99.995,
    1e308,
    -1e308,
    1e20,
    123456789.987654321,
];

pub const SPECIAL_INTS: [i32; 8] = [0, 1, -1, 2, -2, i32::MIN, i32::MAX, 1_000_000];

// ============================================================================
// C types (independent re-declarations, matching the verified C layout)
// ============================================================================

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_CATEGORY_LENGTH: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ItemT {
    pub id: c_int,
    pub name: [u8; MAX_NAME_LENGTH],
    pub category: [u8; MAX_CATEGORY_LENGTH],
    pub price: f64,
    pub quantity: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrderT {
    pub customer_id: c_int,
    pub customer_name: [u8; MAX_NAME_LENGTH],
    pub total_amount: f64,
}

#[repr(C)]
pub struct ArrayRaw<T> {
    pub data: *mut T,
    pub size: usize,
    pub capacity: usize,
}

#[repr(C)]
pub struct ListNodeRaw<T> {
    pub data: T,
    pub next: *mut ListNodeRaw<T>,
}

#[repr(C)]
pub struct ListRaw<T> {
    pub head: *mut ListNodeRaw<T>,
    pub tail: *mut ListNodeRaw<T>,
    pub size: usize,
}

/// NUL-terminated copy of `bytes`, for passing as `const char *`.
pub fn cstring(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ============================================================================
// Bitwise comparison (NaN payloads must match, not just "both NaN")
// ============================================================================

pub trait BitEq {
    fn bit_eq(&self, other: &Self) -> bool;
    fn describe(&self) -> String;
}

impl BitEq for c_int {
    fn bit_eq(&self, other: &Self) -> bool {
        self == other
    }
    fn describe(&self) -> String {
        format!("{self}")
    }
}

impl BitEq for f64 {
    fn bit_eq(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
    fn describe(&self) -> String {
        format!("{self} ({:#018x})", self.to_bits())
    }
}

impl BitEq for ItemT {
    fn bit_eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.category == other.category
            && self.price.to_bits() == other.price.to_bits()
            && self.quantity == other.quantity
    }
    fn describe(&self) -> String {
        format!(
            "item(id={}, name={:?}, category={:?}, price={:#018x}, quantity={})",
            self.id,
            String::from_utf8_lossy(&self.name),
            String::from_utf8_lossy(&self.category),
            self.price.to_bits(),
            self.quantity
        )
    }
}

impl BitEq for OrderT {
    fn bit_eq(&self, other: &Self) -> bool {
        self.customer_id == other.customer_id
            && self.customer_name == other.customer_name
            && self.total_amount.to_bits() == other.total_amount.to_bits()
    }
    fn describe(&self) -> String {
        format!(
            "order(id={}, name={:?}, total={:#018x})",
            self.customer_id,
            String::from_utf8_lossy(&self.customer_name),
            self.total_amount.to_bits()
        )
    }
}

// ============================================================================
// Element generation per instantiation
// ============================================================================

pub trait Elem: Copy + BitEq + 'static {
    /// Suffix used in the exported symbol names (`array_<SUFFIX>_create`).
    const SUFFIX: &'static str;
    fn rand(rng: &mut Rng) -> Self;
}

impl Elem for c_int {
    const SUFFIX: &'static str = "int";
    fn rand(rng: &mut Rng) -> Self {
        if rng.below(4) == 0 {
            SPECIAL_INTS[rng.below(SPECIAL_INTS.len())]
        } else {
            rng.i32()
        }
    }
}

impl Elem for f64 {
    const SUFFIX: &'static str = "double";
    fn rand(rng: &mut Rng) -> Self {
        rng.f64_any()
    }
}

impl Elem for ItemT {
    const SUFFIX: &'static str = "item_t";
    fn rand(rng: &mut Rng) -> Self {
        let mut name = [0u8; MAX_NAME_LENGTH];
        let mut category = [0u8; MAX_CATEGORY_LENGTH];
        let n = rng.ascii_string(70);
        let c = rng.ascii_string(40);
        let nl = n.len().min(MAX_NAME_LENGTH - 1);
        let cl = c.len().min(MAX_CATEGORY_LENGTH - 1);
        name[..nl].copy_from_slice(&n[..nl]);
        category[..cl].copy_from_slice(&c[..cl]);
        ItemT {
            id: rng.i32(),
            name,
            category,
            price: rng.f64_any(),
            quantity: rng.i32(),
        }
    }
}

impl Elem for OrderT {
    const SUFFIX: &'static str = "order_t";
    fn rand(rng: &mut Rng) -> Self {
        let mut customer_name = [0u8; MAX_NAME_LENGTH];
        let n = rng.ascii_string(70);
        let nl = n.len().min(MAX_NAME_LENGTH - 1);
        customer_name[..nl].copy_from_slice(&n[..nl]);
        OrderT {
            customer_id: rng.i32(),
            customer_name,
            total_amount: rng.f64_any(),
        }
    }
}

// ============================================================================
// Exported-symbol bindings
// ============================================================================

pub struct ArrayApi<T: 'static> {
    pub create: Symbol<'static, unsafe extern "C" fn(usize) -> *mut ArrayRaw<T>>,
    pub destroy: Symbol<'static, unsafe extern "C" fn(*mut ArrayRaw<T>)>,
    pub push: Symbol<'static, unsafe extern "C" fn(*mut ArrayRaw<T>, T) -> c_int>,
    pub get: Symbol<'static, unsafe extern "C" fn(*mut ArrayRaw<T>, usize) -> T>,
    pub size: Symbol<'static, unsafe extern "C" fn(*mut ArrayRaw<T>) -> usize>,
    pub clear: Symbol<'static, unsafe extern "C" fn(*mut ArrayRaw<T>)>,
}

impl<T: Elem> ArrayApi<T> {
    pub fn new(lib: &'static Library) -> ArrayApi<T> {
        let p = T::SUFFIX;
        unsafe {
            ArrayApi {
                create: sym(lib, &format!("array_{p}_create")),
                destroy: sym(lib, &format!("array_{p}_destroy")),
                push: sym(lib, &format!("array_{p}_push")),
                get: sym(lib, &format!("array_{p}_get")),
                size: sym(lib, &format!("array_{p}_size")),
                clear: sym(lib, &format!("array_{p}_clear")),
            }
        }
    }
}

pub struct ListApi<T: 'static> {
    pub create: Symbol<'static, unsafe extern "C" fn() -> *mut ListRaw<T>>,
    pub destroy: Symbol<'static, unsafe extern "C" fn(*mut ListRaw<T>)>,
    pub append: Symbol<'static, unsafe extern "C" fn(*mut ListRaw<T>, T) -> c_int>,
    pub prepend: Symbol<'static, unsafe extern "C" fn(*mut ListRaw<T>, T) -> c_int>,
    pub size: Symbol<'static, unsafe extern "C" fn(*mut ListRaw<T>) -> usize>,
    pub clear: Symbol<'static, unsafe extern "C" fn(*mut ListRaw<T>)>,
}

impl<T: Elem> ListApi<T> {
    pub fn new(lib: &'static Library) -> ListApi<T> {
        let p = T::SUFFIX;
        unsafe {
            ListApi {
                create: sym(lib, &format!("list_{p}_create")),
                destroy: sym(lib, &format!("list_{p}_destroy")),
                append: sym(lib, &format!("list_{p}_append")),
                prepend: sym(lib, &format!("list_{p}_prepend")),
                size: sym(lib, &format!("list_{p}_size")),
                clear: sym(lib, &format!("list_{p}_clear")),
            }
        }
    }
}

pub struct InventoryApi {
    pub print_item: Symbol<'static, unsafe extern "C" fn(ItemT)>,
    pub print_order: Symbol<'static, unsafe extern "C" fn(OrderT)>,
    pub create_item: Symbol<
        'static,
        unsafe extern "C" fn(c_int, *const c_char, *const c_char, f64, c_int) -> ItemT,
    >,
    pub create_order:
        Symbol<'static, unsafe extern "C" fn(c_int, *const c_char, f64) -> OrderT>,
    pub calculate_inventory_stats: Symbol<'static, unsafe extern "C" fn(*mut ArrayRaw<ItemT>)>,
    pub calculate_order_stats: Symbol<'static, unsafe extern "C" fn(*mut ListRaw<OrderT>)>,
    pub find_items_by_category:
        Symbol<'static, unsafe extern "C" fn(*mut ArrayRaw<ItemT>, *const c_char)>,
    pub find_expensive_items: Symbol<'static, unsafe extern "C" fn(*mut ListRaw<ItemT>, f64)>,
}

impl InventoryApi {
    pub fn new(lib: &'static Library) -> InventoryApi {
        unsafe {
            InventoryApi {
                print_item: sym(lib, "print_item"),
                print_order: sym(lib, "print_order"),
                create_item: sym(lib, "create_item"),
                create_order: sym(lib, "create_order"),
                calculate_inventory_stats: sym(lib, "calculate_inventory_stats"),
                calculate_order_stats: sym(lib, "calculate_order_stats"),
                find_items_by_category: sym(lib, "find_items_by_category"),
                find_expensive_items: sym(lib, "find_expensive_items"),
            }
        }
    }
}

pub struct DemoApi {
    pub print_menu: Symbol<'static, unsafe extern "C" fn()>,
    pub demo_integer_containers: Symbol<'static, unsafe extern "C" fn()>,
    pub demo_double_containers: Symbol<'static, unsafe extern "C" fn()>,
    pub demo_inventory_array: Symbol<'static, unsafe extern "C" fn()>,
    pub demo_order_list: Symbol<'static, unsafe extern "C" fn()>,
    pub demo_mixed_operations: Symbol<'static, unsafe extern "C" fn()>,
}

impl DemoApi {
    pub fn new(lib: &'static Library) -> DemoApi {
        unsafe {
            DemoApi {
                print_menu: sym(lib, "print_menu"),
                demo_integer_containers: sym(lib, "demo_integer_containers"),
                demo_double_containers: sym(lib, "demo_double_containers"),
                demo_inventory_array: sym(lib, "demo_inventory_array"),
                demo_order_list: sym(lib, "demo_order_list"),
                demo_mixed_operations: sym(lib, "demo_mixed_operations"),
            }
        }
    }
}

/// Looks a symbol up by name, failing loudly (a missing symbol is a translation
/// bug, not a test skip).
pub unsafe fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    lib.get::<T>(&bytes)
        .unwrap_or_else(|e| panic!("symbol {name} missing from library: {e}"))
}

/// Both libraries' bindings for one element type, plus a scratch RNG.
pub struct ArrayPair<T: 'static> {
    pub c: ArrayApi<T>,
    pub rs: ArrayApi<T>,
}

impl<T: Elem> ArrayPair<T> {
    pub fn new() -> ArrayPair<T> {
        ArrayPair {
            c: ArrayApi::new(c_lib()),
            rs: ArrayApi::new(rs_lib()),
        }
    }
}

pub struct ListPair<T: 'static> {
    pub c: ListApi<T>,
    pub rs: ListApi<T>,
}

impl<T: Elem> ListPair<T> {
    pub fn new() -> ListPair<T> {
        ListPair {
            c: ListApi::new(c_lib()),
            rs: ListApi::new(rs_lib()),
        }
    }
}

// ============================================================================
// Container inspection helpers
// ============================================================================

/// Reads back `(size, capacity, elements[0..size])` from an array header.
pub unsafe fn array_state<T: Copy>(arr: *mut ArrayRaw<T>) -> (usize, usize, Vec<T>) {
    assert!(!arr.is_null(), "array pointer is NULL");
    let size = (*arr).size;
    let capacity = (*arr).capacity;
    let mut elements = Vec::with_capacity(size);
    for i in 0..size {
        elements.push(*(*arr).data.add(i));
    }
    (size, capacity, elements)
}

/// Walks a list, returning `(size field, head/tail relationship, elements)`.
pub unsafe fn list_state<T: Copy>(list: *mut ListRaw<T>) -> (usize, bool, bool, Vec<T>) {
    assert!(!list.is_null(), "list pointer is NULL");
    let size = (*list).size;
    let mut elements = Vec::new();
    let mut node = (*list).head;
    let mut last = std::ptr::null_mut();
    while !node.is_null() {
        elements.push((*node).data);
        last = node;
        node = (*node).next;
    }
    let head_null = (*list).head.is_null();
    let tail_is_last = (*list).tail == last;
    (size, head_null, tail_is_last, elements)
}

pub fn assert_elems_eq<T: BitEq>(what: &str, c: &[T], rs: &[T]) {
    assert_eq!(
        c.len(),
        rs.len(),
        "{what}: element count {} (C) vs {} (Rust)",
        c.len(),
        rs.len()
    );
    for (i, (a, b)) in c.iter().zip(rs.iter()).enumerate() {
        assert!(
            a.bit_eq(b),
            "{what}: element {i} differs: C {} vs Rust {}",
            a.describe(),
            b.describe()
        );
    }
}

/// Frees a pointer with libc `free` (the same allocator both libraries use).
pub unsafe fn libc_free<T>(p: *mut T) {
    libc::free(p as *mut c_void);
}
