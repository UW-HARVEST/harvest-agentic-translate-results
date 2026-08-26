// Out-of-process driver for the differential test-suite.
//
//   so_runner <path-to-shared-object> <mode> [args...]
//
// The runner `dlopen`s the shared object it is pointed at (either the C
// `libcdriver.so` or the Rust `libdriver.so`) and calls one of its exported
// C-ABI symbols.  Running it in a separate process lets the tests
//
//   * feed the `main` export a real stdin and capture its real stdout,
//   * capture what `driver` prints without fighting libtest over fd 1, and
//   * observe the *termination signal* of the calls that are expected to fault
//     (NULL pointers with a positive `len`), which cannot be done in-process.
//
// Modes:
//   main                     call `int main(void)`, exit with its return value
//   main_n <n>               call `main` n times in the same process
//   fma_misaligned <len>     `fma_array` over four windows at a misaligned base
//   driver <len>             elements are read from stdin as decimal text;
//                            calls `driver(buf, len)`; the resulting buffer is
//                            written to *stderr* as space separated decimals
//                            while whatever `driver` printed goes to stdout
//   driver2 <len>            same, but calls `driver` twice on the same buffer
//   fma <len> <o> <m1> <m2> <a>
//                            elements from stdin, calls
//                            `fma_array(buf+o, buf+m1, buf+m2, buf+a, len)`,
//                            resulting buffer to stderr
//   fma_null <len>           call `fma_array(NULL, NULL, NULL, NULL, len)`
//   driver_null <len>        call `driver(NULL, len)`
//   fma_huge_len             call `fma_array` on a small buffer with len=INT_MAX
//   driver_huge_len          call `driver` on a small buffer with len=INT_MAX

use std::io::Read;
use std::os::raw::c_int;

type FmaFn = unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
type DriverFn = unsafe extern "C" fn(*mut c_int, c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

/// Reads the element vector from stdin (whitespace separated decimals).
fn read_elements() -> Vec<i32> {
    let mut s = String::new();
    std::io::stdin()
        .read_to_string(&mut s)
        .expect("read elements from stdin");
    s.split_whitespace()
        .map(|t| t.parse::<i32>().unwrap_or_else(|e| panic!("bad element {t:?}: {e}")))
        .collect()
}

fn dump_elements(v: &[i32]) {
    let mut s = String::new();
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&x.to_string());
    }
    eprint!("{s}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: so_runner <so> <mode> [args...]");
        std::process::exit(2);
    }
    let so = &args[1];
    let mode = &args[2];
    let arg = |i: usize| -> c_int {
        args.get(i)
            .unwrap_or_else(|| panic!("mode {mode} needs argument #{i}"))
            .parse()
            .expect("integer argument")
    };

    let lib = unsafe { libloading::Library::new(so) }
        .unwrap_or_else(|e| panic!("dlopen({so}) failed: {e}"));

    let code = unsafe {
        match mode.as_str() {
            "main" => {
                let f = lib
                    .get::<MainFn>(b"main\0")
                    .expect("no `main` symbol in shared object");
                let rc = f();
                // The C implementation leaves its output in glibc's stdout
                // buffer; flush every stream before we exit.
                libc::fflush(std::ptr::null_mut());
                rc
            }
            "main_n" => {
                // `main` called repeatedly in one process: C's `stdin` FILE is
                // global, so call n+1 continues where call n stopped.
                let n = arg(3);
                let f = lib
                    .get::<MainFn>(b"main\0")
                    .expect("no `main` symbol in shared object");
                let mut rc = 0;
                for _ in 0..n {
                    rc = f();
                    libc::fflush(std::ptr::null_mut());
                }
                rc
            }
            "fma_misaligned" => {
                // Four windows of `len` elements each, at a deliberately
                // misaligned base address: C just emits unaligned loads on
                // x86-64, there is no check anywhere.
                let len = arg(3);
                let n = len.max(0) as usize;
                let elements = read_elements();
                assert!(elements.len() >= 4 * n);
                let mut bytes = vec![0u8; 4 * n * 4 + 4];
                for (i, v) in elements.iter().take(4 * n).enumerate() {
                    bytes[1 + 4 * i..1 + 4 * i + 4].copy_from_slice(&v.to_ne_bytes());
                }
                let base = bytes.as_mut_ptr().add(1) as *mut c_int;
                let f = lib.get::<FmaFn>(b"fma_array\0").expect("no `fma_array`");
                f(
                    base,
                    base.add(n),
                    base.add(2 * n),
                    base.add(3 * n),
                    len,
                );
                libc::fflush(std::ptr::null_mut());
                let out: Vec<i32> = (0..4 * n)
                    .map(|i| {
                        let mut b = [0u8; 4];
                        b.copy_from_slice(&bytes[1 + 4 * i..1 + 4 * i + 4]);
                        i32::from_ne_bytes(b)
                    })
                    .collect();
                dump_elements(&out);
                0
            }
            "driver" | "driver2" => {
                let len = arg(3);
                let mut buf = read_elements();
                let f = lib.get::<DriverFn>(b"driver\0").expect("no `driver`");
                let p = buf.as_mut_ptr();
                f(p, len);
                if mode == "driver2" {
                    f(p, len);
                }
                libc::fflush(std::ptr::null_mut());
                dump_elements(&buf);
                0
            }
            "fma" => {
                let len = arg(3);
                let (o, m1, m2, a) = (arg(4), arg(5), arg(6), arg(7));
                let mut buf = read_elements();
                let f = lib.get::<FmaFn>(b"fma_array\0").expect("no `fma_array`");
                let p = buf.as_mut_ptr();
                f(
                    p.offset(o as isize),
                    p.offset(m1 as isize),
                    p.offset(m2 as isize),
                    p.offset(a as isize),
                    len,
                );
                libc::fflush(std::ptr::null_mut());
                dump_elements(&buf);
                0
            }
            "fma_null" => {
                let len = arg(3);
                let f = lib.get::<FmaFn>(b"fma_array\0").expect("no `fma_array`");
                f(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                );
                libc::fflush(std::ptr::null_mut());
                0
            }
            "driver_null" => {
                let len = arg(3);
                let f = lib.get::<DriverFn>(b"driver\0").expect("no `driver`");
                f(std::ptr::null_mut(), len);
                libc::fflush(std::ptr::null_mut());
                0
            }
            "fma_huge_len" => {
                let mut buf = vec![1i32, 2, 3, 4];
                let f = lib.get::<FmaFn>(b"fma_array\0").expect("no `fma_array`");
                let p = buf.as_mut_ptr();
                f(p, p, p, p, c_int::MAX);
                libc::fflush(std::ptr::null_mut());
                0
            }
            "driver_huge_len" => {
                let mut buf = vec![1i32, 2, 3, 4];
                let f = lib.get::<DriverFn>(b"driver\0").expect("no `driver`");
                f(buf.as_mut_ptr(), c_int::MAX);
                libc::fflush(std::ptr::null_mut());
                0
            }
            other => {
                eprintln!("unknown mode: {other}");
                2
            }
        }
    };

    std::process::exit(code);
}
