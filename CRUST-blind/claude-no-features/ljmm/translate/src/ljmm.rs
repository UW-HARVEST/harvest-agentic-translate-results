use std::sync::Mutex;

const BUFFER_SZ: usize = 8192;
const DUMMY_BLK_SZ: usize = 12;
const ADDR_2G: usize = 0x80000000;
#[allow(dead_code)]
const ADDR_1G: usize = 0x40000000;

struct Ljmm {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    dummy_blk: Option<Vec<u8>>,

    map_file: Option<String>,

    buffer: Vec<u8>,
    buf_len: usize,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl Ljmm {
    const fn new() -> Self {
        Ljmm {
            page_size: 0,
            page_mask: 0,
            addr_upbound: 0,
            addr_lowbound: 0,
            dummy_blk: None,
            map_file: None,
            buffer: Vec::new(),
            buf_len: 0,
            os_take_care_1g_2g: true,
            init_succ: false,
        }
    }
}

fn global() -> &'static Mutex<Ljmm> {
    static INSTANCE: Mutex<Ljmm> = Mutex::new(Ljmm::new());
    &INSTANCE
}

/// Determine the system page size in a portable, safe way.
fn detect_page_size() -> usize {
    // Default to 4096 (the most common page size on modern systems).
    // Without unsafe FFI we cannot query sysconf directly, so we use the
    // canonical default which matches what ljmm_test_set_test_param uses.
    4096
}

/// Determine the current program break (sbrk(0)) without using unsafe code.
/// Since we cannot call sbrk() in pure safe Rust we approximate it with 0,
/// which represents "no lower bound" — callers can override via
/// `ljmm_test_set_test_param`.
fn detect_sbrk0() -> usize {
    0
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut state = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    // Default for OS_take_care_1G_2G is 1 (true) when STRESS_TEST is not set.
    state.os_take_care_1g_2g = true;

    state.addr_lowbound = detect_sbrk0();
    state.addr_upbound = ADDR_2G;

    state.page_size = detect_page_size();
    state.page_mask = state.page_size.wrapping_sub(1);

    // Step 1: simulate mmap of a small dummy block. In safe Rust we
    // simply allocate a Vec to model the placeholder.
    state.dummy_blk = Some(vec![0u8; DUMMY_BLK_SZ]);

    state.map_file = Some(String::from("/proc/self/maps"));

    // Step 2: reserve a buffer for reading /proc/$PID/maps content.
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
    let mut state = match global().lock() {
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
    let mut state = match global().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    state.map_file = Some(map_file.to_string());
    state.addr_lowbound = sbrk0;

    // page_size must be a power of two and non-zero.
    debug_assert!(page_size != 0 && (page_size as i64 & (page_size as i64 - 1)) == 0);

    let ps = page_size as usize;
    state.page_size = ps;
    state.page_mask = ps.wrapping_sub(1);
}
