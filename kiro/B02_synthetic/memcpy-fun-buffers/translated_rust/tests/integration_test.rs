use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct buffer_t {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

#[repr(C)]
struct buffer_array_t {
    buffers: *mut buffer_t,
    count: c_int,
    capacity: c_int,
}

impl buffer_t {
    fn new() -> Self {
        buffer_t {
            data: [0u8; 256],
            length: 0,
            checksum: 0,
        }
    }

    fn with_data(d: &[u8]) -> Self {
        let mut b = Self::new();
        b.data[..d.len()].copy_from_slice(d);
        b.length = d.len();
        // checksum will be set by calculate_checksum
        b
    }
}

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn assert_buffers_equal(c_buf: &buffer_t, r_buf: &buffer_t, ctx: &str) {
    assert_eq!(c_buf.length, r_buf.length, "{}: length mismatch", ctx);
    assert_eq!(
        &c_buf.data[..c_buf.length],
        &r_buf.data[..r_buf.length],
        "{}: data mismatch",
        ctx
    );
    assert_eq!(c_buf.checksum, r_buf.checksum, "{}: checksum mismatch", ctx);
}

// ==================== Tests ====================

#[test]
fn test_calculate_checksum() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_fn: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };

    for data in &[
        vec![],
        vec![0u8],
        vec![1, 2, 3, 4, 5],
        vec![255; 256],
        (0..=255).collect::<Vec<u8>>(),
        vec![0, 128, 255, 1, 127],
    ] {
        let c_result = unsafe { c_fn(data.as_ptr(), data.len()) };
        let r_result = memcpy_fun_buffers::calculate_checksum(data.as_ptr(), data.len());
        assert_eq!(c_result, r_result, "checksum mismatch for data {:?}", &data[..data.len().min(10)]);
    }
}

#[test]
fn test_validate_buffer() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_validate: Symbol<unsafe extern "C" fn(*const buffer_t) -> bool> =
        unsafe { lib.get(b"validate_buffer").unwrap() };

    let mut buf = buffer_t::with_data(&[10, 20, 30, 40, 50]);
    buf.checksum = unsafe { c_checksum(buf.data.as_ptr(), buf.length) };

    let c_result = unsafe { c_validate(&buf) };
    let r_result = memcpy_fun_buffers::validate_buffer(&buf as *const _ as *const memcpy_fun_buffers::buffer_t);
    assert_eq!(c_result, r_result, "validate_buffer mismatch");
}

#[test]
fn test_buffer_copy() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_copy: Symbol<unsafe extern "C" fn(*const buffer_t, *mut buffer_t) -> c_int> =
        unsafe { lib.get(b"buffer_copy").unwrap() };

    let mut src = buffer_t::with_data(&[1, 2, 3, 4, 5, 6, 7, 8]);
    src.checksum = unsafe { c_checksum(src.data.as_ptr(), src.length) };

    let mut c_dst = buffer_t::new();
    let mut r_dst = buffer_t::new();

    let c_ret = unsafe { c_copy(&src, &mut c_dst) };
    let r_ret = memcpy_fun_buffers::buffer_copy(
        &src as *const _ as *const memcpy_fun_buffers::buffer_t,
        &mut r_dst as *mut _ as *mut memcpy_fun_buffers::buffer_t,
    );

    assert_eq!(c_ret, r_ret, "buffer_copy return mismatch");
    assert_buffers_equal(&c_dst, &r_dst, "buffer_copy");
}

#[test]
fn test_buffer_reverse() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_reverse: Symbol<unsafe extern "C" fn(*mut buffer_t) -> c_int> =
        unsafe { lib.get(b"buffer_reverse").unwrap() };

    for data in &[
        vec![],
        vec![42u8],
        vec![1, 2, 3, 4, 5],
        (0..100).collect::<Vec<u8>>(),
    ] {
        let mut c_buf = buffer_t::with_data(data);
        c_buf.checksum = unsafe { c_checksum(c_buf.data.as_ptr(), c_buf.length) };
        let mut r_buf = c_buf;

        let c_ret = unsafe { c_reverse(&mut c_buf) };
        let r_ret = memcpy_fun_buffers::buffer_reverse(
            &mut r_buf as *mut _ as *mut memcpy_fun_buffers::buffer_t,
        );

        assert_eq!(c_ret, r_ret, "buffer_reverse return mismatch");
        assert_buffers_equal(&c_buf, &r_buf, "buffer_reverse");
    }
}

