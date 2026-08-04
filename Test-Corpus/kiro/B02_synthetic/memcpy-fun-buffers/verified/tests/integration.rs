use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone)]
struct BufferT {
    data: [u8; 256],
    length: usize,
    checksum: u32,
}

#[repr(C)]
struct BufferArrayT {
    buffers: *mut BufferT,
    count: c_int,
    capacity: c_int,
}

impl BufferT {
    fn zeroed() -> Self {
        BufferT { data: [0u8; 256], length: 0, checksum: 0 }
    }
    fn new(bytes: &[u8]) -> Self {
        let mut b = Self::zeroed();
        b.length = bytes.len();
        b.data[..bytes.len()].copy_from_slice(bytes);
        // checksum will be set by the library
        b
    }
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libtranslated_rust.so");
    p
}

struct Libs {
    c: Library,
    r: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            Libs {
                c: Library::new(c_lib_path()).expect("load C .so"),
                r: Library::new(rust_lib_path()).expect("load Rust .so"),
            }
        }
    }
}

// ==================== calculate_checksum tests ====================

#[test]
fn test_calculate_checksum() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const u8, usize) -> u32;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"calculate_checksum").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"calculate_checksum").unwrap() };

    let cases: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        vec![1, 2, 3, 4, 5],
        vec![255; 256],
        vec![0; 256],
        (0..=255).collect(),
        vec![0x80, 0xFF, 0x01, 0x00, 0x7F],
    ];
    for data in &cases {
        let c_res = unsafe { c_fn(data.as_ptr(), data.len()) };
        let r_res = unsafe { r_fn(data.as_ptr(), data.len()) };
        assert_eq!(c_res, r_res, "checksum mismatch for data len={}", data.len());
    }
}

// ==================== validate_buffer tests ====================

#[test]
fn test_validate_buffer() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const BufferT) -> bool;
    type CkFn = unsafe extern "C" fn(*const u8, usize) -> u32;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"validate_buffer").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"validate_buffer").unwrap() };
    let c_ck: Symbol<CkFn> = unsafe { libs.c.get(b"calculate_checksum").unwrap() };

    // Valid buffer
    let mut buf = BufferT::new(&[1, 2, 3]);
    buf.checksum = unsafe { c_ck(buf.data.as_ptr(), buf.length) };
    assert_eq!(unsafe { c_fn(&buf) }, unsafe { r_fn(&buf) });

    // Wrong checksum
    buf.checksum = 99999;
    assert_eq!(unsafe { c_fn(&buf) }, unsafe { r_fn(&buf) });

    // Empty buffer
    let mut empty = BufferT::zeroed();
    empty.checksum = unsafe { c_ck(empty.data.as_ptr(), 0) };
    assert_eq!(unsafe { c_fn(&empty) }, unsafe { r_fn(&empty) });
}

// ==================== buffer_copy tests ====================

fn make_buf_with_checksum(libs: &Libs, data: &[u8]) -> BufferT {
    type CkFn = unsafe extern "C" fn(*const u8, usize) -> u32;
    let c_ck: Symbol<CkFn> = unsafe { libs.c.get(b"calculate_checksum").unwrap() };
    let mut buf = BufferT::new(data);
    buf.checksum = unsafe { c_ck(buf.data.as_ptr(), buf.length) };
    buf
}

fn assert_bufs_eq(a: &BufferT, b: &BufferT, ctx: &str) {
    assert_eq!(a.length, b.length, "{}: length mismatch", ctx);
    assert_eq!(&a.data[..a.length], &b.data[..b.length], "{}: data mismatch", ctx);
    assert_eq!(a.checksum, b.checksum, "{}: checksum mismatch", ctx);
}

#[test]
fn test_buffer_copy() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const BufferT, *mut BufferT) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_copy").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_copy").unwrap() };

    let cases: Vec<Vec<u8>> = vec![
        vec![1, 2, 3, 4, 5],
        vec![],
        vec![255; 256],
        (0..128).collect(),
    ];
    for data in &cases {
        let src = make_buf_with_checksum(&libs, data);
        let mut c_dst = BufferT::zeroed();
        let mut r_dst = BufferT::zeroed();
        let c_ret = unsafe { c_fn(&src, &mut c_dst) };
        let r_ret = unsafe { r_fn(&src, &mut r_dst) };
        assert_eq!(c_ret, r_ret, "buffer_copy return mismatch");
        if c_ret == 0 {
            assert_bufs_eq(&c_dst, &r_dst, "buffer_copy");
        }
    }
}

// ==================== buffer_reverse tests ====================

