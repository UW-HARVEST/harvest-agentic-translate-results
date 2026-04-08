use ljmm::ljmm;

fn setup(map_file: &str, sbrk0: usize, page_size: i32, os_take_care: i32) {
    ljmm::ljmm_init();
    ljmm::ljmm_let_os_take_care_1g_2g(os_take_care);
    ljmm::ljmm_test_set_test_param(map_file, sbrk0, page_size);
}

// Test 1: basic best-fit with lowbound=0, OS_take_care=0
// Holes in input_001_001.txt (first 8191 bytes):
//   [0, 0x400000) size=0x400000
//   [0x418000, 0x617000) size=0x1FF000  <-- best fit for 0x8000
//   [0x619000, 0x7f2b4b200000) huge
// alloc_sz = 32*1024-1 = 32767, page_aligned = 0x8000
// Expected: 0x418000
#[test]
fn test_best_fit_basic() {
    setup("c_src/test/test_input/input_001_001.txt", 0, 4096, 0);
    let result = ljmm::find_best_fit(32 * 1024 - 1);
    assert_eq!(result, 0x418000);
}

// Test 2: considering lowbound=0x619000
// Only hole >= lowbound: [0x619000, 0x7f2b4b200000)
// Expected: 0x619000
#[test]
fn test_best_fit_with_lowbound() {
    setup("c_src/test/test_input/input_001_001.txt", 0x619000, 4096, 0);
    let result = ljmm::find_best_fit(32 * 1024 - 100);
    assert_eq!(result, 0x619000);
}

// Test 3: OS_take_care_1G_2G=1, lowbound=0
// Same holes, but now when start_addr >= 1G, we check start_addr >= upbound.
// 0x7f2b4b200000 >= 0x80000000 → break. Best fit is still 0x418000.
#[test]
fn test_best_fit_os_take_care() {
    setup("c_src/test/test_input/input_001_001.txt", 0, 4096, 1);
    let result = ljmm::find_best_fit(32 * 1024 - 10);
    assert_eq!(result, 0x418000);
}

// Test 4: buffer not large enough, incomplete last line "7f2b4b200000-\n"
// input_001_002.txt: 3 complete lines + "7f2b4b200000-\n"
// Last line: parse start=0x7f2b4b200000, '-' found, parse end: '\n' → advance=0
// end_addr fallback: upbound(0x80000000) < start_addr → end_addr = start_addr
// Hole before last line: [0x619000, 0x7f2b4b200000) → best fit
// Expected: 0x619000
#[test]
fn test_best_fit_truncated_buffer() {
    setup("c_src/test/test_input/input_001_002.txt", 0x619000, 4096, 0);
    let result = ljmm::find_best_fit(32 * 1024 - 10);
    assert_eq!(result, 0x619000);
}

// Test 5: incomplete start addr on last line "7f2b4b200000\n" (no '-')
// input_001_003.txt: 1 complete line + "7f2b4b200000\n"
// Last line: parse start=0x7f2b4b200000, next char is '\n' not '-' → break
// No valid hole found with lowbound=0x619000
// Expected: 0
#[test]
fn test_best_fit_incomplete_start() {
    setup("c_src/test/test_input/input_001_003.txt", 0x619000, 4096, 0);
    let result = ljmm::find_best_fit(32 * 1024 - 10);
    assert_eq!(result, 0);
}

// Test 6: exact fit hole
// input_001_004.txt has a hole [0x3ffff000, 0x40007000) of size 0x8000
// alloc_sz = 32*1024 = 0x8000, exact match → early break
// Expected: 0x3ffff000
#[test]
fn test_best_fit_exact_fit() {
    setup("c_src/test/test_input/input_001_004.txt", 0x619000, 4096, 0);
    let result = ljmm::find_best_fit(32 * 1024);
    assert_eq!(result, 0x3ffff000);
}

// Test: find_best_fit returns 0 when not initialized
#[test]
fn test_find_best_fit_not_initialized() {
    // Reset state by re-initializing - but we can't un-initialize.
    // Instead test with a nonexistent file after init
    ljmm::ljmm_init();
    ljmm::ljmm_test_set_test_param("/nonexistent/file", 0, 4096);
    let result = ljmm::find_best_fit(4096);
    assert_eq!(result, 0);
}

// Test: ljmm_init returns 1
#[test]
fn test_ljmm_init_returns_1() {
    assert_eq!(ljmm::ljmm_init(), 1);
}

// Test: page_align_addr behavior (tested indirectly through find_best_fit)
// With length=1, page_aligned = 4096. The smallest hole that fits is [0x418000, 0x617000)
#[test]
fn test_best_fit_small_alloc() {
    setup("c_src/test/test_input/input_001_001.txt", 0, 4096, 0);
    let result = ljmm::find_best_fit(1);
    assert_eq!(result, 0x418000);
}

// Test: length=0 → page_aligned=0, any hole fits. Best fit = smallest hole.
// Holes: [0,0x400000)=0x400000, [0x418000,0x617000)=0x1FF000, [0x619000,huge)
// Smallest is [0x619000-0x618000=0x1000 between blocks 2&3? No:
// block2 end=0x618000, block3 start=0x618000 → hole=0. 
// Actually [0x418000,0x617000) size=0x1FF000 is smallest non-zero.
// But length=0 means any hole of size>=0 fits. hole_size=0 >= 0 is true.
// First hole with size 0: between block2(end=0x618000) and block3(start=0x618000).
// hole_start=0x618000, hole_size=0. 0>=0 ✓. But 0 < best_fit_size(MAX) ✓.
// Actually wait: the first hole [0,0x400000) has hole_start=0, size=0x400000.
// 0x400000 >= 0 ✓, 0+0 <= 0x80000000 ✓, 0x400000 < MAX ✓. best=(0, 0x400000).
// Then [0x418000,0x617000) size=0x1FF000 < 0x400000 ✓. best=(0x418000, 0x1FF000).
// Then [0x618000,0x618000) size=0. 0 < 0x1FF000 ✓. best=(0x618000, 0).
// 0 == length(0) → break early.
#[test]
fn test_best_fit_zero_length() {
    setup("c_src/test/test_input/input_001_001.txt", 0, 4096, 0);
    let result = ljmm::find_best_fit(0);
    assert_eq!(result, 0x618000);
}

fn main() {}
