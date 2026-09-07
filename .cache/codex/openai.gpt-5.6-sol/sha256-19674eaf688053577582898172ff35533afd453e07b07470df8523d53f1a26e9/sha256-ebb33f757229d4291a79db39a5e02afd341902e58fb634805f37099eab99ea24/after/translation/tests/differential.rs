use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::slice;
use std::sync::Mutex;

#[repr(C)]
union TypeConfusion {
    int_val: c_int,
    float_val: f32,
    uint_val: c_uint,
    bytes: [c_char; 4],
}

#[repr(C)]
struct ProcessState {
    flags: c_uint,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

type CreateState = unsafe extern "C" fn(c_int, c_int) -> *mut ProcessState;
type DestroyState = unsafe extern "C" fn(*mut ProcessState);
type ProcessBuffer = unsafe extern "C" fn(*mut ProcessState, c_char) -> c_int;
type UpdateFlags = unsafe extern "C" fn(*mut ProcessState, c_int);
type ConfuseTypes = unsafe extern "C" fn(*mut ProcessState, c_int) -> c_int;
type Confusion = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

struct Api {
    _library: Library,
    create_state: CreateState,
    destroy_state: DestroyState,
    process_buffer: ProcessBuffer,
    update_flags: UpdateFlags,
    confuse_types: ConfuseTypes,
    confusion: Confusion,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));

        unsafe {
            let create_state = *library.get::<CreateState>(b"create_state\0").unwrap();
            let destroy_state = *library.get::<DestroyState>(b"destroy_state\0").unwrap();
            let process_buffer = *library.get::<ProcessBuffer>(b"process_buffer\0").unwrap();
            let update_flags = *library.get::<UpdateFlags>(b"update_flags\0").unwrap();
            let confuse_types = *library.get::<ConfuseTypes>(b"confuse_types\0").unwrap();
            let confusion = *library.get::<Confusion>(b"confusion\0").unwrap();

            Self {
                _library: library,
                create_state,
                destroy_state,
                process_buffer,
                update_flags,
                confuse_types,
                confusion,
            }
        }
    }
}

unsafe extern "C" {
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn fflush(stream: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
}

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn capture_stdout<R>(call: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().unwrap();

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);

        let mut fds = [-1; 2];
        assert_eq!(pipe(fds.as_mut_ptr()), 0);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(fds[1], 1), 1);
        assert_eq!(close(fds[1]), 0);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = read(fds[0], chunk.as_mut_ptr().cast(), chunk.len());
            assert!(count >= 0);
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        assert_eq!(close(fds[0]), 0);

        (result, output)
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", build_dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("so")
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| name.starts_with("libharvest-work-"))
        })
        .collect();
    libraries.sort();
    assert_eq!(libraries.len(), 1, "expected one C shared object");
    libraries.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    let path = manifest_dir().join("target/release/libconfusion_lib.so");
    assert!(
        path.is_file(),
        "missing {}; run cargo build --release first",
        path.display()
    );
    path
}

fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StateSnapshot {
    flags: u32,
    data_bits: u32,
    capacity: i32,
    buffer_is_null: bool,
    initialized_buffer: Vec<u8>,
}

unsafe fn snapshot(state: *mut ProcessState) -> StateSnapshot {
    assert!(!state.is_null());
    unsafe {
        let capacity = ptr::addr_of!((*state).capacity).read();
        let buffer = ptr::addr_of!((*state).buffer).read();
        let initialized_buffer = if buffer.is_null() || capacity <= 0 {
            Vec::new()
        } else {
            let bytes = slice::from_raw_parts(buffer.cast::<u8>(), capacity as usize);
            match bytes.iter().position(|byte| *byte == 0) {
                Some(end) => bytes[..=end].to_vec(),
                None => bytes.to_vec(),
            }
        };

        StateSnapshot {
            flags: ptr::addr_of!((*state).flags).read(),
            data_bits: ptr::addr_of!((*state).data).cast::<u32>().read(),
            capacity,
            buffer_is_null: buffer.is_null(),
            initialized_buffer,
        }
    }
}

unsafe fn set_data_bits(state: *mut ProcessState, bits: u32) {
    unsafe {
        ptr::addr_of_mut!((*state).data).cast::<u32>().write(bits);
    }
}

unsafe fn set_flags(state: *mut ProcessState, flags: u32) {
    unsafe {
        ptr::addr_of_mut!((*state).flags).write(flags);
    }
}

