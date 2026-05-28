use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tflac {
    pub blocksize: u32,
    pub samplerate: u32,
    pub channels: u32,
    pub bitdepth: u32,
    pub channel_mode: u8,
    pub max_rice_value: u8,
    pub min_partition_order: u8,
    pub max_partition_order: u8,
    pub partition_order: u8,
    // 3 bytes of padding to align cur_blocksize to 4 bytes
    pub _pad: [u8; 3],
    pub cur_blocksize: u32,
}

impl Default for Tflac {
    fn default() -> Self {
        Tflac {
            blocksize: 0,
            samplerate: 0,
            channels: 0,
            bitdepth: 0,
            channel_mode: 0,
            max_rice_value: 0,
            min_partition_order: 0,
            max_partition_order: 0,
            partition_order: 0,
            _pad: [0u8; 3],
            cur_blocksize: 0,
        }
    }
}

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/debug/libflac_validate_lib.so";

fn load_libs() -> (Library, Library) {
    unsafe {
        let c_lib = Library::new(C_LIB).expect("Failed to load C lib");
        let rust_lib = Library::new(RUST_LIB).expect("Failed to load Rust lib");
        (c_lib, rust_lib)
    }
}

#[test]
fn test_tflac_size_memory() {
    let (c_lib, rust_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(u32) -> u32> =
            c_lib.get(b"tflac_size_memory").unwrap();
        let rust_fn: Symbol<unsafe extern "C" fn(u32) -> u32> =
            rust_lib.get(b"tflac_size_memory").unwrap();

        let test_cases = [
            0u32, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192,
            16384, 32768, 65535, 65536, 100000, 1_000_000, 0xFFFFFFFF, 0xFFFFFFF0,
            15, 17, 31, 33, 100, 4095, 4097,
        ];

        for &bs in &test_cases {
            let c_result = c_fn(bs);
            let rust_result = rust_fn(bs);
            assert_eq!(
                c_result, rust_result,
                "tflac_size_memory({}): C={}, Rust={}",
                bs, c_result, rust_result
            );
        }
    }
}

fn run_validate_compare(input: Tflac) {
    let (c_lib, rust_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut Tflac) -> c_int> =
            c_lib.get(b"flac_validate").unwrap();
        let rust_fn: Symbol<unsafe extern "C" fn(*mut Tflac) -> c_int> =
            rust_lib.get(b"flac_validate").unwrap();

        let mut c_state = input.clone();
        let mut r_state = input.clone();

        let c_ret = c_fn(&mut c_state as *mut Tflac);
        let r_ret = rust_fn(&mut r_state as *mut Tflac);

        assert_eq!(c_ret, r_ret, "return mismatch for input {:?}", input);

        // Compare struct contents byte-for-byte (excluding padding)
        assert_eq!(c_state.blocksize, r_state.blocksize, "blocksize for {:?}", input);
        assert_eq!(c_state.samplerate, r_state.samplerate, "samplerate for {:?}", input);
        assert_eq!(c_state.channels, r_state.channels, "channels for {:?}", input);
        assert_eq!(c_state.bitdepth, r_state.bitdepth, "bitdepth for {:?}", input);
        assert_eq!(c_state.channel_mode, r_state.channel_mode, "channel_mode for {:?}", input);
        assert_eq!(c_state.max_rice_value, r_state.max_rice_value, "max_rice_value for {:?}", input);
        assert_eq!(c_state.min_partition_order, r_state.min_partition_order, "min_partition_order for {:?}", input);
        assert_eq!(c_state.max_partition_order, r_state.max_partition_order, "max_partition_order for {:?}", input);
        assert_eq!(c_state.partition_order, r_state.partition_order, "partition_order for {:?}", input);
        assert_eq!(c_state.cur_blocksize, r_state.cur_blocksize, "cur_blocksize for {:?}", input);
    }
}

fn make_valid() -> Tflac {
    Tflac {
        blocksize: 4096,
        samplerate: 44100,
        channels: 2,
        bitdepth: 16,
        channel_mode: 0,
        max_rice_value: 0,
        min_partition_order: 0,
        max_partition_order: 0,
        partition_order: 0,
        _pad: [0u8; 3],
        cur_blocksize: 0,
    }
}

