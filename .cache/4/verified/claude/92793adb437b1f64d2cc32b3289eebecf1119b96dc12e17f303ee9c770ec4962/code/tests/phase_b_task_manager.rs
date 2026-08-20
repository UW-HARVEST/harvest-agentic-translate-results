//! Phase B — valid-path differential tests for the task-manager entry points
//! (`create_task_manager`, `add_task`, `print_tasks`, `destroy_task_manager`),
//! driven directly rather than through the `driver` wrapper.
//!
//! Covers `CONFIGS.md` rows 7-20.

mod common;

use common::*;
use std::ffi::c_int;

/// Standard body prologue/epilogue: the logger must be open for the task
/// manager's own `log_*` calls to be observable.
macro_rules! with_logger {
    ($api:expr, $rec:expr, $body:expr) => {{
        unsafe { $rec.ret(($api.initialize_logger)()) };
        $body;
        unsafe { ($api.finalize_logger)() };
    }};
}

/// CONFIGS row 7 — `MAX_TASKS` unset → default limit 10.
fn cfg_07_create_default_max() {
    let obs = diff("cfg_07", &Cfg::fresh().max_unset(), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = (api.create_task_manager)();
            rec.ptr_is_null(m as *const u8);
            rec.manager(m);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        });
    });
    // max_tasks == 10, task_count == 0, tasks != NULL
    assert_eq!(&obs.extra[0..4], &10i32.to_le_bytes());
    assert_eq!(&obs.extra[4..8], &0i32.to_le_bytes());
    assert_eq!(obs.extra[8], 1);
    assert_eq!(obs.stdout, b"Tasks:\n".to_vec());
    assert_eq!(
        obs.log,
        b"[INFO] Logger initialized.\n\
          [INFO] TaskManager created successfully.\n\
          [INFO] TaskManager destroyed successfully.\n\
          [INFO] Logger finalized.\n"
            .to_vec()
    );
}

/// CONFIGS row 8 — explicit numeric `MAX_TASKS` values.
fn cfg_08_create_numeric_max() {
    let _g = lock();
    for (v, want) in [("0", 0), ("1", 1), ("3", 3), ("10", 10), ("64", 64)] {
        let obs = diff_locked(
            &format!("cfg_08 MAX_TASKS={v}"),
            &Cfg::fresh().max(v),
            |api, rec| {
                with_logger!(api, rec, unsafe {
                    let m = (api.create_task_manager)();
                    rec.ptr_is_null(m as *const u8);
                    rec.manager(m);
                    (api.print_tasks)(m);
                    (api.destroy_task_manager)(m);
                });
            },
        );
        assert_eq!(obs.rets, vec![0, 0], "MAX_TASKS={v}: create must succeed");
        assert_eq!(
            &obs.extra[0..4],
            &(want as i32).to_le_bytes(),
            "MAX_TASKS={v}: wrong max_tasks"
        );
    }
}

/// CONFIGS row 9 — `atoi` parsing quirks must be reproduced exactly.
fn cfg_09_create_atoi_quirks() {
    let _g = lock();
    for (v, want) in [
        ("abc", 0),
        ("   7", 7),
        ("+7", 7),
        ("7abc", 7),
        ("0x10", 0),
        ("007", 7),
        ("2.9", 2),
        ("\t 12 ", 12),
        ("", 0),
        ("-0", 0),
    ] {
        let obs = diff_locked(
            &format!("cfg_09 MAX_TASKS={v:?}"),
            &Cfg::fresh().max(v),
            |api, rec| {
                with_logger!(api, rec, unsafe {
                    let m = (api.create_task_manager)();
                    rec.ptr_is_null(m as *const u8);
                    rec.manager(m);
                    let d = cstr(b"one");
                    (api.add_task)(m, d.as_ptr() as *const _, 1);
                    rec.manager(m);
                    (api.print_tasks)(m);
                    (api.destroy_task_manager)(m);
                });
            },
        );
        assert_eq!(
            &obs.extra[0..4],
            &(want as i32).to_le_bytes(),
            "MAX_TASKS={v:?}: atoi result mismatch vs expectation"
        );
    }
}

