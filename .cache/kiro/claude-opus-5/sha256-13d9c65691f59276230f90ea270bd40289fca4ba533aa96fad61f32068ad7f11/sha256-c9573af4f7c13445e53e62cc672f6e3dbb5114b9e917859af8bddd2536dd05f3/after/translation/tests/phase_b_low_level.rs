//! Phase B — valid-path differential tests for the low-level `task_manager`
//! entry points (`CONFIGS.md` rows 15-24).
//!
//! These drive `create_task_manager` / `add_task` / `print_tasks` /
//! `destroy_task_manager` directly (not through `driver`), including states
//! that the `driver` wrapper cannot produce, such as a hand-built
//! `TaskManager` whose `task_count` disagrees with `max_tasks`.

mod common;

use common::{assert_same, c_atoi, cstring, Config, Rng, Task, TaskManager};
use std::ffi::{c_char, c_int};

const SEED: u64 = 0xBEEF_0F00_D15E_A5E;
const N: usize = 40;

/// `create` -> add `descs` -> `print` -> `destroy`, the canonical consumer flow.
fn pipeline(api: &common::Api, items: &[(Vec<u8>, i32)]) -> i64 {
    unsafe {
        let r = (api.initialize_logger)();
        let m = (api.create_task_manager)();
        if m.is_null() {
            (api.finalize_logger)();
            return -1000 + r as i64;
        }
        for (desc, prio) in items {
            (api.add_task)(m, desc.as_ptr() as *const c_char, *prio as c_int);
        }
        (api.print_tasks)(m);
        // Expose the struct fields too, not just the printed side effects.
        let observed = ((*m).max_tasks as i64) << 32 | ((*m).task_count as i64 & 0xffff_ffff);
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
        observed
    }
}

