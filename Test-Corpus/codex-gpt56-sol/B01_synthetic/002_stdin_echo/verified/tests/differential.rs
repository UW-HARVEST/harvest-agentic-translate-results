use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type EntryPoint = unsafe extern "C" fn() -> c_int;

static PROCESS_IO_LOCK: Mutex<()> = Mutex::new(());

extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;

    fn clearerr(stream: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct SavedFd {
    target: c_int,
    saved: c_int,
}

impl SavedFd {
    fn redirect(target: c_int, replacement: c_int) -> Self {
        unsafe {
            let saved = dup(target);
            assert!(saved >= 0, "dup({target}) failed");
            assert_eq!(
                dup2(replacement, target),
                target,
                "dup2({replacement}, {target}) failed"
            );
            Self { target, saved }
        }
    }
}

impl Drop for SavedFd {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(
                dup2(self.saved, self.target),
                self.target,
                "failed to restore fd {}",
                self.target
            );
            assert_eq!(close(self.saved), 0, "failed to close saved fd");
        }
    }
}

fn rust_library_path() -> PathBuf {
    let profile_dir = std::env::current_exe()
        .expect("current test executable")
        .parent()
        .and_then(Path::parent)
        .expect("Cargo profile directory")
        .to_path_buf();
    let direct = profile_dir.join("libdriver.so");
    if direct.is_file() {
        return direct;
    }

    fs::read_dir(profile_dir.join("deps"))
        .expect("read Cargo deps directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libdriver") && name.ends_with(".so"))
        })
        .expect("Cargo did not build the driver cdylib")
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

fn invoke(entry: EntryPoint, input: &[u8]) -> (c_int, Vec<u8>) {
    let (mut input_writer, input_reader) = UnixStream::pair().expect("input socket pair");
    input_writer.write_all(input).expect("write test input");
    input_writer
        .shutdown(std::net::Shutdown::Write)
        .expect("finish test input");
    drop(input_writer);

    let (mut output_reader, output_writer) = UnixStream::pair().expect("output socket pair");

    unsafe {
        fflush(stdout);
    }
    let saved_stdin = SavedFd::redirect(0, input_reader.as_raw_fd());
    let saved_stdout = SavedFd::redirect(1, output_writer.as_raw_fd());
    unsafe {
        clearerr(stdin);
        clearerr(stdout);
    }

    let status = unsafe { entry() };

    unsafe {
        fflush(stdout);
    }
    drop(saved_stdout);
    drop(output_writer);
    drop(saved_stdin);
    unsafe {
        clearerr(stdin);
        clearerr(stdout);
    }

    let mut output = Vec::new();
    output_reader
        .read_to_end(&mut output)
        .expect("read captured output");
    (status, output)
}

fn invoke_read_error(entry: EntryPoint) -> c_int {
    unsafe {
        clearerr(stdin);
        let saved_stdin = dup(0);
        assert!(saved_stdin >= 0, "dup(stdin) failed");
        assert_eq!(close(0), 0, "close(stdin) failed");
        let status = entry();
        assert_eq!(dup2(saved_stdin, 0), 0, "restore stdin failed");
        assert_eq!(close(saved_stdin), 0, "close saved stdin failed");
        clearerr(stdin);
        status
    }
}

fn invoke_write_error(entry: EntryPoint, input: &[u8]) -> (c_int, Vec<u8>) {
    let (mut input_writer, mut input_reader) = UnixStream::pair().expect("input socket pair");
    input_writer.write_all(input).expect("write test input");
    input_writer
        .shutdown(std::net::Shutdown::Write)
        .expect("finish test input");
    drop(input_writer);

    unsafe {
        fflush(stdout);
    }
    let saved_stdin = SavedFd::redirect(0, input_reader.as_raw_fd());
    let status = unsafe {
        clearerr(stdin);
        clearerr(stdout);
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(close(1), 0, "close(stdout) failed");

        let status = entry();
        fflush(stdout);

        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout failed");
        assert_eq!(close(saved_stdout), 0, "close saved stdout failed");
        clearerr(stdout);
        status
    };
    drop(saved_stdin);
    unsafe {
        clearerr(stdin);
    }

    let mut remaining_input = Vec::new();
    input_reader
        .read_to_end(&mut remaining_input)
        .expect("read input remaining after write error");
    (status, remaining_input)
}

fn expected_output(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut position = 0;

    while position < input.len() {
        let limit = (position + 127).min(input.len());
        let end = input[position..limit]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(limit, |newline| position + newline + 1);
        let chunk = &input[position..end];
        let visible = chunk
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(chunk.len());
        output.extend_from_slice(&chunk[..visible]);
        position = end;
    }

    output
}

