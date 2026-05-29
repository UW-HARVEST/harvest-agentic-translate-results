use proj_42_Kocaeli_Printf::ft_printf::{
    format, writechar, writehex, writeint, writepoint, writestring, writeuint, DECIMAL, HEXALOW,
    HEXAUP, LOCATION,
};
use std::any::Any;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
}

// Counter to make captured-output tempfile names unique within the process.
static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn capture_stdout<F: FnOnce() -> R, R>(f: F) -> (R, Vec<u8>) {
    // Make sure any pending Rust stdout buffering is flushed before redirect.
    let _ = std::io::stdout().flush();

    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter: usize = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = format!("/tmp/ftprintf_capture_{}_{}_{}.tmp", pid, nanos, counter);

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create temp file");
    let file_fd = file.as_raw_fd();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup failed");
    unsafe {
        let r = dup2(file_fd, 1);
        assert!(r >= 0, "dup2 failed");
    }

    let result = f();

    // Flush any buffered Rust output to the redirected fd before restoring.
    let _ = std::io::stdout().flush();
    unsafe {
        let r = dup2(saved, 1);
        assert!(r >= 0, "restore dup2 failed");
        close(saved);
    }
    drop(file);

    let mut buf = Vec::new();
    File::open(&path)
        .and_then(|mut f| f.read_to_end(&mut buf))
        .expect("read captured file");
    let _ = std::fs::remove_file(&path);

    (result, buf)
}

