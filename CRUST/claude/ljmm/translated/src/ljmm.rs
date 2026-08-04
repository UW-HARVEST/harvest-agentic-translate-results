use std::sync::Mutex;

const BUFFER_SZ: usize = 8192;

struct LjmmState {
    page_size: usize,
    page_mask: usize,
    addr_upbound: usize,
    addr_lowbound: usize,
    map_file: Option<String>,
    buffer: Vec<u8>,
    buf_len: usize,
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

impl LjmmState {
    const fn new() -> Self {
        Self {
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

const ADDR_2G: usize = 0x80000000;

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

    // Default flag: let OS take care the [1G..2G] space.
    s.os_take_care_1g_2g = true;

    // Mimic sbrk(0) lower-bound. Use 0 as the default placeholder, the test
    // helper `ljmm_test_set_test_param` may override this.
    s.addr_lowbound = 0;
    s.addr_upbound = ADDR_2G;

    // Reasonable default page size of 4096; tests may override via
    // `ljmm_test_set_test_param`.
    let page_size: usize = 4096;
    s.page_size = page_size;
    s.page_mask = page_size - 1;

    // Allocate the in-memory buffer used to read /proc/$PID/maps.
    s.buffer = vec![0u8; BUFFER_SZ];
    s.buf_len = 0;

    s.map_file = Some(String::from("/proc/self/maps"));
    s.init_succ = true;

    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut s = state().lock().unwrap();
    s.os_take_care_1g_2g = turn_on != 0;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut s = state().lock().unwrap();
    s.map_file = Some(map_file.to_string());
    s.addr_lowbound = sbrk0;

    // page-size must be a power-of-two and non-zero.
    let ps = page_size as usize;
    debug_assert!(ps != 0 && (ps & (ps - 1)) == 0);
    s.page_size = ps;
    s.page_mask = ps.wrapping_sub(1);
}
