//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported `extern "C"` symbols — the Rust functions
//! are never called directly, so the `#[no_mangle]` wrappers are under test too.
//!
//! Paths are supplied by `scripts/run_all.sh` via environment variables:
//!   `DIFF_C_SO`, `DIFF_RUST_SO`, `DIFF_C_BIN`, `DIFF_RUST_BIN`.

#![allow(dead_code)]

use std::path::PathBuf;

pub type Op2 = extern "C" fn(i32, i32) -> i32;
pub type Op1 = extern "C" fn(i32) -> i32;

/* ------------------------------------------------------------------ */
/* Build configuration, resolved exactly like `mdmacros.rs` does.      */
/* ------------------------------------------------------------------ */

pub const OP: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

pub const REPEAT: i32 = if cfg!(feature = "7") {
    7
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "0") {
    0
} else {
    5
};

pub const INIT: i32 = if cfg!(feature = "mul") { 1 } else { 0 };

/// `STEP_OP(OP, acc, i)` — the reference (C-semantics) step.
pub fn step(acc: i32, i: i32) -> i32 {
    match OP {
        "mul" => acc.wrapping_mul(i.wrapping_add(1)),
        "sub" => acc.wrapping_sub(i),
        _ => acc.wrapping_add(i),
    }
}

/// `int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, n);` — statically unrolled `n` steps.
pub fn unrolled(n: i32) -> i32 {
    let mut acc = INIT;
    let mut i = 0;
    while i < n {
        acc = step(acc, i);
        i += 1;
    }
    acc
}

/// `accum_<OP>(n)`: `DISPATCH_REP`'s switch only has `case 0..6`; anything else
/// falls through `default:` and leaves the accumulator at `INIT_FOR(OP)`.
pub fn accum_ref(n: i32) -> i32 {
    if (0..=6).contains(&n) {
        unrolled(n)
    } else {
        INIT
    }
}

/* ------------------------------------------------------------------ */
/* Shared-object wrapper                                              */
/* ------------------------------------------------------------------ */

pub struct Lib {
    pub lib: libloading::Library,
    pub tag: &'static str,
}

impl Lib {
    pub fn open(path: &PathBuf, tag: &'static str) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), tag));
        Lib { lib, tag }
    }

    /// Look up an exported `int f(int,int)`.
    pub fn f2(&self, name: &str) -> Op2 {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe {
            *self
                .lib
                .get::<Op2>(&n)
                .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.tag))
        }
    }

    /// Look up an exported `int f(int)`.
    pub fn f1(&self, name: &str) -> Op1 {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe {
            *self
                .lib
                .get::<Op1>(&n)
                .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.tag))
        }
    }

    /// Address of an exported symbol (function or data), as a `usize`.
    pub fn addr(&self, name: &str) -> usize {
        let mut n = name.as_bytes().to_vec();
        n.push(0);
        unsafe {
            *self
                .lib
                .get::<*mut usize>(&n)
                .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.tag))
                as usize
        }
    }

    /// `&G_OP` — a pointer to the exported *writable* function-pointer object.
    pub fn g_op_slot(&self) -> *mut usize {
        self.addr("G_OP") as *mut usize
    }

    /// The current value of `G_OP`, as a callable function pointer.
    pub fn g_op(&self) -> Op2 {
        unsafe { std::mem::transmute::<usize, Op2>(*self.g_op_slot()) }
    }

    /// `&G_OP_NAME` — pointer to the exported writable `const char *` object.
    pub fn g_op_name_slot(&self) -> *mut *const u8 {
        self.addr("G_OP_NAME") as *mut *const u8
    }

    /// The NUL-terminated bytes `G_OP_NAME` points at (NUL excluded).
    pub fn g_op_name(&self) -> Vec<u8> {
        unsafe {
            let p = *self.g_op_name_slot();
            let mut out = Vec::new();
            let mut i = 0isize;
            while *p.offset(i) != 0 {
                out.push(*p.offset(i));
                i += 1;
                assert!(i < 64, "{}: G_OP_NAME not NUL-terminated", self.tag);
            }
            out
        }
    }
}

/// `dlopen` of the same path inside one process returns the *same* mapping, so
/// all tests in a binary share one `G_OP` object. Any test that reads or writes
/// `G_OP` must hold this lock, otherwise the mutating tests race the readers.
pub static G_OP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Acquire exclusive access to the shared `G_OP` objects.
pub fn g_op_guard() -> std::sync::MutexGuard<'static, ()> {
    G_OP_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn env_path(key: &str) -> PathBuf {
    PathBuf::from(std::env::var(key).unwrap_or_else(|_| {
        panic!("environment variable {key} is not set — run the tests via scripts/run_all.sh")
    }))
}

/// Open the C `.so` and the Rust `.so` for the *current* feature configuration.
pub fn pair() -> (Lib, Lib) {
    (
        Lib::open(&env_path("DIFF_C_SO"), "C"),
        Lib::open(&env_path("DIFF_RUST_SO"), "Rust"),
    )
}

pub fn c_bin() -> PathBuf {
    env_path("DIFF_C_BIN")
}
pub fn rust_bin() -> PathBuf {
    env_path("DIFF_RUST_BIN")
}

/* ------------------------------------------------------------------ */
/* Deterministic RNG (SplitMix64) — fixed seed for reproducibility.    */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Rng {
        Rng(0x5DEE_CE66_D)
    }
    pub fn seeded(s: u64) -> Rng {
        Rng(s)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform over the whole `i32` range.
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Biased toward small magnitudes / boundary-adjacent values, which is where
    /// value-dependent bugs live, mixed with full-range draws.
    pub fn i32_mixed(&mut self) -> i32 {
        let r = self.next_u64();
        match r % 5 {
            0 => (r >> 8) as u32 as i32,          // full range
            1 => ((r >> 8) % 65) as i32 - 32,     // tiny
            2 => ((r >> 8) % 131_072) as i32 - 65_536, // around +/-2^16
            3 => i32::MAX - ((r >> 8) % 8) as i32,
            _ => i32::MIN + ((r >> 8) % 8) as i32,
        }
    }
}

/// The boundary values every `int` parameter must be probed with.
pub const EDGE: [i32; 11] = [
    0,
    1,
    -1,
    2,
    -2,
    65_536,
    -65_536,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];