unsafe fn install_buffer(state: *mut ProcessState, bytes: &[u8]) {
    assert!(!bytes.is_empty());
    assert_eq!(bytes.last(), Some(&0));
    unsafe {
        let capacity = ptr::addr_of!((*state).capacity).read();
        assert!(capacity as usize >= bytes.len());
        let buffer = ptr::addr_of!((*state).buffer).read();
        assert!(!buffer.is_null());
        ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.cast::<u8>(), bytes.len());
    }
}

fn assert_pair<T: Eq + Debug>(
    label: &str,
    c_result: T,
    c_output: Vec<u8>,
    rust_result: T,
    rust_output: Vec<u8>,
) {
    assert_eq!(c_result, rust_result, "{label}: return/state mismatch");
    assert_eq!(c_output, rust_output, "{label}: stdout mismatch");
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn below(&mut self, upper: u32) -> u32 {
        assert!(upper > 0);
        self.next_u32() % upper
    }
}

fn boundary_i32(trial: usize, rng: &mut Rng) -> i32 {
    const VALUES: [i32; 9] = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000_000,
        -1,
        0,
        1,
        1_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];
    VALUES.get(trial).copied().unwrap_or_else(|| rng.next_i32())
}

unsafe fn destroy_pair(
    c: &Api,
    rust: &Api,
    c_state: *mut ProcessState,
    rust_state: *mut ProcessState,
) {
    unsafe {
        (c.destroy_state)(c_state);
        (rust.destroy_state)(rust_state);
    }
}

fn compare_create_case(c: &Api, rust: &Api, initial: i32, capacity: i32, label: &str) {
    let (c_state, c_output) = capture_stdout(|| unsafe { (c.create_state)(initial, capacity) });
    let (rust_state, rust_output) =
        capture_stdout(|| unsafe { (rust.create_state)(initial, capacity) });

    assert_eq!(
        c_state.is_null(),
        rust_state.is_null(),
        "{label}: allocation result differs"
    );
    assert_eq!(c_output, rust_output, "{label}: create stdout differs");

    if !c_state.is_null() {
        let c_snapshot = unsafe { snapshot(c_state) };
        let rust_snapshot = unsafe { snapshot(rust_state) };
        assert_eq!(c_snapshot, rust_snapshot, "{label}: created state differs");

        let ((), c_destroy_output) = capture_stdout(|| unsafe { (c.destroy_state)(c_state) });
        let ((), rust_destroy_output) =
            capture_stdout(|| unsafe { (rust.destroy_state)(rust_state) });
        assert_eq!(
            c_destroy_output, rust_destroy_output,
            "{label}: destroy stdout differs"
        );
    }
}

#[test]
fn phase_b_create_and_destroy_rows_c01_to_c07() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x39f4_b32a_88d1_0297);

    for trial in 0..128 {
        let initial = boundary_i32(trial, &mut rng);
        compare_create_case(&c, &rust, initial, 0, &format!("C01 trial {trial}"));
    }

    for trial in 0..128 {
        let initial = boundary_i32(trial, &mut rng);
        compare_create_case(&c, &rust, initial, 1, &format!("C02 trial {trial}"));
    }

    for trial in 0..128 {
        let initial = boundary_i32(trial, &mut rng);
        let rendered_length = format!("State:{initial}:Mode:3").len() as u32;
        let capacity = 2 + rng.below(rendered_length - 2);
        compare_create_case(
            &c,
            &rust,
            initial,
            capacity as i32,
            &format!("C03 trial {trial}"),
        );
    }

    for trial in 0..128 {
        let initial = boundary_i32(trial, &mut rng);
        let rendered_length = format!("State:{initial}:Mode:3").len() as i32;
        compare_create_case(
            &c,
            &rust,
            initial,
            rendered_length,
            &format!("C04 trial {trial}"),
        );
    }

    for trial in 0..128 {
        let initial = boundary_i32(trial, &mut rng);
        let rendered_length = format!("State:{initial}:Mode:3").len() as i32;
        compare_create_case(
            &c,
            &rust,
            initial,
            rendered_length + 1,
            &format!("C05/C07 trial {trial}"),
        );
    }

    for trial in 0..128 {
        let initial = boundary_i32(trial, &mut rng);
        let rendered_length = format!("State:{initial}:Mode:3").len() as i32;
        let capacity = rendered_length + 2 + rng.below(32) as i32;
        compare_create_case(&c, &rust, initial, capacity, &format!("C06 trial {trial}"));
    }
}

