use libloading::{Library, Symbol};
use std::path::PathBuf;

// C-compatible struct layouts matching both .so files
#[repr(C)]
struct IntVec {
    data: *mut i32,
    len: usize,
    cap: usize,
}

#[repr(C)]
struct Program {
    code: *const i32,
    n: usize,
    ip: usize,
}

#[repr(C)]
struct VM {
    stack: IntVec,
    trace: IntVec,
    steps: i32,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

// Helper: load both libraries (each test gets fresh loads = fresh static state)
struct Libs {
    c: Library,
    r: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            Libs {
                c: Library::new(c_lib_path()).expect("load C .so"),
                r: Library::new(rust_lib_path()).expect("load Rust .so"),
            }
        }
    }
}

// ==================== Level 0: target (lib.c) ====================

#[test]
fn test_target() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.c.get(b"target").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.r.get(b"target").unwrap();
        for code in -10..100 {
            let c_val = c_fn(code);
            let r_val = r_fn(code);
            assert_eq!(c_val, r_val, "target({code}): C={c_val} Rust={r_val}");
        }
    }
}

// ==================== Level 0: IntVec operations ====================

#[test]
fn test_iv_operations() {
    let libs = Libs::load();
    unsafe {
        type IvInit = unsafe extern "C" fn(*mut IntVec);
        type IvFree = unsafe extern "C" fn(*mut IntVec);
        type IvPush = unsafe extern "C" fn(*mut IntVec, i32) -> bool;
        type IvPop = unsafe extern "C" fn(*mut IntVec, *mut i32) -> bool;
        type IvPeek = unsafe extern "C" fn(*const IntVec, i32) -> i32;

        let c_init: Symbol<IvInit> = libs.c.get(b"iv_init").unwrap();
        let c_free: Symbol<IvFree> = libs.c.get(b"iv_free").unwrap();
        let c_push: Symbol<IvPush> = libs.c.get(b"iv_push").unwrap();
        let c_pop: Symbol<IvPop> = libs.c.get(b"iv_pop").unwrap();
        let c_peek: Symbol<IvPeek> = libs.c.get(b"iv_peek").unwrap();

        let r_init: Symbol<IvInit> = libs.r.get(b"iv_init").unwrap();
        let r_free: Symbol<IvFree> = libs.r.get(b"iv_free").unwrap();
        let r_push: Symbol<IvPush> = libs.r.get(b"iv_push").unwrap();
        let r_pop: Symbol<IvPop> = libs.r.get(b"iv_pop").unwrap();
        let r_peek: Symbol<IvPeek> = libs.r.get(b"iv_peek").unwrap();

        let mut cv = std::mem::zeroed::<IntVec>();
        let mut rv = std::mem::zeroed::<IntVec>();
        c_init(&mut cv);
        r_init(&mut rv);

        // peek on empty
        assert_eq!(c_peek(&cv, -999), r_peek(&rv, -999), "peek empty");

        // pop on empty
        let mut co = 0i32; let mut ro = 0i32;
        assert_eq!(c_pop(&mut cv, &mut co), r_pop(&mut rv, &mut ro), "pop empty");

        // push and peek
        for i in 0..20 {
            let cp = c_push(&mut cv, i * 3 + 7);
            let rp = r_push(&mut rv, i * 3 + 7);
            assert_eq!(cp, rp, "push {i}");
            assert_eq!(c_peek(&cv, 0), r_peek(&rv, 0), "peek after push {i}");
        }

        // pop all
        for i in (0..20).rev() {
            let cp = c_pop(&mut cv, &mut co);
            let rp = r_pop(&mut rv, &mut ro);
            assert_eq!(cp, rp, "pop {i} success");
            assert_eq!(co, ro, "pop {i} value");
        }

        c_free(&mut cv);
        r_free(&mut rv);
    }
}

// ==================== Level 0: Program operations ====================

