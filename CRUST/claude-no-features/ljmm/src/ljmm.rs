use std::sync::Mutex;

const BUFFER_SZ: usize = 8192;
const ADDR_2G: usize = 0x8000_0000;
#[allow(dead_code)]
const ADDR_1G: usize = 0x4000_0000;

struct Ljmm {
    page_size: usize,
    page_mask: usize,
    addr_upbound: usize,
    addr_lowbound: usize,
    map_file: String,
    buffer: Vec<u8>,
    buf_len: usize,
    os_take_care_1g_2g: i32,
    init_succ: bool,
}

impl Ljmm {
    const fn new() -> Self {
        Ljmm {
            page_size: 0,
            page_mask: 0,
            addr_upbound: 0,
            addr_lowbound: 0,
            map_file: String::new(),
            buffer: Vec::new(),
            buf_len: 0,
            os_take_care_1g_2g: 1,
            init_succ: false,
        }
    }
}

fn global_state() -> &'static Mutex<Ljmm> {
    static STATE: Mutex<Ljmm> = Mutex::new(Ljmm::new());
    &STATE
}

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut s = global_state().lock().unwrap();
    s.os_take_care_1g_2g = 1;
    s.addr_lowbound = 0;
    s.addr_upbound = ADDR_2G;

    // Default page size: 4096 (typical for most systems).
    let page_size: usize = 4096;
    s.page_size = page_size;
    s.page_mask = page_size - 1;

    s.map_file = String::from("/proc/self/maps");
    s.buffer = vec![0u8; BUFFER_SZ];
    s.buf_len = 0;
    s.init_succ = true;
    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut s = global_state().lock().unwrap();
    s.os_take_care_1g_2g = turn_on;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut s = global_state().lock().unwrap();
    s.map_file = map_file.to_string();
    s.addr_lowbound = sbrk0;

    // page_size must be a positive power-of-two
    debug_assert!(page_size > 0 && (page_size & (page_size - 1)) == 0);
    let ps = page_size as usize;
    s.page_size = ps;
    s.page_mask = ps.wrapping_sub(1);
}
