use std::ffi::CString;
use std::os::raw::{c_int, c_long, c_void};

const BUFFER_SZ: usize = 8192;
const DUMMY_BLK_SZ: usize = 12;
const ADDR_2G: usize = 0x80000000;

const PROT_READ: c_int = 0x1;
const PROT_WRITE: c_int = 0x2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const SC_PAGESIZE: c_int = 30;

extern "C" {
    fn sbrk(increment: isize) -> *mut c_void;
    fn sysconf(name: c_int) -> c_long;
    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
}

struct LjmmState {
    page_size: usize,
    page_mask: usize,
    addr_upbound: usize,
    addr_lowbound: usize,
    dummy_blk: *mut c_void,
    map_file: Option<CString>,
    buffer: *mut c_void,
    buf_len: i32,
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

static mut LJMM: LjmmState = LjmmState {
    page_size: 0,
    page_mask: 0,
    addr_upbound: 0,
    addr_lowbound: 0,
    dummy_blk: std::ptr::null_mut(),
    map_file: None,
    buffer: std::ptr::null_mut(),
    buf_len: 0,
    os_take_care_1g_2g: false,
    init_succ: false,
};

#[inline]
fn map_failed() -> *mut c_void {
    !0usize as *mut c_void
}

/// Initializes the ljmm system.
///
/// Mirrors the C `__attribute__((constructor)) static void ljmm_init(void)`:
/// it records the current program break, page size, allocates a tiny dummy
/// block right after `sbrk(0)` to prevent the heap from growing into the
/// reserved region, and allocates a buffer used to read `/proc/self/maps`.
///
/// # Returns
/// `0` on successful initialization, `-1` otherwise.
pub fn ljmm_init() -> i32 {
    unsafe {
        // Match the non-STRESS_TEST default: let the OS take care of [1G..2G].
        LJMM.os_take_care_1g_2g = true;

        LJMM.addr_lowbound = sbrk(0) as usize;
        LJMM.addr_upbound = ADDR_2G;

        LJMM.page_size = sysconf(SC_PAGESIZE) as usize;
        LJMM.page_mask = LJMM.page_size.wrapping_sub(1);

        // Step 1: mmap a tiny block to prevent heap from growing, thereby
        // reserving the space [sbrk(0) .. 2G].
        let p = mmap(
            sbrk(0),
            DUMMY_BLK_SZ,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        if p == map_failed() {
            return -1;
        }
        LJMM.dummy_blk = p;

        LJMM.map_file = Some(CString::new("/proc/self/maps").unwrap());

        // Step 2: create buffer for reading content from /proc/$PID/maps.
        let p = mmap(
            std::ptr::null_mut(),
            BUFFER_SZ,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        if p == map_failed() {
            return -1;
        }
        LJMM.buffer = p;
        LJMM.buf_len = 0;

        LJMM.init_succ = true;
        0
    }
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    unsafe {
        LJMM.os_take_care_1g_2g = turn_on != 0;
    }
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size. Must be a power of two.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    unsafe {
        LJMM.map_file = Some(CString::new(map_file).unwrap());
        LJMM.addr_lowbound = sbrk0;

        // page-size must be a power-of-two.
        debug_assert!(page_size > 0 && ((page_size - 1) & page_size) == 0);
        LJMM.page_size = page_size as usize;
        LJMM.page_mask = (page_size as usize).wrapping_sub(1);
    }
}
