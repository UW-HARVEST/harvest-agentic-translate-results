// Parity tests: compare every exported function's behavior between the
// C-built libcdriver.so and the Rust-built libdriver.so. Both libraries are
// loaded via libloading; we never call Rust functions directly.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};

// -------------------------------------------------------------------
// Lowest level: target() (lib.c)
// -------------------------------------------------------------------
#[test]
fn target_matches() {
    unsafe {
        let (c, r) = open_libs();
        let c_target: libloading::Symbol<FnTarget> = c.sym(b"target");
        let r_target: libloading::Symbol<FnTarget> = r.sym(b"target");
        for x in [-100, -1, 0, 1, 5, 6, 7, 9, 10, 17, 27, 100, 1000, i32::MAX, i32::MIN] {
            let cv = c_target(x);
            let rv = r_target(x);
            assert_eq!(cv, rv, "target({}) mismatch C={} R={}", x, cv, rv);
        }
    }
}

// -------------------------------------------------------------------
// IntVec API
// -------------------------------------------------------------------
#[test]
fn intvec_init_push_pop_peek_free() {
    unsafe {
        let (c, r) = open_libs();
        // Drive each library independently using its own functions, then
        // compare the externally-visible state of the IntVec.
        let c_init: libloading::Symbol<FnIvInit> = c.sym(b"iv_init");
        let c_free: libloading::Symbol<FnIvFree> = c.sym(b"iv_free");
        let c_push: libloading::Symbol<FnIvPush> = c.sym(b"iv_push");
        let c_pop: libloading::Symbol<FnIvPop> = c.sym(b"iv_pop");
        let c_peek: libloading::Symbol<FnIvPeek> = c.sym(b"iv_peek");
        let c_reserve: libloading::Symbol<FnIvReserve> = c.sym(b"iv_reserve");

        let r_init: libloading::Symbol<FnIvInit> = r.sym(b"iv_init");
        let r_free: libloading::Symbol<FnIvFree> = r.sym(b"iv_free");
        let r_push: libloading::Symbol<FnIvPush> = r.sym(b"iv_push");
        let r_pop: libloading::Symbol<FnIvPop> = r.sym(b"iv_pop");
        let r_peek: libloading::Symbol<FnIvPeek> = r.sym(b"iv_peek");
        let r_reserve: libloading::Symbol<FnIvReserve> = r.sym(b"iv_reserve");

        // Heap-allocate the IntVec backing storage so we own it. Use
        // MaybeUninit and have each library initialize it.
        let mut cv: IntVec = IntVec::default();
        let mut rv: IntVec = IntVec::default();
        c_init(&mut cv);
        r_init(&mut rv);
        assert_eq!(cv.len, rv.len);
        assert_eq!(cv.cap, rv.cap);

        // peek on empty
        assert_eq!(c_peek(&cv, -42), r_peek(&rv, -42));

        // push values
        for x in [1, -2, 3, 4, 5, 6, 7, 8, 9, 10, 100, -100] {
            let cb = c_push(&mut cv, x);
            let rb = r_push(&mut rv, x);
            assert_eq!(cb, rb);
            assert_eq!(cv.len, rv.len, "len after push {}", x);
            assert_eq!(cv.cap, rv.cap, "cap after push {}", x);
            assert_eq!(c_peek(&cv, 0), r_peek(&rv, 0));
        }

        // reserve more
        let cb = c_reserve(&mut cv, 1000);
        let rb = r_reserve(&mut rv, 1000);
        assert_eq!(cb, rb);
        assert_eq!(cv.cap, rv.cap, "cap after reserve");
        assert_eq!(cv.len, rv.len, "len after reserve");

        // pop one
        let mut cout: c_int = 0;
        let mut rout: c_int = 0;
        let cb = c_pop(&mut cv, &mut cout);
        let rb = r_pop(&mut rv, &mut rout);
        assert_eq!(cb, rb);
        assert_eq!(cout, rout);
        assert_eq!(cv.len, rv.len);

        // pop NULL out
        let cb = c_pop(&mut cv, std::ptr::null_mut());
        let rb = r_pop(&mut rv, std::ptr::null_mut());
        assert_eq!(cb, rb);
        assert_eq!(cv.len, rv.len);

        // pop until empty + then more
        for _ in 0..30 {
            let cb = c_pop(&mut cv, &mut cout);
            let rb = r_pop(&mut rv, &mut rout);
            assert_eq!(cb, rb);
            if cb {
                assert_eq!(cout, rout);
            }
            assert_eq!(cv.len, rv.len);
        }

        // free both
        c_free(&mut cv);
        r_free(&mut rv);
        assert!(cv.data.is_null());
        assert!(rv.data.is_null());
        assert_eq!(cv.len, 0);
        assert_eq!(rv.len, 0);
        assert_eq!(cv.cap, 0);
        assert_eq!(rv.cap, 0);
    }
}