#[test]
fn test_buffer_reverse() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut BufferT) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_reverse").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_reverse").unwrap() };

    let cases: Vec<Vec<u8>> = vec![
        vec![1, 2, 3, 4, 5],
        vec![],
        vec![42],
        (0..=255).collect(),
    ];
    for data in &cases {
        let mut c_buf = make_buf_with_checksum(&libs, data);
        let mut r_buf = c_buf.clone();
        let c_ret = unsafe { c_fn(&mut c_buf) };
        let r_ret = unsafe { r_fn(&mut r_buf) };
        assert_eq!(c_ret, r_ret, "buffer_reverse return mismatch");
        assert_bufs_eq(&c_buf, &r_buf, "buffer_reverse");
    }
}

// ==================== buffer_merge tests ====================

#[test]
fn test_buffer_merge() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const BufferT, *const BufferT, *mut BufferT) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_merge").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_merge").unwrap() };

    let cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (vec![1, 2, 3], vec![4, 5, 6]),
        (vec![], vec![1, 2]),
        (vec![1, 2], vec![]),
        (vec![], vec![]),
        (vec![255; 128], vec![0; 128]),
    ];
    for (d1, d2) in &cases {
        let s1 = make_buf_with_checksum(&libs, d1);
        let s2 = make_buf_with_checksum(&libs, d2);
        let mut c_dst = BufferT::zeroed();
        let mut r_dst = BufferT::zeroed();
        let c_ret = unsafe { c_fn(&s1, &s2, &mut c_dst) };
        let r_ret = unsafe { r_fn(&s1, &s2, &mut r_dst) };
        assert_eq!(c_ret, r_ret, "buffer_merge return mismatch");
        if c_ret == 0 {
            assert_bufs_eq(&c_dst, &r_dst, "buffer_merge");
        }
    }

    // Overflow case
    let s1 = make_buf_with_checksum(&libs, &[1; 200]);
    let s2 = make_buf_with_checksum(&libs, &[2; 100]);
    let mut c_dst = BufferT::zeroed();
    let mut r_dst = BufferT::zeroed();
    let c_ret = unsafe { c_fn(&s1, &s2, &mut c_dst) };
    let r_ret = unsafe { r_fn(&s1, &s2, &mut r_dst) };
    assert_eq!(c_ret, r_ret, "buffer_merge overflow return mismatch");
}

// ==================== buffer_split tests ====================

#[test]
fn test_buffer_split() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const BufferT, usize, *mut BufferT, *mut BufferT) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_split").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_split").unwrap() };

    let data: Vec<u8> = (1..=10).collect();
    let src = make_buf_with_checksum(&libs, &data);

    for pos in [0, 1, 5, 10] {
        let mut c_d1 = BufferT::zeroed();
        let mut c_d2 = BufferT::zeroed();
        let mut r_d1 = BufferT::zeroed();
        let mut r_d2 = BufferT::zeroed();
        let c_ret = unsafe { c_fn(&src, pos, &mut c_d1, &mut c_d2) };
        let r_ret = unsafe { r_fn(&src, pos, &mut r_d1, &mut r_d2) };
        assert_eq!(c_ret, r_ret, "buffer_split return mismatch at pos={}", pos);
        if c_ret == 0 {
            assert_bufs_eq(&c_d1, &r_d1, &format!("buffer_split part1 pos={}", pos));
            assert_bufs_eq(&c_d2, &r_d2, &format!("buffer_split part2 pos={}", pos));
        }
    }

    // Out of bounds
    let mut c_d1 = BufferT::zeroed();
    let mut c_d2 = BufferT::zeroed();
    let mut r_d1 = BufferT::zeroed();
    let mut r_d2 = BufferT::zeroed();
    let c_ret = unsafe { c_fn(&src, 20, &mut c_d1, &mut c_d2) };
    let r_ret = unsafe { r_fn(&src, 20, &mut r_d1, &mut r_d2) };
    assert_eq!(c_ret, r_ret, "buffer_split OOB return mismatch");
}

// ==================== buffer_interleave tests ====================

#[test]
fn test_buffer_interleave() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const BufferT, *const BufferT, *mut BufferT) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_interleave").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_interleave").unwrap() };

    let cases: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (vec![1, 3, 5], vec![2, 4, 6]),
        (vec![1, 2, 3], vec![10]),
        (vec![10], vec![1, 2, 3]),
        (vec![], vec![1, 2]),
        (vec![1, 2], vec![]),
        (vec![], vec![]),
    ];
    for (d1, d2) in &cases {
        let s1 = make_buf_with_checksum(&libs, d1);
        let s2 = make_buf_with_checksum(&libs, d2);
        let mut c_dst = BufferT::zeroed();
        let mut r_dst = BufferT::zeroed();
        let c_ret = unsafe { c_fn(&s1, &s2, &mut c_dst) };
        let r_ret = unsafe { r_fn(&s1, &s2, &mut r_dst) };
        assert_eq!(c_ret, r_ret, "buffer_interleave return mismatch");
        if c_ret == 0 {
            assert_bufs_eq(&c_dst, &r_dst, "buffer_interleave");
        }
    }
}