#[test]
fn test_prog_operations() {
    let libs = Libs::load();
    unsafe {
        type ProgInit = unsafe extern "C" fn(*mut Program, *const i32, usize);
        type ProgFetch = unsafe extern "C" fn(*mut Program, *mut i32) -> bool;

        let c_init: Symbol<ProgInit> = libs.c.get(b"prog_init").unwrap();
        let c_fetch: Symbol<ProgFetch> = libs.c.get(b"prog_fetch").unwrap();
        let r_init: Symbol<ProgInit> = libs.r.get(b"prog_init").unwrap();
        let r_fetch: Symbol<ProgFetch> = libs.r.get(b"prog_fetch").unwrap();

        let data = [10, 20, 30, 40, 50];
        let mut cp = std::mem::zeroed::<Program>();
        let mut rp = std::mem::zeroed::<Program>();
        c_init(&mut cp, data.as_ptr(), data.len());
        r_init(&mut rp, data.as_ptr(), data.len());

        for _ in 0..6 {
            let mut co = 0i32; let mut ro = 0i32;
            let cf = c_fetch(&mut cp, &mut co);
            let rf = r_fetch(&mut rp, &mut ro);
            assert_eq!(cf, rf, "fetch success");
            if cf { assert_eq!(co, ro, "fetch value"); }
        }
    }
}

// ==================== Level 0: VM init/free ====================

#[test]
fn test_vm_init_free() {
    let libs = Libs::load();
    unsafe {
        type VmInit = unsafe extern "C" fn(*mut VM);
        type VmFree = unsafe extern "C" fn(*mut VM);
        type VmTrace = unsafe extern "C" fn(*mut VM, i32);

        let c_init: Symbol<VmInit> = libs.c.get(b"vm_init").unwrap();
        let c_free: Symbol<VmFree> = libs.c.get(b"vm_free").unwrap();
        let c_trace: Symbol<VmTrace> = libs.c.get(b"vm_trace").unwrap();
        let r_init: Symbol<VmInit> = libs.r.get(b"vm_init").unwrap();
        let r_free: Symbol<VmFree> = libs.r.get(b"vm_free").unwrap();
        let r_trace: Symbol<VmTrace> = libs.r.get(b"vm_trace").unwrap();

        let mut cvm = std::mem::zeroed::<VM>();
        let mut rvm = std::mem::zeroed::<VM>();
        c_init(&mut cvm);
        r_init(&mut rvm);

        assert_eq!(cvm.steps, rvm.steps, "steps after init");
        assert_eq!(cvm.stack.len, rvm.stack.len, "stack.len after init");
        assert_eq!(cvm.trace.len, rvm.trace.len, "trace.len after init");

        for t in [0, 5, 10, 14] {
            c_trace(&mut cvm, t);
            r_trace(&mut rvm, t);
        }
        assert_eq!(cvm.trace.len, rvm.trace.len, "trace.len after traces");
        for i in 0..cvm.trace.len {
            assert_eq!(*cvm.trace.data.add(i), *rvm.trace.data.add(i), "trace[{i}]");
        }

        c_free(&mut cvm);
        r_free(&mut rvm);
    }
}

// ==================== Level 1: call_a_once ====================
// Note: call_a_once uses static state_a. Each .so has its own copy, both start at 0.
// We must call in the same sequence on both.

#[test]
fn test_call_a_once() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.c.get(b"call_a_once").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.r.get(b"call_a_once").unwrap();
        for x in -5..30 {
            let c_val = c_fn(x);
            let r_val = r_fn(x);
            assert_eq!(c_val, r_val, "call_a_once({x}): C={c_val} Rust={r_val}");
        }
    }
}

// ==================== Level 1: process_a_stream ====================

#[test]
fn test_process_a_stream() {
    // Fresh load to get fresh static state
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs.c.get(b"process_a_stream").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs.r.get(b"process_a_stream").unwrap();

        let test_cases: Vec<Vec<i32>> = vec![
            vec![],
            vec![0],
            vec![1, 2, 3],
            vec![-1, 0, 1],
            vec![10, 20, 30, 40, 50],
            vec![100, -50, 77, 0, -1, 255],
            (0..20).collect(),
        ];

        for tc in &test_cases {
            // Fresh load for each test case to reset static state
            let libs2 = Libs::load();
            let c_fn2: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs2.c.get(b"process_a_stream").unwrap();
            let r_fn2: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs2.r.get(b"process_a_stream").unwrap();
            let c_val = c_fn2(tc.as_ptr(), tc.len());
            let r_val = r_fn2(tc.as_ptr(), tc.len());
            assert_eq!(c_val, r_val, "process_a_stream({tc:?}): C={c_val} Rust={r_val}");
        }
    }
}

// ==================== Level 1: call_b_once ====================

