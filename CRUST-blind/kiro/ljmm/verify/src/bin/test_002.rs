use ljmm::ljmm;

#[test]
fn test_ljmm_init() {
    let ret = ljmm::ljmm_init();
    assert_eq!(ret, 1);
}

#[test]
fn test_find_best_fit_all_cases() {
    // Test 1: basic find_best_fit, lowbound=0, os_take_care=false
    ljmm::ljmm_init();
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param(
        "c_src/test/test_input/input_001_001.txt", 0, 4096,
    );
    assert_eq!(ljmm::ljmm_find_best_fit(32 * 1024 - 1), 0x418000);

    // Test 2: considering low-bound=0x619000
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param(
        "c_src/test/test_input/input_001_001.txt", 0x619000, 4096,
    );
    assert_eq!(ljmm::ljmm_find_best_fit(32 * 1024 - 100), 0x619000);

    // Test 3: OS takes care of [1G,2G], still finds best fit below 1G
    ljmm::ljmm_let_os_take_care_1g_2g(1);
    ljmm::ljmm_test_set_test_param(
        "c_src/test/test_input/input_001_001.txt", 0, 4096,
    );
    assert_eq!(ljmm::ljmm_find_best_fit(32 * 1024 - 10), 0x418000);

    // Test 4: buffer not large enough, incomplete last line with end addr
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param(
        "c_src/test/test_input/input_001_002.txt", 0x619000, 4096,
    );
    assert_eq!(ljmm::ljmm_find_best_fit(32 * 1024 - 10), 0x619000);

    // Test 5: incomplete start addr on last line, no fit found
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param(
        "c_src/test/test_input/input_001_003.txt", 0x619000, 4096,
    );
    assert_eq!(ljmm::ljmm_find_best_fit(32 * 1024 - 10), 0x0);

    // Test 6: fit at high address
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_test_set_test_param(
        "c_src/test/test_input/input_001_004.txt", 0x619000, 4096,
    );
    assert_eq!(ljmm::ljmm_find_best_fit(32 * 1024), 0x3ffff000);
}

#[test]
fn test_ljmm_let_os_take_care() {
    ljmm::ljmm_init();
    // Should not panic when called with 0 or 1
    ljmm::ljmm_let_os_take_care_1g_2g(0);
    ljmm::ljmm_let_os_take_care_1g_2g(1);
}

#[test]
fn test_ljmm_test_set_test_param() {
    ljmm::ljmm_init();
    // Should not panic
    ljmm::ljmm_test_set_test_param("nonexistent_file.txt", 0x400000, 4096);
}

#[test]
fn test_find_best_fit_nonexistent_file() {
    ljmm::ljmm_init();
    ljmm::ljmm_test_set_test_param("nonexistent_file.txt", 0, 4096);
    assert_eq!(ljmm::ljmm_find_best_fit(4096), 0);
}

fn main() {}
