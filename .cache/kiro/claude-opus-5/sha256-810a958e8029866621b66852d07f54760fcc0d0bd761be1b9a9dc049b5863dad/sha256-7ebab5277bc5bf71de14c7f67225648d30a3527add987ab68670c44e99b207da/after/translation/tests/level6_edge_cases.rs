//! Level 6: edge cases that live at the FFI boundary — raw (non-UTF-8) bytes in
//! the environment and in strings, NULL `%s` arguments, and unusual log targets.

mod harness;

use harness::{compare, compare_raw, cstr, record_manager, Api};

/// `getenv` returns raw bytes; neither `MAX_TASKS` nor `LOG_FILE` is required to
/// be valid UTF-8.
#[test]
fn non_utf8_max_tasks() {
    let values: Vec<Vec<u8>> = vec![
        vec![0xff, 0xfe],
        vec![b'4', 0xff, b'2'],
        vec![0x80, b'7'],
        vec![b'1', b'2', 0xc3],
        vec![0xc3, 0x28, b'5'],
    ];
    for (i, v) in values.iter().enumerate() {
        compare_raw(
            &format!("non-utf8 MAX_TASKS #{i}"),
            &[("MAX_TASKS", Some(v.clone()))],
            |api: &Api, t| unsafe {
                t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
                let m = (api.create_task_manager)();
                record_manager(t, m);
                if !m.is_null() {
                    (api.destroy_task_manager)(m);
                }
                (api.finalize_logger)();
            },
        );
    }
}

/// A log path that is not valid UTF-8 still has to be opened, and echoed
/// verbatim to stderr when the open fails.
#[test]
fn non_utf8_log_file() {
    let dir = harness::scratch_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let mut ok_path = dir.as_os_str().as_encoded_bytes().to_vec();
    ok_path.extend_from_slice(b"/log-\xff\xfe.txt");

    compare_raw(
        "non-utf8 LOG_FILE (openable)",
        &[("LOG_FILE", Some(ok_path))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(b"raw path");
            (api.log_info)(m.as_ptr());
            (api.finalize_logger)();
        },
    );

    // Same, but inside a directory that does not exist, so the path lands in
    // the stderr diagnostic instead.
    let mut bad_path = dir.as_os_str().as_encoded_bytes().to_vec();
    bad_path.extend_from_slice(b"/missing-\xff\xfe/log.txt");
    compare_raw(
        "non-utf8 LOG_FILE (open fails)",
        &[("LOG_FILE", Some(bad_path))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            (api.finalize_logger)();
        },
    );
}

/// glibc's `printf`/`fprintf` print the literal text `(null)` for a NULL `%s`
/// argument, so `log_info(NULL)` produces a real log line rather than crashing.
#[test]
fn null_log_message() {
    compare("log_*(NULL)", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        (api.log_info)(std::ptr::null());
        (api.log_warning)(std::ptr::null());
        (api.log_error)(std::ptr::null());
        (api.finalize_logger)();
    });
}

/// `print_tasks` also formats descriptions with `%s`, but a description is an
/// in-struct array so it can never be NULL — this checks the array is always
/// NUL-terminated even when the source string filled it completely.
#[test]
fn print_tasks_full_description() {
    compare("print_tasks 255-byte description", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        let m = (api.create_task_manager)();
        assert!(!m.is_null());
        for len in [253usize, 254, 255, 256, 260] {
            let d = cstr(&vec![b'F'; len]);
            (api.add_task)(m, d.as_ptr(), len as i32);
        }
        record_manager(t, m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

/// `/dev/null` accepts everything and returns nothing.
#[test]
fn log_to_dev_null() {
    compare(
        "LOG_FILE=/dev/null",
        &[("LOG_FILE", Some("/dev/null"))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(b"into the void");
            (api.log_info)(m.as_ptr());
            (api.finalize_logger)();
        },
    );
}

/// A path longer than `PATH_MAX` fails with ENAMETOOLONG.
#[test]
fn log_path_too_long() {
    let long = format!("/tmp/{}", "d/".repeat(3000));
    compare(
        "LOG_FILE too long",
        &[("LOG_FILE", Some(long.as_str()))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            (api.finalize_logger)();
        },
    );
}

/// A log file that exists but is not writable: `fopen(..., "a")` fails.
#[test]
fn log_file_not_writable() {
    let dir = harness::scratch_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("readonly.log");
    std::fs::write(&path, b"pre-existing\n").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o444);
    }
    std::fs::set_permissions(&path, perms).unwrap();

    compare(
        "LOG_FILE read-only",
        &[("LOG_FILE", Some(path.to_str().unwrap()))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(b"denied");
            (api.log_info)(m.as_ptr());
            (api.finalize_logger)();
        },
    );

    // `compare` deletes the log target between runs, so re-create it for the
    // second run to see the same state; then clean up.
    let _ = std::fs::remove_file(&path);
}

/// A directory component that is a symlink, and a symlinked log file: both must
/// be followed identically.
#[test]
fn log_file_through_symlink() {
    let dir = harness::scratch_dir();
    std::fs::create_dir_all(dir.join("real")).unwrap();
    let link = dir.join("link");
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.join("real"), &link).unwrap();

    let target = link.join("through-symlink.log");
    compare(
        "LOG_FILE via symlinked directory",
        &[("LOG_FILE", Some(target.to_str().unwrap()))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(b"symlinked");
            (api.log_info)(m.as_ptr());
            (api.finalize_logger)();
        },
    );

    // A dangling symlink as the log file itself: fopen(..., "a") creates the
    // final target.
    let dangling = dir.join("dangling.log");
    let _ = std::fs::remove_file(&dangling);
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.join("real/created-via-symlink.log"), &dangling).unwrap();
    compare(
        "LOG_FILE is a dangling symlink",
        &[("LOG_FILE", Some(dangling.to_str().unwrap()))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = cstr(b"created through a dangling link");
            (api.log_info)(m.as_ptr());
            (api.finalize_logger)();
            t.push(format!(
                "real target size = {:?}",
                std::fs::metadata(harness::scratch_dir().join("real/created-via-symlink.log"))
                    .map(|m| m.len())
                    .ok()
            ));
            let _ =
                std::fs::remove_file(harness::scratch_dir().join("real/created-via-symlink.log"));
        },
    );
}

/// The file mode a brand-new log file is created with (`0666 & ~umask`).
#[test]
fn new_log_file_permissions() {
    compare("new log file mode", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        (api.finalize_logger)();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            t.push(format!(
                "mode = {:o}",
                std::fs::metadata(harness::default_log_path())
                    .map(|m| m.permissions().mode())
                    .unwrap_or(0)
            ));
        }
    });
}
