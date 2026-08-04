#![allow(unused_imports, dead_code)]

use ljmm::ljmm;
use std::sync::Mutex;

// All public APIs in ljmm operate on a single piece of global state. Tests
// would race against each other if cargo runs them in parallel, so we
// serialize them with this lock.
fn test_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

#[test]
fn test_ljmm_init_returns_one() {
    let _g = test_lock().lock().unwrap();
    let r = ljmm::ljmm_init();
    assert_eq!(r, 1, "ljmm_init must return 1 on success");
    assert_eq!(ljmm::_test_get_init_succ(), 1);
}

#[test]
fn test_ljmm_init_sets_default_state() {
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    // matches C #if !STRESS_TEST: OS_take_care_1G_2G defaults to 1
    assert_eq!(ljmm::_test_get_os_take_care_1g_2g(), 1);

    // From C: addr_upbound = ADDR_2G = 0x80000000
    assert_eq!(ljmm::_test_get_addr_upbound(), 0x80000000);

    // From C: page_size = sysconf(_SC_PAGESIZE) which on this system is 4096
    // (verified by running a C probe).
    assert_eq!(ljmm::_test_get_page_size(), 4096);
    assert_eq!(ljmm::_test_get_page_mask(), 4095);

    // From C: map_file = "/proc/self/maps"
    assert_eq!(ljmm::_test_get_map_file(), "/proc/self/maps");

    // The internal buffer must be allocated with capacity BUFFER_SZ (8192).
    assert_eq!(ljmm::_test_get_buffer_len(), 8192);
}

#[test]
fn test_let_os_take_care_1g_2g_on() {
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    ljmm::ljmm_let_os_take_care_1g_2g(1);
    assert_eq!(ljmm::_test_get_os_take_care_1g_2g(), 1);
}

#[test]
fn test_let_os_take_care_1g_2g_off() {
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    ljmm::ljmm_let_os_take_care_1g_2g(0);
    assert_eq!(ljmm::_test_get_os_take_care_1g_2g(), 0);
}

#[test]
fn test_let_os_take_care_1g_2g_truncates_to_char() {
    // The C field is a `char` so high bits get dropped on assignment. Verify
    // the Rust translation matches: turning on with a value whose low 8 bits
    // are zero must result in a stored 0.
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    // 0x100 has low byte 0 -> stored as 0
    ljmm::ljmm_let_os_take_care_1g_2g(0x100);
    assert_eq!(ljmm::_test_get_os_take_care_1g_2g(), 0);

    // 0x101 has low byte 1 -> stored as 1
    ljmm::ljmm_let_os_take_care_1g_2g(0x101);
    assert_eq!(ljmm::_test_get_os_take_care_1g_2g(), 1);

    // 0xFF as i32 -> low byte is 0xFF which signed-as-i8 is -1
    ljmm::ljmm_let_os_take_care_1g_2g(0xFF);
    assert_eq!(ljmm::_test_get_os_take_care_1g_2g(), -1);
}

#[test]
fn test_set_test_param_basic() {
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    ljmm::ljmm_test_set_test_param("test_input/input_001_001.txt", 0x619000, 4096);
    assert_eq!(
        ljmm::_test_get_map_file(),
        "test_input/input_001_001.txt"
    );
    assert_eq!(ljmm::_test_get_addr_lowbound(), 0x619000);
    assert_eq!(ljmm::_test_get_page_size(), 4096);
    assert_eq!(ljmm::_test_get_page_mask(), 4095);
}

#[test]
fn test_set_test_param_zero_lowbound() {
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    // test 1 from c_src/test/test_001.c uses low_bound = 0
    ljmm::ljmm_test_set_test_param("test_input/input_001_001.txt", 0, 4096);
    assert_eq!(ljmm::_test_get_addr_lowbound(), 0);
    assert_eq!(ljmm::_test_get_page_size(), 4096);
    assert_eq!(ljmm::_test_get_page_mask(), 4095);
}

#[test]
fn test_set_test_param_various_page_sizes() {
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    // page_size must be a power of two; verify multiple legal values.
    for ps in [4096i32, 8192, 16384, 65536] {
        ljmm::ljmm_test_set_test_param("/x", 0, ps);
        assert_eq!(ljmm::_test_get_page_size(), ps as usize);
        assert_eq!(ljmm::_test_get_page_mask(), (ps - 1) as usize);
    }
}

#[test]
fn test_set_test_param_replaces_map_file() {
    let _g = test_lock().lock().unwrap();
    ljmm::ljmm_init();

    ljmm::ljmm_test_set_test_param("first.txt", 0, 4096);
    assert_eq!(ljmm::_test_get_map_file(), "first.txt");

    ljmm::ljmm_test_set_test_param("second.txt", 0x1000, 4096);
    assert_eq!(ljmm::_test_get_map_file(), "second.txt");
    assert_eq!(ljmm::_test_get_addr_lowbound(), 0x1000);
}

#[test]
fn test_init_resets_upbound_and_default_lowbound() {
    let _g = test_lock().lock().unwrap();
    // Mutate state first
    ljmm::ljmm_init();
    ljmm::ljmm_test_set_test_param("foo", 0xdeadbeef, 8192);
    ljmm::ljmm_let_os_take_care_1g_2g(0);

    // Re-init must restore defaults
    let r = ljmm::ljmm_init();
    assert_eq!(r, 1);
    assert_eq!(ljmm::_test_get_addr_upbound(), 0x80000000);
    assert_eq!(ljmm::_test_get_page_size(), 4096);
    assert_eq!(ljmm::_test_get_page_mask(), 4095);
    assert_eq!(ljmm::_test_get_os_take_care_1g_2g(), 1);
    assert_eq!(ljmm::_test_get_map_file(), "/proc/self/maps");
}

fn main() {}
