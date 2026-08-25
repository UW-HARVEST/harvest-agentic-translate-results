use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Debug)]
struct IntVec {
    data: *mut c_int,
    len: usize,
    cap: usize,
}

#[repr(C)]
#[derive(Debug)]
struct Program {
    code: *const c_int,
    n: usize,
    ip: usize,
}

#[repr(C)]
#[derive(Debug)]
struct VM {
    stack: IntVec,
    trace: IntVec,
    steps: c_int,
}

type ScalarFn = unsafe extern "C" fn(c_int) -> c_int;
type StreamFn = unsafe extern "C" fn(*const c_int, usize) -> c_int;
type IvInitFn = unsafe extern "C" fn(*mut IntVec);
type IvFreeFn = unsafe extern "C" fn(*mut IntVec);
type IvReserveFn = unsafe extern "C" fn(*mut IntVec, usize) -> bool;
type IvPushFn = unsafe extern "C" fn(*mut IntVec, c_int) -> bool;
type IvPopFn = unsafe extern "C" fn(*mut IntVec, *mut c_int) -> bool;
type IvPeekFn = unsafe extern "C" fn(*const IntVec, c_int) -> c_int;
type ProgInitFn = unsafe extern "C" fn(*mut Program, *const c_int, usize);
type ProgFetchFn = unsafe extern "C" fn(*mut Program, *mut c_int) -> bool;
type VmInitFn = unsafe extern "C" fn(*mut VM);
type VmFreeFn = unsafe extern "C" fn(*mut VM);
type VmTraceFn = unsafe extern "C" fn(*mut VM, c_int);
type VmPrintFn = unsafe extern "C" fn(*mut c_void, *const c_char, *const VM);
type RunEngineFn = unsafe extern "C" fn(c_int, *const c_int, usize, *mut VM) -> c_int;
type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

struct Api {
    _library: Library,
    target: ScalarFn,
    call_a_once: ScalarFn,
    call_b_once: ScalarFn,
    process_a_stream: StreamFn,
    process_b_stream: StreamFn,
    iv_init: IvInitFn,
    iv_free: IvFreeFn,
    iv_reserve: IvReserveFn,
    iv_push: IvPushFn,
    iv_pop: IvPopFn,
    iv_peek: IvPeekFn,
    prog_init: ProgInitFn,
    prog_fetch: ProgFetchFn,
    vm_init: VmInitFn,
    vm_free: VmFreeFn,
    vm_trace: VmTraceFn,
    vm_print: VmPrintFn,
    run_engine: RunEngineFn,
    main: MainFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = Library::new(path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *library
                    .get::<$ty>(concat!($name, "\0").as_bytes())
                    .unwrap_or_else(|error| {
                        panic!("missing {} in {}: {error}", $name, path.display())
                    })
            };
        }
        Self {
            target: symbol!("target", ScalarFn),
            call_a_once: symbol!("call_a_once", ScalarFn),
            call_b_once: symbol!("call_b_once", ScalarFn),
            process_a_stream: symbol!("process_a_stream", StreamFn),
            process_b_stream: symbol!("process_b_stream", StreamFn),
            iv_init: symbol!("iv_init", IvInitFn),
            iv_free: symbol!("iv_free", IvFreeFn),
            iv_reserve: symbol!("iv_reserve", IvReserveFn),
            iv_push: symbol!("iv_push", IvPushFn),
            iv_pop: symbol!("iv_pop", IvPopFn),
            iv_peek: symbol!("iv_peek", IvPeekFn),
            prog_init: symbol!("prog_init", ProgInitFn),
            prog_fetch: symbol!("prog_fetch", ProgFetchFn),
            vm_init: symbol!("vm_init", VmInitFn),
            vm_free: symbol!("vm_free", VmFreeFn),
            vm_trace: symbol!("vm_trace", VmTraceFn),
            vm_print: symbol!("vm_print", VmPrintFn),
            run_engine: symbol!("run_engine", RunEngineFn),
            main: symbol!("main", MainFn),
            _library: library,
        }
    }
}

struct Apis {
    c: Api,
    rust: Api,
}

