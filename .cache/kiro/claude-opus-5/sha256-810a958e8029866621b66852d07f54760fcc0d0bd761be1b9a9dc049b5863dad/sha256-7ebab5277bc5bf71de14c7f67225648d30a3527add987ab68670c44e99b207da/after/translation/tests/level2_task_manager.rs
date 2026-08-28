//! Level 2: `task_manager.h`, which sits on top of the logger.
//!
//! C reference: c_src/src/task_manager.c

mod harness;

use harness::{compare, cstr, record_manager, Api};

/// `create_task_manager()` with `MAX_TASKS` unset: 10 slots, empty.
#[test]
fn create_default() {
    compare("create default", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        let m = (api.create_task_manager)();
        record_manager(t, m);
        if !m.is_null() {
            (api.destroy_task_manager)(m);
        }
        (api.finalize_logger)();
    });
}

/// `manager->max_tasks = atoi(getenv("MAX_TASKS"))`.  `atoi` is
/// `(int)strtol(s, NULL, 10)`, so all of these have well-defined answers that
/// the translation has to reproduce, including the wrap-around ones.
#[test]
fn max_tasks_env_parsing() {
    let values: Vec<&str> = vec![
        "0",
        "1",
        "2",
        "10",
        "255",
        "",
        " ",
        "   7",
        "\t\n\u{b}\u{c}\r8",
        "+9",
        "-0",
        "abc",
        "5abc",
        "abc5",
        "3.9",
        "0x10",
        "010",
        "1e3",
        "  +  4",
        "2147483647",
        "2147483648",
        "4294967295",
        "4294967296",
        "4294967306",
        "9223372036854775807",
        "9223372036854775808",
        "99999999999999999999",
        "-1",
        "-2",
        "-10",
        "-2147483648",
        "-2147483649",
        "-4294967296",
        "-99999999999999999999",
        "1000000000",
        "8000000",
        "--5",
        "+-5",
        "0000000000000000000000005",
    ];

    for v in values {
        compare(
            &format!("MAX_TASKS={v:?}"),
            &[("MAX_TASKS", Some(v))],
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

/// `add_task` copies with `strncpy(dst, src, 255)` and then forces `dst[255]`
/// to NUL, so the whole 256-byte buffer (padding included) is deterministic and
/// can be compared byte for byte.
#[test]
fn add_task_descriptions() {
    let cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"%s%d%n".to_vec(),
        b"with\ttab".to_vec(),
        b"with\nnewline".to_vec(),
        "unicode \u{e9}\u{4e2d}\u{1f600}".as_bytes().to_vec(),
        vec![0xff, 0xfe, 0x80, 0x01],
        vec![b'x'; 254],
        vec![b'x'; 255],
        vec![b'x'; 256],
        vec![b'x'; 257],
        vec![b'x'; 300],
        vec![b'x'; 1024],
    ];

    for (i, desc) in cases.iter().enumerate() {
        compare(
            &format!("add_task desc #{i} (len {})", desc.len()),
            &[],
            |api: &Api, t| unsafe {
                t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
                let m = (api.create_task_manager)();
                assert!(!m.is_null());
                let d = cstr(desc);
                (api.add_task)(m, d.as_ptr(), -3);
                (api.add_task)(m, d.as_ptr(), 0);
                (api.add_task)(m, d.as_ptr(), i32::MAX);
                (api.add_task)(m, d.as_ptr(), i32::MIN);
                record_manager(t, m);
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            },
        );
    }
}

/// Filling the manager exactly, then overflowing it: the `task_count >=
/// max_tasks` guard logs a warning and leaves the manager untouched.
#[test]
fn add_task_capacity_limit() {
    for max in ["0", "1", "3", "10"] {
        compare(
            &format!("capacity MAX_TASKS={max}"),
            &[("MAX_TASKS", Some(max))],
            |api: &Api, t| unsafe {
                t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
                let m = (api.create_task_manager)();
                record_manager(t, m);
                assert!(!m.is_null());
                for i in 0..14i32 {
                    let d = cstr(format!("task-{i}").as_bytes());
                    (api.add_task)(m, d.as_ptr(), i * 100);
                    t.push(format!("after add {i}: count={}", (*m).task_count));
                }
                record_manager(t, m);
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            },
        );
    }
}

/// A negative `max_tasks` makes the `>=` guard true immediately, so `add_task`
/// always refuses.  (`create_task_manager` itself fails first for most negative
/// values because `malloc` gets a colossal size, which is also compared.)
#[test]
fn add_task_with_negative_capacity() {
    for max in ["-1", "-5", "-2147483648"] {
        compare(
            &format!("negative capacity MAX_TASKS={max}"),
            &[("MAX_TASKS", Some(max))],
            |api: &Api, t| unsafe {
                t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
                let m = (api.create_task_manager)();
                record_manager(t, m);
                if !m.is_null() {
                    let d = cstr(b"nope");
                    (api.add_task)(m, d.as_ptr(), 1);
                    record_manager(t, m);
                    (api.print_tasks)(m);
                    (api.destroy_task_manager)(m);
                }
                (api.finalize_logger)();
            },
        );
    }
}

/// `print_tasks` on an empty manager prints only the header.
#[test]
fn print_tasks_empty() {
    compare("print_tasks empty", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        let m = (api.create_task_manager)();
        assert!(!m.is_null());
        (api.print_tasks)(m);
        (api.print_tasks)(m);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
    });
}

/// `print_tasks` walks `for (int i = 0; i < manager->task_count; i++)`, so a
/// non-positive `task_count` prints the header alone.
#[test]
fn print_tasks_non_positive_count() {
    for count in [0i32, -1, -100, i32::MIN] {
        compare(
            &format!("print_tasks task_count={count}"),
            &[],
            |api: &Api, t| unsafe {
                t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
                let m = (api.create_task_manager)();
                assert!(!m.is_null());
                let d = cstr(b"present but hidden");
                (api.add_task)(m, d.as_ptr(), 42);
                (*m).task_count = count;
                (api.print_tasks)(m);
                // Restore a sane count so destroy_task_manager frees normally.
                (*m).task_count = 1;
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
            },
        );
    }
}

/// `destroy_task_manager` logs after freeing; the log line must appear even
/// though the manager is gone.
#[test]
fn destroy_logs_after_free() {
    compare("destroy logs", &[], |api: &Api, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        let m = (api.create_task_manager)();
        assert!(!m.is_null());
        (api.destroy_task_manager)(m);
        let m2 = (api.create_task_manager)();
        assert!(!m2.is_null());
        (api.destroy_task_manager)(m2);
        (api.finalize_logger)();
    });
}