fn run_process_case(c: &Api, rust: &Api, bytes: &[u8], target: i8, label: &str) {
    let capacity = bytes.len().max(1) as i32;
    let c_state = unsafe { (c.create_state)(12345, capacity) };
    let rust_state = unsafe { (rust.create_state)(12345, capacity) };
    assert!(
        !c_state.is_null() && !rust_state.is_null(),
        "{label}: setup"
    );

    unsafe {
        install_buffer(c_state, bytes);
        install_buffer(rust_state, bytes);
    }

    let c_before = unsafe { snapshot(c_state) };
    let rust_before = unsafe { snapshot(rust_state) };
    assert_eq!(c_before, rust_before, "{label}: setup state differs");

    let (c_result, c_output) =
        capture_stdout(|| unsafe { (c.process_buffer)(c_state, target as c_char) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.process_buffer)(rust_state, target as c_char) });
    assert_pair(label, c_result, c_output, rust_result, rust_output);

    let c_after = unsafe { snapshot(c_state) };
    let rust_after = unsafe { snapshot(rust_state) };
    assert_eq!(c_after, rust_after, "{label}: final state differs");
    assert_eq!(c_before, c_after, "{label}: C unexpectedly mutated state");
    assert_eq!(
        rust_before, rust_after,
        "{label}: Rust unexpectedly mutated state"
    );

    unsafe { destroy_pair(c, rust, c_state, rust_state) };
}

fn random_nonzero_byte(rng: &mut Rng) -> u8 {
    (rng.below(255) + 1) as u8
}

#[test]
fn phase_b_process_buffer_rows_c08_to_c12() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x8f87_e2c3_19b4_a5d6);

    for trial in 0..128 {
        run_process_case(
            &c,
            &rust,
            &[0],
            rng.next_u32() as i8,
            &format!("C08 {trial}"),
        );
    }

    for trial in 0..256 {
        let target = match trial % 3 {
            0 => 0_u8,
            1 => 0x80_u8.wrapping_add(rng.below(128) as u8),
            _ => random_nonzero_byte(&mut rng),
        };
        let length = 1 + rng.below(96) as usize;
        let mut bytes = Vec::with_capacity(length + 1);
        while bytes.len() < length {
            let byte = random_nonzero_byte(&mut rng);
            if byte != target {
                bytes.push(byte);
            }
        }
        bytes.push(0);
        run_process_case(&c, &rust, &bytes, target as i8, &format!("C09 {trial}"));
    }

    for trial in 0..256 {
        let target = random_nonzero_byte(&mut rng);
        let length = 1 + rng.below(96) as usize;
        let position = rng.below(length as u32) as usize;
        let mut bytes = Vec::with_capacity(length + 1);
        for index in 0..length {
            if index == position {
                bytes.push(target);
            } else {
                loop {
                    let byte = random_nonzero_byte(&mut rng);
                    if byte != target {
                        bytes.push(byte);
                        break;
                    }
                }
            }
        }
        bytes.push(0);
        run_process_case(&c, &rust, &bytes, target as i8, &format!("C10 {trial}"));
    }

    for trial in 0..256 {
        let target = random_nonzero_byte(&mut rng);
        let length = 2 + rng.below(95) as usize;
        let mut bytes = vec![target; length];
        for byte in &mut bytes {
            if rng.below(3) == 0 {
                *byte = random_nonzero_byte(&mut rng);
            }
        }
        bytes[0] = target;
        bytes[length - 1] = target;
        if length > 2 {
            bytes[length / 2] = target;
            bytes[length / 2 - 1] = target;
        }
        bytes.push(0);
        run_process_case(&c, &rust, &bytes, target as i8, &format!("C11 {trial}"));
    }

    for trial in 0..256 {
        let target = random_nonzero_byte(&mut rng);
        let prefix_length = rng.below(32) as usize;
        let suffix_length = 1 + rng.below(32) as usize;
        let mut bytes = Vec::with_capacity(prefix_length + suffix_length + 2);
        for _ in 0..prefix_length {
            let mut byte = random_nonzero_byte(&mut rng);
            if trial % 2 == 0 && bytes.is_empty() {
                byte = target;
            } else if byte == target {
                byte = target.wrapping_add(1).max(1);
            }
            bytes.push(byte);
        }
        bytes.push(0);
        bytes.extend(std::iter::repeat_n(target, suffix_length));
        bytes.push(0);
        run_process_case(&c, &rust, &bytes, target as i8, &format!("C12 {trial}"));
    }
}

