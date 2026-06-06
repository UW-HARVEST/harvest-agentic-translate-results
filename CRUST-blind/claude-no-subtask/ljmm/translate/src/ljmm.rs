use std::sync::Mutex;

const ADDR_2G: usize = 0x80000000;
const BUFFER_SZ: usize = 8192;
#[allow(dead_code)]
const DUMMY_BLK_SZ: usize = 12;

/// Internal global state mirroring the C `ljmm_t` struct.
struct LjmmState {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    map_file: String,

    buffer: Vec<u8>,
    buf_len: usize,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: bool,
    init_succ: bool,

    /// dummy_blk is represented as a marker that allocation succeeded.
    dummy_blk_allocated: bool,
}

impl LjmmState {
    const fn new() -> Self {
        LjmmState {
            page_size: 0,
            page_mask: 0,
            addr_upbound: 0,
            addr_lowbound: 0,
            map_file: String::new(),
            buffer: Vec::new(),
            buf_len: 0,
            os_take_care_1g_2g: false,
            init_succ: false,
            dummy_blk_allocated: false,
        }
    }
}

static LJMM: Mutex<LjmmState> = Mutex::new(LjmmState::new());

/// Returns a default page size (4096 bytes is the typical Linux page size).
fn default_page_size() -> usize {
    4096
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut state = match LJMM.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    // In the C code, OS_take_care_1G_2G defaults to 1 (unless STRESS_TEST).
    state.os_take_care_1g_2g = true;

    // sbrk(0) returns the current program break; in Rust we cannot safely
    // query that, so we default to 0. Tests override this via
    // `ljmm_test_set_test_param`.
    state.addr_lowbound = 0;
    state.addr_upbound = ADDR_2G;

    let page_size = default_page_size();
    state.page_size = page_size;
    state.page_mask = page_size - 1;

    // Step 1: "mmap a tiny block to prevent heap from growing".
    // In safe Rust we cannot call mmap; treat the dummy-block allocation as
    // a logical step that always succeeds.
    state.dummy_blk_allocated = true;

    state.map_file = String::from("/proc/self/maps");

    // Step 2: create buffer for reading content from /proc/$PID/maps.
    state.buffer = vec![0u8; BUFFER_SZ];
    state.buf_len = 0;

    state.init_succ = true;

    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut state = match LJMM.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.os_take_care_1g_2g = turn_on != 0;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut state = match LJMM.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    state.map_file = map_file.to_string();
    state.addr_lowbound = sbrk0;

    // page_size must be a power-of-two (matching C ASSERT). If invalid, fall
    // back to the default to keep the system in a usable state instead of
    // aborting.
    let ps = page_size as usize;
    if page_size > 0 && (ps & (ps - 1)) == 0 {
        state.page_size = ps;
        state.page_mask = ps - 1;
    } else {
        let ps = default_page_size();
        state.page_size = ps;
        state.page_mask = ps - 1;
    }
}
