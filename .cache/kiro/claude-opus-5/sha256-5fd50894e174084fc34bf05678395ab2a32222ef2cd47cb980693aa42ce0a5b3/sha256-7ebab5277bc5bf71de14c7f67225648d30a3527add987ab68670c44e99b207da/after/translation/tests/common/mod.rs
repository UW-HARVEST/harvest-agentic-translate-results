//! Shared harness: locates and loads the C and Rust shared libraries and
//! exposes a uniform way to call `smallestValue` through the FFI boundary.
//!
//! Both implementations are always exercised via `libloading`, i.e. through
//! their exported dynamic symbols, never by calling Rust code directly.

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// `struct ListNode` exactly as declared in `c_src/include/simplestruct.h`.
///
/// Defined locally (rather than imported from the crate) so the tests act as
/// an independent external caller compiled against the C header.
#[repr(C)]
pub struct ListNode {
    pub value: i32,
    pub next: *mut ListNode,
}

/// `int smallestValue(struct ListNode *)`
pub type SmallestValueFn = unsafe extern "C" fn(*mut ListNode) -> i32;

/// Repository root (the directory holding both `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the C shared library produced by the CMake build.
pub fn c_library_path() -> PathBuf {
    let path = repo_root().join("c_src/build/libSimpleList.so");
    assert!(
        path.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        path.display()
    );
    path
}

/// Path to the Rust `cdylib`. Prefers the profile the tests were built with,
/// but accepts whichever of debug/release is present.
///
/// Set `SIMPLELIST_RUST_SO` to test a specific artifact instead.
pub fn rust_library_path() -> PathBuf {
    if let Some(explicit) = std::env::var_os("SIMPLELIST_RUST_SO") {
        let path = PathBuf::from(explicit);
        assert!(
            path.exists(),
            "SIMPLELIST_RUST_SO points at a missing file: {}",
            path.display()
        );
        return path;
    }

    let target = repo_root().join("translation/target");
    // Ordered by preference: the profile matching this test binary first.
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };

    let mut tried = Vec::new();
    for profile in preferred {
        let path = target.join(profile).join("libSimpleList.so");
        if path.exists() {
            return path;
        }
        tried.push(path);
    }

    panic!(
        "Rust shared library not found. Looked in:{}\nBuild it with: cd translation && cargo build",
        tried
            .iter()
            .map(|p| format!("\n  {}", p.display()))
            .collect::<String>()
    );
}

/// A loaded implementation of the API under test.
pub struct Impl {
    /// Kept alive so the loaded symbols stay valid.
    _lib: Library,
    smallest_value: SmallestValueFn,
    name: &'static str,
}

impl Impl {
    fn load(path: &Path, name: &'static str) -> Self {
        // SAFETY: loading a shared library runs its initialisers; both
        // libraries here are plain leaf libraries built from this repo.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {} ({}): {e}", name, path.display()));

        let smallest_value = unsafe {
            let sym: Symbol<SmallestValueFn> = lib
                .get(b"smallestValue\0")
                .unwrap_or_else(|e| panic!("{name} does not export smallestValue: {e}"));
            *sym
        };

        Self {
            _lib: lib,
            smallest_value,
            name,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Calls `smallestValue` through the library's exported symbol.
    ///
    /// # Safety
    ///
    /// `head` must be NULL or point to a valid NULL-terminated `ListNode`
    /// chain, per the C contract.
    pub unsafe fn smallest_value(&self, head: *mut ListNode) -> i32 {
        unsafe { (self.smallest_value)(head) }
    }
}

/// The C and Rust implementations, both loaded dynamically.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load(&c_library_path(), "C libSimpleList.so"),
        rust: Impl::load(&rust_library_path(), "Rust libSimpleList.so"),
    }
}

/// Owns the storage backing a linked list so raw pointers stay valid.
pub struct ListStorage {
    nodes: Vec<Box<ListNode>>,
}

impl ListStorage {
    /// Builds a NULL-terminated chain from `values`. An empty slice yields a
    /// NULL head.
    pub fn new(values: &[i32]) -> Self {
        let mut nodes: Vec<Box<ListNode>> = values
            .iter()
            .map(|&value| {
                Box::new(ListNode {
                    value,
                    next: std::ptr::null_mut(),
                })
            })
            .collect();

        for i in (1..nodes.len()).rev() {
            let next: *mut ListNode = &mut *nodes[i];
            nodes[i - 1].next = next;
        }

        Self { nodes }
    }

    /// Head pointer for the chain, or NULL when empty.
    pub fn head(&mut self) -> *mut ListNode {
        match self.nodes.first_mut() {
            Some(node) => &mut **node,
            None => std::ptr::null_mut(),
        }
    }
}

/// Runs both implementations over `values` and asserts the results are
/// bit-identical.
pub fn assert_same(pair: &Pair, values: &[i32]) -> i32 {
    // Each implementation gets a freshly built list so neither can observe
    // mutations made by the other.
    let mut c_storage = ListStorage::new(values);
    let mut rust_storage = ListStorage::new(values);

    let c_result = unsafe { pair.c.smallest_value(c_storage.head()) };
    let rust_result = unsafe { pair.rust.smallest_value(rust_storage.head()) };

    assert_eq!(
        c_result.to_ne_bytes(),
        rust_result.to_ne_bytes(),
        "mismatch for input {values:?}: {} returned {c_result}, {} returned {rust_result}",
        pair.c.name(),
        pair.rust.name()
    );

    c_result
}
