//! Phase B — valid-path differential tests, one per row of `CONFIGS.md`.
//!
//! Every row drives BOTH shared objects through `dlsym`'d `helloworld` in the
//! same configuration and compares the emitted bytes and return values.
//! Randomized (fixed-seed) inputs are used per row, never a single value.

mod harness;

use harness::*;
use std::ffi::{c_char, c_double, c_int, c_void};

fn run_n(addr: usize, n: usize) -> Vec<c_int> {
    (0..n).map(|_| unsafe { call0(addr) }).collect()
}

fn zeros(n: usize) -> Vec<c_int> {
    vec![0; n]
}

// --- C1 ---------------------------------------------------------------------

#[allow(dead_code)]
fn b_c1_baseline_single_call() {
    let _g = serial();
    let (c, r) = addrs();
    for _ in 0..8 {
        let (cb, cr) = capture_file(BufCfg::Default, || unsafe { call0(c) });
        let (rb, rr) = capture_file(BufCfg::Default, || unsafe { call0(r) });
        assert_same_bytes("C1 baseline", &cb, &rb);
        assert_same_rets("C1 baseline", &cr, &rr);
        assert_eq!(cb, expected(1), "C1: C did not emit exactly one line");
        assert_eq!(cr, 0, "C1: C return value");
    }
}

// --- C2 ---------------------------------------------------------------------

#[allow(dead_code)]
fn b_c2_no_output_at_dlopen() {
    let _g = serial();
    // Load (and unload) each library with stdout captured and never call the
    // function: neither .so may write anything from a constructor/.init_array.
    let (cb, _) = capture_file(BufCfg::NoBuf, || {
        let l = open_lib(&c_so_path());
        let a = hello_addr(&l);
        drop(l);
        a
    });
    let (rb, _) = capture_file(BufCfg::NoBuf, || {
        let l = open_lib(&rust_so_path());
        let a = hello_addr(&l);
        drop(l);
        a
    });
    assert_same_bytes("C2 load-time output", &cb, &rb);
    assert!(cb.is_empty(), "C2: C wrote {} bytes at load time", cb.len());
}

// --- C3 ---------------------------------------------------------------------

#[allow(dead_code)]
fn b_c3_n_calls_default_buffer() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..32 {
        let n = rng.range(1, 64) as usize;
        let (cb, cr) = capture_file(BufCfg::Default, || run_n(c, n));
        let (rb, rr) = capture_file(BufCfg::Default, || run_n(r, n));
        assert_same_bytes(&format!("C3 n={n}"), &cb, &rb);
        assert_same_rets(&format!("C3 n={n}"), &cr, &rr);
        assert_eq!(cb, expected(n), "C3: C output for n={n}");
        assert_eq!(cr, zeros(n), "C3: C returns for n={n}");
    }
}

// --- C4 ---------------------------------------------------------------------

#[allow(dead_code)]
fn b_c4_full_buffering_sizes() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 4);
    for size in [1usize, 2, 3, 13, 14, 64, 4096] {
        for _ in 0..6 {
            let n = rng.range(1, 40) as usize;
            let (cb, cr) = capture_file(BufCfg::Full(size), || run_n(c, n));
            let (rb, rr) = capture_file(BufCfg::Full(size), || run_n(r, n));
            let tag = format!("C4 _IOFBF size={size} n={n}");
            assert_same_bytes(&tag, &cb, &rb);
            assert_same_rets(&tag, &cr, &rr);
            assert_eq!(cb, expected(n), "{tag}: C output");
            assert_eq!(cr, zeros(n), "{tag}: C returns");
        }
    }
}

// --- C5 ---------------------------------------------------------------------

#[allow(dead_code)]
fn b_c5_line_buffering() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 5);
    for size in [1usize, 13, 14, 64, 4096] {
        for _ in 0..6 {
            let n = rng.range(1, 40) as usize;
            let (cb, cr) = capture_file(BufCfg::Line(size), || run_n(c, n));
            let (rb, rr) = capture_file(BufCfg::Line(size), || run_n(r, n));
            let tag = format!("C5 _IOLBF size={size} n={n}");
            assert_same_bytes(&tag, &cb, &rb);
            assert_same_rets(&tag, &cr, &rr);
            assert_eq!(cb, expected(n), "{tag}: C output");
        }
    }
}

