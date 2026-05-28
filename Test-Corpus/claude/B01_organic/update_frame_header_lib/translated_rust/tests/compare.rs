//! Cross-language tests comparing C and Rust implementations of
//! `update_frame_header` byte-for-byte through their FFI boundaries.

use libloading::{Library, Symbol};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tflac {
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    // Note: there is implicit alignment padding here between the u8 and the
    // following u32. The C struct layout matches.
    frame_header: u32,
    cur_blocksize: u32,
}

type UpdateFrameHeaderFn = unsafe extern "C" fn(*mut Tflac);

fn c_lib_path() -> &'static str {
    "c_src/build/libtranslated_rust.so"
}

fn rust_lib_path() -> &'static str {
    // Tests run with the working dir = crate root
    if std::path::Path::new("target/debug/libupdate_frame_header_lib.so").exists() {
        "target/debug/libupdate_frame_header_lib.so"
    } else {
        "target/release/libupdate_frame_header_lib.so"
    }
}

fn run_both(input: Tflac) -> (Tflac, Tflac) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("failed to load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("failed to load Rust lib");
        let c_fn: Symbol<UpdateFrameHeaderFn> =
            c_lib.get(b"update_frame_header").expect("no C symbol");
        let rust_fn: Symbol<UpdateFrameHeaderFn> =
            rust_lib.get(b"update_frame_header").expect("no Rust symbol");

        let mut c_state = input;
        let mut r_state = input;
        c_fn(&mut c_state as *mut Tflac);
        rust_fn(&mut r_state as *mut Tflac);
        (c_state, r_state)
    }
}

fn assert_match(input: Tflac) {
    let (c, r) = run_both(input);
    assert_eq!(
        c.frame_header, r.frame_header,
        "frame_header mismatch for input {:?}: C=0x{:08X} Rust=0x{:08X}",
        input, c.frame_header, r.frame_header
    );
    // All other fields should be untouched & equal too
    assert_eq!(c.samplerate, r.samplerate);
    assert_eq!(c.channels, r.channels);
    assert_eq!(c.bitdepth, r.bitdepth);
    assert_eq!(c.channel_mode, r.channel_mode);
    assert_eq!(c.cur_blocksize, r.cur_blocksize);
}

fn make(samplerate: u32, channels: u32, bitdepth: u32, channel_mode: u8, cur_blocksize: u32) -> Tflac {
    Tflac {
        samplerate,
        channels,
        bitdepth,
        channel_mode,
        frame_header: 0,
        cur_blocksize,
    }
}

#[test]
fn test_known_blocksizes() {
    let blocks = [192u32, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768];
    for &b in &blocks {
        assert_match(make(44100, 2, 16, 0, b));
    }
}

#[test]
fn test_blocksize_default_branches() {
    // <= 256 path
    for b in [1u32, 2, 100, 200, 255] {
        assert_match(make(44100, 2, 16, 0, b));
    }
    // > 256 path (and not in the explicit case list)
    for b in [257u32, 300, 1000, 5000, 10000, 100000, 0xFFFF_FFFF] {
        assert_match(make(44100, 2, 16, 0, b));
    }
}

#[test]
fn test_known_samplerates() {
    let rates = [
        882000u32, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
    ];
    for &r in &rates {
        assert_match(make(r, 2, 16, 0, 4096));
    }
}

#[test]
fn test_samplerate_default_branches() {
    // multiples of 1000, kHz < 256 -> 0xC
    for r in [1000u32, 11000, 100000, 255000] {
        assert_match(make(r, 2, 16, 0, 4096));
    }
    // multiples of 1000, kHz >= 256 -> nothing
    for r in [256000u32, 300000, 1_000_000] {
        assert_match(make(r, 2, 16, 0, 4096));
    }
    // not a multiple of 1000, but < 65536 -> 0xD
    for r in [12345u32, 65535, 11025, 33333] {
        assert_match(make(r, 2, 16, 0, 4096));
    }
    // not a multiple of 1000 but a multiple of 10, /10 < 65536 -> 0xE
    for r in [80000u32, 200010, 655350, 100010] {
        assert_match(make(r, 2, 16, 0, 4096));
    }
    // multiples of 10, /10 >= 65536 -> nothing
    for r in [655360u32, 1_000_000, 6_553_600] {
        assert_match(make(r, 2, 16, 0, 4096));
    }
    // truly arbitrary cases
    for r in [0u32, 1, 7, 65537, 100001, 1_000_001, u32::MAX] {
        assert_match(make(r, 2, 16, 0, 4096));
    }
}

#[test]
fn test_channel_modes() {
    // independent: channels 1..=8
    for ch in 1u32..=8u32 {
        assert_match(make(44100, ch, 16, 0, 4096));
    }
    // left/side, side/right, mid/side
    for cm in [1u8, 2, 3] {
        assert_match(make(44100, 2, 16, cm, 4096));
    }
    // mode % 4 wraps around: e.g., 4 -> independent
    for cm in [4u8, 5, 6, 7, 8, 9, 10, 11, 100, 255] {
        assert_match(make(44100, 2, 16, cm, 4096));
    }
}

#[test]
fn test_bitdepths() {
    for bd in [8u32, 12, 16, 20, 24, 32] {
        assert_match(make(44100, 2, 16, 0, 4096).clone());
        assert_match(make(44100, 2, bd, 0, 4096));
    }
    // unknown bitdepths -> no bits set in that field
    for bd in [1u32, 4, 6, 7, 9, 10, 11, 13, 17, 25, 31, 33, 64, 1000] {
        assert_match(make(44100, 2, bd, 0, 4096));
    }
}

#[test]
fn test_combinations() {
    let samplerates = [44100u32, 48000, 22050, 8000, 100000, 12345, 655360, 0, u32::MAX];
    let channels = [1u32, 2, 4, 8];
    let bitdepths = [8u32, 16, 24, 32, 17];
    let modes = [0u8, 1, 2, 3, 4, 7];
    let blocks = [4096u32, 192, 100, 1000, 32768];
    for &sr in &samplerates {
        for &ch in &channels {
            for &bd in &bitdepths {
                for &cm in &modes {
                    for &b in &blocks {
                        assert_match(make(sr, ch, bd, cm, b));
                    }
                }
            }
        }
    }
}

#[test]
fn test_initial_frame_header_overwritten() {
    // Even if the input frame_header is non-zero, the function should overwrite it
    // (it begins with `t->frame_header = 0xFFF8U << 16;`).
    let mut t = make(44100, 2, 16, 0, 4096);
    t.frame_header = 0xDEAD_BEEF;
    assert_match(t);
}
