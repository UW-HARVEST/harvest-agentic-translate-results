use libloading::{Library, Symbol};
use update_frame_header_lib::tflac;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libupdate_frame_header_lib.so");

fn call_c(t: &mut tflac) {
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let func: Symbol<unsafe extern "C" fn(*mut tflac)> =
            lib.get(b"update_frame_header").expect("Failed to find symbol");
        func(t as *mut tflac);
    }
}

fn call_rust(t: &mut tflac) {
    unsafe { update_frame_header_lib::update_frame_header(t as *mut tflac) };
}

fn compare(samplerate: u32, channels: u32, bitdepth: u32, channel_mode: u8, cur_blocksize: u32) {
    let mut c = tflac { samplerate, channels, bitdepth, channel_mode, frame_header: 0, cur_blocksize };
    let mut r = tflac { samplerate, channels, bitdepth, channel_mode, frame_header: 0, cur_blocksize };
    call_c(&mut c);
    call_rust(&mut r);
    assert_eq!(
        c.frame_header, r.frame_header,
        "Mismatch: sr={samplerate} ch={channels} bd={bitdepth} cm={channel_mode} bs={cur_blocksize}: C=0x{:08X} Rust=0x{:08X}",
        c.frame_header, r.frame_header
    );
}

#[test]
fn test_blocksizes() {
    for bs in [192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 100, 300] {
        compare(44100, 2, 16, 0, bs);
    }
}

#[test]
fn test_samplerates() {
    for sr in [882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000, 11000, 33333, 700000] {
        compare(sr, 2, 16, 0, 4096);
    }
}

#[test]
fn test_channel_modes() {
    for cm in 0..=4u8 {
        compare(44100, 2, 16, cm, 4096);
    }
}

#[test]
fn test_bitdepths() {
    for bd in [8, 12, 16, 20, 24, 32, 4] {
        compare(44100, 2, bd, 0, 4096);
    }
}

#[test]
fn test_combined() {
    for bs in [192, 1024, 100] {
        for sr in [44100, 8000, 11000] {
            for cm in 0..=3u8 {
                for bd in [16, 24, 8] {
                    compare(sr, 2, bd, cm, bs);
                }
            }
        }
    }
}
