//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH `.so`s through `libloading` and compares stdout,
//! stderr, the log-file bytes and the reported observations (return codes and
//! raw `TaskManager` / `Task` memory) byte-for-byte.
//!
//! Rows C1..C28 exercise the LOW-LEVEL entry points directly (hand-composed
//! sequences, not the `driver` wrapper); rows C29..C40 exercise the wrapper.

mod common;

use common::*;
use std::ffi::c_int;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

fn app() -> LogCfg {
    LogCfg::InDir("app.log".into())
}

// =========================================================================
// Logger — low-level entry points
// =========================================================================

/// C1 — `initialize_logger` with a fresh writable `LOG_FILE`.
#[test]
fn c01_initialize_logger_fresh_path() {
    let _g = lock();
    diff("C1", app(), None, |api, ctx| {
        let rc = unsafe { (api.initialize_logger)() };
        let existed = ctx.log_path.is_file();
        unsafe { (api.finalize_logger)() };
        format!("init={rc};file_created={existed}")
    });
}

/// C2 — `LOG_FILE` unset → the default `"default.log"` relative to CWD.
#[test]
fn c02_initialize_logger_default_path() {
    let _g = lock();
    diff("C2", LogCfg::Unset, None, |api, ctx| {
        let rc = unsafe { (api.initialize_logger)() };
        let existed = ctx.dir.join("default.log").is_file();
        unsafe { (api.log_info)(cs(b"after default open").as_ptr()) };
        unsafe { (api.finalize_logger)() };
        format!("init={rc};default_log_created={existed}")
    });
}

/// C3 — `"a"` append mode must preserve pre-existing content.
#[test]
fn c03_append_mode_preserves_prefix() {
    let _g = lock();
    diff(
        "C3",
        LogCfg::Seeded("app.log".into(), b"PREEXISTING LINE 1\nPREEXISTING 2\n".to_vec()),
        None,
        |api, _| {
            let rc = unsafe { (api.initialize_logger)() };
            unsafe { (api.log_warning)(cs(b"appended").as_ptr()) };
            unsafe { (api.finalize_logger)() };
            format!("init={rc}")
        },
    );
}

/// C4 — `initialize_logger` called twice (the first stream is leaked, the
/// static is overwritten; both calls must still return 0).
#[test]
fn c04_initialize_logger_twice() {
    let _g = lock();
    diff("C4", app(), None, |api, _| {
        let a = unsafe { (api.initialize_logger)() };
        let b = unsafe { (api.initialize_logger)() };
        unsafe { (api.log_info)(cs(b"after second init").as_ptr()) };
        unsafe { (api.finalize_logger)() };
        format!("init1={a};init2={b}")
    });
}

/// C5 — all three `log_*` functions while `log_file == NULL`: silent no-ops.
#[test]
fn c05_log_functions_uninitialised() {
    let _g = lock();
    diff("C5", app(), None, |api, ctx| {
        let mut rng = Rng::new(SEED ^ 5);
        for _ in 0..32 {
            let m = cs(&rng.ascii_below(64));
            unsafe {
                (api.log_info)(m.as_ptr());
                (api.log_warning)(m.as_ptr());
                (api.log_error)(m.as_ptr());
            }
        }
        format!("log_file_created={}", ctx.log_path.exists())
    });
}

fn log_many(which: usize, tag: u64) -> impl Fn(&Api, &Ctx) -> String {
    move |api: &Api, _: &Ctx| {
        let mut rng = Rng::new(SEED ^ tag);
        let rc = unsafe { (api.initialize_logger)() };
        for _ in 0..200 {
            let len = rng.below(201);
            let m = cs(&rng.ascii(len));
            let f = match which {
                0 => api.log_info,
                1 => api.log_warning,
                _ => api.log_error,
            };
            unsafe { f(m.as_ptr()) };
        }
        unsafe { (api.finalize_logger)() };
        format!("init={rc}")
    }
}

/// C6 — 200 randomized `log_info` messages.
#[test]
fn c06_log_info_random() {
    let _g = lock();
    diff("C6", app(), None, log_many(0, 6));
}

