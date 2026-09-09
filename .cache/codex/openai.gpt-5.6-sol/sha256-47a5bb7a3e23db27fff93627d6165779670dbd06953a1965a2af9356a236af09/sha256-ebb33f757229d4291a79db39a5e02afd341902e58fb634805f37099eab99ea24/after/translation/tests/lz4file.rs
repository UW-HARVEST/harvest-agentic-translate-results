mod common;

use common::{Libraries, Rng, ptr_or_dangling};
use std::ffi::{c_int, c_long, c_void};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FrameInfo {
    block_size_id: c_int,
    block_mode: c_int,
    content_checksum_flag: c_int,
    frame_type: c_int,
    content_size: u64,
    dict_id: u32,
    block_checksum_flag: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Preferences {
    frame_info: FrameInfo,
    compression_level: c_int,
    auto_flush: u32,
    favor_dec_speed: u32,
    reserved: [u32; 3],
}

unsafe extern "C" {
    fn tmpfile() -> *mut c_void;
    fn rewind(stream: *mut c_void);
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fwrite(ptr: *const c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fseek(stream: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn fclose(stream: *mut c_void) -> c_int;
}

#[test]
fn file_write_and_read_lifecycles_match_byte_for_byte() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Rng::new(0x9216_d5d9_8979_fb1b);
        for iteration in 0..20 {
            let len = (rng.next_u64() as usize) % 180_000;
            let mut input = rng.bytes(len);
            if iteration & 1 == 0 {
                for byte in &mut input {
                    *byte &= 7;
                }
            }
            let prefs = Preferences {
                frame_info: FrameInfo {
                    block_size_id: [4, 5, 6, 7][iteration % 4],
                    block_mode: (iteration & 1) as c_int,
                    content_checksum_flag: ((iteration >> 1) & 1) as c_int,
                    content_size: input.len() as u64,
                    block_checksum_flag: ((iteration >> 2) & 1) as c_int,
                    ..FrameInfo::default()
                },
                compression_level: [-3, 0, 9, 12][iteration % 4],
                auto_flush: (iteration & 1) as u32,
                favor_dec_speed: (iteration >= 10) as u32,
                reserved: [0; 3],
            };
            let c_encoded = write_file(&libs.c, &input, &prefs);
            let r_encoded = write_file(&libs.rust, &input, &prefs);
            assert_eq!(c_encoded, r_encoded);
            assert_eq!(read_file(&libs.c, &c_encoded, 1 + iteration * 37), input);
            assert_eq!(read_file(&libs.rust, &r_encoded, 1 + iteration * 37), input);
        }
    }
}

unsafe fn write_file(lib: &libloading::Library, input: &[u8], prefs: &Preferences) -> Vec<u8> {
    let open = unsafe {
        lib.get::<unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize>(
            b"LZ4F_writeOpen\0",
        )
        .unwrap()
    };
    let write = unsafe {
        lib.get::<unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize>(b"LZ4F_write\0")
            .unwrap()
    };
    let close = unsafe {
        lib.get::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_writeClose\0")
            .unwrap()
    };
    let is_error = unsafe {
        lib.get::<unsafe extern "C" fn(usize) -> u32>(b"LZ4F_isError\0")
            .unwrap()
    };
    let file = unsafe { tmpfile() };
    assert!(!file.is_null());
    let mut state = std::ptr::null_mut();
    let opened = unsafe { open(&mut state, file, prefs) };
    assert_eq!(unsafe { is_error(opened) }, 0);
    for chunk in input.chunks(3001) {
        let written = unsafe { write(state, ptr_or_dangling(chunk), chunk.len()) };
        assert_eq!(unsafe { is_error(written) }, 0);
        assert_eq!(written, chunk.len());
    }
    let closed = unsafe { close(state) };
    assert_eq!(unsafe { is_error(closed) }, 0);
    let bytes = unsafe { read_all(file) };
    unsafe { fclose(file) };
    bytes
}

unsafe fn read_file(lib: &libloading::Library, encoded: &[u8], chunk_size: usize) -> Vec<u8> {
    let open = unsafe {
        lib.get::<unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize>(b"LZ4F_readOpen\0")
            .unwrap()
    };
    let read = unsafe {
        lib.get::<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>(b"LZ4F_read\0")
            .unwrap()
    };
    let close = unsafe {
        lib.get::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_readClose\0")
            .unwrap()
    };
    let is_error = unsafe {
        lib.get::<unsafe extern "C" fn(usize) -> u32>(b"LZ4F_isError\0")
            .unwrap()
    };
    let file = unsafe { tmpfile() };
    assert!(!file.is_null());
    assert_eq!(
        unsafe { fwrite(ptr_or_dangling(encoded), 1, encoded.len(), file) },
        encoded.len()
    );
    unsafe { rewind(file) };
    let mut state = std::ptr::null_mut();
    let opened = unsafe { open(&mut state, file) };
    assert_eq!(unsafe { is_error(opened) }, 0);
    let mut output = Vec::new();
    loop {
        let mut buffer = vec![0u8; chunk_size.max(1)];
        let amount = unsafe { read(state, buffer.as_mut_ptr().cast(), chunk_size) };
        assert_eq!(unsafe { is_error(amount) }, 0);
        if amount == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..amount]);
    }
    assert_eq!(unsafe { is_error(close(state)) }, 0);
    unsafe { fclose(file) };
    output
}

