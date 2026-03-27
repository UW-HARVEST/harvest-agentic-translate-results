use libloading::{Library, Symbol};
use std::os::raw::c_int;

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
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libflac_validate_lib.so"
    );
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

#[test]
fn test_tflac_size_memory() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(u32) -> u32> =
        unsafe { lib.get(b"tflac_size_memory").unwrap() };

    let test_values: &[u32] = &[0, 1, 16, 64, 128, 255, 256, 1024, 4096, 65535, 0xFFFFFFFF];
    for &bs in test_values {
        let c_result = unsafe { c_fn(bs) };
        let rust_result = flac_validate_lib::tflac_size_memory(bs);
        assert_eq!(
            c_result, rust_result,
            "tflac_size_memory({bs}): C={c_result}, Rust={rust_result}"
        );
    }
}

#[test]
fn test_flac_validate() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*mut Tflac) -> c_int> =
        unsafe { lib.get(b"flac_validate").unwrap() };

    let cases: Vec<Tflac> = vec![
        // Valid basic case
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // blocksize too small
        Tflac { blocksize: 8, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // blocksize too large
        Tflac { blocksize: 65536, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // samplerate 0
        Tflac { blocksize: 4096, samplerate: 0, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // channels 0
        Tflac { blocksize: 4096, samplerate: 44100, channels: 0, bitdepth: 16,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // channels > 8
        Tflac { blocksize: 4096, samplerate: 44100, channels: 9, bitdepth: 16,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // bitdepth 0
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 0,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // bitdepth > 32
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 33,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // channel_mode non-zero with channels!=2 -> reset to 0
        Tflac { blocksize: 4096, samplerate: 44100, channels: 4, bitdepth: 16,
                channel_mode: 1, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // channel_mode non-zero with bitdepth==32 -> reset to 0
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 32,
                channel_mode: 2, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // channel_mode non-zero, channels==2, bitdepth<32 -> keep
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 3, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // max_rice_value > 30
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 31, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // max_rice_value set, bitdepth > 16
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 24,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 8, partition_order: 0, cur_blocksize: 0 },
        // max_partition_order > 15
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 14, min_partition_order: 0,
                max_partition_order: 16, partition_order: 0, cur_blocksize: 0 },
        // min > max partition order
        Tflac { blocksize: 4096, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 14, min_partition_order: 5,
                max_partition_order: 3, partition_order: 0, cur_blocksize: 0 },
        // partition_order loop: blocksize=1024, min=0, max=10
        Tflac { blocksize: 1024, samplerate: 44100, channels: 2, bitdepth: 16,
                channel_mode: 0, max_rice_value: 14, min_partition_order: 0,
                max_partition_order: 10, partition_order: 0, cur_blocksize: 0 },
        // blocksize=16 (minimum valid)
        Tflac { blocksize: 16, samplerate: 1, channels: 1, bitdepth: 1,
                channel_mode: 0, max_rice_value: 1, min_partition_order: 0,
                max_partition_order: 0, partition_order: 0, cur_blocksize: 0 },
        // non-power-of-2 blocksize
        Tflac { blocksize: 4000, samplerate: 48000, channels: 2, bitdepth: 24,
                channel_mode: 0, max_rice_value: 0, min_partition_order: 0,
                max_partition_order: 15, partition_order: 0, cur_blocksize: 0 },
    ];

    for (i, case) in cases.iter().enumerate() {
        let mut c_copy = case.clone();
        let mut r_copy = case.clone();

        let c_ret = unsafe { c_fn(&mut c_copy as *mut Tflac) };
        let rust_ret = unsafe {
            flac_validate_lib::flac_validate(
                &mut r_copy as *mut Tflac as *mut flac_validate_lib::tflac,
            )
        };

        assert_eq!(c_ret, rust_ret, "case {i}: return mismatch: C={c_ret}, Rust={rust_ret}");

        // Compare struct bytes
        let c_bytes = unsafe {
            std::slice::from_raw_parts(
                &c_copy as *const Tflac as *const u8,
                std::mem::size_of::<Tflac>(),
            )
        };
        let r_bytes = unsafe {
            std::slice::from_raw_parts(
                &r_copy as *const Tflac as *const u8,
                std::mem::size_of::<Tflac>(),
            )
        };
        assert_eq!(c_bytes, r_bytes, "case {i}: struct bytes differ after flac_validate");
    }
}