#[test]
fn test_buffer_merge() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_merge: Symbol<
        unsafe extern "C" fn(*const buffer_t, *const buffer_t, *mut buffer_t) -> c_int,
    > = unsafe { lib.get(b"buffer_merge").unwrap() };

    let mut src1 = buffer_t::with_data(&[1, 2, 3]);
    src1.checksum = unsafe { c_checksum(src1.data.as_ptr(), src1.length) };
    let mut src2 = buffer_t::with_data(&[4, 5, 6, 7]);
    src2.checksum = unsafe { c_checksum(src2.data.as_ptr(), src2.length) };

    let mut c_dst = buffer_t::new();
    let mut r_dst = buffer_t::new();

    let c_ret = unsafe { c_merge(&src1, &src2, &mut c_dst) };
    let r_ret = memcpy_fun_buffers::buffer_merge(
        &src1 as *const _ as *const memcpy_fun_buffers::buffer_t,
        &src2 as *const _ as *const memcpy_fun_buffers::buffer_t,
        &mut r_dst as *mut _ as *mut memcpy_fun_buffers::buffer_t,
    );

    assert_eq!(c_ret, r_ret, "buffer_merge return mismatch");
    assert_buffers_equal(&c_dst, &r_dst, "buffer_merge");
}

#[test]
fn test_buffer_split() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_split: Symbol<
        unsafe extern "C" fn(*const buffer_t, usize, *mut buffer_t, *mut buffer_t) -> c_int,
    > = unsafe { lib.get(b"buffer_split").unwrap() };

    let mut src = buffer_t::with_data(&[10, 20, 30, 40, 50, 60]);
    src.checksum = unsafe { c_checksum(src.data.as_ptr(), src.length) };

    for split_pos in [0usize, 3, 6] {
        let mut c_d1 = buffer_t::new();
        let mut c_d2 = buffer_t::new();
        let mut r_d1 = buffer_t::new();
        let mut r_d2 = buffer_t::new();

        let c_ret = unsafe { c_split(&src, split_pos, &mut c_d1, &mut c_d2) };
        let r_ret = memcpy_fun_buffers::buffer_split(
            &src as *const _ as *const memcpy_fun_buffers::buffer_t,
            split_pos,
            &mut r_d1 as *mut _ as *mut memcpy_fun_buffers::buffer_t,
            &mut r_d2 as *mut _ as *mut memcpy_fun_buffers::buffer_t,
        );

        assert_eq!(c_ret, r_ret, "buffer_split return mismatch at pos {}", split_pos);
        assert_buffers_equal(&c_d1, &r_d1, &format!("buffer_split dst1 pos={}", split_pos));
        assert_buffers_equal(&c_d2, &r_d2, &format!("buffer_split dst2 pos={}", split_pos));
    }
}

#[test]
fn test_buffer_interleave() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_interleave: Symbol<
        unsafe extern "C" fn(*const buffer_t, *const buffer_t, *mut buffer_t) -> c_int,
    > = unsafe { lib.get(b"buffer_interleave").unwrap() };

    // Equal length
    let mut s1 = buffer_t::with_data(&[1, 3, 5]);
    s1.checksum = unsafe { c_checksum(s1.data.as_ptr(), s1.length) };
    let mut s2 = buffer_t::with_data(&[2, 4, 6]);
    s2.checksum = unsafe { c_checksum(s2.data.as_ptr(), s2.length) };

    let mut c_dst = buffer_t::new();
    let mut r_dst = buffer_t::new();

    let c_ret = unsafe { c_interleave(&s1, &s2, &mut c_dst) };
    let r_ret = memcpy_fun_buffers::buffer_interleave(
        &s1 as *const _ as *const memcpy_fun_buffers::buffer_t,
        &s2 as *const _ as *const memcpy_fun_buffers::buffer_t,
        &mut r_dst as *mut _ as *mut memcpy_fun_buffers::buffer_t,
    );
    assert_eq!(c_ret, r_ret);
    assert_buffers_equal(&c_dst, &r_dst, "buffer_interleave equal");

    // Unequal length
    let mut s1 = buffer_t::with_data(&[1, 2, 3, 4, 5]);
    s1.checksum = unsafe { c_checksum(s1.data.as_ptr(), s1.length) };
    let mut s2 = buffer_t::with_data(&[10, 20]);
    s2.checksum = unsafe { c_checksum(s2.data.as_ptr(), s2.length) };

    let mut c_dst = buffer_t::new();
    let mut r_dst = buffer_t::new();

    let c_ret = unsafe { c_interleave(&s1, &s2, &mut c_dst) };
    let r_ret = memcpy_fun_buffers::buffer_interleave(
        &s1 as *const _ as *const memcpy_fun_buffers::buffer_t,
        &s2 as *const _ as *const memcpy_fun_buffers::buffer_t,
        &mut r_dst as *mut _ as *mut memcpy_fun_buffers::buffer_t,
    );
    assert_eq!(c_ret, r_ret);
    assert_buffers_equal(&c_dst, &r_dst, "buffer_interleave unequal");
}