fn run_update_case(
    c: &Api,
    rust: &Api,
    c_state: *mut ProcessState,
    rust_state: *mut ProcessState,
    initial_flags: u32,
    param: i32,
    label: &str,
) {
    unsafe {
        set_flags(c_state, initial_flags);
        set_flags(rust_state, initial_flags);
    }

    let ((), c_output) = capture_stdout(|| unsafe { (c.update_flags)(c_state, param) });
    let ((), rust_output) = capture_stdout(|| unsafe { (rust.update_flags)(rust_state, param) });
    let c_snapshot = unsafe { snapshot(c_state) };
    let rust_snapshot = unsafe { snapshot(rust_state) };
    assert_pair(label, c_snapshot, c_output, rust_snapshot, rust_output);
}

#[test]
fn phase_b_update_flags_rows_c13_and_c14() {
    const COUNTER_MASK: u32 = 0x1f << 3;

    let (c, rust) = load_apis();
    let c_state = unsafe { (c.create_state)(0x1234_5678, 128) };
    let rust_state = unsafe { (rust.create_state)(0x1234_5678, 128) };
    assert!(!c_state.is_null() && !rust_state.is_null());
    let mut rng = Rng::new(0x51f0_5e1d_a629_4403);

    for low_options in 0_u32..64 {
        for counter in 0_u32..31 {
            let initial_flags = (rng.next_u32() & !COUNTER_MASK) | ((counter << 3) & COUNTER_MASK);
            let param = ((rng.next_u32() & !0x3f) | low_options) as i32;
            run_update_case(
                &c,
                &rust,
                c_state,
                rust_state,
                initial_flags,
                param,
                &format!("C13 options={low_options} counter={counter}"),
            );
        }
    }

    for low_options in 0_u32..64 {
        for trial in 0..32 {
            let initial_flags = (rng.next_u32() & !COUNTER_MASK) | ((31 << 3) & COUNTER_MASK);
            let param = ((rng.next_u32() & !0x3f) | low_options) as i32;
            run_update_case(
                &c,
                &rust,
                c_state,
                rust_state,
                initial_flags,
                param,
                &format!("C14 options={low_options} trial={trial}"),
            );
        }
    }

    unsafe { destroy_pair(&c, &rust, c_state, rust_state) };
}

fn run_confuse_case(
    c: &Api,
    rust: &Api,
    c_state: *mut ProcessState,
    rust_state: *mut ProcessState,
    bits: u32,
    operation: i32,
    label: &str,
) {
    unsafe {
        set_data_bits(c_state, bits);
        set_data_bits(rust_state, bits);
    }

    let (c_result, c_output) = capture_stdout(|| unsafe { (c.confuse_types)(c_state, operation) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.confuse_types)(rust_state, operation) });
    assert_pair(label, c_result, c_output, rust_result, rust_output);

    let c_snapshot = unsafe { snapshot(c_state) };
    let rust_snapshot = unsafe { snapshot(rust_state) };
    assert_eq!(c_snapshot, rust_snapshot, "{label}: final state differs");
}