/// CONFIGS row 10 — `atoi` integer overflow (`"99999999999999"`), where glibc
/// truncates the parsed `long` to `int`. The resulting `max_tasks` (and whether
/// the `max_tasks * 260` allocation then succeeds) must agree.
fn cfg_10_create_atoi_overflow() {
    let _g = lock();
    for v in [
        "99999999999999",
        "2147483648",
        "-2147483649",
        "9223372036854775807",
        "99999999999999999999999999",
    ] {
        diff_locked(
            &format!("cfg_10 MAX_TASKS={v}"),
            &Cfg::fresh().max(v),
            |api, rec| {
                with_logger!(api, rec, unsafe {
                    let m = (api.create_task_manager)();
                    rec.ptr_is_null(m as *const u8);
                    if !m.is_null() {
                        rec.manager(m);
                        (api.print_tasks)(m);
                        (api.destroy_task_manager)(m);
                    }
                });
            },
        );
    }
}

/// CONFIGS row 11 — N0: no tasks at all.
fn cfg_11_tm_zero_tasks() {
    let obs = diff("cfg_11", &Cfg::fresh(), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = (api.create_task_manager)();
            rec.manager(m);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        });
    });
    assert_eq!(obs.stdout, b"Tasks:\n".to_vec());
}

/// CONFIGS row 12 — N1: exactly one task.
fn cfg_12_tm_one_task() {
    let obs = diff("cfg_12", &Cfg::fresh(), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = (api.create_task_manager)();
            let d = cstr(b"write the tests");
            (api.add_task)(m, d.as_ptr() as *const _, 7);
            rec.manager(m);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        });
    });
    assert_eq!(
        obs.stdout,
        b"Tasks:\n  [1] write the tests (Priority: 7)\n".to_vec()
    );
    assert_eq!(&obs.extra[4..8], &1i32.to_le_bytes());
}

/// CONFIGS row 13 — N2/N3: fill up to `max_tasks - 1` and to exactly
/// `max_tasks`, with randomized descriptions and priorities (60 seeds).
fn cfg_13_tm_fill_to_limit_random() {
    let _g = lock();
    const SEED: u64 = 0xB0B1_C0DE_1234_0013;
    for max in [1usize, 2, 3, 10, 17] {
        for count in [max.saturating_sub(1), max] {
            let cfg = Cfg::fresh().max(&max.to_string());
            let label = format!("cfg_13 max={max} count={count}");
            diff_locked(&label, &cfg, |api, rec| {
                with_logger!(api, rec, unsafe {
                    let mut rng = Rng::new(SEED ^ (max as u64) << 32 ^ count as u64);
                    let m = (api.create_task_manager)();
                    rec.ptr_is_null(m as *const u8);
                    for _ in 0..count {
                        let len = rng.range(0, 300);
                        let d = cstr(&rng.text(len));
                        (api.add_task)(m, d.as_ptr() as *const _, rng.i32());
                        rec.manager(m);
                    }
                    (api.print_tasks)(m);
                    (api.destroy_task_manager)(m);
                });
            });
        }
    }
}

/// CONFIGS row 14 — N4: more `add_task` calls than `max_tasks` allows.
fn cfg_14_tm_overflow_beyond_max() {
    let obs = diff("cfg_14", &Cfg::fresh().max("64"), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = (api.create_task_manager)();
            for i in 0..80i32 {
                let d = cstr(format!("task-{i}").as_bytes());
                (api.add_task)(m, d.as_ptr() as *const _, i);
                rec.manager(m);
            }
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        });
    });
    // 64 stored, task_count clamped at 64, and 16 rejection warnings.
    let text = String::from_utf8_lossy(&obs.log);
    assert_eq!(
        text.matches("[WARNING] Cannot add task: Maximum task limit reached.")
            .count(),
        16
    );
    assert_eq!(text.matches("[INFO] Task added successfully.").count(), 64);
    let printed = String::from_utf8_lossy(&obs.stdout);
    assert_eq!(printed.lines().count(), 65); // "Tasks:" + 64 rows
    assert!(printed.contains("  [64] task-63 (Priority: 63)\n"));
}