// ==================== buffer_rotate tests ====================

#[test]
fn test_buffer_rotate() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut BufferT, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_rotate").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_rotate").unwrap() };

    let data: Vec<u8> = (1..=5).collect();
    for positions in [-7, -5, -3, -1, 0, 1, 2, 3, 5, 7, 100] {
        let mut c_buf = make_buf_with_checksum(&libs, &data);
        let mut r_buf = c_buf.clone();
        let c_ret = unsafe { c_fn(&mut c_buf, positions) };
        let r_ret = unsafe { r_fn(&mut r_buf, positions) };
        assert_eq!(c_ret, r_ret, "buffer_rotate return mismatch pos={}", positions);
        assert_bufs_eq(&c_buf, &r_buf, &format!("buffer_rotate pos={}", positions));
    }

    // Empty buffer
    let mut c_buf = make_buf_with_checksum(&libs, &[]);
    let mut r_buf = c_buf.clone();
    assert_eq!(unsafe { c_fn(&mut c_buf, 3) }, unsafe { r_fn(&mut r_buf, 3) });
}

// ==================== buffer_conditional_copy tests ====================

#[test]
fn test_buffer_conditional_copy() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const BufferT, *mut BufferT, u8, bool) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_conditional_copy").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_conditional_copy").unwrap() };

    let src = make_buf_with_checksum(&libs, &[1, 2, 3, 2, 5, 2]);
    for (pattern, copy_matching) in [(2u8, true), (2, false), (99, true), (99, false)] {
        let mut c_dst = BufferT::zeroed();
        let mut r_dst = BufferT::zeroed();
        let c_ret = unsafe { c_fn(&src, &mut c_dst, pattern, copy_matching) };
        let r_ret = unsafe { r_fn(&src, &mut r_dst, pattern, copy_matching) };
        assert_eq!(c_ret, r_ret, "conditional_copy return mismatch p={} m={}", pattern, copy_matching);
        if c_ret == 0 {
            assert_bufs_eq(&c_dst, &r_dst, &format!("conditional_copy p={} m={}", pattern, copy_matching));
        }
    }
}

// ==================== buffer_copy_strided tests ====================

#[test]
fn test_buffer_copy_strided() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const BufferT, *mut BufferT, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"buffer_copy_strided").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.r.get(b"buffer_copy_strided").unwrap() };

    let src = make_buf_with_checksum(&libs, &(0..20).collect::<Vec<u8>>());
    for stride in [1, 2, 3, 5, 7, 20, 100] {
        let mut c_dst = BufferT::zeroed();
        let mut r_dst = BufferT::zeroed();
        let c_ret = unsafe { c_fn(&src, &mut c_dst, stride) };
        let r_ret = unsafe { r_fn(&src, &mut r_dst, stride) };
        assert_eq!(c_ret, r_ret, "copy_strided return mismatch stride={}", stride);
        if c_ret == 0 {
            assert_bufs_eq(&c_dst, &r_dst, &format!("copy_strided stride={}", stride));
        }
    }

    // Invalid stride
    let mut c_dst = BufferT::zeroed();
    let mut r_dst = BufferT::zeroed();
    assert_eq!(
        unsafe { c_fn(&src, &mut c_dst, 0) },
        unsafe { r_fn(&src, &mut r_dst, 0) },
        "copy_strided stride=0 return mismatch"
    );
    assert_eq!(
        unsafe { c_fn(&src, &mut c_dst, -1) },
        unsafe { r_fn(&src, &mut r_dst, -1) },
        "copy_strided stride=-1 return mismatch"
    );
}

// ==================== init/free buffer_array tests ====================