#[test]
fn phase_b_confuse_types_rows_c15_to_c18() {
    let (c, rust) = load_apis();
    let c_state = unsafe { (c.create_state)(0, 128) };
    let rust_state = unsafe { (rust.create_state)(0, 128) };
    assert!(!c_state.is_null() && !rust_state.is_null());
    let mut rng = Rng::new(0xb9dd_6a07_10c3_e25f);

    for trial in 0..256 {
        let bits = rng.next_u32();
        run_confuse_case(
            &c,
            &rust,
            c_state,
            rust_state,
            bits,
            0,
            &format!("C15 trial={trial} bits={bits:#010x}"),
        );
    }

    const FLOAT_EDGE_BITS: [u32; 24] = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f00_0000,
        0x3f80_0000,
        0x4000_0000,
        0x42c8_0000,
        0x4b00_0000,
        0x4f00_0000,
        0x7f7f_ffff,
        0x8000_0001,
        0x807f_ffff,
        0x8080_0000,
        0xbf00_0000,
        0xbf80_0000,
        0xc2c8_0000,
        0xcb00_0000,
        0xcf00_0000,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0001,
    ];
    for (trial, bits) in FLOAT_EDGE_BITS
        .into_iter()
        .chain((0..2048).map(|_| rng.next_u32()))
        .enumerate()
    {
        run_confuse_case(
            &c,
            &rust,
            c_state,
            rust_state,
            bits,
            1,
            &format!("C16 trial={trial} bits={bits:#010x}"),
        );
    }

    for low_byte in 0_u32..=255 {
        for trial in 0..4 {
            let bits = (rng.next_u32() & !0xff) | low_byte;
            run_confuse_case(
                &c,
                &rust,
                c_state,
                rust_state,
                bits,
                2,
                &format!("C17 byte={low_byte} trial={trial}"),
            );
        }
    }

    const BYTE_EDGE_BITS: [u32; 12] = [
        0x0000_0000,
        0x0101_0101,
        0x7f7f_7f7f,
        0x8080_8080,
        0xffff_ffff,
        0x0000_7f80,
        0x0000_807f,
        0x0000_ff01,
        0x0000_01ff,
        0x7fff_0080,
        0x8000_007f,
        0x1234_abcd,
    ];
    for (trial, bits) in BYTE_EDGE_BITS
        .into_iter()
        .chain((0..2048).map(|_| rng.next_u32()))
        .enumerate()
    {
        run_confuse_case(
            &c,
            &rust,
            c_state,
            rust_state,
            bits,
            3,
            &format!("C18 trial={trial} bits={bits:#010x}"),
        );
    }

    unsafe { destroy_pair(&c, &rust, c_state, rust_state) };
}

fn value_with_remainder(rng: &mut Rng, divisor: i32, remainder: i32) -> i32 {
    assert!(remainder.abs() < divisor);
    let quotient = rng.below(10_000_000) as i32;
    let value = if remainder < 0 {
        -(quotient * divisor + remainder.abs())
    } else if remainder > 0 {
        quotient * divisor + remainder
    } else if rng.next_u32() & 1 == 0 {
        quotient * divisor
    } else {
        -(quotient * divisor)
    };
    assert_eq!(value % divisor, remainder);
    value
}

#[test]
fn phase_b_confusion_rows_c19_to_c25() {
    let (c, rust) = load_apis();
    let mut rng = Rng::new(0x29c8_f304_aa73_195d);

    for param4_remainder in -3_i32..=3 {
        for low_options in 0_u32..64 {
            for param3_remainder in -9_i32..=9 {
                let ordinal = ((param4_remainder + 3) as usize * 64 * 19)
                    + (low_options as usize * 19)
                    + (param3_remainder + 9) as usize;
                let param1 = boundary_i32(ordinal % 23, &mut rng);
                let param2 = ((rng.next_u32() & !0x3f) | low_options) as i32;
                let param3 = value_with_remainder(&mut rng, 10, param3_remainder);
                let param4 = value_with_remainder(&mut rng, 4, param4_remainder);
                let label = format!(
                    "C{:02} options={low_options} p3rem={param3_remainder} p4rem={param4_remainder}",
                    22 + param4_remainder
                );

                let (c_result, c_output) =
                    capture_stdout(|| unsafe { (c.confusion)(param1, param2, param3, param4) });
                let (rust_result, rust_output) =
                    capture_stdout(|| unsafe { (rust.confusion)(param1, param2, param3, param4) });
                assert_pair(&label, c_result, c_output, rust_result, rust_output);
            }
        }
    }
}