fn assert_matches(c_entry: EntryPoint, rust_entry: EntryPoint, row: &str, input: &[u8]) {
    let expected = expected_output(input);
    let c_result = invoke(c_entry, input);
    let rust_result = invoke(rust_entry, input);
    assert_eq!(c_result.0, 0, "{row}: C status");
    assert_eq!(rust_result.0, c_result.0, "{row}: status mismatch");
    assert_eq!(c_result.1, expected, "{row}: C model mismatch");
    assert_eq!(rust_result.1, c_result.1, "{row}: output mismatch");
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + (self.next() as usize % (end - start))
    }

    fn non_special_bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length)
            .map(|_| loop {
                let byte = self.next() as u8;
                if byte != 0 && byte != b'\n' {
                    break byte;
                }
            })
            .collect()
    }
}

#[test]
fn differential_surface() {
    let _guard = PROCESS_IO_LOCK.lock().expect("process I/O lock");
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C shared object: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared object: {rust_path:?}"
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared object");
        let rust_library = Library::new(&rust_path).expect("load Rust shared object");
        let c_symbol: Symbol<EntryPoint> = c_library.get(b"main\0").expect("load C main symbol");
        let rust_symbol: Symbol<EntryPoint> =
            rust_library.get(b"main\0").expect("load Rust main symbol");
        let c_entry = *c_symbol;
        let rust_entry = *rust_symbol;
        let mut rng = Rng::new(0x6d5a_56da_4f3c_2b1a);

        // C1: immediate EOF has no data axis, so repeat it to exercise clean state.
        for _ in 0..64 {
            assert_matches(c_entry, rust_entry, "C1", &[]);
        }

        for _ in 0..64 {
            // C2: one short newline-terminated chunk.
            let length = rng.range(0, 127);
            let mut input = rng.non_special_bytes(length);
            input.push(b'\n');
            assert_matches(c_entry, rust_entry, "C2", &input);

            // C3: several short newline-terminated chunks.
            let mut input = Vec::new();
            for _ in 0..rng.range(2, 9) {
                let length = rng.range(0, 127);
                input.extend(rng.non_special_bytes(length));
                input.push(b'\n');
            }
            assert_matches(c_entry, rust_entry, "C3", &input);

            // C4: a short unterminated final chunk.
            let length = rng.range(1, 127);
            let input = rng.non_special_bytes(length);
            assert_matches(c_entry, rust_entry, "C4", &input);

            // C5: exactly one full fgets payload.
            let input = rng.non_special_bytes(127);
            assert_matches(c_entry, rust_entry, "C5", &input);

            // C6: one logical line spanning multiple reads.
            let length = rng.range(128, 1025);
            let mut input = rng.non_special_bytes(length);
            if rng.next() & 1 == 0 {
                input.push(b'\n');
            }
            assert_matches(c_entry, rust_entry, "C6", &input);

            // C7: newline in the final slot or one byte beyond a full read.
            let mut at_127 = rng.non_special_bytes(126);
            at_127.push(b'\n');
            assert_matches(c_entry, rust_entry, "C7", &at_127);
            let mut at_128 = rng.non_special_bytes(127);
            at_128.push(b'\n');
            assert_matches(c_entry, rust_entry, "C7", &at_128);

            // C8: one short read containing an embedded NUL.
            let length = rng.range(2, 127);
            let mut input = rng.non_special_bytes(length);
            input[rng.range(0, length)] = 0;
            if rng.next() & 1 == 0 {
                input.push(b'\n');
            }
            assert_matches(c_entry, rust_entry, "C8", &input);

            // C9: NUL truncation in multiple full or partial fgets chunks.
            let length = rng.range(255, 1025);
            let mut input = rng.non_special_bytes(length);
            for chunk_start in (0..length).step_by(127) {
                let chunk_end = (chunk_start + 127).min(length);
                input[rng.range(chunk_start, chunk_end)] = 0;
            }
            if rng.next() & 1 == 0 {
                input.push(b'\n');
            }
            assert_matches(c_entry, rust_entry, "C9", &input);
        }

        // E1: fgets returns NULL for an unreadable stdin.
        assert_eq!(invoke_read_error(c_entry), 0, "E1: C status");
        assert_eq!(invoke_read_error(rust_entry), 0, "E1: Rust status differs");

        // E2: fputs errors are ignored while all input is consumed.
        let error_input = rng.non_special_bytes(32 * 1024);
        let c_result = invoke_write_error(c_entry, &error_input);
        let rust_result = invoke_write_error(rust_entry, &error_input);
        assert_eq!(c_result.0, 0, "E2: C status");
        assert!(c_result.1.is_empty(), "E2: C left input unread");
        assert_eq!(rust_result.0, c_result.0, "E2: Rust status differs");
        assert_eq!(
            rust_result.1, c_result.1,
            "E2: Rust input consumption differs"
        );
    }
}