/// C7 — 200 randomized `log_warning` messages.
#[test]
fn c07_log_warning_random() {
    let _g = lock();
    diff("C7", app(), None, log_many(1, 7));
}

/// C8 — 200 randomized `log_error` messages.
#[test]
fn c08_log_error_random() {
    let _g = lock();
    diff("C8", app(), None, log_many(2, 8));
}

/// C9 — 300 interleaved calls across all three severities (ordering + buffering).
#[test]
fn c09_log_interleaved_random() {
    let _g = lock();
    diff("C9", app(), None, |api, _| {
        let mut rng = Rng::new(SEED ^ 9);
        let rc = unsafe { (api.initialize_logger)() };
        for _ in 0..300 {
            let len = rng.below(120);
            let m = cs(&rng.ascii(len));
            match rng.below(3) {
                0 => unsafe { (api.log_info)(m.as_ptr()) },
                1 => unsafe { (api.log_warning)(m.as_ptr()) },
                _ => unsafe { (api.log_error)(m.as_ptr()) },
            }
        }
        unsafe { (api.finalize_logger)() };
        format!("init={rc}")
    });
}

/// C10 — empty message, and a message full of `printf` conversion specifiers
/// (which must appear literally because it is a `%s` argument).
#[test]
fn c10_log_empty_and_format_specifiers() {
    let _g = lock();
    diff("C10", app(), None, |api, _| {
        let rc = unsafe { (api.initialize_logger)() };
        for m in [
            &b""[..],
            b"%s",
            b"%d %d %d",
            b"%%",
            b"%n",
            b"%1000000d",
            b"100%% done: %s/%p/%x",
        ] {
            let c = cs(m);
            unsafe {
                (api.log_info)(c.as_ptr());
                (api.log_warning)(c.as_ptr());
                (api.log_error)(c.as_ptr());
            }
        }
        unsafe { (api.finalize_logger)() };
        format!("init={rc}")
    });
}

/// C11 — messages of 4096 and 65536 bytes (cross glibc's stdio buffer).
#[test]
fn c11_log_very_long_messages() {
    let _g = lock();
    diff("C11", app(), None, |api, _| {
        let mut rng = Rng::new(SEED ^ 11);
        let rc = unsafe { (api.initialize_logger)() };
        for len in [4095usize, 4096, 4097, 8192, 65536] {
            let m = cs(&rng.ascii(len));
            unsafe { (api.log_info)(m.as_ptr()) };
        }
        unsafe { (api.finalize_logger)() };
        format!("init={rc}")
    });
}

/// C12 — non-UTF-8 / high-bit bytes and embedded control characters.
#[test]
fn c12_log_non_utf8_bytes() {
    let _g = lock();
    diff("C12", app(), None, |api, _| {
        let mut rng = Rng::new(SEED ^ 12);
        let rc = unsafe { (api.initialize_logger)() };
        // exhaustive single non-NUL byte messages
        for b in 1u8..=255 {
            let m = cs(&[b]);
            unsafe { (api.log_info)(m.as_ptr()) };
        }
        // and randomized full-byte-range blobs
        for _ in 0..64 {
            let len = rng.below(96);
            let m = cs(&rng.bytes_no_nl(len));
            unsafe { (api.log_error)(m.as_ptr()) };
        }
        let mixed = cs(b"tab\there\rcr\x1b[31mesc\xff\xfe\x80end");
        unsafe { (api.log_warning)(mixed.as_ptr()) };
        unsafe { (api.finalize_logger)() };
        format!("init={rc}")
    });
}

/// C13 — full open → finalize cycle; `fclose` must flush the trailer.
#[test]
fn c13_initialize_then_finalize() {
    let _g = lock();
    diff("C13", app(), None, |api, _| {
        let rc = unsafe { (api.initialize_logger)() };
        unsafe { (api.finalize_logger)() };
        format!("init={rc}")
    });
}

