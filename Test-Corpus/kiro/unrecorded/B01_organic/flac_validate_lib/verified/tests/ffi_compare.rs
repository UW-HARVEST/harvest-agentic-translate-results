use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone)]
struct Tflac {
    blocksize: u32,
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    max_rice_value: u8,
    min_partition_order: u8,
    max_partition_order: u8,
    partition_order: u8,
    cur_blocksize: u32,
}

fn c_lib() -> Library {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/libtranslated_rust.so");
    unsafe { Library::new(&p).expect("load C .so") }
}

fn rust_lib() -> Library {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libflac_validate_lib.so");
    unsafe { Library::new(&p).expect("load Rust .so") }
}

#[test]
fn test_tflac_size_memory() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(u32) -> u32> = unsafe { c.get(b"tflac_size_memory").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(u32) -> u32> = unsafe { r.get(b"tflac_size_memory").unwrap() };

    let test_vals: &[u32] = &[0, 1, 15, 16, 17, 100, 255, 256, 1024, 4096, 65535, 0xFFFFFFFF];
    for &bs in test_vals {
        let cv = unsafe { c_fn(bs) };
        let rv = unsafe { r_fn(bs) };
        assert_eq!(cv, rv, "tflac_size_memory({bs}): C={cv} Rust={rv}");
    }
}

fn make_tflac(blocksize: u32, samplerate: u32, channels: u32, bitdepth: u32,
              channel_mode: u8, max_rice_value: u8, min_po: u8, max_po: u8) -> Tflac {
    Tflac { blocksize, samplerate, channels, bitdepth, channel_mode,
            max_rice_value, min_partition_order: min_po, max_partition_order: max_po,
            partition_order: 0, cur_blocksize: 0 }
}

fn tflac_bytes(t: &Tflac) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts(t as *const Tflac as *const u8, std::mem::size_of::<Tflac>()).to_vec()
    }
}

#[test]
fn test_flac_validate() {
    let c = c_lib();
    let r = rust_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*mut Tflac) -> i32> = unsafe { c.get(b"flac_validate").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*mut Tflac) -> i32> = unsafe { r.get(b"flac_validate").unwrap() };

    let cases: Vec<Tflac> = vec![
        // valid basic
        make_tflac(4096, 44100, 2, 16, 0, 0, 0, 0),
        // blocksize too small
        make_tflac(15, 44100, 2, 16, 0, 0, 0, 0),
        // blocksize too large
        make_tflac(65536, 44100, 2, 16, 0, 0, 0, 0),
        // samplerate 0
        make_tflac(4096, 0, 2, 16, 0, 0, 0, 0),
        // samplerate too large
        make_tflac(4096, 655351, 2, 16, 0, 0, 0, 0),
        // channels 0
        make_tflac(4096, 44100, 0, 16, 0, 0, 0, 0),
        // channels > 8
        make_tflac(4096, 44100, 9, 16, 0, 0, 0, 0),
        // bitdepth 0
        make_tflac(4096, 44100, 2, 0, 0, 0, 0, 0),
        // bitdepth > 32
        make_tflac(4096, 44100, 2, 33, 0, 0, 0, 0),
        // channel_mode non-independent, channels=2, bitdepth=32 -> reset
        make_tflac(4096, 44100, 2, 32, 1, 0, 0, 0),
        // channel_mode non-independent, channels!=2 -> reset
        make_tflac(4096, 44100, 3, 16, 2, 0, 0, 0),
        // channel_mode non-independent, channels=2, bitdepth<32 -> keep
        make_tflac(4096, 44100, 2, 16, 3, 0, 0, 0),
        // max_rice_value > 30
        make_tflac(4096, 44100, 2, 16, 0, 31, 0, 0),
        // max_rice_value = 1 (valid nonzero)
        make_tflac(4096, 44100, 2, 16, 0, 1, 0, 0),
        // max_rice_value = 0, bitdepth > 16 -> set to 30
        make_tflac(4096, 44100, 2, 24, 0, 0, 0, 0),
        // max_partition_order > 15
        make_tflac(4096, 44100, 2, 16, 0, 0, 0, 16),
        // min > max partition order
        make_tflac(4096, 44100, 2, 16, 0, 0, 5, 3),
        // partition order iteration: blocksize=4096, min=0, max=15
        make_tflac(4096, 44100, 2, 16, 0, 0, 0, 15),
        // partition order: blocksize=1024, min=0, max=10
        make_tflac(1024, 44100, 2, 16, 0, 0, 0, 10),
        // partition order: blocksize not power of 2
        make_tflac(48, 44100, 2, 16, 0, 0, 0, 5),
        // edge: blocksize=16 (minimum valid)
        make_tflac(16, 44100, 1, 8, 0, 0, 0, 0),
        // edge: blocksize=65535 (maximum valid)
        make_tflac(65535, 44100, 8, 32, 0, 0, 0, 0),
        // max_rice_value=30 (boundary)
        make_tflac(4096, 44100, 2, 16, 0, 30, 0, 0),
    ];

    for (i, base) in cases.iter().enumerate() {
        let mut ct = base.clone();
        let mut rt = base.clone();
        let cret = unsafe { c_fn(&mut ct) };
        let rret = unsafe { r_fn(&mut rt) };
        assert_eq!(cret, rret, "case {i}: return mismatch C={cret} Rust={rret}");
        let cb = tflac_bytes(&ct);
        let rb = tflac_bytes(&rt);
        assert_eq!(cb, rb, "case {i}: struct bytes differ after call");
    }
}