// --- C6 ---------------------------------------------------------------------

#[allow(dead_code)]
fn b_c6_unbuffered() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..16 {
        let n = rng.range(1, 40) as usize;
        let (cb, cr) = capture_file(BufCfg::NoBuf, || run_n(c, n));
        let (rb, rr) = capture_file(BufCfg::NoBuf, || run_n(r, n));
        let tag = format!("C6 _IONBF n={n}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(n), "{tag}: C output");
    }
}

// --- C7 / C8 ----------------------------------------------------------------

#[allow(dead_code)]
fn b_c7_pipe_fully_buffered() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..16 {
        let n = rng.range(1, 64) as usize;
        let (cb, cr) = capture_pipe(BufCfg::Default, || run_n(c, n));
        let (rb, rr) = capture_pipe(BufCfg::Default, || run_n(r, n));
        let tag = format!("C7 pipe _IOFBF n={n}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(n), "{tag}: C output");
    }
}

#[allow(dead_code)]
fn b_c8_pipe_unbuffered() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..16 {
        let n = rng.range(1, 64) as usize;
        let (cb, cr) = capture_pipe(BufCfg::NoBuf, || run_n(c, n));
        let (rb, rr) = capture_pipe(BufCfg::NoBuf, || run_n(r, n));
        let tag = format!("C8 pipe _IONBF n={n}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(n), "{tag}: C output");
    }
}

// --- C9 ---------------------------------------------------------------------

#[allow(dead_code)]
fn b_c9_append_mode() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..12 {
        let n = rng.range(1, 24) as usize;
        let plen = rng.range(0, 100) as usize;
        let prefix = rng.ascii_blob(plen);
        let buf = rng.pick(&[BufCfg::Default, BufCfg::NoBuf, BufCfg::Full(3)]);
        let (cb, cr) = capture_append(buf, &prefix, || run_n(c, n));
        let (rb, rr) = capture_append(buf, &prefix, || run_n(r, n));
        let tag = format!("C9 O_APPEND n={n} prefix={} buf={buf:?}", prefix.len());
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        let mut want = prefix.clone();
        want.extend_from_slice(&expected(n));
        assert_eq!(cb, want, "{tag}: C output");
    }
}

// --- C10 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c10_dev_null() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..12 {
        let n = rng.range(1, 64) as usize;
        let buf = rng.pick(&[BufCfg::Default, BufCfg::NoBuf, BufCfg::Line(13)]);
        let cr = with_stdout_device("/dev/null", buf, || run_n(c, n));
        let rr = with_stdout_device("/dev/null", buf, || run_n(r, n));
        let tag = format!("C10 /dev/null n={n} buf={buf:?}");
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cr, zeros(n), "{tag}: C returns");
    }
    clear_stdout_error();
}

// --- C11 --------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    Call,
    Write(Vec<u8>),
}

fn run_script(addr: usize, script: &[Op]) -> Vec<c_int> {
    let mut rets = Vec::new();
    for op in script {
        match op {
            Op::Call => rets.push(unsafe { call0(addr) }),
            Op::Write(b) => caller_write(b),
        }
    }
    rets
}

#[allow(dead_code)]
fn b_c11_interleaved_with_caller_writes() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..24 {
        let len = rng.range(1, 20) as usize;
        let script: Vec<Op> = (0..len)
            .map(|_| {
                if rng.bool() {
                    Op::Call
                } else {
                    let blen = rng.range(0, 40) as usize;
                    Op::Write(rng.ascii_blob(blen))
                }
            })
            .collect();
        let buf = rng.pick(&[
            BufCfg::Default,
            BufCfg::NoBuf,
            BufCfg::Line(64),
            BufCfg::Full(2),
        ]);
        let (cb, cr) = capture_file(buf, || run_script(c, &script));
        let (rb, rr) = capture_file(buf, || run_script(r, &script));
        let tag = format!("C11 interleaved len={len} buf={buf:?}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        // Independently derived expectation: exact concatenation, in order.
        let mut want = Vec::new();
        for op in &script {
            match op {
                Op::Call => want.extend_from_slice(HELLO_LINE),
                Op::Write(b) => want.extend_from_slice(b),
            }
        }
        assert_eq!(cb, want, "{tag}: C output ordering");
    }
}