// =========================================================================
// TaskManager — low-level entry points
// =========================================================================

/// C14 — `MAX_TASKS` unset → capacity 10.
#[test]
fn c14_create_default_capacity() {
    let _g = lock();
    diff("C14", app(), None, |api, _| {
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        }
    });
}

/// C15 — `MAX_TASKS` = 16 random values in `1..=64`.
#[test]
fn c15_create_random_capacity() {
    let _g = lock();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..16 {
        let n = 1 + rng.below(64);
        let s = n.to_string();
        diff(&format!("C15/MAX_TASKS={s}"), app(), Some(&s), |api, _| {
            unsafe {
                let rc = (api.initialize_logger)();
                let m = (api.create_task_manager)();
                let obs = format!("init={rc};{}", dump_manager(m));
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
                obs
            }
        });
    }
}

/// C16 — `MAX_TASKS=0`: `malloc(0)` still succeeds, creation succeeds.
#[test]
fn c16_create_zero_capacity() {
    let _g = lock();
    diff("C16", app(), Some("0"), |api, _| {
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        }
    });
}

/// C17 — every `atoi` input shape the C can be handed via `MAX_TASKS`.
#[test]
fn c17_create_atoi_shapes() {
    let _g = lock();
    for v in [
        " 7", "\t9", "+3", "0012", "12abc", "abc", "3.9", "", "  ", "-0", "1e3", "2147483647",
        "0x10", "  -4 ", "9 9",
    ] {
        diff(&format!("C17/MAX_TASKS={v:?}"), app(), Some(v), |api, _| {
            unsafe {
                let rc = (api.initialize_logger)();
                let m = (api.create_task_manager)();
                let obs = format!("init={rc};{}", dump_manager(m));
                if !m.is_null() {
                    (api.destroy_task_manager)(m);
                }
                (api.finalize_logger)();
                obs
            }
        });
    }
}

/// C18 — create/destroy while the logger is uninitialised: succeeds, logs nothing.
#[test]
fn c18_create_without_logger() {
    let _g = lock();
    diff("C18", app(), Some("4"), |api, ctx| {
        unsafe {
            let m = (api.create_task_manager)();
            let obs = dump_manager(m);
            let d = cs(b"no logger");
            (api.add_task)(m, d.as_ptr(), 42);
            let obs2 = dump_manager(m);
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            format!("{obs}\n{obs2}\nlog_exists={}", ctx.log_path.exists())
        }
    });
}

/// C19 — add `k` tasks for every `k` in `0..=10` at capacity 10, comparing the
/// raw 260-byte `Task` records.
#[test]
fn c19_add_task_counts_0_to_10() {
    let _g = lock();
    for k in 0..=10usize {
        diff(&format!("C19/k={k}"), app(), Some("10"), move |api, _| {
            let mut rng = Rng::new(SEED ^ 19 ^ (k as u64) << 32);
            unsafe {
                let rc = (api.initialize_logger)();
                let m = (api.create_task_manager)();
                for _ in 0..k {
                    let d = cs(&rng.ascii_below(40));
                    let p = rng.next_i32();
                    (api.add_task)(m, d.as_ptr(), p);
                }
                let obs = format!("init={rc};{}", dump_manager(m));
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
                obs
            }
        });
    }
}

/// C20 — description length exactly at / around the 255-byte truncation edge.
#[test]
fn c20_add_task_description_truncation() {
    let _g = lock();
    diff("C20", app(), Some("16"), |api, _| {
        let mut rng = Rng::new(SEED ^ 20);
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            for len in [0usize, 1, 253, 254, 255, 256, 257, 300, 512, 1024] {
                let d = cs(&rng.ascii(len));
                (api.add_task)(m, d.as_ptr(), len as c_int);
            }
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        }
    });
}