// All cases live in a single #[test] function so that the test harness does
// not run them in parallel with other tests writing to stdout (which would
// contaminate our fd-redirected captures).
#[test]
fn test_ft_printf_module() {
    // ---------- constants ----------
    assert_eq!(DECIMAL, "0123456789");
    assert_eq!(HEXALOW, "0123456789abcdef");
    assert_eq!(HEXAUP, "0123456789ABCDEF");
    assert_eq!(LOCATION, 2);

    // ---------- writechar ----------
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writechar('A', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"A");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writechar('B', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"B");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writechar('z', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"z");
    }
    {
        let mut len: i32 = 5;
        let (rv, out) = capture_stdout(|| writechar('B', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 6);
        assert_eq!(out, b"B");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writechar('\n', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"\n");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writechar('\0', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, &[0u8]);
    }

    // ---------- writestring ----------
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writestring("Hello", &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 5);
        assert_eq!(out, b"Hello");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writestring("World!", &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 6);
        assert_eq!(out, b"World!");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writestring("", &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 0);
        assert_eq!(out, b"");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writestring("1234567890", &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 10);
        assert_eq!(out, b"1234567890");
    }
    {
        let mut len: i32 = 10;
        let (rv, out) = capture_stdout(|| writestring("World!", &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 16);
        assert_eq!(out, b"World!");
    }

    // ---------- writeint ----------
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(0, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"0");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(42, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"42");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(-42, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 3);
        assert_eq!(out, b"-42");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(1, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"1");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(-1, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"-1");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(123456, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 6);
        assert_eq!(out, b"123456");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(2147483647, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 10);
        assert_eq!(out, b"2147483647");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeint(i32::MIN, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 11);
        assert_eq!(out, b"-2147483648");
    }

    // ---------- writeuint ----------
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeuint(0, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"0");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeuint(42, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"42");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeuint(4_294_967_295, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 10);
        assert_eq!(out, b"4294967295");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writeuint(123_456_789, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 9);
        assert_eq!(out, b"123456789");
    }

    // ---------- writehex ----------
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(0, 'x', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"0");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(0, 'X', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"0");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(42, 'x', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"2a");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(42, 'X', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"2A");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(255, 'x', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"ff");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(255, 'X', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"FF");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(15, 'x', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"f");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(0x1234abcd, 'x', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 8);
        assert_eq!(out, b"1234abcd");
    }
    {
        let mut len: i32 = 0;
        let (rv, out) = capture_stdout(|| writehex(0x1234ABCD, 'X', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 8);
        assert_eq!(out, b"1234ABCD");
    }

    // ---------- writepoint ----------
    {
        let mut len: i32 = 0;
        let ptr = 0x1234usize as *const std::ffi::c_void;
        let (rv, out) = capture_stdout(|| writepoint(ptr, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 6);
        assert_eq!(out, b"0x1234");
    }
    {
        let mut len: i32 = 0;
        let ptr: *const std::ffi::c_void = std::ptr::null();
        let (rv, out) = capture_stdout(|| writepoint(ptr, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 5);
        assert_eq!(out, b"(nil)");
    }
    {
        let mut len: i32 = 0;
        let ptr = 0x1usize as *const std::ffi::c_void;
        let (rv, out) = capture_stdout(|| writepoint(ptr, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 3);
        assert_eq!(out, b"0x1");
    }
    {
        let mut len: i32 = 0;
        let ptr = 0xDEADBEEFusize as *const std::ffi::c_void;
        let (rv, out) = capture_stdout(|| writepoint(ptr, &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 10);
        assert_eq!(out, b"0xdeadbeef");
    }

    // ---------- format (single arg per call) ----------
    {
        // '%' specifier does not consume an argument.
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![];
        let (rv, out) = capture_stdout(|| format(&args, '%', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"%");
    }
    {
        // 'c' with a char argument.
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new('Q')];
        let (rv, out) = capture_stdout(|| format(&args, 'c', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"Q");
    }
    {
        // 'c' with an i32 argument (matches C's `va_arg(*args, int)`).
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(65i32)];
        let (rv, out) = capture_stdout(|| format(&args, 'c', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 1);
        assert_eq!(out, b"A");
    }
    {
        // 's' with a string slice.
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new("Hello")];
        let (rv, out) = capture_stdout(|| format(&args, 's', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 5);
        assert_eq!(out, b"Hello");
    }
    {
        // 's' with an owned String.
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(String::from("Owned"))];
        let (rv, out) = capture_stdout(|| format(&args, 's', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 5);
        assert_eq!(out, b"Owned");
    }
    {
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(-123i32)];
        let (rv, out) = capture_stdout(|| format(&args, 'd', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 4);
        assert_eq!(out, b"-123");
    }
    {
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(42i32)];
        let (rv, out) = capture_stdout(|| format(&args, 'i', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"42");
    }
    {
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(42u32)];
        let (rv, out) = capture_stdout(|| format(&args, 'u', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"42");
    }
    {
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(255u32)];
        let (rv, out) = capture_stdout(|| format(&args, 'x', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"ff");
    }
    {
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(255u32)];
        let (rv, out) = capture_stdout(|| format(&args, 'X', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 2);
        assert_eq!(out, b"FF");
    }
    {
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(0x1234usize)];
        let (rv, out) = capture_stdout(|| format(&args, 'p', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 6);
        assert_eq!(out, b"0x1234");
    }
    {
        let mut len: i32 = 0;
        let null_ptr: *const std::ffi::c_void = std::ptr::null();
        let args: Vec<Box<dyn Any>> = vec![Box::new(null_ptr)];
        let (rv, out) = capture_stdout(|| format(&args, 'p', &mut len));
        assert_eq!(rv, 1);
        assert_eq!(len, 5);
        assert_eq!(out, b"(nil)");
    }
    {
        // Unknown specifier -> -1.
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new(42i32)];
        let (rv, _out) = capture_stdout(|| format(&args, 'z', &mut len));
        assert_eq!(rv, -1);
    }

    // ---------- format consumes args in order ----------
    {
        let mut len: i32 = 0;
        let args: Vec<Box<dyn Any>> = vec![Box::new("World"), Box::new(42i32)];
        let (rv1, out1) = capture_stdout(|| format(&args, 's', &mut len));
        assert_eq!(rv1, 1);
        assert_eq!(len, 5);
        assert_eq!(out1, b"World");
        let (rv2, out2) = capture_stdout(|| format(&args, 'd', &mut len));
        assert_eq!(rv2, 1);
        assert_eq!(len, 7);
        assert_eq!(out2, b"42");
    }

    // ---------- multi-call accumulation ----------
    {
        let mut len: i32 = 0;
        let (_rv1, out1) = capture_stdout(|| writeint(42, &mut len));
        assert_eq!(len, 2);
        assert_eq!(out1, b"42");
        let (_rv2, out2) = capture_stdout(|| writestring(" + ", &mut len));
        assert_eq!(len, 5);
        assert_eq!(out2, b" + ");
        let (_rv3, out3) = capture_stdout(|| writeint(-1, &mut len));
        assert_eq!(len, 7);
        assert_eq!(out3, b"-1");
    }
}

fn main() {}
