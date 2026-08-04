#[allow(unused_imports)]
use ljmm::ljmm;

#[test]
fn test_ljmm_init_returns_zero() {
    // The C ljmm_init() (declared in header as `int`, called via constructor)
    // The Rust translation should return 0 on success.
    let r = ljmm::ljmm_init();
    assert_eq!(r, 0);
}

#[test]
fn test_ljmm_init_is_idempotent() {
    // Calling ljmm_init multiple times should still return 0 (no panic).
    assert_eq!(ljmm::ljmm_init(), 0);
    assert_eq!(ljmm::ljmm_init(), 0);
    assert_eq!(ljmm::ljmm_init(), 0);
}

#[test]
fn test_let_os_take_care_1g_2g_zero() {
    // turn_on = 0 should disable
    ljmm::ljmm_init();
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    // No panic, no return value to compare.
}

#[test]
fn test_let_os_take_care_1g_2g_one() {
    // turn_on = 1 should enable
    ljmm::ljmm_init();
    ljmm::ljmm_let_os_take_care_1g_2g(1);
}

#[test]
fn test_let_os_take_care_1g_2g_nonzero_values() {
    // Any non-zero value should be treated as 'enable'.
    // This matches C's `turn_on` which is just an int.
    ljmm::ljmm_init();
    ljmm::ljmm_let_os_take_care_1g_2g(2);
    ljmm::ljmm_let_os_take_care_1g_2g(-1);
    ljmm::ljmm_let_os_take_care_1g_2g(i32::MAX);
    ljmm::ljmm_let_os_take_care_1g_2g(i32::MIN);
    ljmm::ljmm_let_os_take_care_1g_2g(0);
}

#[test]
fn test_set_test_param_basic() {
    ljmm::ljmm_init();
    // page_size must be power of two (>= 1).
    ljmm::ljmm_test_set_test_param("/proc/self/maps", 0x1000, 4096);
}

#[test]
fn test_set_test_param_various_page_sizes() {
    ljmm::ljmm_init();
    // Power-of-two page sizes.
    ljmm::ljmm_test_set_test_param("/some/file", 0, 1);
    ljmm::ljmm_test_set_test_param("/some/file", 0x1000, 2);
    ljmm::ljmm_test_set_test_param("/some/file", 0x1000, 4);
    ljmm::ljmm_test_set_test_param("/some/file", 0x1000, 4096);
    ljmm::ljmm_test_set_test_param("/some/file", 0x1000, 8192);
    ljmm::ljmm_test_set_test_param("/some/file", 0x1000, 65536);
}

#[test]
fn test_set_test_param_with_various_sbrk0() {
    ljmm::ljmm_init();
    ljmm::ljmm_test_set_test_param("foo", 0, 4096);
    ljmm::ljmm_test_set_test_param("foo", 0x619000, 4096);
    ljmm::ljmm_test_set_test_param("foo", 0x418000, 4096);
    ljmm::ljmm_test_set_test_param("foo", usize::MAX, 4096);
}

#[test]
fn test_set_test_param_empty_path() {
    ljmm::ljmm_init();
    ljmm::ljmm_test_set_test_param("", 0, 4096);
}

#[test]
fn test_full_lifecycle() {
    // Replicate the test_001.c lifecycle:
    //   ljmm_let_OS_take_care_1G_2G(p->OS_take_care_1G_2G);
    //   ljmm_test_set_test_param(input_path, (void*)p->low_bound, 4096);

    let result = ljmm::ljmm_init();
    assert_eq!(result, 0);

    // Test 1
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param("test_input/input_001_001.txt", 0, 4096);

    // Test 2
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param("test_input/input_001_001.txt", 0x619000, 4096);

    // Test 3
    ljmm::ljmm_let_os_take_care_1g_2g(1);
    ljmm::ljmm_test_set_test_param("test_input/input_001_001.txt", 0, 4096);

    // Test 4
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param("test_input/input_001_002.txt", 0x619000, 4096);

    // Test 5
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param("test_input/input_001_003.txt", 0x619000, 4096);

    // Test 6
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param("test_input/input_001_004.txt", 0x619000, 4096);
}

fn main() {}
