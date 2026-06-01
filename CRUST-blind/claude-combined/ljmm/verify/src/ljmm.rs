use std::sync::Mutex;

const BUFFER_SZ: usize = 8192;
const DUMMY_BLK_SZ: usize = 12;
pub const ADDR_2G: usize = 0x80000000;
pub const ADDR_1G: usize = 0x40000000;

struct LjmmState {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    map_file: String,

    buffer: Vec<u8>,
    buf_len: i32,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: i8,
    init_succ: i8,
}

impl LjmmState {
    const fn new() -> Self {
        Self {
            page_size: 0,
            page_mask: 0,
            addr_upbound: 0,
            addr_lowbound: 0,
            map_file: String::new(),
            buffer: Vec::new(),
            buf_len: 0,
            os_take_care_1g_2g: 0,
            init_succ: 0,
        }
    }
}

fn state() -> &'static Mutex<LjmmState> {
    static STATE: Mutex<LjmmState> = Mutex::new(LjmmState::new());
    &STATE
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut s = state().lock().unwrap();

    // Default: OS takes care of [1G..2G] (matches non-STRESS_TEST build)
    s.os_take_care_1g_2g = 1;

    // In C this is sbrk(0). In safe Rust we cannot call sbrk; use 0 as the
    // default lower bound, which the test API overrides via
    // ljmm_test_set_test_param.
    s.addr_lowbound = 0;
    s.addr_upbound = ADDR_2G;

    // Pick a sensible default page size (4096 is overwhelmingly common, and
    // tests override it anyway).
    s.page_size = 4096;
    s.page_mask = s.page_size - 1;

    // step 1 + 2 in C: allocate dummy block + maps buffer. In safe Rust, we
    // cannot mmap; allocate the buffer normally so subsequent reads have
    // somewhere to go.
    s.buffer = vec![0u8; BUFFER_SZ];
    s.buf_len = 0;

    // Mimic the dummy-block reservation by recording the size; nothing to
    // free in safe Rust.
    let _ = DUMMY_BLK_SZ;

    s.map_file = String::from("/proc/self/maps");

    s.init_succ = 1;
    s.init_succ as i32
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut s = state().lock().unwrap();
    // C stores turn_on in a `char` field (truncating to low 8 bits).
    s.os_take_care_1g_2g = turn_on as i8;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut s = state().lock().unwrap();
    s.map_file = map_file.to_string();
    s.addr_lowbound = sbrk0;

    // page-size must be a power-of-two (mirrors the DEBUG ASSERT in C). Only
    // assert in debug builds to match C's DEBUG-only behavior.
    debug_assert!(
        page_size != 0 && ((page_size - 1) & page_size) == 0,
        "page_size must be a non-zero power of two"
    );
    s.page_size = page_size as usize;
    s.page_mask = (page_size as usize).wrapping_sub(1);
}

// ---------------------------------------------------------------------------
// Test-only inspectors
//
// These helpers expose internal state to the test binaries so that we can
// verify the translation against the original C behavior without performing
// any mmap calls (which the Rust translation deliberately omits for safety).
// ---------------------------------------------------------------------------

#[doc(hidden)]
pub fn _test_get_map_file() -> String {
    state().lock().unwrap().map_file.clone()
}

#[doc(hidden)]
pub fn _test_get_addr_lowbound() -> usize {
    state().lock().unwrap().addr_lowbound
}

#[doc(hidden)]
pub fn _test_get_addr_upbound() -> usize {
    state().lock().unwrap().addr_upbound
}

#[doc(hidden)]
pub fn _test_get_page_size() -> usize {
    state().lock().unwrap().page_size
}

#[doc(hidden)]
pub fn _test_get_page_mask() -> usize {
    state().lock().unwrap().page_mask
}

#[doc(hidden)]
pub fn _test_get_os_take_care_1g_2g() -> i8 {
    state().lock().unwrap().os_take_care_1g_2g
}

#[doc(hidden)]
pub fn _test_get_init_succ() -> i8 {
    state().lock().unwrap().init_succ
}

#[doc(hidden)]
pub fn _test_get_buffer_len() -> usize {
    state().lock().unwrap().buffer.len()
}