#[test]
fn phase_c_nonallocation_error_rows_e03_to_e09() {
    let (c, rust) = load_apis();

    let ((), c_output) = capture_stdout(|| unsafe { (c.destroy_state)(ptr::null_mut()) });
    let ((), rust_output) = capture_stdout(|| unsafe { (rust.destroy_state)(ptr::null_mut()) });
    assert_pair("E03", (), c_output, (), rust_output);

    unsafe {
        let c_state = malloc(std::mem::size_of::<ProcessState>()).cast::<ProcessState>();
        let rust_state = malloc(std::mem::size_of::<ProcessState>()).cast::<ProcessState>();
        assert!(!c_state.is_null() && !rust_state.is_null());
        c_state.write(ProcessState {
            flags: 0xdead_beef,
            data: TypeConfusion {
                uint_val: 0x1234_5678,
            },
            buffer: ptr::null_mut(),
            capacity: 0,
        });
        rust_state.write(ProcessState {
            flags: 0xdead_beef,
            data: TypeConfusion {
                uint_val: 0x1234_5678,
            },
            buffer: ptr::null_mut(),
            capacity: 0,
        });
        let ((), c_output) = capture_stdout(|| (c.destroy_state)(c_state));
        let ((), rust_output) = capture_stdout(|| (rust.destroy_state)(rust_state));
        assert_pair("E04", (), c_output, (), rust_output);
    }

    for target in [i8::MIN, -1, 0, 1, i8::MAX] {
        let (c_result, c_output) =
            capture_stdout(|| unsafe { (c.process_buffer)(ptr::null_mut(), target) });
        let (rust_result, rust_output) =
            capture_stdout(|| unsafe { (rust.process_buffer)(ptr::null_mut(), target) });
        assert_pair("E05", c_result, c_output, rust_result, rust_output);
    }

    let mut c_null_buffer = ProcessState {
        flags: 0,
        data: TypeConfusion { uint_val: 0 },
        buffer: ptr::null_mut(),
        capacity: 0,
    };
    let mut rust_null_buffer = ProcessState {
        flags: 0,
        data: TypeConfusion { uint_val: 0 },
        buffer: ptr::null_mut(),
        capacity: 0,
    };
    for target in [i8::MIN, -1, 0, 1, i8::MAX] {
        let (c_result, c_output) =
            capture_stdout(|| unsafe { (c.process_buffer)(&mut c_null_buffer, target) });
        let (rust_result, rust_output) =
            capture_stdout(|| unsafe { (rust.process_buffer)(&mut rust_null_buffer, target) });
        assert_pair("E06", c_result, c_output, rust_result, rust_output);
    }

    let ((), c_output) = capture_stdout(|| unsafe { (c.update_flags)(ptr::null_mut(), i32::MIN) });
    let ((), rust_output) =
        capture_stdout(|| unsafe { (rust.update_flags)(ptr::null_mut(), i32::MIN) });
    assert_pair("E07", (), c_output, (), rust_output);

    let (c_result, c_output) = capture_stdout(|| unsafe { (c.confuse_types)(ptr::null_mut(), 0) });
    let (rust_result, rust_output) =
        capture_stdout(|| unsafe { (rust.confuse_types)(ptr::null_mut(), 0) });
    assert_pair("E08", c_result, c_output, rust_result, rust_output);

    let c_state = unsafe { (c.create_state)(0, 128) };
    let rust_state = unsafe { (rust.create_state)(0, 128) };
    assert!(!c_state.is_null() && !rust_state.is_null());
    let mut rng = Rng::new(0xd0f2_5bac_7761_390e);
    let mut invalid_operations = vec![i32::MIN, -100, -2, -1, 4, 5, 100, i32::MAX];
    for _ in 0..256 {
        let value = rng.next_i32();
        invalid_operations.push(if (0..=3).contains(&value) {
            value.wrapping_add(4)
        } else {
            value
        });
    }
    for operation in invalid_operations {
        let bits = rng.next_u32();
        unsafe {
            set_data_bits(c_state, bits);
            set_data_bits(rust_state, bits);
        }
        let c_before = unsafe { snapshot(c_state) };
        let rust_before = unsafe { snapshot(rust_state) };
        let (c_result, c_output) =
            capture_stdout(|| unsafe { (c.confuse_types)(c_state, operation) });
        let (rust_result, rust_output) =
            capture_stdout(|| unsafe { (rust.confuse_types)(rust_state, operation) });
        assert_pair("E09", c_result, c_output, rust_result, rust_output);
        assert_eq!(unsafe { snapshot(c_state) }, c_before);
        assert_eq!(unsafe { snapshot(rust_state) }, rust_before);
    }
    unsafe { destroy_pair(&c, &rust, c_state, rust_state) };

    for capacity in [-1, i32::MIN] {
        let (c_state, c_output) = capture_stdout(|| unsafe { (c.create_state)(123, capacity) });
        let (rust_state, rust_output) =
            capture_stdout(|| unsafe { (rust.create_state)(123, capacity) });
        assert_pair(
            "generic oversized capacity",
            c_state.is_null(),
            c_output,
            rust_state.is_null(),
            rust_output,
        );
        assert!(c_state.is_null() && rust_state.is_null());
    }
}