/// CONFIGS row 15 — D0..D6: description lengths around the 255-byte
/// `strncpy` limit. The full 260 raw bytes of every slot are compared, so this
/// also pins `strncpy`'s NUL zero-padding of the unused tail.
fn cfg_15_tm_description_lengths() {
    let _g = lock();
    for len in [0usize, 1, 2, 100, 254, 255, 256, 257, 300, 1000, 4096] {
        let label = format!("cfg_15 len={len}");
        diff_locked(&label, &Cfg::fresh(), |api, rec| {
            with_logger!(api, rec, unsafe {
                let m = (api.create_task_manager)();
                // A pattern where truncation is visible: repeating 0..9 digits.
                let body: Vec<u8> = (0..len).map(|i| b'0' + (i % 10) as u8).collect();
                let d = cstr(&body);
                (api.add_task)(m, d.as_ptr() as *const _, len as c_int);
                rec.manager(m);
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
            });
        });
    }

    // Absolute check of the C truncation boundary.
    let obs = diff("cfg_15 boundary", &Cfg::fresh(), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = (api.create_task_manager)();
            for len in [255usize, 256] {
                let body = vec![b'x'; len];
                let d = cstr(&body);
                (api.add_task)(m, d.as_ptr() as *const _, 0);
            }
            rec.manager(m);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        });
    });
    let printed = String::from_utf8_lossy(&obs.stdout);
    let rows: Vec<&str> = printed.lines().skip(1).collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].matches('x').count(), 255, "255 must be kept whole");
    assert_eq!(rows[1].matches('x').count(), 255, "256 must truncate to 255");
}

/// CONFIGS row 16 — P1..P4 plus random `i32` priorities.
fn cfg_16_tm_priority_extremes() {
    const SEED: u64 = 0xB0B1_C0DE_1234_0016;
    let obs = diff("cfg_16", &Cfg::fresh().max("40"), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = (api.create_task_manager)();
            let mut rng = Rng::new(SEED);
            let mut prios: Vec<c_int> = vec![0, -1, 1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
            while prios.len() < 40 {
                prios.push(rng.i32());
            }
            for (i, p) in prios.iter().enumerate() {
                let d = cstr(format!("p{i}").as_bytes());
                (api.add_task)(m, d.as_ptr() as *const _, *p);
            }
            rec.manager(m);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        });
    });
    let printed = String::from_utf8_lossy(&obs.stdout);
    assert!(printed.contains(&format!("(Priority: {})\n", i32::MIN)));
    assert!(printed.contains(&format!("(Priority: {})\n", i32::MAX)));
}