unsafe fn read_all(file: *mut c_void) -> Vec<u8> {
    assert_eq!(unsafe { fseek(file, 0, 2) }, 0);
    let length = unsafe { ftell(file) };
    assert!(length >= 0);
    unsafe { rewind(file) };
    let mut bytes = vec![0u8; length as usize];
    assert_eq!(
        unsafe { fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), file) },
        bytes.len()
    );
    bytes
}

#[test]
fn file_api_null_and_empty_file_errors_match_exactly() {
    unsafe {
        let libs = Libraries::load();
        let (c_read_open, r_read_open) = libs
            .pair::<unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize>(
                b"LZ4F_readOpen\0",
            );
        let (c_write_open, r_write_open) = libs.pair::<unsafe extern "C" fn(
            *mut *mut c_void,
            *mut c_void,
            *const Preferences,
        ) -> usize>(b"LZ4F_writeOpen\0");
        let mut cs = std::ptr::null_mut();
        let mut rs = std::ptr::null_mut();
        assert_eq!(
            c_read_open(&mut cs, std::ptr::null_mut()),
            r_read_open(&mut rs, std::ptr::null_mut())
        );
        assert_eq!(
            c_read_open(std::ptr::null_mut(), std::ptr::null_mut()),
            r_read_open(std::ptr::null_mut(), std::ptr::null_mut())
        );
        assert_eq!(
            c_write_open(&mut cs, std::ptr::null_mut(), std::ptr::null()),
            r_write_open(&mut rs, std::ptr::null_mut(), std::ptr::null())
        );

        let c_file = tmpfile();
        let r_file = tmpfile();
        assert_eq!(c_read_open(&mut cs, c_file), r_read_open(&mut rs, r_file));
        fclose(c_file);
        fclose(r_file);

        let (c_read, r_read) = libs
            .pair::<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>(b"LZ4F_read\0");
        let (c_write, r_write) =
            libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize>(
                b"LZ4F_write\0",
            );
        let mut byte = 0u8;
        assert_eq!(
            c_read(std::ptr::null_mut(), (&mut byte as *mut u8).cast(), 1),
            r_read(std::ptr::null_mut(), (&mut byte as *mut u8).cast(), 1)
        );
        assert_eq!(
            c_write(std::ptr::null_mut(), (&byte as *const u8).cast(), 1),
            r_write(std::ptr::null_mut(), (&byte as *const u8).cast(), 1)
        );

        let (c_read_close, r_read_close) =
            libs.pair::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_readClose\0");
        let (c_write_close, r_write_close) =
            libs.pair::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_writeClose\0");
        assert_eq!(
            c_read_close(std::ptr::null_mut()),
            r_read_close(std::ptr::null_mut())
        );
        assert_eq!(
            c_write_close(std::ptr::null_mut()),
            r_write_close(std::ptr::null_mut())
        );
    }
}