#[test]
fn test_call_b_once() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.c.get(b"call_b_once").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.r.get(b"call_b_once").unwrap();
        for x in -5..30 {
            let c_val = c_fn(x);
            let r_val = r_fn(x);
            assert_eq!(c_val, r_val, "call_b_once({x}): C={c_val} Rust={r_val}");
        }
    }
}

// ==================== Level 1: process_b_stream ====================

#[test]
fn test_process_b_stream() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs.c.get(b"process_b_stream").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs.r.get(b"process_b_stream").unwrap();

        let test_cases: Vec<Vec<i32>> = vec![
            vec![],
            vec![0],
            vec![1, 2, 3],
            vec![-1, 0, 1],
            vec![10, 20, 30, 40, 50],
            (0..15).collect(),
        ];

        for tc in &test_cases {
            let libs2 = Libs::load();
            let c_fn2: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs2.c.get(b"process_b_stream").unwrap();
            let r_fn2: Symbol<unsafe extern "C" fn(*const i32, usize) -> i32> = libs2.r.get(b"process_b_stream").unwrap();
            let c_val = c_fn2(tc.as_ptr(), tc.len());
            let r_val = r_fn2(tc.as_ptr(), tc.len());
            assert_eq!(c_val, r_val, "process_b_stream({tc:?}): C={c_val} Rust={r_val}");
        }
    }
}

// ==================== Level 2: run_engine ====================

#[test]
fn test_run_engine() {
    // Test programs for the VM
    let programs: Vec<(&str, Vec<i32>)> = vec![
        // Simple: push 42, halt
        ("push_halt", vec![0, 42, 10]),
        // Push two, add, halt
        ("add", vec![0, 3, 0, 4, 1, 10]),
        // Push two, mul, halt
        ("mul", vec![0, 5, 0, 6, 2, 10]),
        // Dup, halt
        ("dup", vec![0, 7, 3, 10]),
        // Push, drop, halt
        ("drop", vec![0, 99, 4, 10]),
        // Push, classify, halt
        ("classify", vec![0, 10, 5, 10]),
        // Push 0, skip 1 (cond=0 so no skip), push 77, halt
        ("skip_false", vec![0, 0, 6, 1, 0, 77, 10]),
        // Push 1, skip 2 (cond=1 so skip 2), push 88, push 99, halt
        ("skip_true", vec![0, 1, 6, 2, 0, 88, 10]),
        // Push 5, classify2, halt
        ("classify2", vec![0, 5, 8, 10]),
        // Empty program (no halt)
        ("empty", vec![]),
        // Just halt
        ("just_halt", vec![10]),
        // Unknown opcode
        ("unknown_op", vec![99]),
        // Push, push, push, process_stream with m=2, halt
        ("process_stream", vec![0, 1, 0, 2, 0, 3, 0, 4, 9, 2, 10]),
        // Repeat: push 0 (opcode 0 needs imm, so this will error in inner), halt
        ("repeat", vec![0, 10, 7, 3, 0, 42, 10]),
    ];

    for (name, prog) in &programs {
        for impl_id in 0..3 {
            // Fresh load for each test to reset static state
            let libs = Libs::load();
            unsafe {
                type VmInit = unsafe extern "C" fn(*mut VM);
                type VmFree = unsafe extern "C" fn(*mut VM);
                type RunEngine = unsafe extern "C" fn(i32, *const i32, usize, *mut VM) -> i32;

                let c_vm_init: Symbol<VmInit> = libs.c.get(b"vm_init").unwrap();
                let c_vm_free: Symbol<VmFree> = libs.c.get(b"vm_free").unwrap();
                let c_run: Symbol<RunEngine> = libs.c.get(b"run_engine").unwrap();

                let r_vm_init: Symbol<VmInit> = libs.r.get(b"vm_init").unwrap();
                let r_vm_free: Symbol<VmFree> = libs.r.get(b"vm_free").unwrap();
                let r_run: Symbol<RunEngine> = libs.r.get(b"run_engine").unwrap();

                let mut cvm = std::mem::zeroed::<VM>();
                let mut rvm = std::mem::zeroed::<VM>();
                c_vm_init(&mut cvm);
                r_vm_init(&mut rvm);

                let c_rc = c_run(impl_id, prog.as_ptr(), prog.len(), &mut cvm);
                let r_rc = r_run(impl_id, prog.as_ptr(), prog.len(), &mut rvm);

                assert_eq!(c_rc, r_rc, "run_engine impl={impl_id} prog={name}: rc C={c_rc} Rust={r_rc}");
                assert_eq!(cvm.steps, rvm.steps, "run_engine impl={impl_id} prog={name}: steps");
                assert_eq!(cvm.stack.len, rvm.stack.len, "run_engine impl={impl_id} prog={name}: stack.len");
                assert_eq!(cvm.trace.len, rvm.trace.len, "run_engine impl={impl_id} prog={name}: trace.len");

                // Compare stack contents
                for i in 0..cvm.stack.len {
                    let cv = *cvm.stack.data.add(i);
                    let rv = *rvm.stack.data.add(i);
                    assert_eq!(cv, rv, "run_engine impl={impl_id} prog={name}: stack[{i}]");
                }

                // Compare trace contents
                for i in 0..cvm.trace.len {
                    let cv = *cvm.trace.data.add(i);
                    let rv = *rvm.trace.data.add(i);
                    assert_eq!(cv, rv, "run_engine impl={impl_id} prog={name}: trace[{i}]");
                }

                c_vm_free(&mut cvm);
                r_vm_free(&mut rvm);
            }
        }
    }
}

