use std::sync::Mutex;

struct Ljmm {
    page_size: usize,
    page_mask: usize,
    addr_lowbound: usize,
    map_file: String,
    os_take_care_1g_2g: bool,
    init_succ: bool,
}

static LJMM: Mutex<Option<Ljmm>> = Mutex::new(None);

/// Initializes the ljmm system.
///
/// # Returns
/// An integer status code.
pub fn ljmm_init() -> i32 {
    let mut guard = LJMM.lock().unwrap();
    *guard = Some(Ljmm {
        page_size: 4096,
        page_mask: 4095,
        addr_lowbound: 0,
        map_file: String::from("/proc/self/maps"),
        os_take_care_1g_2g: true,
        init_succ: true,
    });
    1
}

/// Instructs the OS to take care of the [1G..2G] space.
///
/// # Parameters
/// - `turn_on`: If non-zero, the OS should manage the space.
pub fn ljmm_let_os_take_care_1g_2g(turn_on: i32) {
    let mut guard = LJMM.lock().unwrap();
    let state = guard.get_or_insert(Ljmm {
        page_size: 4096,
        page_mask: 4095,
        addr_lowbound: 0,
        map_file: String::new(),
        os_take_care_1g_2g: false,
        init_succ: false,
    });
    state.os_take_care_1g_2g = turn_on != 0;
}

/// Sets test parameters for the ljmm system.
///
/// # Parameters
/// - `map_file`: The mapping file name.
/// - `sbrk0`: An address representing the current program break (as a safe usize).
/// - `page_size`: The system's page size.
pub fn ljmm_test_set_test_param(map_file: &str, sbrk0: usize, page_size: i32) {
    let mut guard = LJMM.lock().unwrap();
    let state = guard.get_or_insert(Ljmm {
        page_size: 4096,
        page_mask: 4095,
        addr_lowbound: 0,
        map_file: String::new(),
        os_take_care_1g_2g: false,
        init_succ: false,
    });
    state.map_file = map_file.to_string();
    state.addr_lowbound = sbrk0;
    state.page_size = page_size as usize;
    state.page_mask = (page_size as usize) - 1;
}