/// CONFIGS row 17 — S9/S10/S11: descriptions containing high bytes, `printf`
/// metacharacters, control bytes, and (systematically) every non-NUL byte value.
fn cfg_17_tm_byte_range_descriptions() {
    let _g = lock();

    // 1) hand-picked awkward payloads
    let payloads: Vec<Vec<u8>> = vec![
        b"%s".to_vec(),
        b"%d %d %d".to_vec(),
        b"%n".to_vec(),
        b"100%".to_vec(),
        b"%%%%".to_vec(),
        b"tab\there".to_vec(),
        b"cr\rhere".to_vec(),
        b"bell\x07vt\x0bff\x0c".to_vec(),
        "caf\u{e9} \u{4e2d}\u{6587} \u{1f600}".as_bytes().to_vec(),
        vec![0xFFu8; 10],
        vec![0x80u8, 0x81, 0xC0, 0xC1, 0xFE, 0xFF],
    ];
    diff_locked("cfg_17 payloads", &Cfg::fresh().max("32"), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = (api.create_task_manager)();
            for (i, p) in payloads.iter().enumerate() {
                let d = cstr(p);
                (api.add_task)(m, d.as_ptr() as *const _, i as c_int);
            }
            rec.manager(m);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        });
    });

    // 2) every single byte value 0x01..=0xFF, one per task, in chunks
    for chunk in (1u16..=255).collect::<Vec<_>>().chunks(32) {
        let bytes: Vec<u8> = chunk.iter().map(|&b| b as u8).collect();
        let label = format!("cfg_17 bytes {:#04x}..", bytes[0]);
        diff_locked(&label, &Cfg::fresh().max("32"), |api, rec| {
            with_logger!(api, rec, unsafe {
                let m = (api.create_task_manager)();
                for (i, b) in bytes.iter().enumerate() {
                    let d = cstr(&[*b, *b, *b]);
                    (api.add_task)(m, d.as_ptr() as *const _, i as c_int);
                }
                rec.manager(m);
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
            });
        });
    }
}

/// CONFIGS row 18 — `print_tasks` on a **caller-crafted** `TaskManager`
/// (test-owned memory), covering T0..T3 and randomized contents. This pins the
/// struct layout as seen from an external caller.
fn cfg_18_print_caller_struct_random() {
    let _g = lock();
    const SEED: u64 = 0xB0B1_C0DE_1234_0018;

    for (max_tasks, task_count) in [
        (0i32, 0i32),
        (1, 0),
        (1, 1),
        (8, 1),
        (8, 3),
        (8, 8),
        (32, 32),
        (64, 5),
    ] {
        let label = format!("cfg_18 max={max_tasks} count={task_count}");
        diff_locked(&label, &Cfg::fresh(), |api, rec| {
            with_logger!(api, rec, unsafe {
                let mut rng = Rng::new(SEED ^ (max_tasks as u64) << 20 ^ task_count as u64);
                let m = craft_manager(max_tasks, task_count, 0);
                for i in 0..task_count.max(0) as usize {
                    let len = rng.range(0, 400);
                    let text = rng.text(len);
                    set_slot(m, i, &text, rng.i32());
                }
                rec.manager(m);
                (api.print_tasks)(m);
                free_manager(m);
            });
        });
    }

    // A description that fills all 256 bytes with no NUL, immediately followed
    // by a zero `priority` — `%s` then stops exactly at the priority bytes.
    // Reads stay inside the 260-byte slot, so this is deterministic and is
    // exactly what the C code does.
    diff_locked("cfg_18 unterminated desc", &Cfg::fresh(), |api, rec| {
        with_logger!(api, rec, unsafe {
            let m = craft_manager(2, 1, 0);
            let d = (&raw mut (*(*m).tasks).description).cast::<u8>();
            for i in 0..DESC_LEN {
                *d.add(i) = b'A' + (i % 26) as u8;
            }
            (*(*m).tasks).priority = 0;
            rec.manager(m);
            (api.print_tasks)(m);
            free_manager(m);
        });
    });
}