impl Apis {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver_c.so");
        let rust_path = root.join("target/debug/deps/libdriver.so");
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
        Self {
            c: Api::load(&c_path),
            rust: Api::load(&rust_path),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct VecSnapshot {
    len: usize,
    cap: usize,
    values: Vec<c_int>,
}

#[derive(Debug, Eq, PartialEq)]
struct VmSnapshot {
    stack: VecSnapshot,
    trace: VecSnapshot,
    steps: c_int,
}

unsafe fn snapshot_vec(vector: &IntVec) -> VecSnapshot {
    let values = if vector.len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(vector.data, vector.len).to_vec()
    };
    VecSnapshot {
        len: vector.len,
        cap: vector.cap,
        values,
    }
}

unsafe fn snapshot_vm(vm: &VM) -> VmSnapshot {
    VmSnapshot {
        stack: snapshot_vec(&vm.stack),
        trace: snapshot_vec(&vm.trace),
        steps: vm.steps,
    }
}

unsafe fn new_vm(api: &Api) -> VM {
    let mut vm = VM {
        stack: IntVec {
            data: 1usize as *mut c_int,
            len: usize::MAX,
            cap: usize::MAX,
        },
        trace: IntVec {
            data: 1usize as *mut c_int,
            len: usize::MAX,
            cap: usize::MAX,
        },
        steps: -1,
    };
    (api.vm_init)(&mut vm);
    vm
}

unsafe fn push_values(api: &Api, vector: *mut IntVec, values: &[c_int]) {
    for &value in values {
        assert!((api.iv_push)(vector, value));
    }
}

unsafe fn run_pair(
    apis: &Apis,
    impl_id: c_int,
    initial_stack: &[c_int],
    code: &[c_int],
) -> (c_int, VmSnapshot) {
    let mut c_vm = new_vm(&apis.c);
    let mut rust_vm = new_vm(&apis.rust);
    push_values(&apis.c, &mut c_vm.stack, initial_stack);
    push_values(&apis.rust, &mut rust_vm.stack, initial_stack);

    let code_ptr = if code.is_empty() {
        ptr::null()
    } else {
        code.as_ptr()
    };
    let c_result = (apis.c.run_engine)(impl_id, code_ptr, code.len(), &mut c_vm);
    let rust_result = (apis.rust.run_engine)(impl_id, code_ptr, code.len(), &mut rust_vm);
    let c_snapshot = snapshot_vm(&c_vm);
    let rust_snapshot = snapshot_vm(&rust_vm);
    assert_eq!(
        rust_result, c_result,
        "return mismatch: impl={impl_id} code={code:?}"
    );
    assert_eq!(
        rust_snapshot, c_snapshot,
        "VM mismatch: impl={impl_id} initial={initial_stack:?} code={code:?}"
    );

    (apis.c.vm_free)(&mut c_vm);
    (apis.rust.vm_free)(&mut rust_vm);
    (c_result, c_snapshot)
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Self(0x4d59_5df4_d0f3_3173)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 as u32
    }

    fn small_i32(&mut self) -> i32 {
        (self.next_u32() % 2001) as i32 - 1000
    }
}

