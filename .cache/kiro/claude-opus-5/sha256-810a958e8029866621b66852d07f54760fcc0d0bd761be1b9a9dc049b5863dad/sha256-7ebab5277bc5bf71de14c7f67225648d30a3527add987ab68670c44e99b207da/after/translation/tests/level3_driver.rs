//! Level 3: `driver()`, the top of the call hierarchy.
//!
//! C reference: c_src/src/driver.c

mod harness;

use harness::{compare, cstr, Api};

fn run_driver(input: Vec<u8>, env: &[(&str, Option<&str>)], case: &str) {
    compare(case, env, move |api: &Api, t| unsafe {
        let s = cstr(&input);
        t.push(format!("driver -> {}", (api.driver)(s.as_ptr())));
    });
}

/// The newline splitting loop: the tricky part is that the trailing `end`
/// pointer only advances past a `'\n'`, so a string that does not end in a
/// newline still yields its final segment.
#[test]
fn task_splitting() {
    let inputs: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"\n".to_vec(),
        b"\n\n".to_vec(),
        b"\n\n\n".to_vec(),
        b"a\n".to_vec(),
        b"a\nb".to_vec(),
        b"a\nb\n".to_vec(),
        b"a\n\nb".to_vec(),
        b"\na".to_vec(),
        b"one\ntwo\nthree".to_vec(),
        b"one\ntwo\nthree\n".to_vec(),
        b"first task\nsecond task\nthird task\n".to_vec(),
        b"  leading spaces\ntrailing spaces  ".to_vec(),
        b"\r\n\r\n".to_vec(),
        b"tab\there\nand\tthere".to_vec(),
        b"%s\n%d\n%%\n".to_vec(),
        "\u{e9}\n\u{4e2d}\u{6587}\n\u{1f600}".as_bytes().to_vec(),
        vec![0xff, b'\n', 0x80, 0xfe],
    ];

    for (i, input) in inputs.into_iter().enumerate() {
        run_driver(input, &[], &format!("split #{i}"));
    }
}

/// Segments longer than the 255-byte description buffer are truncated by
/// `strncpy`, and `print_tasks` prints the truncated form.
#[test]
fn long_segments() {
    for len in [254usize, 255, 256, 257, 500, 1000] {
        let mut input = vec![b'L'; len];
        input.push(b'\n');
        input.extend(std::iter::repeat(b'M').take(len));
        run_driver(input, &[], &format!("long segment len={len}"));
    }
}

/// More segments than `MAX_TASKS` allows: the surplus is dropped with a warning
/// but `priority` keeps incrementing for every segment.
#[test]
fn more_tasks_than_capacity() {
    for max in ["0", "1", "2", "3", "10"] {
        let input: Vec<u8> = (0..15)
            .map(|i| format!("task {i}"))
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();
        run_driver(
            input,
            &[("MAX_TASKS", Some(max))],
            &format!("overflow MAX_TASKS={max}"),
        );
    }
}

/// `create_task_manager()` failing makes `driver()` return EXIT_FAILURE without
/// calling `finalize_logger()`, so the banner stays in the stdio buffer.
#[test]
fn driver_when_create_task_manager_fails() {
    for max in ["-1", "-5", "-2147483648", "99999999999999999999"] {
        run_driver(
            b"anything\ngoes\n".to_vec(),
            &[("MAX_TASKS", Some(max))],
            &format!("create fails MAX_TASKS={max}"),
        );
    }
}

/// `initialize_logger()` failing makes `driver()` return EXIT_FAILURE before
/// anything is printed.
#[test]
fn driver_when_logger_fails() {
    for path in ["/tmp", "/", "", "/no/such/dir/x.log"] {
        run_driver(
            b"a\nb\nc\n".to_vec(),
            &[("LOG_FILE", Some(path))],
            &format!("logger fails LOG_FILE={path:?}"),
        );
    }
}

/// Repeated `driver()` calls in one process: each one re-opens the log in
/// append mode and closes it again.
#[test]
fn repeated_driver_calls() {
    compare("repeated driver", &[("MAX_TASKS", Some("3"))], |api: &Api, t| unsafe {
        for round in 0..4 {
            let s = cstr(format!("r{round}-x\nr{round}-y\nr{round}-z\nr{round}-w").as_bytes());
            t.push(format!("driver -> {}", (api.driver)(s.as_ptr())));
        }
    });
}

/// Enough output to push both the log stream and stdout past their buffers.
#[test]
fn bulk_driver() {
    let input: Vec<u8> = (0..500)
        .map(|i| format!("bulk task number {i} with some padding text"))
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes();
    run_driver(input, &[("MAX_TASKS", Some("1000"))], "bulk 500 tasks");
}

/// `MAX_TASKS` and `LOG_FILE` varied together, end to end.
#[test]
fn driver_env_matrix() {
    let inputs: Vec<&[u8]> = vec![b"", b"solo", b"a\nb", b"x\ny\nz\n"];
    let maxes = ["0", "1", "2", "5", "abc", "", "-0", "+3"];
    for input in &inputs {
        for max in &maxes {
            run_driver(
                input.to_vec(),
                &[("MAX_TASKS", Some(max))],
                &format!("matrix input={:?} MAX_TASKS={max:?}", harness::show(input)),
            );
        }
    }
}