#[test]
fn intvec_growth_pattern() {
    unsafe {
        let (c, r) = open_libs();
        let c_init: libloading::Symbol<FnIvInit> = c.sym(b"iv_init");
        let c_free: libloading::Symbol<FnIvFree> = c.sym(b"iv_free");
        let c_push: libloading::Symbol<FnIvPush> = c.sym(b"iv_push");
        let r_init: libloading::Symbol<FnIvInit> = r.sym(b"iv_init");
        let r_free: libloading::Symbol<FnIvFree> = r.sym(b"iv_free");
        let r_push: libloading::Symbol<FnIvPush> = r.sym(b"iv_push");

        let mut cv: IntVec = IntVec::default();
        let mut rv: IntVec = IntVec::default();
        c_init(&mut cv);
        r_init(&mut rv);

        // Track cap progression
        for i in 0..200 {
            c_push(&mut cv, i);
            r_push(&mut rv, i);
            assert_eq!(cv.cap, rv.cap, "i={} cap diverged", i);
            assert_eq!(cv.len, rv.len, "i={} len diverged", i);
        }
        c_free(&mut cv);
        r_free(&mut rv);
    }
}

// -------------------------------------------------------------------
// Program API
// -------------------------------------------------------------------
#[test]
fn program_init_fetch() {
    unsafe {
        let (c, r) = open_libs();
        let c_init: libloading::Symbol<FnProgInit> = c.sym(b"prog_init");
        let c_fetch: libloading::Symbol<FnProgFetch> = c.sym(b"prog_fetch");
        let r_init: libloading::Symbol<FnProgInit> = r.sym(b"prog_init");
        let r_fetch: libloading::Symbol<FnProgFetch> = r.sym(b"prog_fetch");

        let code: [c_int; 6] = [10, 20, -3, 4, 99, 0];

        let mut cp: Program = Program::default();
        let mut rp: Program = Program::default();
        c_init(&mut cp, code.as_ptr(), code.len());
        r_init(&mut rp, code.as_ptr(), code.len());
        assert_eq!(cp.n, rp.n);
        assert_eq!(cp.ip, rp.ip);

        for _ in 0..10 {
            let mut cv: c_int = 0;
            let mut rv: c_int = 0;
            let cb = c_fetch(&mut cp, &mut cv);
            let rb = r_fetch(&mut rp, &mut rv);
            assert_eq!(cb, rb);
            if cb {
                assert_eq!(cv, rv);
            }
            assert_eq!(cp.ip, rp.ip);
        }
    }
}