/// C21 — empty / format-specifier / high-byte / embedded-newline descriptions.
#[test]
fn c21_add_task_special_descriptions() {
    let _g = lock();
    diff("C21", app(), Some("16"), |api, _| {
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            for d in [
                &b""[..],
                b"%d",
                b"%s%s%s",
                b"100%%",
                b"embedded\nnewline",
                b"tab\tsep",
                b"\xff\xfe\x80\x01\x7f",
                b"trailing space ",
                b" leading space",
            ] {
                let c = cs(d);
                (api.add_task)(m, c.as_ptr(), 7);
            }
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        }
    });
}

/// C22 — the full `int` domain for `priority`, including both extremes.
#[test]
fn c22_add_task_priority_extremes() {
    let _g = lock();
    diff("C22", app(), Some("64"), |api, _| {
        let mut rng = Rng::new(SEED ^ 22);
        let mut ps: Vec<c_int> = vec![c_int::MIN, c_int::MIN + 1, -1, 0, 1, c_int::MAX - 1, c_int::MAX];
        for _ in 0..32 {
            ps.push(rng.next_i32());
        }
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            for (i, p) in ps.iter().enumerate() {
                let d = cs(format!("prio-{i}").as_bytes());
                (api.add_task)(m, d.as_ptr(), *p);
            }
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        }
    });
}

/// C23 — fill to exactly `max_tasks` for several capacities (the accept/reject
/// boundary), then one more (rejected).
#[test]
fn c23_add_task_capacity_boundary() {
    let _g = lock();
    for cap in [1usize, 2, 10, 64] {
        let s = cap.to_string();
        diff(&format!("C23/cap={cap}"), app(), Some(&s), move |api, _| {
            let mut rng = Rng::new(SEED ^ 23 ^ (cap as u64) << 16);
            unsafe {
                let rc = (api.initialize_logger)();
                let m = (api.create_task_manager)();
                let mut steps = String::new();
                for i in 0..cap + 3 {
                    let d = cs(&rng.ascii_1_to(20));
                    (api.add_task)(m, d.as_ptr(), i as c_int);
                    steps += &format!("after {i}: count={};", (*m).task_count);
                }
                let obs = format!("init={rc};{steps}\n{}", dump_manager(m));
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
                obs
            }
        });
    }
}

/// C24 — `print_tasks` on an empty manager → exactly `Tasks:\n`.
#[test]
fn c24_print_tasks_empty() {
    let _g = lock();
    diff("C24", app(), Some("10"), |api, _| {
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            (api.print_tasks)(m);
            (api.print_tasks)(m); // twice: idempotent
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        }
    });
}

/// C25 — `print_tasks` with 1 / 2 / 10 / 64 randomized tasks.
#[test]
fn c25_print_tasks_many() {
    let _g = lock();
    for n in [1usize, 2, 10, 64] {
        let s = n.to_string();
        diff(&format!("C25/n={n}"), app(), Some(&s), move |api, _| {
            let mut rng = Rng::new(SEED ^ 25 ^ (n as u64) << 8);
            unsafe {
                let rc = (api.initialize_logger)();
                let m = (api.create_task_manager)();
                for _ in 0..n {
                    let d = cs(&rng.bytes_below(80));
                    (api.add_task)(m, d.as_ptr(), rng.next_i32());
                }
                (api.print_tasks)(m);
                let obs = format!("init={rc};{}", dump_manager(m));
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
                obs
            }
        });
    }
}

/// C26 — `print_tasks` must print the *truncated* 255-byte descriptions.
#[test]
fn c26_print_tasks_truncated() {
    let _g = lock();
    diff("C26", app(), Some("8"), |api, _| {
        let mut rng = Rng::new(SEED ^ 26);
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            for len in [255usize, 256, 300, 1000] {
                let d = cs(&rng.ascii(len));
                (api.add_task)(m, d.as_ptr(), len as c_int);
            }
            (api.print_tasks)(m);
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        }
    });
}

/// C27 — `destroy_task_manager` logs when the logger is open, and logs nothing
/// when it was never opened.
#[test]
fn c27_destroy_logging() {
    let _g = lock();
    // (a) logger open
    diff("C27a", app(), Some("2"), |api, _| {
        unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            format!("init={rc}")
        }
    });
    // (b) logger never opened
    diff("C27b", app(), Some("2"), |api, ctx| {
        unsafe {
            let m = (api.create_task_manager)();
            (api.destroy_task_manager)(m);
            format!("log_exists={}", ctx.log_path.exists())
        }
    });
}

