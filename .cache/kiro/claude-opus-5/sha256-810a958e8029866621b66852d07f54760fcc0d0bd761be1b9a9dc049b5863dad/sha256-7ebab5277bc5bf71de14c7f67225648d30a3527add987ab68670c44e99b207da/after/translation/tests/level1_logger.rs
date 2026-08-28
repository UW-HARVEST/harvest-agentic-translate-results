//! Level 1: `logger.h` — the lowest layer, no dependencies of its own.
//!
//! C reference: c_src/src/logger.c

mod harness;

use harness::{compare, cstr, default_log_path, Api};

/// Happy path: init writes the banner, each level gets its own prefix, and
/// `finalize_logger` appends the closing line and flushes via `fclose`.
#[test]
fn initialize_log_finalize() {
    compare("init/log/finalize", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        let a = cstr(b"first");
        let b = cstr(b"second");
        let c = cstr(b"third");
        (api.log_info)(a.as_ptr());
        (api.log_warning)(b.as_ptr());
        (api.log_error)(c.as_ptr());
        (api.finalize_logger)();
    });
}

/// Messages are passed to `fprintf` as a `%s` argument, so `printf` escapes
/// inside them must be copied through verbatim and never interpreted.
#[test]
fn message_payloads() {
    let payloads: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"plain ascii".to_vec(),
        b"%s %d %n %%".to_vec(),
        b"%99999999d".to_vec(),
        b"trailing newline\n".to_vec(),
        b"embedded\nnewline".to_vec(),
        b"tab\there".to_vec(),
        "utf8: \u{e9}\u{4e2d}\u{6587}\u{1f600}".as_bytes().to_vec(),
        vec![0x80, 0xfe, 0xff, 0x01, 0x7f],
        vec![b'x'; 1000],
        vec![b'y'; 8192],
    ];

    for (i, payload) in payloads.iter().enumerate() {
        compare(&format!("payload #{i}"), &[], |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(payload);
            (api.log_info)(m.as_ptr());
            (api.log_warning)(m.as_ptr());
            (api.log_error)(m.as_ptr());
            (api.finalize_logger)();
        });
    }
}

/// `fopen(path, "a")` failures: the return value is -1 and the path is echoed
/// to stderr with `%s`.
#[test]
fn initialize_logger_open_failures() {
    // A directory, a missing parent directory, an empty path and a path whose
    // "directory" component is a regular file.
    let sentinel = default_log_path().to_str().unwrap().to_string();
    let not_a_dir = format!("{sentinel}-plain/inside.log");
    std::fs::create_dir_all(harness::scratch_dir()).unwrap();
    std::fs::write(default_log_path().with_file_name("test.log-plain"), b"x").unwrap();

    let bad_paths: Vec<String> = vec![
        "/tmp".to_string(),
        "/".to_string(),
        "/no/such/directory/anywhere/x.log".to_string(),
        String::new(),
        not_a_dir,
        "/proc/self/mem/nope".to_string(),
    ];

    for (i, path) in bad_paths.iter().enumerate() {
        compare(
            &format!("open failure #{i} ({path})"),
            &[("LOG_FILE", Some(path.as_str()))],
            |api: &Api, t| unsafe {
                t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
                // log_file is NULL, so these are all no-ops.
                let m = cstr(b"after failure");
                (api.log_info)(m.as_ptr());
                (api.finalize_logger)();
            },
        );
    }
}

/// `LOG_FILE` unset falls back to the relative path "default.log".
#[test]
fn log_file_env_unset_uses_default_log() {
    compare(
        "LOG_FILE unset",
        &[("LOG_FILE", None)],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(b"into default.log");
            (api.log_info)(m.as_ptr());
            (api.finalize_logger)();
            t.push(format!(
                "default.log size = {:?}",
                std::fs::metadata("default.log").map(|m| m.len()).ok()
            ));
        },
    );
}

/// `fopen(..., "a")` appends: a second session must not truncate the first.
#[test]
fn append_mode_across_sessions() {
    compare("append across sessions", &[], |api: &Api, t| unsafe {
        for round in 0..3 {
            t.push(format!(
                "round {round} initialize_logger -> {}",
                (api.initialize_logger)()
            ));
            let m = cstr(format!("round {round}").as_bytes());
            (api.log_info)(m.as_ptr());
            (api.finalize_logger)();
        }
    });
}

/// stdio buffering is observable: nothing reaches the file until the buffer
/// fills or the stream is closed.  This pins down the buffer size and the
/// "flush exactly at the boundary" behaviour.
#[test]
fn buffering_boundary() {
    compare("buffering", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        // "[INFO] Logger initialized.\n" == 27 bytes are already buffered.
        let size = |t: &mut Vec<String>| {
            t.push(format!(
                "size = {:?}",
                std::fs::metadata(default_log_path()).map(|m| m.len()).ok()
            ))
        };
        size(t);
        // Each entry is "[INFO] " + 93 'z' + "\n" == 101 bytes.
        let m = cstr(&vec![b'z'; 93]);
        for _ in 0..60 {
            (api.log_info)(m.as_ptr());
            size(t);
        }
        (api.finalize_logger)();
        size(t);
    });
}

/// A single `fwrite` larger than the stdio buffer.
#[test]
fn buffering_single_large_write() {
    for len in [4000usize, 4068, 4069, 4070, 4096, 8191, 8192, 20000] {
        compare(&format!("large write {len}"), &[], |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(&vec![b'q'; len]);
            (api.log_info)(m.as_ptr());
            t.push(format!(
                "buffered size = {:?}",
                std::fs::metadata(default_log_path()).map(|m| m.len()).ok()
            ));
            (api.finalize_logger)();
            t.push(format!(
                "final size = {:?}",
                std::fs::metadata(default_log_path()).map(|m| m.len()).ok()
            ));
        });
    }
}