// --- C12 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c12_c_and_rust_interleaved_in_one_stream() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..24 {
        let len = rng.range(1, 40) as usize;
        let which: Vec<bool> = (0..len).map(|_| rng.bool()).collect();
        let buf = rng.pick(&[BufCfg::Default, BufCfg::NoBuf, BufCfg::Full(1)]);
        // Mixed C/Rust stream.
        let (mixed, mrets) = capture_file(buf, || {
            which
                .iter()
                .map(|&use_c| unsafe { call0(if use_c { c } else { r }) })
                .collect::<Vec<_>>()
        });
        // All-C reference stream for the same script.
        let (all_c, crets) = capture_file(buf, || run_n(c, len));
        // All-Rust reference stream for the same script.
        let (all_r, rrets) = capture_file(buf, || run_n(r, len));
        let tag = format!("C12 mixed len={len} buf={buf:?}");
        assert_same_bytes(&tag, &all_c, &mixed);
        assert_same_bytes(&tag, &all_c, &all_r);
        assert_same_rets(&tag, &crets, &mrets);
        assert_same_rets(&tag, &crets, &rrets);
        assert_eq!(mixed, expected(len), "{tag}: mixed stream content");
    }
}

// --- C13 --------------------------------------------------------------------

unsafe fn call_with_ints(addr: usize, args: &[c_int]) -> c_int {
    unsafe {
        match args.len() {
            0 => call0(addr),
            1 => std::mem::transmute::<usize, Hello1I>(addr)(args[0]),
            2 => std::mem::transmute::<usize, Hello2I>(addr)(args[0], args[1]),
            3 => std::mem::transmute::<usize, Hello3I>(addr)(args[0], args[1], args[2]),
            4 => std::mem::transmute::<usize, Hello4I>(addr)(args[0], args[1], args[2], args[3]),
            5 => std::mem::transmute::<usize, Hello5I>(addr)(
                args[0], args[1], args[2], args[3], args[4],
            ),
            6 => std::mem::transmute::<usize, Hello6I>(addr)(
                args[0], args[1], args[2], args[3], args[4], args[5],
            ),
            _ => unreachable!(),
        }
    }
}

#[allow(dead_code)]
fn b_c13_extra_integer_arguments() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 13);
    let interesting = [0, -1, 1, i32::MIN, i32::MAX, 0x7FFF_FFFE, -2];
    for arity in 0..=6usize {
        for _ in 0..8 {
            let args: Vec<c_int> = (0..arity)
                .map(|_| {
                    if rng.bool() {
                        rng.pick(&interesting)
                    } else {
                        rng.i32()
                    }
                })
                .collect();
            let (cb, cr) = capture_file(BufCfg::NoBuf, || unsafe { call_with_ints(c, &args) });
            let (rb, rr) = capture_file(BufCfg::NoBuf, || unsafe { call_with_ints(r, &args) });
            let tag = format!("C13 arity={arity} args={args:?}");
            assert_same_bytes(&tag, &cb, &rb);
            assert_same_rets(&tag, &cr, &rr);
            assert_eq!(cb, expected(1), "{tag}: C output");
            assert_eq!(cr, 0, "{tag}: C return");
        }
    }
}