#[test]
fn test_init_free_buffer_array() {
    let libs = Libs::load();
    type InitFn = unsafe extern "C" fn(c_int) -> *mut BufferArrayT;
    type FreeFn = unsafe extern "C" fn(*mut BufferArrayT);
    let c_init: Symbol<InitFn> = unsafe { libs.c.get(b"init_buffer_array").unwrap() };
    let r_init: Symbol<InitFn> = unsafe { libs.r.get(b"init_buffer_array").unwrap() };
    let c_free: Symbol<FreeFn> = unsafe { libs.c.get(b"free_buffer_array").unwrap() };
    let r_free: Symbol<FreeFn> = unsafe { libs.r.get(b"free_buffer_array").unwrap() };

    // Valid capacity
    let c_arr = unsafe { c_init(10) };
    let r_arr = unsafe { r_init(10) };
    assert!(!c_arr.is_null());
    assert!(!r_arr.is_null());
    unsafe {
        assert_eq!((*c_arr).count, (*r_arr).count);
        assert_eq!((*c_arr).capacity, (*r_arr).capacity);
        c_free(c_arr);
        r_free(r_arr);
    }

    // Invalid capacity
    let c_arr = unsafe { c_init(0) };
    let r_arr = unsafe { r_init(0) };
    assert_eq!(c_arr.is_null(), r_arr.is_null());
    let c_arr = unsafe { c_init(-5) };
    let r_arr = unsafe { r_init(-5) };
    assert_eq!(c_arr.is_null(), r_arr.is_null());
}

// ==================== process_buffer_array tests ====================

#[test]
fn test_process_buffer_array() {
    let libs = Libs::load();
    type InitFn = unsafe extern "C" fn(c_int) -> *mut BufferArrayT;
    type FreeFn = unsafe extern "C" fn(*mut BufferArrayT);
    type ProcFn = unsafe extern "C" fn(*mut BufferArrayT, c_int, c_int) -> c_int;
    type CkFn = unsafe extern "C" fn(*const u8, usize) -> u32;

    let c_init: Symbol<InitFn> = unsafe { libs.c.get(b"init_buffer_array").unwrap() };
    let r_init: Symbol<InitFn> = unsafe { libs.r.get(b"init_buffer_array").unwrap() };
    let c_free: Symbol<FreeFn> = unsafe { libs.c.get(b"free_buffer_array").unwrap() };
    let r_free: Symbol<FreeFn> = unsafe { libs.r.get(b"free_buffer_array").unwrap() };
    let c_proc: Symbol<ProcFn> = unsafe { libs.c.get(b"process_buffer_array").unwrap() };
    let r_proc: Symbol<ProcFn> = unsafe { libs.r.get(b"process_buffer_array").unwrap() };
    let c_ck: Symbol<CkFn> = unsafe { libs.c.get(b"calculate_checksum").unwrap() };

    let fill_array = |init_fn: &Symbol<InitFn>, ck_fn: &Symbol<CkFn>| -> *mut BufferArrayT {
        let arr = unsafe { init_fn(4) };
        unsafe {
            for i in 0..3 {
                let buf = &mut *(*arr).buffers.add(i);
                let data: Vec<u8> = ((i as u8 * 10)..((i as u8 * 10) + 5)).collect();
                buf.data[..5].copy_from_slice(&data);
                buf.length = 5;
                buf.checksum = ck_fn(buf.data.as_ptr(), buf.length);
            }
            (*arr).count = 3;
        }
        arr
    };

    // Test OP_COPY (0), OP_REVERSE (1), OP_ROTATE (5), OP_CHECKSUM (6)
    for (op, param) in [(0, 0), (1, 0), (5, 2), (6, 0)] {
        let c_arr = fill_array(&c_init, &c_ck);
        let r_arr = fill_array(&r_init, &c_ck);
        let c_ret = unsafe { c_proc(c_arr, op, param) };
        let r_ret = unsafe { r_proc(r_arr, op, param) };
        assert_eq!(c_ret, r_ret, "process_buffer_array op={} return mismatch", op);
        if c_ret == 0 {
            unsafe {
                let count = (*c_arr).count as usize;
                for i in 0..count {
                    let cb = &*(*c_arr).buffers.add(i);
                    let rb = &*(*r_arr).buffers.add(i);
                    assert_bufs_eq(cb, rb, &format!("process_buffer_array op={} buf={}", op, i));
                }
            }
        }
        unsafe { c_free(c_arr); r_free(r_arr); }
    }

    // Test OP_MERGE (2) with even count
    let c_arr = fill_array(&c_init, &c_ck);
    let r_arr = fill_array(&r_init, &c_ck);
    // Set count to 2 for clean merge
    unsafe { (*c_arr).count = 2; (*r_arr).count = 2; }
    let c_ret = unsafe { c_proc(c_arr, 2, 0) };
    let r_ret = unsafe { r_proc(r_arr, 2, 0) };
    assert_eq!(c_ret, r_ret, "process_buffer_array OP_MERGE return mismatch");
    if c_ret == 0 {
        unsafe {
            let cb = &*(*c_arr).buffers.add(0);
            let rb = &*(*r_arr).buffers.add(0);
            assert_bufs_eq(cb, rb, "process_buffer_array OP_MERGE buf=0");
        }
    }
    unsafe { c_free(c_arr); r_free(r_arr); }
}
