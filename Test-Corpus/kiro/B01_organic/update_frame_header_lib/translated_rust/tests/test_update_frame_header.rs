use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Debug)]
struct Tflac {
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    frame_header: u32,
    cur_blocksize: u32,
}

type UpdateFrameHeaderFn = unsafe extern "C" fn(*mut Tflac);

fn libs() -> (Library, Library) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("c_src/build/libtranslated_rust.so");
    let rust_path = manifest.join("target/debug/libupdate_frame_header_lib.so");
    unsafe {
        (
            Library::new(&c_path).expect("load C lib"),
            Library::new(&rust_path).expect("load Rust lib"),
        )
    }
}

fn call_both(c_lib: &Library, r_lib: &Library, input: &Tflac) -> (Tflac, Tflac) {
    unsafe {
        let c_fn: Symbol<UpdateFrameHeaderFn> = c_lib.get(b"update_frame_header").unwrap();
        let r_fn: Symbol<UpdateFrameHeaderFn> = r_lib.get(b"update_frame_header").unwrap();
        let mut c_t = input.clone();
        let mut r_t = input.clone();
        c_fn(&mut c_t);
        r_fn(&mut r_t);
        (c_t, r_t)
    }
}

fn base() -> Tflac {
    Tflac {
        samplerate: 44100,
        channels: 2,
        bitdepth: 16,
        channel_mode: 0,
        frame_header: 0,
        cur_blocksize: 4096,
    }
}

fn assert_match(c: &Tflac, r: &Tflac, label: &str) {
    assert_eq!(
        c.frame_header, r.frame_header,
        "{label}: C=0x{:08X} Rust=0x{:08X}",
        c.frame_header, r.frame_header
    );
}

#[test]
fn test_blocksizes() {
    let (c_lib, r_lib) = libs();
    let blocksizes: &[u32] = &[
        192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
        // defaults: <=256 and >256
        1, 100, 255, 257, 500, 65535,
    ];
    for &bs in blocksizes {
        let mut t = base();
        t.cur_blocksize = bs;
        let (c, r) = call_both(&c_lib, &r_lib, &t);
        assert_match(&c, &r, &format!("blocksize={bs}"));
    }
}

#[test]
fn test_samplerates() {
    let (c_lib, r_lib) = libs();
    let rates: &[u32] = &[
        882000, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
        // default: divisible by 1000, /1000 < 256
        5000, 255000,
        // default: divisible by 1000, /1000 >= 256
        256000, 999000,
        // default: not div by 1000, < 65536
        11025, 65535,
        // default: not div by 1000, >= 65536, div by 10, /10 < 65536
        88200, 655350,
        // default: not div by 1000, >= 65536, div by 10, /10 >= 65536
        882010,
        // default: not div by 1000, >= 65536, not div by 10
        100001,
        // zero
        0,
    ];
    for &sr in rates {
        let mut t = base();
        t.samplerate = sr;
        let (c, r) = call_both(&c_lib, &r_lib, &t);
        assert_match(&c, &r, &format!("samplerate={sr}"));
    }
}

#[test]
fn test_channel_modes() {
    let (c_lib, r_lib) = libs();
    for mode in 0u8..=7 {
        for ch in 1u32..=8 {
            let mut t = base();
            t.channel_mode = mode;
            t.channels = ch;
            let (c, r) = call_both(&c_lib, &r_lib, &t);
            assert_match(&c, &r, &format!("mode={mode} ch={ch}"));
        }
    }
}

#[test]
fn test_bitdepths() {
    let (c_lib, r_lib) = libs();
    let depths: &[u32] = &[8, 12, 16, 20, 24, 32, 0, 1, 4, 10, 15, 48];
    for &bd in depths {
        let mut t = base();
        t.bitdepth = bd;
        let (c, r) = call_both(&c_lib, &r_lib, &t);
        assert_match(&c, &r, &format!("bitdepth={bd}"));
    }
}

#[test]
fn test_combined_sweep() {
    let (c_lib, r_lib) = libs();
    let blocksizes = [192, 4096, 100, 500];
    let rates = [44100, 5000, 11025, 88200];
    let modes = [0u8, 1, 2, 3];
    let depths = [16u32, 24, 8, 0];
    for &bs in &blocksizes {
        for &sr in &rates {
            for &m in &modes {
                for &bd in &depths {
                    let t = Tflac {
                        samplerate: sr,
                        channels: 2,
                        bitdepth: bd,
                        channel_mode: m,
                        frame_header: 0xDEADBEEF,
                        cur_blocksize: bs,
                    };
                    let (c, r) = call_both(&c_lib, &r_lib, &t);
                    assert_match(&c, &r, &format!("bs={bs} sr={sr} m={m} bd={bd}"));
                }
            }
        }
    }
}