// --- C14 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c14_float_and_varargs_call_shapes() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..16 {
        let f = [rng.f64(), rng.f64(), rng.f64(), rng.f64()];
        let i = [rng.i32(), rng.i32()];
        // 4 doubles in xmm0..3 + 2 ints in rdi/rsi.
        let (cb, cr) = capture_file(BufCfg::NoBuf, || unsafe {
            std::mem::transmute::<usize, HelloFloats>(c)(f[0], f[1], f[2], f[3], i[0], i[1])
        });
        let (rb, rr) = capture_file(BufCfg::NoBuf, || unsafe {
            std::mem::transmute::<usize, HelloFloats>(r)(f[0], f[1], f[2], f[3], i[0], i[1])
        });
        let tag = format!("C14 floats {f:?} {i:?}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(1), "{tag}: C output");

        // True variadic call site: %al is set to the number of vector regs.
        let (cb2, cr2) = capture_file(BufCfg::NoBuf, || unsafe {
            std::mem::transmute::<usize, HelloVariadic>(c)(i[0], f[0], i[1], f[1])
        });
        let (rb2, rr2) = capture_file(BufCfg::NoBuf, || unsafe {
            std::mem::transmute::<usize, HelloVariadic>(r)(i[0], f[0], i[1], f[1])
        });
        let tag = format!("C14 variadic {f:?} {i:?}");
        assert_same_bytes(&tag, &cb2, &rb2);
        assert_same_rets(&tag, &cr2, &rr2);
        assert_eq!(cb2, expected(1), "{tag}: C output");
    }
}

// --- C15 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c15_pointer_shaped_arguments() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 15);
    let live: Vec<u8> = b"a live buffer the callee must not touch\0".to_vec();
    for _ in 0..16 {
        let p0: *const c_void = match rng.range(0, 2) {
            0 => std::ptr::null(),
            1 => live.as_ptr() as *const c_void,
            _ => 0xDEAD_BEEFusize as *const c_void,
        };
        let p1: *const c_void = if rng.bool() {
            std::ptr::null()
        } else {
            0xFFFF_FFFF_FFFF_F000usize as *const c_void
        };
        let p2 = live.as_ptr() as *const c_char;
        let (cb, cr) = capture_file(BufCfg::NoBuf, || unsafe {
            std::mem::transmute::<usize, HelloPtrs>(c)(p0, p1, p2)
        });
        let (rb, rr) = capture_file(BufCfg::NoBuf, || unsafe {
            std::mem::transmute::<usize, HelloPtrs>(r)(p0, p1, p2)
        });
        let tag = format!("C15 ptrs {p0:?} {p1:?}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(1), "{tag}: C output");
        assert_eq!(cr, 0, "{tag}: C return");
    }
    assert_eq!(&live[..6], b"a live", "C15: caller buffer was clobbered");
}

// --- C16 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c16_return_value_width() {
    let _g = serial();
    let (c, r) = addrs();
    for _ in 0..16 {
        let (cb, cr) = capture_file(BufCfg::NoBuf, || unsafe { call0_long(c) });
        let (rb, rr) = capture_file(BufCfg::NoBuf, || unsafe { call0_long(r) });
        assert_same_bytes("C16 i64 return", &cb, &rb);
        assert_same_rets("C16 i64 return", &cr, &rr);
        assert_eq!(cr, 0, "C16: C did not zero all 64 bits of the return");
        assert_eq!(cr as i32, 0);

        let (cb, cr) = capture_file(BufCfg::NoBuf, || unsafe { call0(c) });
        let (rb, rr) = capture_file(BufCfg::NoBuf, || unsafe { call0(r) });
        assert_same_bytes("C16 i32 return", &cb, &rb);
        assert_same_rets("C16 i32 return", &cr, &rr);
    }
}

