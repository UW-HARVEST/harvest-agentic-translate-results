use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.out",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let mut output = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create stdout capture file");
    std::fs::remove_file(&path).expect("unlink stdout capture file");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush stdout");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(output.as_raw_fd(), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );

        call();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    output
        .seek(SeekFrom::Start(0))
        .expect("rewind stdout capture");
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).expect("read stdout capture");
    bytes
}

fn test_inputs() -> Vec<c_int> {
    let mut inputs = vec![
        c_int::MIN,
        -1,
        0,
        1,
        c_int::MAX,
        0x0000_00ff,
        0x0000_ff00,
        0x00ff_0000,
        0x7f00_0000,
        0x0102_0304,
        0x1020_3040,
    ];

    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..4096 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state as u32 as c_int);
    }
    inputs
}

#[test]
fn driver_matches_c_for_all_configured_input_shapes() {
    assert_eq!(std::mem::size_of::<c_int>(), 4);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest_dir.join("c_src/build/libdriver.so");
    let rust_path = manifest_dir.join("target/debug/libdriver.so");
    assert!(
        c_path.is_file(),
        "missing C shared object: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {}",
        rust_path.display()
    );

    let _stdout_guard = STDOUT_LOCK.lock().expect("lock stdout capture");
    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared object");
        let rust_library = Library::new(&rust_path).expect("load Rust shared object");
        let c_driver: Symbol<Driver> = c_library.get(b"driver\0").expect("load C driver");
        let rust_driver: Symbol<Driver> = rust_library.get(b"driver\0").expect("load Rust driver");
        let inputs = test_inputs();

        let c_output = capture_stdout(|| {
            for &input in &inputs {
                c_driver(input);
            }
        });
        let rust_output = capture_stdout(|| {
            for &input in &inputs {
                rust_driver(input);
            }
        });

        assert_eq!(rust_output, c_output);
    }
}