fn rand_items(rng: &mut Rng, count: usize, max_len: usize) -> Vec<(Vec<u8>, i32)> {
    (0..count)
        .map(|_| {
            let len = rng.below(max_len + 1);
            let body = rng.cstr_body(len);
            (cstring(&body), rng.priority())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Row 15: default capacity
// ---------------------------------------------------------------------------

#[test]
fn cfg15_manager_default_capacity() {
    // $MAX_TASKS unset -> max_tasks == 10, tasks array of 2600 bytes.
    assert_same("cfg15-empty", &Config::new(), |api| pipeline(api, &[]));

    let mut rng = Rng::new(SEED);
    for i in 0..N {
        let n = rng.below(11);
        let items = rand_items(&mut rng, n, 60);
        assert_same(&format!("cfg15-{i}"), &Config::new(), |api| {
            pipeline(api, &items)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 16: capacity sweep x fill level
// ---------------------------------------------------------------------------

#[test]
fn cfg16_manager_capacities() {
    let mut rng = Rng::new(SEED + 1);
    for cap in ["1", "2", "3", "10", "64", "1000"] {
        let capn = c_atoi(cap.as_bytes()) as usize;
        // 0, 1, cap-1, cap, cap+1, cap+3 and a few random fills.
        let mut fills: Vec<usize> = vec![0, 1, capn.saturating_sub(1), capn, capn + 1, capn + 3];
        for _ in 0..4 {
            fills.push(rng.below(capn + 4));
        }
        for (i, fill) in fills.into_iter().enumerate() {
            let items = rand_items(&mut rng, fill, 40);
            let cfg = Config::new().max_tasks(cap);
            assert_same(&format!("cfg16-{cap}-{i}"), &cfg, |api| {
                pipeline(api, &items)
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17-18: $MAX_TASKS parsing (atoi semantics)
// ---------------------------------------------------------------------------

#[test]
fn cfg17_max_tasks_atoi_quirks() {
    let mut rng = Rng::new(SEED + 2);
    for v in [
        "0", " 7", "\t7", "+7", "-0", "7x", "x7", "", "0x10", "007", "3.9", "  +12abc", "1e3",
        "\n5", "  ", "9 ", "12345",
    ] {
        let items = rand_items(&mut rng, 14, 50);
        let cfg = Config::new().max_tasks(v);
        assert_same(&format!("cfg17-{v:?}"), &cfg, |api| pipeline(api, &items));
    }
}

#[test]
fn cfg18_max_tasks_out_of_int_range() {
    // atoi == (int)strtol(...): saturates in `long`, then truncates to `int`.
    //   "2147483648"            -> INT_MIN
    //   "-2147483649"           -> INT_MAX
    //   "4294967296"            -> 0
    //   "99999999999999999999"  -> LONG_MAX -> -1
    //   "-99999999999999999999" -> LONG_MIN -> 0
    let mut rng = Rng::new(SEED + 3);
    for v in [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967296",
        "4294967306",
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775807",
        "9223372036854775808",
    ] {
        let expected = c_atoi(v.as_bytes());
        let items = rand_items(&mut rng, 6, 30);
        let cfg = Config::new().max_tasks(v);
        // Sanity: only capacities that make the tasks malloc succeed reach the
        // add_task path; the rest are covered as ERRORS.md rows 12-13.
        assert_same(&format!("cfg18-{v}-> {expected}"), &cfg, |api| {
            pipeline(api, &items)
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 19-22: description / priority shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg19_description_length_sweep() {
    // strncpy(task->description, description, 255) + description[255] = 0.
    // The interesting boundary is 254 / 255 / 256 / 257.
    let mut rng = Rng::new(SEED + 4);
    let mut lengths: Vec<usize> = (0..=257).collect();
    lengths.extend([300, 511, 512, 513, 1024, 4096]);

    // One task per length, in batches that fit the manager capacity.
    for (batch, chunk) in lengths.chunks(32).enumerate() {
        let items: Vec<(Vec<u8>, i32)> = chunk
            .iter()
            .map(|&len| {
                let body = rng.cstr_body(len);
                (cstring(&body), len as i32)
            })
            .collect();
        let cfg = Config::new().max_tasks("64");
        assert_same(&format!("cfg19-batch{batch}"), &cfg, |api| {
            pipeline(api, &items)
        });
    }

    // ...and each boundary length on its own, several random bodies each.
    for len in [0usize, 1, 254, 255, 256, 257] {
        for i in 0..8 {
            let body = rng.cstr_body(len);
            let items = vec![(cstring(&body), rng.priority())];
            assert_same(&format!("cfg19-len{len}-{i}"), &Config::new(), |api| {
                pipeline(api, &items)
            });
        }
    }
}

#[test]
fn cfg20_priority_values() {
    let mut rng = Rng::new(SEED + 5);
    let fixed = [0i32, 1, -1, i32::MIN, i32::MAX, -2147483647, 2147483646];
    for (i, p) in fixed.into_iter().enumerate() {
        let blen = rng.below(80);
        let body = rng.cstr_body(blen);
        let items = vec![(cstring(&body), p)];
        assert_same(&format!("cfg20-fixed{i}"), &Config::new(), |api| {
            pipeline(api, &items)
        });
    }
    for i in 0..N {
        let n = rng.range(1, 10);
        let items = rand_items(&mut rng, n, 90);
        assert_same(&format!("cfg20-rand{i}"), &Config::new(), |api| {
            pipeline(api, &items)
        });
    }
}

#[test]
fn cfg21_description_non_utf8() {
    let mut rng = Rng::new(SEED + 6);
    for i in 0..N {
        let count = rng.range(1, 10);
        let items: Vec<(Vec<u8>, i32)> = (0..count)
            .map(|_| {
                let len = rng.range(1, 400);
                let body: Vec<u8> = (0..len).map(|_| rng.range(0x80, 0xff) as u8).collect();
                (cstring(&body), rng.priority())
            })
            .collect();
        assert_same(&format!("cfg21-{i}"), &Config::new(), |api| {
            pipeline(api, &items)
        });
    }
}

#[test]
fn cfg22_description_format_specifiers() {
    let pieces: &[&[u8]] = &[b"%s", b"%d", b"%n", b"%p", b"%%", b"%999999d", b"%.*s", b"%hn"];
    let mut rng = Rng::new(SEED + 7);
    for i in 0..N {
        let count = rng.range(1, 10);
        let items: Vec<(Vec<u8>, i32)> = (0..count)
            .map(|_| {
                let mut body = Vec::new();
                for _ in 0..rng.range(1, 12) {
                    body.extend_from_slice(pieces[rng.below(pieces.len())]);
                }
                (cstring(&body), rng.priority())
            })
            .collect();
        assert_same(&format!("cfg22-{i}"), &Config::new(), |api| {
            pipeline(api, &items)
        });
    }
}

// ---------------------------------------------------------------------------
// Row 23: hand-built TaskManager fed straight to print_tasks
// ---------------------------------------------------------------------------

fn task_with(bytes: &[u8], terminate: bool, priority: i32) -> Task {
    let mut t = Task {
        description: [0; 256],
        priority,
    };
    let n = bytes.len().min(if terminate { 255 } else { 256 });
    for i in 0..n {
        t.description[i] = bytes[i] as c_char;
    }
    t
}

#[test]
fn cfg23_print_tasks_handbuilt() {
    let mut rng = Rng::new(SEED + 8);

    for i in 0..N {
        let n = rng.range(1, 12);
        let mut tasks: Vec<Task> = Vec::new();
        for _ in 0..n {
            let len = rng.below(300);
            let body = rng.cstr_body(len);
            tasks.push(task_with(&body, true, rng.priority()));
        }
        // task_count deliberately independent of max_tasks, including
        // task_count == 0, task_count < 0 (loop body never runs) and
        // task_count < tasks.len().
        let variants: [(i32, i32); 6] = [
            (n as i32, n as i32),
            (0, 10),
            (1, 0),
            (-1, 10),
            (i32::MIN, 10),
            ((n as i32) - 1, 1),
        ];
        for (vi, (task_count, max_tasks)) in variants.into_iter().enumerate() {
            let mut tm = TaskManager {
                tasks: tasks.as_mut_ptr(),
                max_tasks,
                task_count,
            };
            let p: *const TaskManager = &tm;
            assert_same(&format!("cfg23-{i}-{vi}"), &Config::new(), |api| unsafe {
                let r = (api.initialize_logger)();
                (api.print_tasks)(p);
                (api.finalize_logger)();
                r as i64
            });
            let _ = &mut tm;
        }
    }

    // A description that fills all 256 bytes (so `strncpy`'s NUL padding never
    // happened).  `priority = 0` makes the four bytes that follow the array
    // zero, so glibc's `%s` terminates deterministically at the field boundary
    // in both builds -- this is exactly what the C would do.
    for i in 0..8 {
        let body = rng.cstr_body(256);
        let mut tasks = vec![task_with(&body, false, 0)];
        let mut tm = TaskManager {
            tasks: tasks.as_mut_ptr(),
            max_tasks: 10,
            task_count: 1,
        };
        let p: *const TaskManager = &tm;
        assert_same(&format!("cfg23-unterminated-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            (api.print_tasks)(p);
            (api.finalize_logger)();
            r as i64
        });
        let _ = &mut tm;
        let _ = &mut tasks;
    }

    // An embedded NUL: `%s` stops at it even though later bytes are non-zero.
    for i in 0..8 {
        let mut body = rng.cstr_body(200);
        body[rng.range(1, 199)] = 0;
        let mut t = Task {
            description: [0; 256],
            priority: rng.priority(),
        };
        for (j, b) in body.iter().enumerate() {
            t.description[j] = *b as c_char;
        }
        let mut tasks = vec![t];
        let mut tm = TaskManager {
            tasks: tasks.as_mut_ptr(),
            max_tasks: 10,
            task_count: 1,
        };
        let p: *const TaskManager = &tm;
        assert_same(&format!("cfg23-embnul-{i}"), &Config::new(), |api| unsafe {
            let r = (api.initialize_logger)();
            (api.print_tasks)(p);
            (api.finalize_logger)();
            r as i64
        });
        let _ = &mut tm;
        let _ = &mut tasks;
    }
}

// ---------------------------------------------------------------------------
// Row 24: several full pipelines in one process (state carry-over)
// ---------------------------------------------------------------------------

#[test]
fn cfg24_repeated_pipelines() {
    let mut rng = Rng::new(SEED + 9);
    for i in 0..N {
        let na = rng.below(12);
        let a = rand_items(&mut rng, na, 70);
        let nb = rng.below(12);
        let b = rand_items(&mut rng, nb, 70);
        let nc = rng.below(12);
        let c = rand_items(&mut rng, nc, 70);
        assert_same(&format!("cfg24-{i}"), &Config::new(), |api| unsafe {
            let mut acc = 0i64;
            let r = (api.initialize_logger)();
            acc += r as i64;
            for batch in [&a, &b, &c] {
                let m = (api.create_task_manager)();
                assert!(!m.is_null());
                for (d, p) in batch.iter() {
                    (api.add_task)(m, d.as_ptr() as *const c_char, *p as c_int);
                }
                (api.print_tasks)(m);
                acc = acc * 31 + (*m).task_count as i64;
                (api.destroy_task_manager)(m);
            }
            (api.finalize_logger)();
            acc
        });
    }
}