// -------------------------------------------------------------------
// VM API (init/free/trace) — externally observable state
// -------------------------------------------------------------------
#[test]
fn vm_init_trace_free() {
    unsafe {
        let (c, r) = open_libs();
        let c_init: libloading::Symbol<FnVmInit> = c.sym(b"vm_init");
        let c_free: libloading::Symbol<FnVmFree> = c.sym(b"vm_free");
        let c_trace: libloading::Symbol<FnVmTrace> = c.sym(b"vm_trace");
        let r_init: libloading::Symbol<FnVmInit> = r.sym(b"vm_init");
        let r_free: libloading::Symbol<FnVmFree> = r.sym(b"vm_free");
        let r_trace: libloading::Symbol<FnVmTrace> = r.sym(b"vm_trace");

        let mut cv: VM = VM::default();
        let mut rv: VM = VM::default();
        c_init(&mut cv);
        r_init(&mut rv);
        assert_eq!(cv.steps, rv.steps);

        for t in [0, 1, 2, 25, 30, -7, 99] {
            c_trace(&mut cv, t);
            r_trace(&mut rv, t);
            assert_eq!(cv.trace.len, rv.trace.len);
            assert_eq!(slice_from_intvec(&cv.trace), slice_from_intvec(&rv.trace));
        }

        c_free(&mut cv);
        r_free(&mut rv);
        assert_eq!(cv.steps, rv.steps);
        assert!(cv.trace.data.is_null());
        assert!(rv.trace.data.is_null());
    }
}

// -------------------------------------------------------------------
// vm_print API — write to a tmpfile and compare contents
// -------------------------------------------------------------------
type FILE = std::ffi::c_void;
extern "C" {
    fn tmpfile() -> *mut FILE;
    fn fclose(f: *mut FILE) -> c_int;
    fn rewind(f: *mut FILE);
    fn fread(ptr: *mut u8, sz: usize, nm: usize, f: *mut FILE) -> usize;
}
type FnVmPrint = unsafe extern "C" fn(*mut FILE, *const c_char, *const VM);

unsafe fn capture_print(
    fnptr: &libloading::Symbol<FnVmPrint>,
    label: &[u8],
    vm: &VM,
) -> Vec<u8> {
    let f = tmpfile();
    assert!(!f.is_null());
    let mut clbl = label.to_vec();
    clbl.push(0);
    fnptr(f, clbl.as_ptr() as *const c_char, vm);
    rewind(f);
    let mut buf = vec![0u8; 8192];
    let n = fread(buf.as_mut_ptr(), 1, buf.len(), f);
    buf.truncate(n);
    fclose(f);
    buf
}

#[test]
fn vm_print_matches() {
    unsafe {
        let (c, r) = open_libs();
        let c_init: libloading::Symbol<FnVmInit> = c.sym(b"vm_init");
        let c_free: libloading::Symbol<FnVmFree> = c.sym(b"vm_free");
        let c_trace: libloading::Symbol<FnVmTrace> = c.sym(b"vm_trace");
        let c_push: libloading::Symbol<FnIvPush> = c.sym(b"iv_push");
        let c_print: libloading::Symbol<FnVmPrint> = c.sym(b"vm_print");
        let r_init: libloading::Symbol<FnVmInit> = r.sym(b"vm_init");
        let r_free: libloading::Symbol<FnVmFree> = r.sym(b"vm_free");
        let r_trace: libloading::Symbol<FnVmTrace> = r.sym(b"vm_trace");
        let r_push: libloading::Symbol<FnIvPush> = r.sym(b"iv_push");
        let r_print: libloading::Symbol<FnVmPrint> = r.sym(b"vm_print");

        let mut cv: VM = VM::default();
        let mut rv: VM = VM::default();
        c_init(&mut cv);
        r_init(&mut rv);

        // empty
        let cb = capture_print(&c_print, b"L1:", &cv);
        let rb = capture_print(&r_print, b"L1:", &rv);
        assert_eq!(cb, rb, "empty: {:?} vs {:?}", cb, rb);

        // populate stack and trace
        for x in [1, 2, 3, -5, 100] {
            c_push(&mut cv.stack, x);
            r_push(&mut rv.stack, x);
        }
        cv.steps = 17;
        rv.steps = 17;
        for t in [0, 1, 2, 25, 26, 27, 50, -1, -3] {
            c_trace(&mut cv, t);
            r_trace(&mut rv, t);
        }
        let cb = capture_print(&c_print, b"PFX:", &cv);
        let rb = capture_print(&r_print, b"PFX:", &rv);
        assert_eq!(cb, rb);

        c_free(&mut cv);
        r_free(&mut rv);
    }
}