/// CONFIGS row 19 — `add_task` on a **caller-crafted** `TaskManager`, including
/// a pre-set `task_count`, then `print_tasks`, then `destroy_task_manager`
/// (which frees the test's own `malloc` blocks).
fn cfg_19_add_caller_struct_random() {
    let _g = lock();
    const SEED: u64 = 0xB0B1_C0DE_1234_0019;

    for (max_tasks, start_count, adds) in [
        (1i32, 0i32, 1usize),
        (1, 1, 1),
        (2, 1, 3),
        (8, 0, 8),
        (8, 7, 4),
        (8, 4, 2),
        (16, 15, 5),
    ] {
        let label = format!("cfg_19 max={max_tasks} start={start_count} adds={adds}");
        diff_locked(&label, &Cfg::fresh(), |api, rec| {
            with_logger!(api, rec, unsafe {
                let mut rng = Rng::new(
                    SEED ^ (max_tasks as u64) << 40 ^ (start_count as u64) << 8 ^ adds as u64,
                );
                let m = craft_manager(max_tasks, start_count, 0);
                // deterministic contents for the pre-existing slots
                for i in 0..start_count.max(0) as usize {
                    set_slot(m, i, format!("pre-{i}").as_bytes(), -(i as c_int) - 1);
                }
                rec.manager(m);
                for _ in 0..adds {
                    let len = rng.range(0, 320);
                    let d = cstr(&rng.text(len));
                    (api.add_task)(m, d.as_ptr() as *const _, rng.i32());
                    rec.manager(m);
                }
                (api.print_tasks)(m);
                // hand the test's own malloc'd blocks to the library's free path
                (api.destroy_task_manager)(m);
            });
        });
    }
}

/// CONFIGS row 20 — several managers alive simultaneously, tasks added
/// round-robin, printed, and destroyed in a different order.
fn cfg_20_tm_multiple_managers() {
    let obs = diff("cfg_20", &Cfg::fresh().max("3"), |api, rec| {
        with_logger!(api, rec, unsafe {
            let a = (api.create_task_manager)();
            let b = (api.create_task_manager)();
            let c = (api.create_task_manager)();
            rec.ptr_is_null(a as *const u8);
            rec.ptr_is_null(b as *const u8);
            rec.ptr_is_null(c as *const u8);
            for i in 0..4i32 {
                for (k, m) in [a, b, c].iter().enumerate() {
                    let d = cstr(format!("m{k}-t{i}").as_bytes());
                    (api.add_task)(*m, d.as_ptr() as *const _, i * 10 + k as c_int);
                }
            }
            for m in [a, b, c] {
                rec.manager(m);
                (api.print_tasks)(m);
            }
            // destroy order differs from creation order
            (api.destroy_task_manager)(b);
            (api.destroy_task_manager)(c);
            (api.destroy_task_manager)(a);
        });
    });
    let printed = String::from_utf8_lossy(&obs.stdout);
    assert_eq!(printed.matches("Tasks:\n").count(), 3);
    // each manager kept exactly 3 of the 4 offered tasks
    assert_eq!(printed.lines().count(), 3 + 9);
}

// ---------------------------------------------------------------------------
// Single serialized entry point.
//
// The libtest harness writes its own "test NAME ... ok" progress lines to fd 1
// from the main thread while other test threads are still running. Because this
// harness temporarily redirects fd 1/fd 2 to capture what the *libraries* print,
// concurrently-running tests would pollute the capture. Exposing exactly one
// #[test] removes that race entirely; each scenario still reports itself through
// the label carried in the assertion message.
// ---------------------------------------------------------------------------
#[test]
fn phase_b_task_manager_all() {
    macro_rules! step { ($f:ident) => {{ eprintln!("--> {}", stringify!($f)); $f(); }} }
    step!(cfg_07_create_default_max);
    step!(cfg_08_create_numeric_max);
    step!(cfg_09_create_atoi_quirks);
    step!(cfg_10_create_atoi_overflow);
    step!(cfg_11_tm_zero_tasks);
    step!(cfg_12_tm_one_task);
    step!(cfg_13_tm_fill_to_limit_random);
    step!(cfg_14_tm_overflow_beyond_max);
    step!(cfg_15_tm_description_lengths);
    step!(cfg_16_tm_priority_extremes);
    step!(cfg_17_tm_byte_range_descriptions);
    step!(cfg_18_print_caller_struct_random);
    step!(cfg_19_add_caller_struct_random);
    step!(cfg_20_tm_multiple_managers);
}