unsafe extern "C" {
    fn open_memstream(buffer: *mut *mut c_char, size: *mut usize) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn free(ptr: *mut c_void);
    fn tmpfile() -> *mut c_void;
    fn fileno(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fwrite(data: *const c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fread(data: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fseek(stream: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn clearerr(stream: *mut c_void);
    static mut stdin: *mut c_void;
}

unsafe fn print_vm(api: &Api, vm: &VM, label: &[u8]) -> Vec<u8> {
    let mut buffer = ptr::null_mut();
    let mut size = 0usize;
    let stream = open_memstream(&mut buffer, &mut size);
    assert!(!stream.is_null());
    (api.vm_print)(stream, label.as_ptr().cast(), vm);
    assert_eq!(fclose(stream), 0);
    let result = std::slice::from_raw_parts(buffer.cast::<u8>(), size).to_vec();
    free(buffer.cast());
    result
}

#[derive(Debug, Eq, PartialEq)]
struct MainResult {
    result: c_int,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

unsafe fn read_file(stream: *mut c_void) -> Vec<u8> {
    assert_eq!(fflush(stream), 0);
    assert_eq!(fseek(stream, 0, 2), 0);
    let length = ftell(stream);
    assert!(length >= 0);
    assert_eq!(fseek(stream, 0, 0), 0);
    let mut bytes = vec![0u8; length as usize];
    if !bytes.is_empty() {
        assert_eq!(
            fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), stream),
            bytes.len()
        );
    }
    bytes
}

unsafe fn capture_main(api: &Api, args: &[&str], input: &[u8]) -> MainResult {
    let input_file = tmpfile();
    let output_file = tmpfile();
    let error_file = tmpfile();
    assert!(!input_file.is_null() && !output_file.is_null() && !error_file.is_null());
    if !input.is_empty() {
        assert_eq!(
            fwrite(input.as_ptr().cast(), 1, input.len(), input_file),
            input.len()
        );
    }
    assert_eq!(fflush(input_file), 0);
    assert_eq!(fseek(input_file, 0, 0), 0);

    let saved = [dup(0), dup(1), dup(2)];
    assert!(saved.iter().all(|&fd| fd >= 0));
    assert_eq!(dup2(fileno(input_file), 0), 0);
    assert_eq!(dup2(fileno(output_file), 1), 1);
    assert_eq!(dup2(fileno(error_file), 2), 2);
    clearerr(stdin);

    let strings: Vec<_> = args
        .iter()
        .map(|value| CString::new(*value).unwrap())
        .collect();
    let mut argv: Vec<_> = strings
        .iter()
        .map(|value| value.as_ptr() as *mut c_char)
        .collect();
    argv.push(ptr::null_mut());
    let result = (api.main)(strings.len() as c_int, argv.as_mut_ptr());
    fflush(ptr::null_mut());

    for (target, saved_fd) in saved.into_iter().enumerate() {
        assert_eq!(dup2(saved_fd, target as c_int), target as c_int);
        assert_eq!(close(saved_fd), 0);
    }
    let stdout_bytes = read_file(output_file);
    let stderr_bytes = read_file(error_file);
    assert_eq!(fclose(input_file), 0);
    assert_eq!(fclose(output_file), 0);
    assert_eq!(fclose(error_file), 0);
    MainResult {
        result,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    }
}

#[test]
fn valid_configuration_surface() {
    let _serial = SERIAL.lock().unwrap();
    unsafe {
        let apis = Apis::load();
        let mut covered = [false; 74];
        let mut rng = Rng::new();

        for code in [i32::MIN, -1, 0, 1, 3, 4, 6, 7, 8, 9, 10, i32::MAX] {
            assert_eq!((apis.rust.target)(code), (apis.c.target)(code));
        }
        for _ in 0..256 {
            let code = rng.next_u32() as i32;
            assert_eq!((apis.rust.target)(code), (apis.c.target)(code));
        }
        for row in 1..=6 {
            covered[row] = true;
        }

        // From fresh state, value 5 contributes +17/+15 on alternating elements.
        // This is the first even length whose accumulator exceeds INT_MAX.
        let clamp_input = vec![5; 134_217_728];
        assert_eq!(
            (apis.rust.process_a_stream)(clamp_input.as_ptr(), clamp_input.len()),
            (apis.c.process_a_stream)(clamp_input.as_ptr(), clamp_input.len())
        );
        covered[13] = true;
        drop(clamp_input);

        for &value in &[-1000, -1, 0, 1, 7, 31, 255, 1000] {
            assert_eq!((apis.rust.call_a_once)(value), (apis.c.call_a_once)(value));
            assert_eq!((apis.rust.call_b_once)(value), (apis.c.call_b_once)(value));
        }
        for _ in 0..256 {
            let value = rng.small_i32();
            assert_eq!((apis.rust.call_a_once)(value), (apis.c.call_a_once)(value));
            assert_eq!((apis.rust.call_b_once)(value), (apis.c.call_b_once)(value));
        }
        for row in [7, 8, 9, 14, 15, 16] {
            covered[row] = true;
        }

        assert_eq!(
            (apis.rust.process_a_stream)(ptr::null(), 0),
            (apis.c.process_a_stream)(ptr::null(), 0)
        );
        assert_eq!(
            (apis.rust.process_b_stream)(ptr::null(), 0),
            (apis.c.process_b_stream)(ptr::null(), 0)
        );
        for len in 1..=8 {
            for _ in 0..64 {
                let values: Vec<_> = (0..len).map(|_| rng.small_i32()).collect();
                assert_eq!(
                    (apis.rust.process_a_stream)(values.as_ptr(), values.len()),
                    (apis.c.process_a_stream)(values.as_ptr(), values.len()),
                    "A stream mismatch for {values:?}"
                );
                assert_eq!(
                    (apis.rust.process_b_stream)(values.as_ptr(), values.len()),
                    (apis.c.process_b_stream)(values.as_ptr(), values.len()),
                    "B stream mismatch for {values:?}"
                );
            }
        }
        for row in [10, 11, 12, 17, 18, 19] {
            covered[row] = true;
        }

        let dirty = || IntVec {
            data: 1usize as *mut c_int,
            len: usize::MAX,
            cap: usize::MAX,
        };
        let mut c_vec = dirty();
        let mut rust_vec = dirty();
        (apis.c.iv_init)(&mut c_vec);
        (apis.rust.iv_init)(&mut rust_vec);
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        covered[20] = true;

        let mut c_push = dirty();
        let mut rust_push = dirty();
        (apis.c.iv_init)(&mut c_push);
        (apis.rust.iv_init)(&mut rust_push);
        assert_eq!(
            (apis.rust.iv_push)(&mut rust_push, 123),
            (apis.c.iv_push)(&mut c_push, 123)
        );
        assert_eq!(snapshot_vec(&rust_push), snapshot_vec(&c_push));
        assert_eq!(c_push.cap, 8);
        covered[27] = true;
        (apis.c.iv_free)(&mut c_push);
        (apis.rust.iv_free)(&mut rust_push);

        (apis.c.iv_free)(&mut c_vec);
        (apis.rust.iv_free)(&mut rust_vec);
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        covered[21] = true;

        assert_eq!(
            (apis.rust.iv_reserve)(&mut rust_vec, 0),
            (apis.c.iv_reserve)(&mut c_vec, 0)
        );
        covered[23] = true;
        assert_eq!(
            (apis.rust.iv_reserve)(&mut rust_vec, 1),
            (apis.c.iv_reserve)(&mut c_vec, 1)
        );
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        assert_eq!(c_vec.cap, 8);
        covered[24] = true;
        assert_eq!(
            (apis.rust.iv_reserve)(&mut rust_vec, 100),
            (apis.c.iv_reserve)(&mut c_vec, 100)
        );
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        assert_eq!(c_vec.cap, 128);
        covered[25] = true;

        for value in 0..130 {
            assert_eq!(
                (apis.rust.iv_push)(&mut rust_vec, value),
                (apis.c.iv_push)(&mut c_vec, value)
            );
            assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        }
        for row in [26, 28] {
            covered[row] = true;
        }

        let mut c_out = -1;
        let mut rust_out = -1;
        assert_eq!(
            (apis.rust.iv_pop)(&mut rust_vec, &mut rust_out),
            (apis.c.iv_pop)(&mut c_vec, &mut c_out)
        );
        assert_eq!(rust_out, c_out);
        covered[29] = true;
        assert_eq!(
            (apis.rust.iv_pop)(&mut rust_vec, ptr::null_mut()),
            (apis.c.iv_pop)(&mut c_vec, ptr::null_mut())
        );
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        covered[30] = true;
        assert_eq!(
            (apis.rust.iv_peek)(&rust_vec, -99),
            (apis.c.iv_peek)(&c_vec, -99)
        );
        covered[32] = true;

        (apis.c.iv_free)(&mut c_vec);
        (apis.rust.iv_free)(&mut rust_vec);
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        covered[22] = true;
        assert_eq!(
            (apis.rust.iv_peek)(&rust_vec, -99),
            (apis.c.iv_peek)(&c_vec, -99)
        );
        covered[31] = true;

        let mut c_program = Program {
            code: 1usize as *const c_int,
            n: usize::MAX,
            ip: usize::MAX,
        };
        let mut rust_program = Program {
            code: 1usize as *const c_int,
            n: usize::MAX,
            ip: usize::MAX,
        };
        (apis.c.prog_init)(&mut c_program, ptr::null(), 0);
        (apis.rust.prog_init)(&mut rust_program, ptr::null(), 0);
        assert_eq!(
            (c_program.code, c_program.n, c_program.ip),
            (rust_program.code, rust_program.n, rust_program.ip)
        );
        covered[33] = true;
        let program_code = [11, 22, 33];
        (apis.c.prog_init)(&mut c_program, program_code.as_ptr(), program_code.len());
        (apis.rust.prog_init)(&mut rust_program, program_code.as_ptr(), program_code.len());
        covered[34] = true;
        for expected in program_code {
            let mut c_value = -1;
            let mut rust_value = -1;
            assert_eq!(
                (apis.rust.prog_fetch)(&mut rust_program, &mut rust_value),
                (apis.c.prog_fetch)(&mut c_program, &mut c_value)
            );
            assert_eq!((rust_value, rust_program.ip), (expected, c_program.ip));
        }
        covered[35] = true;

        let mut c_vm = new_vm(&apis.c);
        let mut rust_vm = new_vm(&apis.rust);
        assert_eq!(snapshot_vm(&rust_vm), snapshot_vm(&c_vm));
        covered[36] = true;
        assert_eq!(
            print_vm(&apis.rust, &rust_vm, b"EMPTY:\0"),
            print_vm(&apis.c, &c_vm, b"EMPTY:\0")
        );
        covered[41] = true;
        (apis.c.vm_free)(&mut c_vm);
        (apis.rust.vm_free)(&mut rust_vm);
        assert_eq!(snapshot_vm(&rust_vm), snapshot_vm(&c_vm));
        covered[37] = true;

        c_vm = new_vm(&apis.c);
        rust_vm = new_vm(&apis.rust);
        for value in -20..=20 {
            (apis.c.vm_trace)(&mut c_vm, value);
            (apis.rust.vm_trace)(&mut rust_vm, value);
            assert_eq!(snapshot_vm(&rust_vm), snapshot_vm(&c_vm));
        }
        for row in [39, 40] {
            covered[row] = true;
        }
        push_values(&apis.c, &mut c_vm.stack, &[12, 34]);
        push_values(&apis.rust, &mut rust_vm.stack, &[12, 34]);
        c_vm.steps = 17;
        rust_vm.steps = 17;
        assert_eq!(
            print_vm(&apis.rust, &rust_vm, b"VM:\0"),
            print_vm(&apis.c, &c_vm, b"VM:\0")
        );
        covered[42] = true;
        (apis.c.vm_free)(&mut c_vm);
        (apis.rust.vm_free)(&mut rust_vm);
        assert_eq!(snapshot_vm(&rust_vm), snapshot_vm(&c_vm));
        covered[38] = true;

        assert_eq!(run_pair(&apis, 0, &[], &[]).0, 0);
        covered[43] = true;
        for _ in 0..64 {
            let a = rng.small_i32();
            let b = rng.small_i32();
            run_pair(&apis, 2, &[], &[0, a]);
            run_pair(&apis, 2, &[a, b], &[1]);
            run_pair(&apis, 2, &[a, b], &[2]);
            run_pair(&apis, 2, &[], &[3]);
            run_pair(&apis, 2, &[a], &[3]);
            run_pair(&apis, 2, &[a], &[4]);
        }
        for row in 44..=49 {
            covered[row] = true;
        }

        for impl_id in [0, 1, -7, 2, 99] {
            let mut trace_classes = BTreeSet::new();
            for x in -256..=512 {
                let (_, snapshot) = run_pair(&apis, impl_id, &[], &[0, x, 5]);
                trace_classes.insert(*snapshot.trace.values.last().unwrap());
            }
            if impl_id == 0 {
                assert_eq!(trace_classes, BTreeSet::from([6, 8, 9]));
            } else if impl_id == 1 {
                assert_eq!(trace_classes, BTreeSet::from([8, 9]));
            } else {
                assert_eq!(trace_classes, BTreeSet::from([5, 6, 7, 8]));
            }
        }
        for row in 50..=52 {
            covered[row] = true;
        }

        run_pair(&apis, 2, &[0], &[6, i32::MIN, 0, 7]);
        run_pair(&apis, 2, &[1], &[6, 0, 0, 7]);
        let (_, jumped) = run_pair(&apis, 2, &[1], &[6, 2, 0, 99, 10, 99]);
        assert_eq!(jumped.steps, 2);
        for row in 53..=55 {
            covered[row] = true;
        }

        run_pair(&apis, 2, &[], &[7, -1, 3]);
        run_pair(&apis, 2, &[], &[7, 0, 3]);
        run_pair(&apis, 2, &[], &[7, 5, 3]);
        let (_, inner_error) = run_pair(&apis, 2, &[], &[7, 5, 4, 10]);
        assert!(inner_error.trace.values.contains(&12));
        for row in 56..=58 {
            covered[row] = true;
        }

        for impl_id in [0, 1, -1, 2, 37] {
            for _ in 0..64 {
                run_pair(&apis, impl_id, &[rng.small_i32()], &[8]);
            }
        }
        for row in 59..=61 {
            covered[row] = true;
        }

        for impl_id in [0, 1, 2] {
            run_pair(&apis, impl_id, &[], &[9, 0]);
            run_pair(&apis, impl_id, &[1, 2], &[9, 2]);
            run_pair(&apis, impl_id, &[1, 2, 3], &[9, 2]);
            for _ in 0..64 {
                let values: Vec<_> = (0..8).map(|_| rng.small_i32()).collect();
                run_pair(&apis, impl_id, &values, &[9, 4]);
            }
        }
        for row in 62..=67 {
            covered[row] = true;
        }

        let (_, halted) = run_pair(&apis, 2, &[], &[10, 99, 99]);
        assert_eq!(halted.steps, 1);
        covered[68] = true;
        for impl_id in [0, 1, 2] {
            for _ in 0..128 {
                let a = rng.small_i32() % 100;
                let b = rng.small_i32() % 100;
                run_pair(&apis, impl_id, &[], &[0, a, 0, b, 1, 3, 8, 4, 10, 99]);
            }
        }
        covered[69] = true;

        let c_help = capture_main(&apis.c, &["driver-c", "--help", "ignored"], b"");
        let rust_help = capture_main(&apis.rust, &["driver-c", "--help", "ignored"], b"");
        assert_eq!(rust_help, c_help);
        covered[70] = true;

        let argv_cases = [
            vec!["driver", "0", "7", "3", "10"],
            vec!["driver", "bad", "0", "5", "10"],
            vec!["driver", "", "10"],
            vec!["driver", "999999999999999999999999", "10"],
        ];
        for args in argv_cases {
            assert_eq!(
                capture_main(&apis.rust, &args, b""),
                capture_main(&apis.c, &args, b""),
                "main argv mismatch for {args:?}"
            );
        }
        covered[71] = true;

        let stdin_cases: &[&[u8]] = &[b"0 7 3 10\n", b"bad\t0 5 10\r\n", b"0\n5\n10\n"];
        for input in stdin_cases {
            let args = ["driver", "--stdin"];
            assert_eq!(
                capture_main(&apis.rust, &args, input),
                capture_main(&apis.c, &args, input),
                "main stdin mismatch for {input:?}"
            );
        }
        covered[72] = true;

        let mut chunked_input = vec![b'1'; 5000];
        chunked_input.extend_from_slice(b" 10\n0\0 99\n");
        let args = ["driver", "--stdin"];
        assert_eq!(
            capture_main(&apis.rust, &args, &chunked_input),
            capture_main(&apis.c, &args, &chunked_input)
        );
        covered[73] = true;

        let missing: Vec<_> = (1..=73).filter(|&row| !covered[row]).collect();
        assert!(missing.is_empty(), "uncovered CONFIGS.md rows: {missing:?}");
    }
}

#[test]
fn error_surface() {
    let _serial = SERIAL.lock().unwrap();
    unsafe {
        let apis = Apis::load();
        let mut covered = [false; 22];

        let mut c_vec = IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        };
        let mut rust_vec = IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        };
        assert_eq!(
            (apis.rust.iv_reserve)(&mut rust_vec, usize::MAX),
            (apis.c.iv_reserve)(&mut c_vec, usize::MAX)
        );
        assert!(!(apis.c.iv_reserve)(&mut c_vec, usize::MAX));
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        covered[1] = true;

        let allocation_failure_need = 1usize << 61;
        assert_eq!(
            (apis.rust.iv_reserve)(&mut rust_vec, allocation_failure_need),
            (apis.c.iv_reserve)(&mut c_vec, allocation_failure_need)
        );
        assert!(!(apis.c.iv_reserve)(&mut c_vec, allocation_failure_need));
        assert_eq!(snapshot_vec(&rust_vec), snapshot_vec(&c_vec));
        covered[2] = true;

        let full_cap = (1usize << 60) + 1;
        let mut c_full = IntVec {
            data: ptr::null_mut(),
            len: full_cap,
            cap: full_cap,
        };
        let mut rust_full = IntVec {
            data: ptr::null_mut(),
            len: full_cap,
            cap: full_cap,
        };
        assert_eq!(
            (apis.rust.iv_push)(&mut rust_full, 123),
            (apis.c.iv_push)(&mut c_full, 123)
        );
        assert!(!(apis.c.iv_push)(&mut c_full, 123));
        assert_eq!(
            (rust_full.data, rust_full.len, rust_full.cap),
            (c_full.data, c_full.len, c_full.cap)
        );
        covered[3] = true;

        let mut c_out = 0x1234_5678;
        let mut rust_out = 0x1234_5678;
        assert_eq!(
            (apis.rust.iv_pop)(&mut rust_vec, &mut rust_out),
            (apis.c.iv_pop)(&mut c_vec, &mut c_out)
        );
        assert!(!(apis.c.iv_pop)(&mut c_vec, &mut c_out));
        assert_eq!(rust_out, c_out);
        covered[4] = true;

        let code = [7];
        let mut c_program = Program {
            code: code.as_ptr(),
            n: code.len(),
            ip: code.len(),
        };
        let mut rust_program = Program {
            code: code.as_ptr(),
            n: code.len(),
            ip: code.len(),
        };
        c_out = 0x1234_5678;
        rust_out = 0x1234_5678;
        assert_eq!(
            (apis.rust.prog_fetch)(&mut rust_program, &mut rust_out),
            (apis.c.prog_fetch)(&mut c_program, &mut c_out)
        );
        assert!(!(apis.c.prog_fetch)(&mut c_program, &mut c_out));
        assert_eq!((rust_out, rust_program.ip), (c_out, c_program.ip));
        covered[5] = true;

        let error_cases: &[(usize, c_int, &[c_int], &[c_int], c_int)] = &[
            (6, 2, &[], &[0], 1),
            (7, 2, &[], &[1], 2),
            (8, 2, &[123], &[1], 2),
            (9, 2, &[], &[2], 3),
            (10, 2, &[123], &[2], 3),
            (11, 2, &[], &[4], 4),
            (12, 2, &[], &[6], 5),
            (13, 2, &[], &[6, 0], 6),
            (14, 2, &[1], &[6, -1, 10], 7),
            (15, 2, &[], &[7], 8),
            (16, 2, &[], &[7, 0], 9),
            (17, 2, &[], &[9], 10),
            (18, 2, &[], &[9, -1], 11),
            (19, 2, &[], &[9, 1], 11),
            (20, 2, &[], &[11], 99),
        ];
        for &(row, impl_id, initial, program, expected) in error_cases {
            let (result, _) = run_pair(&apis, impl_id, initial, program);
            assert_eq!(result, expected, "ERRORS.md row {row}");
            covered[row] = true;
        }

        // Exercise both sides of the jump-length boundary and several unknown opcodes.
        for k in [1, 2, 17, i32::MAX] {
            let (result, _) = run_pair(&apis, 2, &[1], &[6, k]);
            assert_eq!(result, 7);
        }
        for opcode in [i32::MIN, -1, 11, 12, i32::MAX] {
            let (result, _) = run_pair(&apis, 2, &[], &[opcode]);
            assert_eq!(result, 99);
        }

        let args = ["driver"];
        let c_main = capture_main(&apis.c, &args, b"");
        let rust_main = capture_main(&apis.rust, &args, b"");
        assert_eq!(rust_main, c_main);
        assert_eq!(c_main.result, 2);
        covered[21] = true;

        let missing: Vec<_> = (1..=21).filter(|&row| !covered[row]).collect();
        assert!(missing.is_empty(), "uncovered ERRORS.md rows: {missing:?}");
    }
}

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

unsafe fn child_status(action: impl FnOnce()) -> c_int {
    let pid = fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        action();
        _exit(0);
    }
    let mut status = 0;
    assert_eq!(waitpid(pid, &mut status, 0), pid);
    status
}