/// Interleaving several managers: the allocations are independent.
#[test]
fn multiple_managers() {
    compare(
        "multiple managers",
        &[("MAX_TASKS", Some("4"))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let a = (api.create_task_manager)();
            let b = (api.create_task_manager)();
            assert!(!a.is_null() && !b.is_null());
            for i in 0..6i32 {
                let d = cstr(format!("a{i}").as_bytes());
                (api.add_task)(a, d.as_ptr(), i);
                let e = cstr(format!("b{i}").as_bytes());
                (api.add_task)(b, e.as_ptr(), -i);
            }
            record_manager(t, a);
            record_manager(t, b);
            (api.print_tasks)(a);
            (api.print_tasks)(b);
            (api.destroy_task_manager)(a);
            (api.destroy_task_manager)(b);
            (api.finalize_logger)();
        },
    );
}

/// A large but plausible `MAX_TASKS`: the tasks array is ~26 MB, so the
/// allocation succeeds and every slot is usable.
#[test]
fn large_capacity() {
    compare(
        "MAX_TASKS=100000",
        &[("MAX_TASKS", Some("100000"))],
        |api: &Api, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let m = (api.create_task_manager)();
            record_manager(t, m);
            assert!(!m.is_null());
            for i in 0..2000i32 {
                let d = cstr(format!("bulk task number {i}").as_bytes());
                (api.add_task)(m, d.as_ptr(), i);
            }
            t.push(format!("count = {}", (*m).task_count));
            // Spot-check a few slots rather than all 2000.
            for i in [0usize, 1, 999, 1999] {
                let task = &*(*m).tasks.add(i);
                t.push(format!(
                    "task[{i}] priority={} description={}",
                    task.priority,
                    harness::show(&task.description)
                ));
            }
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        },
    );
}