/// C28 — the FULL hand-composed low-level pipeline (never via `driver`),
/// randomized 64 times over capacity, task count, descriptions and priorities.
#[test]
fn c28_low_level_pipeline_randomized() {
    let _g = lock();
    let mut outer = Rng::new(SEED ^ 28);
    for iter in 0..64u64 {
        let caps = ["0", "1", "2", "3", "7", "10", "64"];
        let cap = caps[outer.below(caps.len())];
        let n = outer.below(20);
        diff(
            &format!("C28/iter={iter},cap={cap},n={n}"),
            app(),
            Some(cap),
            move |api, _| {
                let mut rng = Rng::new(SEED ^ 0xC28 ^ iter << 20);
                unsafe {
                    let mut obs = format!("init={};", (api.initialize_logger)());
                    let m = (api.create_task_manager)();
                    if m.is_null() {
                        (api.finalize_logger)();
                        return obs + "manager=NULL";
                    }
                    for i in 0..n {
                        let len = rng.below(300);
                        let d = cs(&rng.bytes_no_nl(len));
                        (api.add_task)(m, d.as_ptr(), rng.next_i32());
                        if rng.below(4) == 0 {
                            (api.print_tasks)(m);
                        }
                        if rng.below(5) == 0 {
                            let msg = cs(format!("checkpoint {i}").as_bytes());
                            (api.log_warning)(msg.as_ptr());
                        }
                    }
                    (api.print_tasks)(m);
                    obs += &dump_manager(m);
                    (api.destroy_task_manager)(m);
                    (api.finalize_logger)();
                    obs
                }
            },
        );
    }
}

// =========================================================================
// driver — the one-shot wrapper
// =========================================================================

fn drive(input: &'static [u8]) -> impl Fn(&Api, &Ctx) -> String {
    move |api: &Api, _: &Ctx| {
        let c = cs(input);
        format!("driver={}", unsafe { (api.driver)(c.as_ptr()) })
    }
}

/// C29 — empty input.
#[test]
fn c29_driver_empty_input() {
    let _g = lock();
    diff("C29", app(), Some("10"), drive(b""));
}

/// C30 — one line without a trailing newline (`strchr` → NULL branch).
#[test]
fn c30_driver_single_line_no_newline() {
    let _g = lock();
    diff("C30", app(), Some("10"), drive(b"only task"));
}

/// C31 — one line with a trailing newline (`*end == '\n'` → `end + 1`).
#[test]
fn c31_driver_single_line_trailing_newline() {
    let _g = lock();
    diff("C31", app(), Some("10"), drive(b"only task\n"));
}

/// C32 — several lines, with and without a trailing newline.
#[test]
fn c32_driver_multiple_lines() {
    let _g = lock();
    diff("C32a", app(), Some("10"), drive(b"a\nbb\nccc"));
    diff("C32b", app(), Some("10"), drive(b"a\nbb\nccc\n"));
    diff("C32c", app(), Some("10"), drive(b"first line\nsecond line\nthird line\nfourth"));
}

/// C33 — consecutive newlines: empty tasks that still consume a priority.
#[test]
fn c33_driver_consecutive_newlines() {
    let _g = lock();
    diff("C33a", app(), Some("10"), drive(b"a\n\nb"));
    diff("C33b", app(), Some("10"), drive(b"a\n\n\n\nb\n"));
    diff("C33c", app(), Some("10"), drive(b"\n\na\n\n"));
}

/// C34 — newline-only inputs and leading newlines.
#[test]
fn c34_driver_newline_only() {
    let _g = lock();
    diff("C34a", app(), Some("10"), drive(b"\n"));
    diff("C34b", app(), Some("10"), drive(b"\n\n\n"));
    diff("C34c", app(), Some("10"), drive(b"\nleading"));
}