// -------------------------------------------------------------------
// a.c: call_a_once / process_a_stream
// -------------------------------------------------------------------
#[test]
fn call_a_once_matches() {
    unsafe {
        let (c, r) = open_libs();
        let c_fn: libloading::Symbol<FnCallOnce> = c.sym(b"call_a_once");
        let r_fn: libloading::Symbol<FnCallOnce> = r.sym(b"call_a_once");
        // Both libraries start with state_a==0. Calls advance state in
        // lockstep across libraries.
        for x in [0, 1, 2, 3, -1, 5, 7, -42, 100, 42, -7, 99, 200, -200, 17] {
            let cv = c_fn(x);
            let rv = r_fn(x);
            assert_eq!(cv, rv, "call_a_once({}) C={} R={}", x, cv, rv);
        }
    }
}

#[test]
fn process_a_stream_matches() {
    unsafe {
        let (c, r) = open_libs();
        let c_fn: libloading::Symbol<FnProcessStream> = c.sym(b"process_a_stream");
        let r_fn: libloading::Symbol<FnProcessStream> = r.sym(b"process_a_stream");
        let xs: Vec<c_int> = vec![1, 2, 3, -4, 5, 0, 9, 13, -100];
        let cv = c_fn(xs.as_ptr(), xs.len());
        let rv = r_fn(xs.as_ptr(), xs.len());
        assert_eq!(cv, rv, "process_a_stream C={} R={}", cv, rv);

        let xs2: Vec<c_int> = (0..50).collect();
        let cv = c_fn(xs2.as_ptr(), xs2.len());
        let rv = r_fn(xs2.as_ptr(), xs2.len());
        assert_eq!(cv, rv);

        // empty
        let cv = c_fn(std::ptr::null(), 0);
        let rv = r_fn(std::ptr::null(), 0);
        assert_eq!(cv, rv);
    }
}

// -------------------------------------------------------------------
// b.c: call_b_once / process_b_stream
// -------------------------------------------------------------------
#[test]
fn call_b_once_matches() {
    unsafe {
        let (c, r) = open_libs();
        let c_fn: libloading::Symbol<FnCallOnce> = c.sym(b"call_b_once");
        let r_fn: libloading::Symbol<FnCallOnce> = r.sym(b"call_b_once");
        for x in [0, 1, 2, 3, -1, 5, 7, -42, 100, 42, -7, 99, 200, -200, 17] {
            let cv = c_fn(x);
            let rv = r_fn(x);
            assert_eq!(cv, rv, "call_b_once({}) C={} R={}", x, cv, rv);
        }
    }
}

#[test]
fn process_b_stream_matches() {
    unsafe {
        let (c, r) = open_libs();
        let c_fn: libloading::Symbol<FnProcessStream> = c.sym(b"process_b_stream");
        let r_fn: libloading::Symbol<FnProcessStream> = r.sym(b"process_b_stream");
        let xs: Vec<c_int> = vec![1, 2, 3, -4, 5, 0, 9, 13, -100];
        let cv = c_fn(xs.as_ptr(), xs.len());
        let rv = r_fn(xs.as_ptr(), xs.len());
        assert_eq!(cv, rv);

        let xs2: Vec<c_int> = (0..30).collect();
        let cv = c_fn(xs2.as_ptr(), xs2.len());
        let rv = r_fn(xs2.as_ptr(), xs2.len());
        assert_eq!(cv, rv);

        let cv = c_fn(std::ptr::null(), 0);
        let rv = r_fn(std::ptr::null(), 0);
        assert_eq!(cv, rv);
    }
}

// -------------------------------------------------------------------
// run_engine — the big integration test, includes many cases per impl_id
// -------------------------------------------------------------------
fn vm_dump_to_string(
    print_fn: &libloading::Symbol<FnVmPrint>,
    label: &[u8],
    vm: &VM,
) -> Vec<u8> {
    unsafe { capture_print(print_fn, label, vm) }
}