fn build_fail_alloc_library() -> PathBuf {
    let output_dir = manifest_dir().join("target/fail-alloc-test");
    fs::create_dir_all(&output_dir).unwrap();
    let output = output_dir.join("libfail_alloc.so");
    let source = manifest_dir().join("tests/fail_alloc.c");
    let status = Command::new("cc")
        .arg("-shared")
        .arg("-fPIC")
        .arg("-O2")
        .arg("-o")
        .arg(&output)
        .arg(&source)
        .arg("-ldl")
        .status()
        .unwrap_or_else(|error| panic!("failed to invoke cc: {error}"));
    assert!(status.success(), "failed to compile {}", source.display());
    assert!(output.is_file());
    output
}

#[test]
fn phase_c_allocation_failure_parent_rows_e01_e02_e10() {
    if env::var_os("CONFUSION_FAIL_ALLOC_CHILD").is_some() {
        return;
    }

    let interposer = build_fail_alloc_library();
    let preload = match env::var_os("LD_PRELOAD") {
        Some(existing) if !existing.is_empty() => {
            format!("{}:{}", interposer.display(), existing.to_string_lossy())
        }
        _ => interposer.display().to_string(),
    };
    let current_test = env::current_exe().unwrap();
    let output = Command::new(&current_test)
        .arg("--exact")
        .arg("phase_c_allocation_failure_child")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env("LD_PRELOAD", preload)
        .env("CONFUSION_FAIL_ALLOC_CHILD", "1")
        .output()
        .unwrap_or_else(|error| panic!("failed to run allocation child: {error}"));

    assert!(
        output.status.success(),
        "allocation child failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn phase_c_allocation_failure_child() {
    if env::var_os("CONFUSION_FAIL_ALLOC_CHILD").is_none() {
        return;
    }

    type ConfigureFailAlloc = unsafe extern "C" fn(usize, c_int, *const c_char);

    let (c, rust) = load_apis();
    let process = libloading::os::unix::Library::this();
    let configure = unsafe {
        *process
            .get::<ConfigureFailAlloc>(b"configure_fail_alloc\0")
            .expect("allocation interposer was not preloaded")
    };
    let state_size = std::mem::size_of::<ProcessState>();
    let c_module = c_library_path();
    let c_module = c_module.file_name().unwrap().to_string_lossy();
    let c_module = std::ffi::CString::new(c_module.as_bytes()).unwrap();
    let rust_module = std::ffi::CString::new("libconfusion_lib.so").unwrap();

    let (c_state, c_output) = capture_stdout(|| unsafe {
        configure(state_size, 1, c_module.as_ptr());
        (c.create_state)(17, 128)
    });
    let (rust_state, rust_output) = capture_stdout(|| unsafe {
        configure(state_size, 1, rust_module.as_ptr());
        (rust.create_state)(17, 128)
    });
    assert_pair(
        "E01",
        c_state.is_null(),
        c_output,
        rust_state.is_null(),
        rust_output,
    );
    assert!(c_state.is_null() && rust_state.is_null());

    let (c_state, c_output) = capture_stdout(|| unsafe {
        configure(128, 1, c_module.as_ptr());
        (c.create_state)(17, 128)
    });
    let (rust_state, rust_output) = capture_stdout(|| unsafe {
        configure(128, 1, rust_module.as_ptr());
        (rust.create_state)(17, 128)
    });
    assert_pair(
        "E02",
        c_state.is_null(),
        c_output,
        rust_state.is_null(),
        rust_output,
    );
    assert!(c_state.is_null() && rust_state.is_null());

    let arguments = (i32::MIN, -1, -9, 3);
    let (c_result, c_output) = capture_stdout(|| unsafe {
        configure(state_size, 1, c_module.as_ptr());
        (c.confusion)(arguments.0, arguments.1, arguments.2, arguments.3)
    });
    let (rust_result, rust_output) = capture_stdout(|| unsafe {
        configure(state_size, 1, rust_module.as_ptr());
        (rust.confusion)(arguments.0, arguments.1, arguments.2, arguments.3)
    });
    assert_pair("E10", c_result, c_output, rust_result, rust_output);
    assert_eq!(c_result, -1);
}
