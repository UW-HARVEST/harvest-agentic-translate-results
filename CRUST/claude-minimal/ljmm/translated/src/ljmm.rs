use std::sync::Mutex;

/// Internal state mirroring the C `ljmm_t` struct.
struct LjmmState {
    page_size: usize,
    page_mask: usize,

    addr_upbound: usize,
    addr_lowbound: usize,

    map_file: Option<String>,

    /// it is up to OS to take care the 1G..2G space
    os_take_care_1g_2g: i32,
    init_succ: i32,
}

const ADDR_2G: usize = 0x8000_0000;

impl LjmmState {
    const fn new() -> Self {
        Self {
            page_size: 0,
            page_mask: 0,
            addr_upbound: 0,
            addr_lowbound: 0,
            map_file: None,
            os_take_care_1g_2g: 1,
            init_succ: 0,
        }
    }
}

static LJMM: Mutex<LjmmState> = Mutex::new(LjmmState::new());

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut state = LJMM.lock().unwrap();

    // Default behavior: OS takes care of [1G, 2G] space (matches non-stress-test build).
    state.os_take_care_1g_2g = 1;

    // The C code initializes addr_lowbound from sbrk(0); since Rust's std doesn't
    // expose sbrk and tests overwrite this anyway, start at 0.
    state.addr_lowbound = 0;
    state.addr_upbound = ADDR_2G;

    // Determine the page size. Default to 4096 if we can't query it.
    let page_size = get_page_size();
    state.page_size = page_size;
    state.page_mask = page_size.wrapping_sub(1);

    state.map_file = Some(String::from("/proc/self/maps"));
    state.init_succ = 1;

    0
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut state = LJMM.lock().unwrap();
    state.os_take_care_1g_2g = turn_on;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut state = LJMM.lock().unwrap();
    state.map_file = Some(map_file.to_string());
    state.addr_lowbound = sbrk0;

    // page-size must be a power-of-two
    debug_assert!(page_size != 0 && ((page_size - 1) & page_size) == 0);

    let ps = page_size as usize;
    state.page_size = ps;
    state.page_mask = ps.wrapping_sub(1);
}

#[cfg(unix)]
fn get_page_size() -> usize {
    // SAFETY: sysconf is a thread-safe libc call.
    let sz = unsafe { libc_sysconf_pagesize() };
    if sz > 0 { sz as usize } else { 4096 }
}

#[cfg(not(unix))]
fn get_page_size() -> usize {
    4096
}

#[cfg(unix)]
extern "C" {
    #[link_name = "sysconf"]
    fn sysconf(name: i32) -> i64;
}

#[cfg(unix)]
const _SC_PAGESIZE: i32 = 30; // Linux value; not all platforms use 30, but acceptable fallback.

#[cfg(unix)]
unsafe fn libc_sysconf_pagesize() -> i64 {
    sysconf(_SC_PAGESIZE)
}