/// C35 — more lines than capacity (capacity 0 / 1 / 3, eight lines).
#[test]
fn c35_driver_over_capacity() {
    let _g = lock();
    for cap in ["0", "1", "3"] {
        diff(
            &format!("C35/cap={cap}"),
            app(),
            Some(cap),
            drive(b"l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8"),
        );
    }
}

/// C36 — a line longer than the 255-byte description field.
#[test]
fn c36_driver_long_line() {
    let _g = lock();
    diff("C36", app(), Some("10"), |api, _| {
        let mut rng = Rng::new(SEED ^ 36);
        let mut input = Vec::new();
        for len in [254usize, 255, 256, 300, 1024] {
            input.extend_from_slice(&rng.ascii(len));
            input.push(b'\n');
        }
        input.extend_from_slice(&rng.ascii(2048));
        let c = cs(&input);
        format!("driver={}", unsafe { (api.driver)(c.as_ptr()) })
    });
}

/// C37 — the default capacity boundary through the wrapper (9 / 10 / 11 lines).
#[test]
fn c37_driver_default_capacity_boundary() {
    let _g = lock();
    for n in [9usize, 10, 11, 12] {
        diff(&format!("C37/n={n}"), app(), None, move |api, _| {
            let mut input = Vec::new();
            for i in 0..n {
                if i > 0 {
                    input.push(b'\n');
                }
                input.extend_from_slice(format!("task number {i}").as_bytes());
            }
            let c = cs(&input);
            format!("driver={}", unsafe { (api.driver)(c.as_ptr()) })
        });
    }
}

/// C38 — 64 randomized `driver` runs: random line counts, random line lengths
/// (including empty lines), random capacity.
#[test]
fn c38_driver_randomized() {
    let _g = lock();
    let mut outer = Rng::new(SEED ^ 38);
    for iter in 0..64u64 {
        let caps: [Option<&str>; 6] = [None, Some("0"), Some("1"), Some("5"), Some("10"), Some("64")];
        let cap = caps[outer.below(caps.len())];
        diff(
            &format!("C38/iter={iter},cap={cap:?}"),
            app(),
            cap,
            move |api, _| {
                let mut rng = Rng::new(SEED ^ 0xD38 ^ iter << 24);
                let lines = rng.below(21);
                let mut input: Vec<u8> = Vec::new();
                for i in 0..lines {
                    if i > 0 {
                        input.push(b'\n');
                    }
                    let len = match rng.below(6) {
                        0 => 0,
                        1 => 1 + rng.below(8),
                        2 => 250 + rng.below(10),
                        _ => rng.below(60),
                    };
                    input.extend_from_slice(&rng.bytes_no_nl(len));
                }
                if rng.below(2) == 0 {
                    input.push(b'\n');
                }
                let c = cs(&input);
                format!(
                    "lines={lines};input_len={};driver={}",
                    input.len(),
                    unsafe { (api.driver)(c.as_ptr()) }
                )
            },
        );
    }
}

/// C39 — `driver` end to end with `LOG_FILE` unset (`default.log`).
#[test]
fn c39_driver_default_log_file() {
    let _g = lock();
    diff("C39", LogCfg::Unset, Some("4"), |api, ctx| {
        let c = cs(b"alpha\nbeta\ngamma\ndelta\nepsilon");
        let rc = unsafe { (api.driver)(c.as_ptr()) };
        format!(
            "driver={rc};default_log={}",
            ctx.dir.join("default.log").is_file()
        )
    });
}

/// C40 — two `driver` calls in one process (re-init, log accumulation).
#[test]
fn c40_driver_twice() {
    let _g = lock();
    diff("C40", app(), Some("3"), |api, _| {
        let a = cs(b"one\ntwo");
        let b = cs(b"three\nfour\nfive\nsix");
        let ra = unsafe { (api.driver)(a.as_ptr()) };
        let rb = unsafe { (api.driver)(b.as_ptr()) };
        format!("driver1={ra};driver2={rb}")
    });
}