#[test]
fn test_buffer_rotate() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_rotate: Symbol<unsafe extern "C" fn(*mut buffer_t, c_int) -> c_int> =
        unsafe { lib.get(b"buffer_rotate").unwrap() };

    for (data, positions) in &[
        (vec![1u8, 2, 3, 4, 5], 2),
        (vec![1, 2, 3, 4, 5], -1),
        (vec![1, 2, 3, 4, 5], 0),
        (vec![1, 2, 3, 4, 5], 5),
        (vec![1, 2, 3, 4, 5], 7),
        (vec![], 3),
    ] {
        let mut c_buf = buffer_t::with_data(data);
        c_buf.checksum = unsafe { c_checksum(c_buf.data.as_ptr(), c_buf.length) };
        let mut r_buf = c_buf;

        let c_ret = unsafe { c_rotate(&mut c_buf, *positions) };
        let r_ret = memcpy_fun_buffers::buffer_rotate(
            &mut r_buf as *mut _ as *mut memcpy_fun_buffers::buffer_t,
            *positions,
        );

        assert_eq!(c_ret, r_ret, "buffer_rotate return mismatch pos={}", positions);
        assert_buffers_equal(&c_buf, &r_buf, &format!("buffer_rotate pos={}", positions));
    }
}

#[test]
fn test_buffer_conditional_copy() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_cond: Symbol<
        unsafe extern "C" fn(*const buffer_t, *mut buffer_t, u8, bool) -> c_int,
    > = unsafe { lib.get(b"buffer_conditional_copy").unwrap() };

    let mut src = buffer_t::with_data(&[1, 2, 3, 2, 5, 2, 7]);
    src.checksum = unsafe { c_checksum(src.data.as_ptr(), src.length) };

    // copy_matching = true (copy bytes that match pattern)
    for (pattern, copy_matching) in &[(2u8, true), (2u8, false), (99u8, true)] {
        let mut c_dst = buffer_t::new();
        let mut r_dst = buffer_t::new();

        let c_ret = unsafe { c_cond(&src, &mut c_dst, *pattern, *copy_matching) };
        let r_ret = memcpy_fun_buffers::buffer_conditional_copy(
            &src as *const _ as *const memcpy_fun_buffers::buffer_t,
            &mut r_dst as *mut _ as *mut memcpy_fun_buffers::buffer_t,
            *pattern,
            *copy_matching,
        );

        assert_eq!(c_ret, r_ret, "conditional_copy return mismatch");
        assert_buffers_equal(
            &c_dst,
            &r_dst,
            &format!("conditional_copy pat={} match={}", pattern, copy_matching),
        );
    }
}

#[test]
fn test_buffer_copy_strided() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_strided: Symbol<
        unsafe extern "C" fn(*const buffer_t, *mut buffer_t, c_int) -> c_int,
    > = unsafe { lib.get(b"buffer_copy_strided").unwrap() };

    let mut src = buffer_t::with_data(&[10, 20, 30, 40, 50, 60, 70, 80]);
    src.checksum = unsafe { c_checksum(src.data.as_ptr(), src.length) };

    for stride in [1, 2, 3, 4, 8] {
        let mut c_dst = buffer_t::new();
        let mut r_dst = buffer_t::new();

        let c_ret = unsafe { c_strided(&src, &mut c_dst, stride) };
        let r_ret = memcpy_fun_buffers::buffer_copy_strided(
            &src as *const _ as *const memcpy_fun_buffers::buffer_t,
            &mut r_dst as *mut _ as *mut memcpy_fun_buffers::buffer_t,
            stride,
        );

        assert_eq!(c_ret, r_ret, "copy_strided return mismatch stride={}", stride);
        assert_buffers_equal(&c_dst, &r_dst, &format!("copy_strided stride={}", stride));
    }
}