// ==================== Full integration: main-like test ====================

#[test]
fn test_full_main_programs() {
    // Programs that exercise multiple opcodes together
    let programs: Vec<Vec<i32>> = vec![
        // Push 10, push 20, add, dup, classify, halt
        vec![0, 10, 0, 20, 1, 3, 5, 10],
        // Push 3, push 4, mul, push 2, add, classify2, halt
        vec![0, 3, 0, 4, 2, 0, 2, 1, 8, 10],
        // Push 1, push 2, push 3, push 4, process_stream m=2, halt
        vec![0, 1, 0, 2, 0, 3, 0, 4, 9, 2, 10],
        // Complex: push, dup, add, dup, mul, classify, drop, halt
        vec![0, 5, 3, 1, 3, 2, 5, 4, 10],
    ];

    for (pi, prog) in programs.iter().enumerate() {
        for impl_id in 0..3 {
            let libs = Libs::load();
            unsafe {
                type VmInit = unsafe extern "C" fn(*mut VM);
                type VmFree = unsafe extern "C" fn(*mut VM);
                type RunEngine = unsafe extern "C" fn(i32, *const i32, usize, *mut VM) -> i32;

                let c_vm_init: Symbol<VmInit> = libs.c.get(b"vm_init").unwrap();
                let c_vm_free: Symbol<VmFree> = libs.c.get(b"vm_free").unwrap();
                let c_run: Symbol<RunEngine> = libs.c.get(b"run_engine").unwrap();
                let r_vm_init: Symbol<VmInit> = libs.r.get(b"vm_init").unwrap();
                let r_vm_free: Symbol<VmFree> = libs.r.get(b"vm_free").unwrap();
                let r_run: Symbol<RunEngine> = libs.r.get(b"run_engine").unwrap();

                let mut cvm = std::mem::zeroed::<VM>();
                let mut rvm = std::mem::zeroed::<VM>();
                c_vm_init(&mut cvm);
                r_vm_init(&mut rvm);

                let c_rc = c_run(impl_id, prog.as_ptr(), prog.len(), &mut cvm);
                let r_rc = r_run(impl_id, prog.as_ptr(), prog.len(), &mut rvm);

                assert_eq!(c_rc, r_rc, "full prog[{pi}] impl={impl_id}: rc");
                assert_eq!(cvm.steps, rvm.steps, "full prog[{pi}] impl={impl_id}: steps");
                assert_eq!(cvm.stack.len, rvm.stack.len, "full prog[{pi}] impl={impl_id}: stack.len");
                assert_eq!(cvm.trace.len, rvm.trace.len, "full prog[{pi}] impl={impl_id}: trace.len");

                for i in 0..cvm.stack.len {
                    assert_eq!(*cvm.stack.data.add(i), *rvm.stack.data.add(i),
                        "full prog[{pi}] impl={impl_id}: stack[{i}]");
                }
                for i in 0..cvm.trace.len {
                    assert_eq!(*cvm.trace.data.add(i), *rvm.trace.data.add(i),
                        "full prog[{pi}] impl={impl_id}: trace[{i}]");
                }

                c_vm_free(&mut cvm);
                r_vm_free(&mut rvm);
            }
        }
    }
}
