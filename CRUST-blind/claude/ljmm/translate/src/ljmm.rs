use std::sync::Mutex;

const BUFFER_SZ: usize = 8192;
const DUMMY_BLK_SZ: usize = 12;
const ADDR_2G: usize = 0x80000000;
#[allow(dead_code)]
const ADDR_1G: usize = 0x40000000;

struct LjmmState {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    map_file: Option<String>,

    buffer: Vec<u8>,
    buf_len: usize,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl LjmmState {
    const fn new() -> Self {
        LjmmState {
            page_size: 0,
            page_mask: 0,
            addr_upbound: 0,
            addr_lowbound: 0,
            map_file: None,
            buffer: Vec::new(),
            buf_len: 0,
            os_take_care_1g_2g: true,
            init_succ: false,
        }
    }
}

fn get_page_size() -> usize {
    // Default page size; in pure-Rust context without libc/FFI we assume 4096.
    4096
}

static LJMM: Mutex<LjmmState> = Mutex::new(LjmmState::new());

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut state = match LJMM.lock() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // STRESS_TEST is not defined in the build, so default to managed by OS.
    state.os_take_care_1g_2g = true;

    // We cannot call sbrk in safe Rust without FFI; default to 0.
    state.addr_lowbound = 0;
    state.addr_upbound = ADDR_2G;

    state.page_size = get_page_size();
    state.page_mask = state.page_size.wrapping_sub(1);

    // Allocate a buffer for reading /proc/self/maps content.
    state.buffer = vec![0u8; BUFFER_SZ];
    state.buf_len = 0;

    // Reserve the equivalent of the dummy block (no-op in pure Rust).
    let _ = DUMMY_BLK_SZ;

    state.map_file = Some(String::from("/proc/self/maps"));

    state.init_succ = true;
    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    if let Ok(mut state) = LJMM.lock() {
        state.os_take_care_1g_2g = turn_on != 0;
    }
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    if let Ok(mut state) = LJMM.lock() {
        state.map_file = Some(map_file.to_string());
        state.addr_lowbound = sbrk0;

        // page-size must be a power-of-two
        debug_assert!(page_size > 0 && (page_size & (page_size - 1)) == 0);
        let ps = page_size as usize;
        state.page_size = ps;
        state.page_mask = ps.wrapping_sub(1);
    }
}
