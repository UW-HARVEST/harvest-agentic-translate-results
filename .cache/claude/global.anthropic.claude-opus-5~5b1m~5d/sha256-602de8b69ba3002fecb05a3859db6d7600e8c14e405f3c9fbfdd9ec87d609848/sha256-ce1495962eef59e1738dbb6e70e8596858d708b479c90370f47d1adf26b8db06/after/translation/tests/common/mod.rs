// Shared harness for the C-vs-Rust differential tests.
//
// Both implementations are loaded as shared objects via `libloading` and called
// only through their exported `smallestValue` symbol. The Rust function is never
// called directly, so the `#[no_mangle] extern "C"` wrapper and the cdylib's ABI
// are part of what gets tested.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::PathBuf;

use libloading::{Library, Symbol};

/// Byte-for-byte mirror of the C `struct ListNode`.
///
/// ```c
/// struct ListNode { int value; struct ListNode* next; };
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ListNode {
    pub value: c_int,
    pub next: *mut ListNode,
}

pub type SmallestValueFn = unsafe extern "C" fn(*mut ListNode) -> c_int;

/// Repository root (parent of the `translation` crate directory).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    repo_root().join("c_src/build/libSimpleList.so")
}

pub fn rust_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libSimpleList.so")
}

/// Holds both loaded libraries. The `Library` values must outlive every call
/// made through the function pointers extracted from them.
pub struct Both {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: SmallestValueFn,
    pub rust: SmallestValueFn,
}

fn load_one(path: &PathBuf, what: &str) -> Library {
    if !path.exists() {
        panic!(
            "missing {what} shared object at {}\n\
             Build both libraries first:\n\
               cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
               cd translation && cargo build --release",
            path.display()
        );
    }
    unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
}

impl Both {
    pub fn load() -> Self {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        let c_lib = load_one(&c_path, "C");
        let rust_lib = load_one(&rust_path, "Rust");

        let c = unsafe {
            let s: Symbol<SmallestValueFn> = c_lib
                .get(b"smallestValue\0")
                .expect("C .so does not export smallestValue");
            *s
        };
        let rust = unsafe {
            let s: Symbol<SmallestValueFn> = rust_lib
                .get(b"smallestValue\0")
                .expect("Rust .so does not export smallestValue");
            *s
        };

        Both {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }
}

/// A heap-allocated, NULL-terminated chain of `ListNode`s with a stable address.
///
/// The nodes live in a boxed slice that is never moved after the `next` pointers
/// are wired up, which is what makes handing `head()` across the FFI boundary
/// sound. This mimics what a real C consumer does: build the chain, then call.
pub struct List {
    nodes: Box<[ListNode]>,
}

impl List {
    /// Builds a chain whose values are `values` in order. An empty slice yields a
    /// list whose `head()` is NULL (the only way to express "zero nodes", since
    /// the C API has no length parameter).
    pub fn new(values: &[i32]) -> Self {
        let mut nodes: Box<[ListNode]> = values
            .iter()
            .map(|&value| ListNode {
                value,
                next: std::ptr::null_mut(),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // Wire `next` now that the allocation is fixed in place.
        let base = nodes.as_mut_ptr();
        let n = nodes.len();
        for i in 0..n.saturating_sub(1) {
            unsafe {
                (*base.add(i)).next = base.add(i + 1);
            }
        }
        List { nodes }
    }

    pub fn head(&mut self) -> *mut ListNode {
        if self.nodes.is_empty() {
            std::ptr::null_mut()
        } else {
            self.nodes.as_mut_ptr()
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Snapshot of (value, raw next) for every node. Only meaningful when
    /// compared against another snapshot of the SAME list, since raw addresses
    /// differ between separately-allocated chains.
    pub fn snapshot(&self) -> Vec<(i32, usize)> {
        self.nodes
            .iter()
            .map(|n| (n.value, n.next as usize))
            .collect()
    }

    /// Address-independent view: each node's value plus its link target expressed
    /// as an index into this list (`None` for NULL). Comparable across two
    /// different chains built from the same values.
    pub fn value_shape(&self) -> Vec<(i32, Option<isize>)> {
        let base = self.nodes.as_ptr();
        self.nodes
            .iter()
            .map(|n| {
                let link = if n.next.is_null() {
                    None
                } else {
                    Some(unsafe { (n.next as *const ListNode).offset_from(base) })
                };
                (n.value, link)
            })
            .collect()
    }
}

/// Calls both `.so` exports on the *same* chain and asserts the returned `int`s
/// are bit-identical. Returns the agreed value.
#[track_caller]
pub fn assert_same(both: &Both, values: &[i32], ctx: &str) -> i32 {
    let mut list_c = List::new(values);
    let mut list_rust = List::new(values);

    // Snapshot each chain against ITSELF; the two chains live at different
    // addresses, so their `next` pointers are legitimately different.
    let before_c = list_c.snapshot();
    let before_rust = list_rust.snapshot();

    let got_c = unsafe { (both.c)(list_c.head()) };
    let got_rust = unsafe { (both.rust)(list_rust.head()) };

    assert_eq!(
        got_c.to_ne_bytes(),
        got_rust.to_ne_bytes(),
        "divergence [{ctx}]: C returned {got_c} ({:#010x}), Rust returned {got_rust} ({:#010x})\n\
         input (len {}): {:?}",
        got_c,
        got_rust,
        values.len(),
        Preview(values),
    );

    // Neither implementation may mutate the caller's chain.
    assert_eq!(
        list_c.snapshot(),
        before_c,
        "C mutated the caller's chain [{ctx}]"
    );
    assert_eq!(
        list_rust.snapshot(),
        before_rust,
        "Rust mutated the caller's chain [{ctx}]"
    );
    // The observable shape (values + link topology as indices) must agree.
    assert_eq!(
        list_c.value_shape(),
        list_rust.value_shape(),
        "chain shape diverged [{ctx}]"
    );

    got_c
}

/// Truncating debug view so a failure on a 100k-node list stays readable.
struct Preview<'a>(&'a [i32]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 32 {
            write!(f, "{:?}", self.0)
        } else {
            write!(f, "{:?}... (+{} more)", &self.0[..32], self.0.len() - 32)
        }
    }
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    /// Uniform over the whole `i32` range, including `INT_MIN` and `INT_MAX`.
    pub fn i32_any(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Uniform in `lo..=hi`.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn usize_in(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + (self.next_u64() % (hi - lo + 1) as u64) as usize
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Reference minimum, mirroring the C traversal exactly (seed with head, then
/// strict `<` over the rest). Used as an independent third opinion.
pub fn reference_min(values: &[i32]) -> i32 {
    match values.split_first() {
        None => -1,
        Some((&first, rest)) => {
            let mut smallest = first;
            for &v in rest {
                if v < smallest {
                    smallest = v;
                }
            }
            smallest
        }
    }
}