unsafe fn assert_same_process_outcome(
    name: &str,
    c_action: impl FnOnce(),
    rust_action: impl FnOnce(),
) {
    let c_status = child_status(c_action);
    let rust_status = child_status(rust_action);
    assert_ne!(c_status, 0, "{name}: C unexpectedly succeeded");
    assert_eq!(
        rust_status, c_status,
        "{name}: process status differs (signal is status & 0x7f)"
    );
}

#[test]
fn generic_ffi_boundaries() {
    let _serial = SERIAL.lock().unwrap();
    unsafe {
        let apis = Apis::load();

        // These null combinations are explicitly valid because the C loops/fetch stop first.
        assert_eq!(
            (apis.rust.process_a_stream)(ptr::null(), 0),
            (apis.c.process_a_stream)(ptr::null(), 0)
        );
        assert_eq!(
            (apis.rust.process_b_stream)(ptr::null(), 0),
            (apis.c.process_b_stream)(ptr::null(), 0)
        );
        assert_eq!(
            (apis.rust.run_engine)(2, ptr::null(), 0, ptr::null_mut()),
            (apis.c.run_engine)(2, ptr::null(), 0, ptr::null_mut())
        );

        assert_same_process_outcome(
            "iv_init(null)",
            || (apis.c.iv_init)(ptr::null_mut()),
            || (apis.rust.iv_init)(ptr::null_mut()),
        );
        assert_same_process_outcome(
            "iv_free(null)",
            || (apis.c.iv_free)(ptr::null_mut()),
            || (apis.rust.iv_free)(ptr::null_mut()),
        );
        assert_same_process_outcome(
            "iv_reserve(null)",
            || {
                (apis.c.iv_reserve)(ptr::null_mut(), 1);
            },
            || {
                (apis.rust.iv_reserve)(ptr::null_mut(), 1);
            },
        );
        assert_same_process_outcome(
            "iv_push(null)",
            || {
                (apis.c.iv_push)(ptr::null_mut(), 1);
            },
            || {
                (apis.rust.iv_push)(ptr::null_mut(), 1);
            },
        );
        assert_same_process_outcome(
            "iv_pop(null)",
            || {
                (apis.c.iv_pop)(ptr::null_mut(), ptr::null_mut());
            },
            || {
                (apis.rust.iv_pop)(ptr::null_mut(), ptr::null_mut());
            },
        );
        assert_same_process_outcome(
            "iv_peek(null)",
            || {
                (apis.c.iv_peek)(ptr::null(), 0);
            },
            || {
                (apis.rust.iv_peek)(ptr::null(), 0);
            },
        );
        assert_same_process_outcome(
            "prog_init(null)",
            || (apis.c.prog_init)(ptr::null_mut(), ptr::null(), 0),
            || (apis.rust.prog_init)(ptr::null_mut(), ptr::null(), 0),
        );
        assert_same_process_outcome(
            "prog_fetch(null)",
            || {
                (apis.c.prog_fetch)(ptr::null_mut(), ptr::null_mut());
            },
            || {
                (apis.rust.prog_fetch)(ptr::null_mut(), ptr::null_mut());
            },
        );
        assert_same_process_outcome(
            "prog_fetch(null code)",
            || {
                let mut program = Program {
                    code: ptr::null(),
                    n: 1,
                    ip: 0,
                };
                let mut output = 0;
                (apis.c.prog_fetch)(&mut program, &mut output);
            },
            || {
                let mut program = Program {
                    code: ptr::null(),
                    n: 1,
                    ip: 0,
                };
                let mut output = 0;
                (apis.rust.prog_fetch)(&mut program, &mut output);
            },
        );
        assert_same_process_outcome(
            "prog_fetch(null output)",
            || {
                let code = [1];
                let mut program = Program {
                    code: code.as_ptr(),
                    n: 1,
                    ip: 0,
                };
                (apis.c.prog_fetch)(&mut program, ptr::null_mut());
            },
            || {
                let code = [1];
                let mut program = Program {
                    code: code.as_ptr(),
                    n: 1,
                    ip: 0,
                };
                (apis.rust.prog_fetch)(&mut program, ptr::null_mut());
            },
        );
        assert_same_process_outcome(
            "vm_init(null)",
            || (apis.c.vm_init)(ptr::null_mut()),
            || (apis.rust.vm_init)(ptr::null_mut()),
        );
        assert_same_process_outcome(
            "vm_free(null)",
            || (apis.c.vm_free)(ptr::null_mut()),
            || (apis.rust.vm_free)(ptr::null_mut()),
        );
        assert_same_process_outcome(
            "vm_trace(null)",
            || (apis.c.vm_trace)(ptr::null_mut(), 1),
            || (apis.rust.vm_trace)(ptr::null_mut(), 1),
        );
        assert_same_process_outcome(
            "vm_print(null FILE)",
            || {
                let vm = new_vm(&apis.c);
                (apis.c.vm_print)(ptr::null_mut(), b"X:\0".as_ptr().cast(), &vm);
            },
            || {
                let vm = new_vm(&apis.rust);
                (apis.rust.vm_print)(ptr::null_mut(), b"X:\0".as_ptr().cast(), &vm);
            },
        );
        assert_same_process_outcome(
            "vm_print(null VM)",
            || {
                (apis.c.vm_print)(ptr::null_mut(), b"X:\0".as_ptr().cast(), ptr::null());
            },
            || {
                (apis.rust.vm_print)(ptr::null_mut(), b"X:\0".as_ptr().cast(), ptr::null());
            },
        );
        assert_same_process_outcome(
            "process_a_stream(null, 1)",
            || {
                (apis.c.process_a_stream)(ptr::null(), 1);
            },
            || {
                (apis.rust.process_a_stream)(ptr::null(), 1);
            },
        );
        assert_same_process_outcome(
            "process_b_stream(null, 1)",
            || {
                (apis.c.process_b_stream)(ptr::null(), 1);
            },
            || {
                (apis.rust.process_b_stream)(ptr::null(), 1);
            },
        );
        let one_value = [1];
        assert_same_process_outcome(
            "process_a_stream(oversized length)",
            || {
                (apis.c.process_a_stream)(one_value.as_ptr(), usize::MAX);
            },
            || {
                (apis.rust.process_a_stream)(one_value.as_ptr(), usize::MAX);
            },
        );
        assert_same_process_outcome(
            "process_b_stream(oversized length)",
            || {
                (apis.c.process_b_stream)(one_value.as_ptr(), usize::MAX);
            },
            || {
                (apis.rust.process_b_stream)(one_value.as_ptr(), usize::MAX);
            },
        );
        assert_same_process_outcome(
            "run_engine(null code, nonzero length)",
            || {
                let mut vm = new_vm(&apis.c);
                (apis.c.run_engine)(2, ptr::null(), 1, &mut vm);
            },
            || {
                let mut vm = new_vm(&apis.rust);
                (apis.rust.run_engine)(2, ptr::null(), 1, &mut vm);
            },
        );
        let one_opcode = [3];
        assert_same_process_outcome(
            "run_engine(null VM)",
            || {
                (apis.c.run_engine)(2, one_opcode.as_ptr(), 1, ptr::null_mut());
            },
            || {
                (apis.rust.run_engine)(2, one_opcode.as_ptr(), 1, ptr::null_mut());
            },
        );
    }
}