fn run_program_and_compare(c: &Lib, r: &Lib, impl_id: c_int, code: &[c_int]) {
    unsafe {
        let c_vm_init: libloading::Symbol<FnVmInit> = c.sym(b"vm_init");
        let c_vm_free: libloading::Symbol<FnVmFree> = c.sym(b"vm_free");
        let c_run: libloading::Symbol<FnRunEngine> = c.sym(b"run_engine");
        let c_print: libloading::Symbol<FnVmPrint> = c.sym(b"vm_print");
        let r_vm_init: libloading::Symbol<FnVmInit> = r.sym(b"vm_init");
        let r_vm_free: libloading::Symbol<FnVmFree> = r.sym(b"vm_free");
        let r_run: libloading::Symbol<FnRunEngine> = r.sym(b"run_engine");
        let r_print: libloading::Symbol<FnVmPrint> = r.sym(b"vm_print");

        let mut cv: VM = VM::default();
        let mut rv: VM = VM::default();
        c_vm_init(&mut cv);
        r_vm_init(&mut rv);

        let crc = c_run(impl_id, code.as_ptr(), code.len(), &mut cv);
        let rrc = r_run(impl_id, code.as_ptr(), code.len(), &mut rv);
        assert_eq!(
            crc, rrc,
            "run_engine impl={} code={:?} rc differs C={} R={}",
            impl_id, code, crc, rrc
        );
        assert_eq!(cv.steps, rv.steps, "steps differ for impl={}", impl_id);
        assert_eq!(
            slice_from_intvec(&cv.stack),
            slice_from_intvec(&rv.stack),
            "stack differs for impl={}",
            impl_id
        );
        assert_eq!(
            slice_from_intvec(&cv.trace),
            slice_from_intvec(&rv.trace),
            "trace differs for impl={}",
            impl_id
        );

        // Also compare vm_print output byte-for-byte
        let cb = vm_dump_to_string(&c_print, b"X:", &cv);
        let rb = vm_dump_to_string(&r_print, b"X:", &rv);
        assert_eq!(cb, rb);

        c_vm_free(&mut cv);
        r_vm_free(&mut rv);
    }
}

#[test]
fn run_engine_simple_programs() {
    unsafe {
        let (c, r) = open_libs();

        // PUSH 42, RET
        let p1: Vec<c_int> = vec![0, 42, 10];
        // PUSH 1, PUSH 2, ADD
        let p2: Vec<c_int> = vec![0, 1, 0, 2, 1];
        // PUSH 3, PUSH 4, MUL
        let p3: Vec<c_int> = vec![0, 3, 0, 4, 2];
        // PUSH 5, DUP, POP
        let p4: Vec<c_int> = vec![0, 5, 3, 4];
        // PUSH 7, OP5 (classify)
        let p5: Vec<c_int> = vec![0, 7, 5];
        // PUSH 1, OP6 (cond jump 0)
        let p6: Vec<c_int> = vec![0, 1, 6, 0, 0, 99, 10];
        // PUSH 2, OP7 (call_n)
        let p7: Vec<c_int> = vec![0, 2, 7, 1, 5];
        // PUSH 9, OP8
        let p8: Vec<c_int> = vec![0, 9, 8];
        // PUSH 1,2,3, OP9 with m=3
        let p9: Vec<c_int> = vec![0, 1, 0, 2, 0, 3, 9, 3];
        // RET-only
        let p10: Vec<c_int> = vec![10];
        // empty
        let p11: Vec<c_int> = vec![];
        // unknown opcode
        let p12: Vec<c_int> = vec![42];

        for prog in [&p1, &p2, &p3, &p4, &p5, &p6, &p7, &p8, &p9, &p10, &p11, &p12] {
            for impl_id in 0..3 {
                run_program_and_compare(&c, &r, impl_id, prog);
            }
        }
    }
}

#[test]
fn run_engine_complex_program() {
    unsafe {
        let (c, r) = open_libs();
        // PUSH 5, PUSH 3, ADD, DUP, OP5 (classify), POP, RET
        let prog: Vec<c_int> = vec![0, 5, 0, 3, 1, 3, 5, 4, 10];
        for impl_id in 0..3 {
            run_program_and_compare(&c, &r, impl_id, &prog);
        }
    }
}