#[test]
fn test_validate_valid_default() {
    run_validate_compare(make_valid());
}

#[test]
fn test_validate_blocksize_bounds() {
    // Below min
    for bs in [0u32, 1, 5, 15] {
        let mut t = make_valid();
        t.blocksize = bs;
        run_validate_compare(t);
    }
    // Above max
    for bs in [65536u32, 100_000, 0xFFFFFFFF] {
        let mut t = make_valid();
        t.blocksize = bs;
        run_validate_compare(t);
    }
    // Boundaries
    for bs in [16u32, 17, 65534, 65535] {
        let mut t = make_valid();
        t.blocksize = bs;
        run_validate_compare(t);
    }
}

#[test]
fn test_validate_samplerate_bounds() {
    for sr in [0u32, 1, 44100, 655350, 655351, 1_000_000, 0xFFFFFFFF] {
        let mut t = make_valid();
        t.samplerate = sr;
        run_validate_compare(t);
    }
}

#[test]
fn test_validate_channels_bounds() {
    for ch in 0u32..=10 {
        let mut t = make_valid();
        t.channels = ch;
        run_validate_compare(t);
    }
    let mut t = make_valid();
    t.channels = 100;
    run_validate_compare(t);
}

#[test]
fn test_validate_bitdepth_bounds() {
    for bd in [0u32, 1, 8, 16, 24, 32, 33, 64] {
        let mut t = make_valid();
        t.bitdepth = bd;
        run_validate_compare(t);
    }
}

#[test]
fn test_validate_channel_mode() {
    // Various channel modes with various channel counts and bitdepths
    for cm in 0u8..=4 {
        for ch in [1u32, 2, 3] {
            for bd in [16u32, 24, 32] {
                let mut t = make_valid();
                t.channel_mode = cm;
                t.channels = ch;
                t.bitdepth = bd;
                run_validate_compare(t);
            }
        }
    }
}

#[test]
fn test_validate_max_rice_value() {
    for mrv in [0u8, 1, 14, 15, 30, 31, 100, 255] {
        for bd in [8u32, 16, 17, 24, 32] {
            let mut t = make_valid();
            t.max_rice_value = mrv;
            t.bitdepth = bd;
            run_validate_compare(t);
        }
    }
}

#[test]
fn test_validate_partition_orders() {
    for max_po in [0u8, 1, 5, 15, 16, 31, 255] {
        for min_po in [0u8, 1, 5, 15, 16] {
            let mut t = make_valid();
            t.max_partition_order = max_po;
            t.min_partition_order = min_po;
            run_validate_compare(t);
        }
    }
}

#[test]
fn test_validate_partition_order_loop() {
    // Test cases that exercise the partition_order increment loop
    let cases = [
        (4096u32, 0u8, 8u8),
        (4096, 0, 15),
        (4096, 5, 10),
        (1024, 0, 10),
        (16, 0, 15),
        (32, 0, 15),
        (16, 0, 0),
        (17, 0, 5),
        (48, 0, 5),
        (4080, 0, 15),
    ];
    for &(bs, min_po, max_po) in &cases {
        let mut t = make_valid();
        t.blocksize = bs;
        t.min_partition_order = min_po;
        t.max_partition_order = max_po;
        run_validate_compare(t);
    }
}

#[test]
fn test_validate_combinatorial() {
    // A small grid of combinations
    let blocksizes = [16u32, 1024, 4096, 65535];
    let samplerates = [1u32, 44100, 655350];
    let channels = [1u32, 2, 8];
    let bitdepths = [1u32, 16, 24, 32];
    let cms = [0u8, 1, 2, 3];

    for &bs in &blocksizes {
        for &sr in &samplerates {
            for &ch in &channels {
                for &bd in &bitdepths {
                    for &cm in &cms {
                        let mut t = make_valid();
                        t.blocksize = bs;
                        t.samplerate = sr;
                        t.channels = ch;
                        t.bitdepth = bd;
                        t.channel_mode = cm;
                        run_validate_compare(t);
                    }
                }
            }
        }
    }
}