// --- C17 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c17_rtld_flag_combinations() {
    let _g = serial();
    for (name, flags) in [
        ("RTLD_NOW|RTLD_LOCAL", libc::RTLD_NOW | libc::RTLD_LOCAL),
        ("RTLD_LAZY|RTLD_LOCAL", libc::RTLD_LAZY | libc::RTLD_LOCAL),
        ("RTLD_NOW|RTLD_GLOBAL", libc::RTLD_NOW | libc::RTLD_GLOBAL),
        ("RTLD_LAZY|RTLD_GLOBAL", libc::RTLD_LAZY | libc::RTLD_GLOBAL),
    ] {
        let cl = open_lib_flags(&c_so_path(), flags);
        let rl = open_lib_flags(&rust_so_path(), flags);
        let ca = hello_addr_os(&cl);
        let ra = hello_addr_os(&rl);
        assert_ne!(ca, ra, "{name}: handles collapsed to one symbol");
        let mut rng = Rng::new(SEED ^ 17 ^ flags as u64);
        let n = rng.range(1, 8) as usize;
        let (cb, cr) = capture_file(BufCfg::NoBuf, || run_n(ca, n));
        let (rb, rr) = capture_file(BufCfg::NoBuf, || run_n(ra, n));
        let tag = format!("C17 {name} n={n}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(n), "{tag}: C output");
        drop(cl);
        drop(rl);
    }
}

// --- C18 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c18_load_order_both_ways() {
    let _g = serial();
    for c_first in [true, false] {
        let (ca, ra, _keep);
        if c_first {
            let cl = open_lib(&c_so_path());
            let rl = open_lib(&rust_so_path());
            ca = hello_addr(&cl);
            ra = hello_addr(&rl);
            _keep = (cl, rl);
        } else {
            let rl = open_lib(&rust_so_path());
            let cl = open_lib(&c_so_path());
            ra = hello_addr(&rl);
            ca = hello_addr(&cl);
            _keep = (cl, rl);
        }
        let mut rng = Rng::new(SEED ^ 18 ^ c_first as u64);
        let len = rng.range(1, 20) as usize;
        let which: Vec<bool> = (0..len).map(|_| rng.bool()).collect();
        let (mixed, mrets) = capture_file(BufCfg::NoBuf, || {
            which
                .iter()
                .map(|&u| unsafe { call0(if u { ca } else { ra }) })
                .collect::<Vec<_>>()
        });
        let tag = format!("C18 c_first={c_first} len={len}");
        assert_eq!(mixed, expected(len), "{tag}: interposition changed output");
        assert_eq!(mrets, zeros(len), "{tag}: returns");
    }
}

// --- C19 --------------------------------------------------------------------

#[allow(dead_code)]
fn b_c19_dlopen_dlclose_cycles() {
    let _g = serial();
    let mut rng = Rng::new(SEED ^ 19);
    let cycles = rng.range(4, 16) as usize;
    for i in 0..cycles {
        let n = rng.range(1, 6) as usize;
        let (cb, cr) = capture_file(BufCfg::NoBuf, || {
            let l = open_lib(&c_so_path());
            let a = hello_addr(&l);
            let rets = run_n(a, n);
            drop(l);
            rets
        });
        let (rb, rr) = capture_file(BufCfg::NoBuf, || {
            let l = open_lib(&rust_so_path());
            let a = hello_addr(&l);
            let rets = run_n(a, n);
            drop(l);
            rets
        });
        let tag = format!("C19 cycle={i} n={n}");
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cb, expected(n), "{tag}: C output (state leaked across loads?)");
    }
}

// --- C20 / C21 --------------------------------------------------------------

fn threaded(addr: usize, threads: usize, per_thread: usize) -> Vec<c_int> {
    let mut all = Vec::new();
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(move || (0..per_thread).map(|_| unsafe { call0(addr) }).collect::<Vec<_>>())
            })
            .collect();
        for h in handles {
            all.extend(h.join().expect("worker thread panicked"));
        }
    });
    all
}

fn assert_all_lines_intact(tag: &str, bytes: &[u8], expected_calls: usize) {
    assert_eq!(
        bytes.len(),
        HELLO_LINE.len() * expected_calls,
        "{tag}: wrong total byte count"
    );
    for (i, line) in bytes.split_inclusive(|&b| b == b'\n').enumerate() {
        assert_eq!(
            line,
            HELLO_LINE,
            "{tag}: line {i} is torn: {:?}",
            String::from_utf8_lossy(line)
        );
    }
}