#[test]
fn test_process_buffer_array_copy() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_init: Symbol<unsafe extern "C" fn(c_int) -> *mut buffer_array_t> =
        unsafe { lib.get(b"init_buffer_array").unwrap() };
    let c_process: Symbol<unsafe extern "C" fn(*mut buffer_array_t, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"process_buffer_array").unwrap() };
    let c_free: Symbol<unsafe extern "C" fn(*mut buffer_array_t)> =
        unsafe { lib.get(b"free_buffer_array").unwrap() };

    // Test OP_COPY (0): copy first buffer to all others
    let c_arr = unsafe { c_init(3) };
    assert!(!c_arr.is_null());

    // Set up buffers
    unsafe {
        let b0 = &mut *(*c_arr).buffers.add(0);
        b0.data[..5].copy_from_slice(&[10, 20, 30, 40, 50]);
        b0.length = 5;
        b0.checksum = c_checksum(b0.data.as_ptr(), b0.length);

        let b1 = &mut *(*c_arr).buffers.add(1);
        b1.length = 0;
        b1.checksum = 0;

        let b2 = &mut *(*c_arr).buffers.add(2);
        b2.length = 0;
        b2.checksum = 0;

        (*c_arr).count = 3;
    }

    // Rust side
    let r_arr = memcpy_fun_buffers::init_buffer_array(3);
    assert!(!r_arr.is_null());
    unsafe {
        let r_arr_ref = &mut *(r_arr as *mut buffer_array_t);
        let b0 = &mut *r_arr_ref.buffers.add(0);
        b0.data[..5].copy_from_slice(&[10, 20, 30, 40, 50]);
        b0.length = 5;
        b0.checksum = c_checksum(b0.data.as_ptr(), b0.length);
        r_arr_ref.count = 3;
    }

    let c_ret = unsafe { c_process(c_arr, 0, 0) };
    let r_ret = memcpy_fun_buffers::process_buffer_array(
        r_arr as *mut memcpy_fun_buffers::buffer_array_t,
        0,
        0,
    );
    assert_eq!(c_ret, r_ret, "process_buffer_array OP_COPY return mismatch");

    // Compare all buffers
    for i in 0..3 {
        let c_buf = unsafe { &*(*c_arr).buffers.add(i) };
        let r_buf = unsafe { &*(*(r_arr as *mut buffer_array_t)).buffers.add(i) };
        assert_buffers_equal(c_buf, r_buf, &format!("process OP_COPY buf[{}]", i));
    }

    unsafe { c_free(c_arr) };
    memcpy_fun_buffers::free_buffer_array(r_arr as *mut memcpy_fun_buffers::buffer_array_t);
}

#[test]
fn test_process_buffer_array_reverse() {
    let lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_checksum: Symbol<unsafe extern "C" fn(*const u8, usize) -> u32> =
        unsafe { lib.get(b"calculate_checksum").unwrap() };
    let c_init: Symbol<unsafe extern "C" fn(c_int) -> *mut buffer_array_t> =
        unsafe { lib.get(b"init_buffer_array").unwrap() };
    let c_process: Symbol<unsafe extern "C" fn(*mut buffer_array_t, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"process_buffer_array").unwrap() };
    let c_free: Symbol<unsafe extern "C" fn(*mut buffer_array_t)> =
        unsafe { lib.get(b"free_buffer_array").unwrap() };

    let c_arr = unsafe { c_init(2) };
    let r_arr = memcpy_fun_buffers::init_buffer_array(2);

    let data_sets: [&[u8]; 2] = [&[1, 2, 3, 4], &[10, 20, 30]];
    for (i, data) in data_sets.iter().enumerate() {
        unsafe {
            let cb = &mut *(*c_arr).buffers.add(i);
            cb.data[..data.len()].copy_from_slice(data);
            cb.length = data.len();
            cb.checksum = c_checksum(cb.data.as_ptr(), cb.length);

            let rb = &mut *(*(r_arr as *mut buffer_array_t)).buffers.add(i);
            rb.data[..data.len()].copy_from_slice(data);
            rb.length = data.len();
            rb.checksum = c_checksum(rb.data.as_ptr(), rb.length);
        }
    }
    unsafe {
        (*c_arr).count = 2;
        (*(r_arr as *mut buffer_array_t)).count = 2;
    }

    let c_ret = unsafe { c_process(c_arr, 1, 0) };
    let r_ret = memcpy_fun_buffers::process_buffer_array(
        r_arr as *mut memcpy_fun_buffers::buffer_array_t,
        1,
        0,
    );
    assert_eq!(c_ret, r_ret);

    for i in 0..2 {
        let c_buf = unsafe { &*(*c_arr).buffers.add(i) };
        let r_buf = unsafe { &*(*(r_arr as *mut buffer_array_t)).buffers.add(i) };
        assert_buffers_equal(c_buf, r_buf, &format!("process OP_REVERSE buf[{}]", i));
    }

    unsafe { c_free(c_arr) };
    memcpy_fun_buffers::free_buffer_array(r_arr as *mut memcpy_fun_buffers::buffer_array_t);
}
