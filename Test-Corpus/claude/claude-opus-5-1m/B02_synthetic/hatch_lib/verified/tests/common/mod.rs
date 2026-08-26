//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls every function
//! through its exported C symbol — never through Rust linkage — so the
//! `#[no_mangle]` / `extern "C"` wrappers are part of what is under test.
//!
//! Both libraries keep private `static` state (`global_counter`,
//! `global_accumulator`). To keep the two copies in lockstep, *every* call is
//! issued to C and then to Rust while holding one global mutex, and the harness
//! additionally maintains a *shadow model* of that state so tests can assert
//! absolute expected values (guarding against vacuous "both agree on garbage").

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// DataRecord — must be ABI-identical to the C struct:
//   typedef struct { int id; int value; time_t timestamp; char name[32]; }
// x86-64 Linux: size 48, align 8, id@0 value@4 timestamp@8 name@16
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: i64,
    pub name: [c_char; 32],
}

impl DataRecord {
    pub fn zeroed() -> Self {
        DataRecord { id: 0, value: 0, timestamp: 0, name: [0; 32] }
    }
    pub fn as_bytes(&self) -> [u8; 48] {
        // SAFETY: DataRecord is repr(C), 48 bytes, no padding holes that matter
        // for comparison purposes (all 48 bytes are initialised by callers).
        unsafe { std::mem::transmute_copy(self) }
    }
}

pub const DATARECORD_SIZE: usize = 48;