#[allow(dead_code)]
fn b_c20_threads_fully_buffered() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..6 {
        let t = rng.range(2, 8) as usize;
        let k = rng.range(1, 32) as usize;
        let (cb, cr) = capture_file(BufCfg::Default, || threaded(c, t, k));
        let (rb, rr) = capture_file(BufCfg::Default, || threaded(r, t, k));
        let tag = format!("C20 threads={t} per={k}");
        assert_all_lines_intact(&format!("{tag} [C]"), &cb, t * k);
        assert_all_lines_intact(&format!("{tag} [Rust]"), &rb, t * k);
        assert_same_bytes(&tag, &cb, &rb); // identical because every line is identical
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cr, zeros(t * k), "{tag}: C returns");
    }
}

#[allow(dead_code)]
fn b_c21_threads_unbuffered() {
    let _g = serial();
    let (c, r) = addrs();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..6 {
        let t = rng.range(2, 8) as usize;
        let k = rng.range(1, 32) as usize;
        let (cb, cr) = capture_file(BufCfg::NoBuf, || threaded(c, t, k));
        let (rb, rr) = capture_file(BufCfg::NoBuf, || threaded(r, t, k));
        let tag = format!("C21 threads={t} per={k} _IONBF");
        assert_all_lines_intact(&format!("{tag} [C]"), &cb, t * k);
        assert_all_lines_intact(&format!("{tag} [Rust]"), &rb, t * k);
        assert_same_bytes(&tag, &cb, &rb);
        assert_same_rets(&tag, &cr, &rr);
        assert_eq!(cr, zeros(t * k), "{tag}: C returns");
    }
}

// Keep the `c_double` import meaningful even if the float row is trimmed.
const _: Option<c_double> = None;

// --- the single #[test] entry point: every CONFIGS.md row, in order ---------

#[test]
fn phase_b_every_configs_row() {
    let mut rows = Rows::new("Phase B — CONFIGS.md");
    rows.row("C1  baseline, 1 call, file, default buffering", b_c1_baseline_single_call);
    rows.row("C2  0 calls: no output at dlopen/dlclose", b_c2_no_output_at_dlopen);
    rows.row("C3  N randomized calls, file, default buffering", b_c3_n_calls_default_buffer);
    rows.row("C4  _IOFBF with buffer sizes 1..4096", b_c4_full_buffering_sizes);
    rows.row("C5  _IOLBF with buffer sizes 1..4096", b_c5_line_buffering);
    rows.row("C6  _IONBF unbuffered", b_c6_unbuffered);
    rows.row("C7  stdout is a pipe, _IOFBF", b_c7_pipe_fully_buffered);
    rows.row("C8  stdout is a pipe, _IONBF", b_c8_pipe_unbuffered);
    rows.row("C9  O_APPEND onto a non-empty file", b_c9_append_mode);
    rows.row("C10 stdout is /dev/null", b_c10_dev_null);
    rows.row("C11 interleaved with the caller's own writes", b_c11_interleaved_with_caller_writes);
    rows.row("C12 C and Rust interleaved in one stream", b_c12_c_and_rust_interleaved_in_one_stream);
    rows.row("C13 extra integer arguments (arity 0..6)", b_c13_extra_integer_arguments);
    rows.row("C14 float / true-varargs call shapes", b_c14_float_and_varargs_call_shapes);
    rows.row("C15 pointer-shaped arguments", b_c15_pointer_shaped_arguments);
    rows.row("C16 return value read as i32 and i64", b_c16_return_value_width);
    rows.row("C17 RTLD_NOW/LAZY x LOCAL/GLOBAL", b_c17_rtld_flag_combinations);
    rows.row("C18 load order: C first / Rust first", b_c18_load_order_both_ways);
    rows.row("C19 repeated dlopen/dlclose cycles", b_c19_dlopen_dlclose_cycles);
    rows.row("C20 many threads, _IOFBF", b_c20_threads_fully_buffered);
    rows.row("C21 many threads, _IONBF", b_c21_threads_unbuffered);
    rows.finish();
}
