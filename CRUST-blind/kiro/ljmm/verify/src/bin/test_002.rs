use ljmm::ljmm;

// Test ljmm_let_os_take_care_1g_2g toggle behavior
// When turned off (0), blocks >= 1G are skipped immediately
// When turned on (non-zero), blocks in [1G,2G) are still considered
#[test]
fn test_os_take_care_toggle() {
    ljmm::ljmm_init();
    // With OS_take_care=0 and lowbound above all blocks < 1G,
    // the search should find the hole before the first >=1G block
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param("c_src/test/test_input/input_001_001.txt", 0x619000, 4096);
    let r1 = ljmm::find_best_fit(4096);
    assert_eq!(r1, 0x619000);

    // Toggle on: same result since the >=1G block is also >= upbound
    ljmm::ljmm_let_os_take_care_1g_2g(1);
    ljmm::ljmm_test_set_test_param("c_src/test/test_input/input_001_001.txt", 0x619000, 4096);
    let r2 = ljmm::find_best_fit(4096);
    assert_eq!(r2, 0x619000);
}

// Test that ljmm_let_os_take_care_1g_2g with non-zero values all mean true
#[test]
fn test_os_take_care_nonzero_is_true() {
    ljmm::ljmm_init();
    ljmm::ljmm_let_os_take_care_1g_2g(42);
    ljmm::ljmm_test_set_test_param("c_src/test/test_input/input_001_001.txt", 0, 4096);
    let result = ljmm::find_best_fit(32 * 1024 - 10);
    // Same as test 3 in test_001: best fit is 0x418000
    assert_eq!(result, 0x418000);
}

// Test with empty/missing map file
#[test]
fn test_missing_map_file() {
    ljmm::ljmm_init();
    ljmm::ljmm_test_set_test_param("/no/such/file", 0, 4096);
    assert_eq!(ljmm::find_best_fit(4096), 0);
}

// Test that ljmm_init can be called multiple times
#[test]
fn test_double_init() {
    assert_eq!(ljmm::ljmm_init(), 1);
    assert_eq!(ljmm::ljmm_init(), 1);
}

fn main() {}