// ---------------------------------------------------------------------------
// Function-pointer types for the 12 exported symbols
// ---------------------------------------------------------------------------
pub type FnVoid2 = unsafe extern "C" fn(c_int, c_int);
pub type FnOp3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
/// `op` is typed as a raw pointer (ABI-identical to a function pointer) so the
/// error-path tests can pass NULL and other invalid bit patterns.
pub type FnApplyOperation = unsafe extern "C" fn(*const c_void, c_int, c_int, c_int) -> c_int;
pub type FnShiftArrayData = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnProcessPointerData = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnCompute = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnGetTime = unsafe extern "C" fn(c_int) -> c_int;
pub type FnManipulateRecords = unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int;
pub type FnHatch = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Every exported symbol of one `.so`, resolved once.
pub struct Api {
    pub tag: &'static str,
    pub increment_counter: FnVoid2,
    pub update_accumulator: FnVoid2,
    pub apply_operation: FnApplyOperation,
    pub add_three: FnOp3,
    pub multiply_add: FnOp3,
    pub complex_calc: FnOp3,
    pub shift_array_data: FnShiftArrayData,
    pub process_pointer_data: FnProcessPointerData,
    pub compute_with_dynamic_memory: FnCompute,
    pub get_time_based_value: FnGetTime,
    pub manipulate_records: FnManipulateRecords,
    pub hatch: FnHatch,
    /// Raw addresses, for use as `apply_operation` callbacks.
    pub addr_add_three: *const c_void,
    pub addr_multiply_add: *const c_void,
    pub addr_complex_calc: *const c_void,
    _lib: Library,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe { lib.get(name) }
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

/// Address of an exported function, as an opaque pointer, so it can be handed
/// back to `apply_operation` as its `operation_func` argument.
fn fnaddr(f: FnOp3) -> *const c_void {
    f as usize as *const c_void
}

impl Api {
    fn load(tag: &'static str, path: &PathBuf) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("cannot dlopen {}: {e}", path.display()));
        unsafe {
            let add_three: FnOp3 = sym(&lib, b"add_three\0");
            let multiply_add: FnOp3 = sym(&lib, b"multiply_add\0");
            let complex_calc: FnOp3 = sym(&lib, b"complex_calc\0");
            Api {
                tag,
                increment_counter: sym(&lib, b"increment_counter\0"),
                update_accumulator: sym(&lib, b"update_accumulator\0"),
                apply_operation: sym(&lib, b"apply_operation\0"),
                add_three,
                multiply_add,
                complex_calc,
                shift_array_data: sym(&lib, b"shift_array_data\0"),
                process_pointer_data: sym(&lib, b"process_pointer_data\0"),
                compute_with_dynamic_memory: sym(&lib, b"compute_with_dynamic_memory\0"),
                get_time_based_value: sym(&lib, b"get_time_based_value\0"),
                manipulate_records: sym(&lib, b"manipulate_records\0"),
                hatch: sym(&lib, b"hatch\0"),
                addr_add_three: fnaddr(add_three),
                addr_multiply_add: fnaddr(multiply_add),
                addr_complex_calc: fnaddr(complex_calc),
                _lib: lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HATCH_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HATCH_RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe = <manifest>/target/<profile>/deps/<testbin>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    for cand in [
        profile_dir.join("libhatch_lib.so"),
        manifest_dir().join("target/debug/libhatch_lib.so"),
        manifest_dir().join("target/release/libhatch_lib.so"),
    ] {
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust cdylib libhatch_lib.so not found; run `cargo build` first");
}

pub struct Both {
    pub c: Api,
    pub r: Api,
}

static BOTH: OnceLock<Both> = OnceLock::new();

pub fn both() -> &'static Both {
    BOTH.get_or_init(|| Both {
        c: Api::load("C", &c_so_path()),
        r: Api::load("Rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Shadow model of the two libraries' private `static` state
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, Default)]
pub struct Shadow {
    pub counter: i32,
    pub accum: i32,
}

static STATE: Mutex<Shadow> = Mutex::new(Shadow { counter: 0, accum: 0 });

/// Serialised access to both libraries + the shadow model.
pub struct H {
    pub c: &'static Api,
    pub r: &'static Api,
    guard: MutexGuard<'static, Shadow>,
}

pub fn harness() -> H {
    let b = both();
    let guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    H { c: &b.c, r: &b.r, guard }
}

impl H {
    pub fn state(&self) -> Shadow {
        *self.guard
    }

    // ---- pure leaves ---------------------------------------------------
    pub fn add_three(&self, a: i32, b: i32, c: i32) -> i32 {
        let vc = unsafe { (self.c.add_three)(a, b, c) };
        let vr = unsafe { (self.r.add_three)(a, b, c) };
        assert_eq!(vc, vr, "add_three({a},{b},{c}) C={vc} Rust={vr}");
        let model = a.wrapping_add(b).wrapping_add(c);
        assert_eq!(vc, model, "add_three({a},{b},{c}) model mismatch");
        vc
    }

    pub fn multiply_add(&self, a: i32, b: i32, c: i32) -> i32 {
        let vc = unsafe { (self.c.multiply_add)(a, b, c) };
        let vr = unsafe { (self.r.multiply_add)(a, b, c) };
        assert_eq!(vc, vr, "multiply_add({a},{b},{c}) C={vc} Rust={vr}");
        let model = a.wrapping_mul(b).wrapping_add(c);
        assert_eq!(vc, model, "multiply_add({a},{b},{c}) model mismatch");
        vc
    }

    pub fn complex_calc(&self, a: i32, b: i32, c: i32) -> i32 {
        let vc = unsafe { (self.c.complex_calc)(a, b, c) };
        let vr = unsafe { (self.r.complex_calc)(a, b, c) };
        assert_eq!(
            vc, vr,
            "complex_calc({a},{b},{c}) @counter={} C={vc} Rust={vr}",
            self.guard.counter
        );
        let model = a.wrapping_sub(b).wrapping_mul(c).wrapping_add(self.guard.counter);
        assert_eq!(
            vc, model,
            "complex_calc({a},{b},{c}) @counter={} model mismatch",
            self.guard.counter
        );
        vc
    }

    pub fn get_time_based_value(&self, seed: i32) -> i32 {
        let vc = unsafe { (self.c.get_time_based_value)(seed) };
        let vr = unsafe { (self.r.get_time_based_value)(seed) };
        assert_eq!(vc, vr, "get_time_based_value({seed}) C={vc} Rust={vr}");
        assert_eq!(vc, model_gtbv(seed), "get_time_based_value({seed}) model mismatch");
        vc
    }

    pub fn compute_with_dynamic_memory(&self, base: i32, count: i32) -> i32 {
        let vc = unsafe { (self.c.compute_with_dynamic_memory)(base, count) };
        let vr = unsafe { (self.r.compute_with_dynamic_memory)(base, count) };
        assert_eq!(vc, vr, "compute_with_dynamic_memory({base},{count}) C={vc} Rust={vr}");
        assert_eq!(
            vc,
            model_compute(base, count),
            "compute_with_dynamic_memory({base},{count}) model mismatch"
        );
        vc
    }

    // ---- state mutators ------------------------------------------------
    pub fn increment_counter(&mut self, value: i32, unused: i32) {
        unsafe { (self.c.increment_counter)(value, unused) };
        unsafe { (self.r.increment_counter)(value, unused) };
        self.guard.counter = self.guard.counter.wrapping_add(value);
    }

    pub fn update_accumulator(&mut self, value: i32, unused: i32) {
        unsafe { (self.c.update_accumulator)(value, unused) };
        unsafe { (self.r.update_accumulator)(value, unused) };
        self.guard.accum = self.guard.accum.wrapping_mul(2).wrapping_add(value);
    }

    /// Drive `global_counter` to an exact value in both libraries.
    pub fn set_counter(&mut self, target: i32) {
        let delta = target.wrapping_sub(self.guard.counter);
        self.increment_counter(delta, 0);
        assert_eq!(self.guard.counter, target);
    }

    /// Drive `global_accumulator` to an exact value in both libraries.
    /// `acc' = acc*2 + v`, so `v = target - acc*2`.
    pub fn set_accum(&mut self, target: i32) {
        let v = target.wrapping_sub(self.guard.accum.wrapping_mul(2));
        self.update_accumulator(v, 0);
        assert_eq!(self.guard.accum, target);
    }

    // ---- higher-order --------------------------------------------------
    /// Calls C's `apply_operation` with C's callback and Rust's with Rust's.
    pub fn apply_operation_own(&self, which: Which, a: i32, b: i32, c: i32) -> i32 {
        let vc = unsafe { (self.c.apply_operation)(which.addr(self.c), a, b, c) };
        let vr = unsafe { (self.r.apply_operation)(which.addr(self.r), a, b, c) };
        assert_eq!(vc, vr, "apply_operation({which:?},{a},{b},{c}) C={vc} Rust={vr}");
        assert_eq!(
            vc,
            which.model(a, b, c, self.guard.counter),
            "apply_operation({which:?},{a},{b},{c}) model mismatch"
        );
        vc
    }

    /// Calls both `apply_operation`s with the *same* explicit callback address.
    pub fn apply_operation_with(
        &self,
        op: *const c_void,
        a: i32,
        b: i32,
        c: i32,
        model: i32,
    ) -> i32 {
        let vc = unsafe { (self.c.apply_operation)(op, a, b, c) };
        let vr = unsafe { (self.r.apply_operation)(op, a, b, c) };
        assert_eq!(vc, vr, "apply_operation(<explicit>,{a},{b},{c}) C={vc} Rust={vr}");
        assert_eq!(vc, model, "apply_operation(<explicit>,{a},{b},{c}) model mismatch");
        vc
    }

    // ---- pointer / buffer functions ------------------------------------
    /// Runs `shift_array_data` on identical copies of `data` (with `pad` guard
    /// elements past the end) and compares the return-side effects byte-for-byte.
    pub fn shift_array_data(&self, data: &[i32], size: i32, shift_by: i32, pad: usize) -> Vec<i32> {
        let mut bc: Vec<i32> = data.to_vec();
        let mut br: Vec<i32> = data.to_vec();
        bc.extend(std::iter::repeat(0x5A5A_5A5Ai32).take(pad));
        br.extend(std::iter::repeat(0x5A5A_5A5Ai32).take(pad));
        unsafe { (self.c.shift_array_data)(bc.as_mut_ptr(), size, shift_by) };
        unsafe { (self.r.shift_array_data)(br.as_mut_ptr(), size, shift_by) };
        assert_eq!(
            bc, br,
            "shift_array_data(size={size}, shift_by={shift_by}) buffers diverged\n C   ={bc:?}\n Rust={br:?}"
        );
        bc
    }

    pub fn process_pointer_data(&self, arr: &[i32], idx: usize, multiplier: i32) -> i32 {
        let mut ac = arr.to_vec();
        let mut ar = arr.to_vec();
        let vc = unsafe { (self.c.process_pointer_data)(ac.as_mut_ptr().add(idx), multiplier) };
        let vr = unsafe { (self.r.process_pointer_data)(ar.as_mut_ptr().add(idx), multiplier) };
        assert_eq!(
            vc, vr,
            "process_pointer_data(*={}, mult={multiplier}) @accum={} C={vc} Rust={vr}",
            arr[idx], self.guard.accum
        );
        assert_eq!(ac, ar, "process_pointer_data must not modify the buffer");
        assert_eq!(&ac[..], arr, "process_pointer_data must not modify the buffer");
        let model = arr[idx].wrapping_mul(multiplier).wrapping_add(self.guard.accum);
        assert_eq!(vc, model, "process_pointer_data model mismatch");
        vc
    }

    /// Runs `manipulate_records` on identical copies (plus `pad` guard records)
    /// and compares both the return value and the full post-call byte image.
    pub fn manipulate_records(
        &self,
        recs: &[DataRecord],
        num_records: i32,
        shift: i32,
        pad: usize,
    ) -> (i32, Vec<DataRecord>) {
        let guard_rec = DataRecord {
            id: 0x5A5A_5A5A,
            value: 0x3C3C_3C3C,
            timestamp: 0x1234_5678_9ABC_DEF0,
            name: [0x41; 32],
        };
        let mut rc: Vec<DataRecord> = recs.to_vec();
        let mut rr: Vec<DataRecord> = recs.to_vec();
        rc.extend(std::iter::repeat(guard_rec).take(pad));
        rr.extend(std::iter::repeat(guard_rec).take(pad));
        let vc = unsafe { (self.c.manipulate_records)(rc.as_mut_ptr(), num_records, shift) };
        let vr = unsafe { (self.r.manipulate_records)(rr.as_mut_ptr(), num_records, shift) };
        assert_eq!(
            vc, vr,
            "manipulate_records(n={num_records}, shift={shift}) C={vc} Rust={vr}"
        );
        // Full byte image comparison (catches any DataRecord stride/offset skew).
        let bytes_c: Vec<u8> = rc.iter().flat_map(|r| r.as_bytes()).collect();
        let bytes_r: Vec<u8> = rr.iter().flat_map(|r| r.as_bytes()).collect();
        assert_eq!(
            bytes_c, bytes_r,
            "manipulate_records(n={num_records}, shift={shift}) memmove side effect diverged"
        );
        (vc, rc)
    }

    pub fn hatch(&mut self, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
        // Model must be computed against the pre-call state.
        let model = model_hatch(&mut *self.guard, p1, p2, p3, p4);
        let vc = unsafe { (self.c.hatch)(p1, p2, p3, p4) };
        let vr = unsafe { (self.r.hatch)(p1, p2, p3, p4) };
        assert_eq!(vc, vr, "hatch({p1},{p2},{p3},{p4}) C={vc} Rust={vr}");
        assert_eq!(vc, model, "hatch({p1},{p2},{p3},{p4}) model mismatch");
        vc
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Which {
    AddThree,
    MultiplyAdd,
    ComplexCalc,
}

impl Which {
    pub fn addr(self, api: &Api) -> *const c_void {
        match self {
            Which::AddThree => api.addr_add_three,
            Which::MultiplyAdd => api.addr_multiply_add,
            Which::ComplexCalc => api.addr_complex_calc,
        }
    }
    pub fn model(self, a: i32, b: i32, c: i32, counter: i32) -> i32 {
        match self {
            Which::AddThree => a.wrapping_add(b).wrapping_add(c),
            Which::MultiplyAdd => a.wrapping_mul(b).wrapping_add(c),
            Which::ComplexCalc => a.wrapping_sub(b).wrapping_mul(c).wrapping_add(counter),
        }
    }
}

// ---------------------------------------------------------------------------
// Independent models of the C semantics (transcribed from c_src/src/lib.c)
// ---------------------------------------------------------------------------

/// `int get_time_based_value(int seed)`
///
/// `reference_time = current_time - (seed*3600)` so
/// `difftime(current, reference) == (double)(time_t)(int)(seed*3600)`
/// — independent of the wall clock.
pub fn model_gtbv(seed: i32) -> i32 {
    let x = seed.wrapping_mul(3600) as i64;
    let diff = x as f64;
    ((diff / 100.0) as i32).wrapping_add(seed)
}

/// `int compute_with_dynamic_memory(int base, int count)`
pub fn model_compute(base: i32, count: i32) -> i32 {
    let mut sum: i32 = 0;
    let mut i: i32 = 0;
    while i < count {
        sum = sum.wrapping_add(base.wrapping_add(i.wrapping_mul(3)));
        i = i.wrapping_add(1);
    }
    sum
}

/// `int manipulate_records(DataRecord*, int, int)` for the in-bounds cases.
pub fn model_manipulate(values: &[i32], num_records: i32, shift: i32) -> i32 {
    let mut v: Vec<i32> = values.to_vec();
    if shift > 0 && shift < num_records {
        let n = (num_records - shift) as usize;
        let s = shift as usize;
        for i in 0..n {
            v[i] = values[i + s];
        }
    }
    let mut total: i32 = 0;
    let mut i: i32 = 0;
    while i < num_records.wrapping_sub(shift) {
        total = total.wrapping_add(v[i as usize]);
        i = i.wrapping_add(1);
    }
    total
}

/// `void shift_array_data(int*, int, int)`
pub fn model_shift(data: &[i32], size: i32, shift_by: i32) -> Vec<i32> {
    let mut v = data.to_vec();
    if shift_by > 0 && shift_by < size {
        let s = shift_by as usize;
        let rem = (size - shift_by) as usize;
        for i in 0..rem {
            v[i] = data[i + s];
        }
        for i in 0..s {
            v[rem + i] = 0;
        }
    }
    v
}

/// `int hatch(int,int,int,int)` — advances `st` exactly as the C statics do.
pub fn model_hatch(st: &mut Shadow, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    // mod_func = increment_counter; mod_func(param1, 999);
    st.counter = st.counter.wrapping_add(p1);
    // mod_func = update_accumulator; mod_func(param2, 888);
    st.accum = st.accum.wrapping_mul(2).wrapping_add(p2);

    let mut r: i32 = 0;
    // add_three(p1,p2,p3)
    r = r.wrapping_add(p1.wrapping_add(p2).wrapping_add(p3));
    // multiply_add(p2,p3,p4)
    r = r.wrapping_add(p2.wrapping_mul(p3).wrapping_add(p4));
    // complex_calc(p1,p3,p4)
    r = r.wrapping_add(p1.wrapping_sub(p3).wrapping_mul(p4).wrapping_add(st.counter));
    // process_pointer_data(&dynamic_data[5], p2) where dynamic_data[i] = p1+i
    r = r.wrapping_add(p1.wrapping_add(5).wrapping_mul(p2).wrapping_add(st.accum));
    // shift_array_data(dynamic_data,10,3); result += dynamic_data[0] (== p1+3)
    r = r.wrapping_add(p1.wrapping_add(3));
    // get_time_based_value(p3)
    r = r.wrapping_add(model_gtbv(p3));
    // manipulate_records(records,5,2) with records[i].value = p4 + i*10
    //   -> (p4+20) + (p4+30) + (p4+40)
    r = r.wrapping_add(
        p4.wrapping_add(20)
            .wrapping_add(p4.wrapping_add(30))
            .wrapping_add(p4.wrapping_add(40)),
    );
    // compute_with_dynamic_memory(p1, 8) == 8*p1 + 84
    r = r.wrapping_add(model_compute(p1, 8));
    // global_counter + global_accumulator
    r = r.wrapping_add(st.counter.wrapping_add(st.accum));
    r
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x5DEE_CE66_D;

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
    /// Uniform over the whole `i32` range.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// A value biased toward interesting magnitudes.
    pub fn spicy_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => self.range(-1000, 1000),
            _ => self.next_i32(),
        }
    }
}

/// Interesting scalar corner values.
pub const EDGE: [i32; 11] = [
    i32::MIN,
    i32::MIN + 1,
    -1_000_000,
    -1000,
    -1,
    0,
    1,
    1000,
    1_000_000,
    i32::MAX - 1,
    i32::MAX,
];

pub fn random_records(rng: &mut Rng, n: usize) -> Vec<DataRecord> {
    (0..n)
        .map(|i| {
            let mut name = [0i8; 32];
            for b in name.iter_mut() {
                *b = (rng.next_u64() & 0xFF) as u8 as i8;
            }
            DataRecord {
                id: rng.next_i32(),
                value: if i % 3 == 0 { rng.spicy_i32() } else { rng.next_i32() },
                timestamp: rng.next_u64() as i64,
                name,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Master row lists. `tests/doc_coverage.rs` asserts these are exactly the rows
// documented in CONFIGS.md / ERRORS.md, so a doc row can never go untested and
// a test can never claim a row that isn't documented.
// ---------------------------------------------------------------------------

/// Every row of `CONFIGS.md` (Phase B) — exercised by `tests/valid_paths.rs`.
pub const CONFIG_ROWS: &[&str] = &[
    "C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "C11", "C12", "C13", "C14",
    "C15", "C16", "C17", "C18", "C19", "C20", "C21", "C22", "C23", "C24", "C25", "C26", "C27",
    "C28", "C29", "C30", "C31", "C32", "C33", "C34", "C35", "C36", "C37", "C38", "C39", "C40",
    "C41", "C42", "C43", "C44", "C45", "C46", "C47", "C48", "C49", "C50", "C51", "C52", "C53",
    "C54", "C55", "C56",
];

/// `ERRORS.md` rows whose C behaviour is a value/no-op — `tests/error_paths.rs`.
pub const ERROR_ROWS_NONFATAL: &[&str] = &[
    "E1", "E2", "E3", "E4", "E5", "E6", "E7", "E8", "E9", "E10", "E13", "E14", "E15", "E16",
    "E17", "E18", "E19", "E20", "E21", "E22", "E23", "E24", "E25", "E26", "E27", "E28", "E29",
    "E30", "E34", "E35", "E36", "E37", "E38", "E39", "E40", "E41", "E42",
];

/// `ERRORS.md` rows whose C behaviour is a fatal signal — `tests/crash_parity.rs`.
pub const ERROR_ROWS_FATAL: &[&str] = &["E11", "E12", "E31", "E32", "E33"];

/// The 12 symbols the C `.so` exports (Phase D) — `tests/symbol_parity.rs`.
pub const EXPECTED_SYMBOLS: &[&str] = &[
    "add_three",
    "apply_operation",
    "complex_calc",
    "compute_with_dynamic_memory",
    "get_time_based_value",
    "hatch",
    "increment_counter",
    "manipulate_records",
    "multiply_add",
    "process_pointer_data",
    "shift_array_data",
    "update_accumulator",
];

/// Row-coverage bookkeeping so a doc row can never silently go untested.
pub struct Coverage {
    seen: std::collections::BTreeSet<String>,
}

impl Coverage {
    pub fn new() -> Coverage {
        Coverage { seen: std::collections::BTreeSet::new() }
    }
    pub fn hit(&mut self, row: &str) {
        self.seen.insert(row.to_string());
    }
    pub fn assert_complete(&self, expected: &[&str], what: &str) {
        let exp: std::collections::BTreeSet<String> =
            expected.iter().map(|s| s.to_string()).collect();
        let missing: Vec<&String> = exp.difference(&self.seen).collect();
        let extra: Vec<&String> = self.seen.difference(&exp).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "{what} row coverage mismatch\n  untested rows: {missing:?}\n  unknown rows:  {extra:?}"
        );
        println!("{what}: all {} rows exercised", exp.len());
    }
}
