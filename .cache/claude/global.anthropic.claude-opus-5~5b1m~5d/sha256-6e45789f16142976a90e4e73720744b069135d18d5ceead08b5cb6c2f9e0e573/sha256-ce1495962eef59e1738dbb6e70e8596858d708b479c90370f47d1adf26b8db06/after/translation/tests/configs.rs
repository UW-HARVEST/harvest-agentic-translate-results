//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH `.so`s through `libloading` only, and the harness
//! compares return values, `TaskManager`/`Task` memory, `stdout`, `stderr` and
//! every produced file byte-for-byte.
//!
//! Run with `-- --test-threads=1` (process-global state: env, fds, cwd).

mod common;

use common::*;
use std::os::raw::c_char;

// ------------------------------------------------------------------ helpers --

/// Point `LOG_FILE` at this side's private log and open the logger.
fn open_logger(api: &Api, side: &Side, rec: &mut Record) {
    set_env_path("LOG_FILE", &side.log_path());
    let rc = unsafe { (api.initialize_logger)() };
    rec.kv("initialize_logger", rc);
    assert_eq!(rc, 0, "logger should open for {:?}", side.log_path());
}

fn set_max_tasks(v: Option<&[u8]>) {
    match v {
        Some(b) => set_env("MAX_TASKS", b),
        None => unset_env("MAX_TASKS"),
    }
}

fn add(api: &Api, m: *mut TaskManager, desc: &[u8], prio: i32) {
    let c = cstr(desc);
    unsafe { (api.add_task)(m, c.as_ptr(), prio) };
}

fn log_all(api: &Api, msg: &[u8]) {
    let c = cstr(msg);
    unsafe {
        (api.log_info)(c.as_ptr());
        (api.log_warning)(c.as_ptr());
        (api.log_error)(c.as_ptr());
    }
}

fn run_driver(api: &Api, input: &[u8], rec: &mut Record, tag: &str) {
    let c = cstr(input);
    let d = api.driver();
    let rc = unsafe { d(c.as_ptr()) };
    rec.kv(tag, rc);
}

// ================================================== rows 1-5: LOG_FILE axis ==

/// Row 1 — `LOG_FILE` unset: the C falls back to the relative path
/// `"default.log"`, i.e. it lands in the process cwd.
#[test]
fn row01_log_file_unset_uses_default_log_in_cwd() {
    differential("row01_default_log", |api, side, rec| {
        unset_env("LOG_FILE");
        let saved = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&side.dir).expect("chdir");
        let rc = unsafe { (api.initialize_logger)() };
        rec.kv("initialize_logger", rc);
        unsafe {
            (api.log_info)(c"hello".as_ptr());
            (api.finalize_logger)();
        }
        std::env::set_current_dir(saved).expect("restore cwd");
    });
}

/// Row 2 — `LOG_FILE` = a fresh path that does not exist yet.
#[test]
fn row02_log_file_fresh_path() {
    differential("row02_fresh", |api, side, rec| {
        open_logger(api, side, rec);
        log_all(api, b"fresh path message");
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 3 — `LOG_FILE` = an existing file: `fopen(.., "a")` must append.
#[test]
fn row03_log_file_appends_to_existing() {
    differential("row03_append", |api, side, rec| {
        std::fs::write(side.log_path(), b"PREEXISTING\nBYTES\n").unwrap();
        open_logger(api, side, rec);
        log_all(api, b"appended");
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 4 — `initialize_logger` twice on the same path (the first `FILE*` is
/// leaked, `log_file` is overwritten), then a single `finalize_logger`.
#[test]
fn row04_double_initialize_same_path() {
    differential("row04_double_init", |api, side, rec| {
        open_logger(api, side, rec);
        unsafe { (api.log_info)(c"between".as_ptr()) };
        let rc2 = unsafe { (api.initialize_logger)() };
        rec.kv("initialize_logger#2", rc2);
        unsafe {
            (api.log_info)(c"after re-init".as_ptr());
            (api.finalize_logger)();
        }
    });
}

/// Row 5 — `initialize_logger` twice with a *different* `LOG_FILE` in between.
#[test]
fn row05_double_initialize_different_path() {
    differential("row05_reinit_other", |api, side, rec| {
        open_logger(api, side, rec);
        unsafe { (api.log_info)(c"goes to first".as_ptr()) };
        set_env_path("LOG_FILE", &side.path("second.log"));
        let rc2 = unsafe { (api.initialize_logger)() };
        rec.kv("initialize_logger#2", rc2);
        unsafe {
            (api.log_warning)(c"goes to second".as_ptr());
            (api.finalize_logger)();
        }
    });
}

// ============================================ rows 6-7: log_* message shapes ==

/// Row 6 — 200 randomised messages through each of the three tag functions.
#[test]
fn row06_randomised_log_messages() {
    differential("row06_rand_msgs", |api, side, rec| {
        open_logger(api, side, rec);
        let mut rng = Rng::new(0xC0FFEE_1234);
        let alpha = alphabet_wide();
        for i in 0..200u32 {
            let len = match i % 7 {
                0 => 0,
                1 => 1,
                2 => rng.below(32) as usize,
                3 => 255,
                4 => 256,
                5 => rng.below(512) as usize,
                _ => rng.below(4096) as usize,
            };
            let msg = rng.bytes(len, &alpha);
            let c = cstr(&msg);
            unsafe {
                match i % 3 {
                    0 => (api.log_info)(c.as_ptr()),
                    1 => (api.log_warning)(c.as_ptr()),
                    _ => (api.log_error)(c.as_ptr()),
                }
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 7 — the three tag functions interleaved in randomised order.
#[test]
fn row07_interleaved_log_levels() {
    differential("row07_interleaved", |api, side, rec| {
        open_logger(api, side, rec);
        let mut rng = Rng::new(0xABCD_1111);
        let alpha = alphabet_wide();
        for _ in 0..300 {
            let n = rng.below(40) as usize + 1;
            let msg = rng.bytes(n, &alpha);
            let c = cstr(&msg);
            unsafe {
                match rng.below(3) {
                    0 => (api.log_info)(c.as_ptr()),
                    1 => (api.log_warning)(c.as_ptr()),
                    _ => (api.log_error)(c.as_ptr()),
                }
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

// ================================== rows 8-15: create_task_manager / MAX_TASKS ==

fn create_destroy_row(label: &str, max_tasks: Option<&'static [u8]>) {
    differential(label, |api, side, rec| {
        open_logger(api, side, rec);
        set_max_tasks(max_tasks);
        let m = unsafe { (api.create_task_manager)() };
        rec.manager("manager", m);
        if !m.is_null() {
            unsafe { (api.destroy_task_manager)(m) };
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 8 — `MAX_TASKS` unset → the hard-coded default of 10.
#[test]
fn row08_max_tasks_default_is_ten() {
    create_destroy_row("row08_default_10", None);
}

/// Row 9 — `MAX_TASKS=0` → `malloc(0)`; the manager is "full" immediately.
#[test]
fn row09_max_tasks_zero() {
    create_destroy_row("row09_zero", Some(b"0"));
}

/// Row 10 — `MAX_TASKS=1`.
#[test]
fn row10_max_tasks_one() {
    create_destroy_row("row10_one", Some(b"1"));
}

/// Row 11 — 30 randomised `MAX_TASKS` values in `1..=512`.
#[test]
fn row11_max_tasks_randomised() {
    differential("row11_rand_max", |api, side, rec| {
        open_logger(api, side, rec);
        let mut rng = Rng::new(0x5EED_0011);
        for _ in 0..30 {
            let n = rng.range(1, 512);
            set_env("MAX_TASKS", n.to_string().as_bytes());
            let m = unsafe { (api.create_task_manager)() };
            rec.manager(&format!("mgr({n})"), m);
            if !m.is_null() {
                unsafe { (api.destroy_task_manager)(m) };
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 12 — a large but satisfiable `MAX_TASKS` (260 MB of `Task`s).
#[test]
fn row12_max_tasks_large_but_valid() {
    create_destroy_row("row12_million", Some(b"1000000"));
}

/// Row 13 — every `atoi` quirk the C inherits verbatim.
#[test]
fn row13_max_tasks_atoi_quirks() {
    differential("row13_atoi_quirks", |api, side, rec| {
        open_logger(api, side, rec);
        for v in [
            &b" 7"[..],
            b"+7",
            b"-0",
            b"7abc",
            b"0x10",
            b"abc",
            b"",
            b"   ",
            b"007",
            b"\t12",
            b"3.9",
            b"1e3",
            b"2147483647",
        ] {
            set_env("MAX_TASKS", v);
            let m = unsafe { (api.create_task_manager)() };
            rec.manager(&format!("mgr({:?})", String::from_utf8_lossy(v)), m);
            if !m.is_null() {
                // Prove the derived limit is actually used.
                add(api, m, b"probe", 42);
                rec.manager(&format!("after_add({:?})", String::from_utf8_lossy(v)), m);
                unsafe { (api.destroy_task_manager)(m) };
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 14 — `create_task_manager` with the logger *never* initialised, so its
/// internal `log_info` hits the `log_file == NULL` guard in the other module.
#[test]
fn row14_create_without_logger() {
    differential("row14_no_logger", |api, _side, rec| {
        unset_env("MAX_TASKS");
        let m = unsafe { (api.create_task_manager)() };
        rec.manager("manager", m);
        add(api, m, b"no logger here", 7);
        rec.manager("after_add", m);
        unsafe {
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
        }
    });
}

/// Row 15 — logger open first, so the module's own log lines reach the file.
#[test]
fn row15_create_with_logger() {
    differential("row15_with_logger", |api, side, rec| {
        open_logger(api, side, rec);
        unset_env("MAX_TASKS");
        let m = unsafe { (api.create_task_manager)() };
        rec.manager("manager", m);
        unsafe {
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

// ============================================= rows 16-20: add_task shapes ==

/// Row 16 — a single zero-length description.
#[test]
fn row16_add_empty_description() {
    differential("row16_empty_desc", |api, side, rec| {
        open_logger(api, side, rec);
        unset_env("MAX_TASKS");
        let m = unsafe { (api.create_task_manager)() };
        add(api, m, b"", 1);
        rec.manager("m", m);
        unsafe {
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

/// Row 17 — description lengths straddling the 255-byte `strncpy` boundary.
#[test]
fn row17_description_length_boundary() {
    differential("row17_len_boundary", |api, side, rec| {
        open_logger(api, side, rec);
        set_env("MAX_TASKS", b"16");
        let m = unsafe { (api.create_task_manager)() };
        let mut rng = Rng::new(0x1234_5678);
        let alpha = alphabet_wide();
        for (i, len) in [0usize, 1, 10, 254, 255, 256, 257, 1024].iter().enumerate() {
            let d = rng.bytes(*len, &alpha);
            add(api, m, &d, i as i32);
        }
        rec.manager("m", m);
        unsafe {
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

/// Row 18 — fill to exactly `max_tasks`, then five more (limit gate).
#[test]
fn row18_fill_then_overflow() {
    differential("row18_fill_overflow", |api, side, rec| {
        open_logger(api, side, rec);
        let mut rng = Rng::new(0x9999_0001);
        let alpha = alphabet_wide();
        for cap in [0usize, 1, 2, 10] {
            set_env("MAX_TASKS", cap.to_string().as_bytes());
            let m = unsafe { (api.create_task_manager)() };
            rec.manager(&format!("cap{cap}/fresh"), m);
            for i in 0..(cap + 5) {
                let d = rng.bytes_upto(300, &alpha);
                add(api, m, &d, i as i32 * 3 - 1);
                rec.manager(&format!("cap{cap}/after{i}"), m);
            }
            unsafe {
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 19 — boundary and randomised `int` priorities.
#[test]
fn row19_priority_values() {
    differential("row19_priorities", |api, side, rec| {
        open_logger(api, side, rec);
        set_env("MAX_TASKS", b"200");
        let m = unsafe { (api.create_task_manager)() };
        let mut fixed = vec![0i32, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
        let mut rng = Rng::new(0x7777_0042);
        for _ in 0..100 {
            fixed.push(rng.i32());
        }
        for (i, p) in fixed.iter().enumerate() {
            add(api, m, format!("task-{i}").as_bytes(), *p);
        }
        rec.manager("m", m);
        unsafe {
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

/// Row 20 — 200 randomised (description, priority) pairs into a randomised-cap
/// manager, then a full struct + stdout comparison.
#[test]
fn row20_randomised_add_task_stream() {
    differential("row20_rand_add", |api, side, rec| {
        open_logger(api, side, rec);
        let mut rng = Rng::new(0xDEAD_BEEF_01);
        let alpha = alphabet_wide();
        for round in 0..10 {
            let cap = rng.range(0, 40);
            set_env("MAX_TASKS", cap.to_string().as_bytes());
            let m = unsafe { (api.create_task_manager)() };
            if m.is_null() {
                rec.note(format!("round{round}: NULL manager"));
                continue;
            }
            for _ in 0..200 {
                let len = if rng.below(4) == 0 {
                    rng.below(600) as usize
                } else {
                    rng.below(40) as usize
                };
                let d = rng.bytes(len, &alpha);
                add(api, m, &d, rng.i32());
            }
            rec.manager(&format!("round{round}"), m);
            unsafe {
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

// ============================================ rows 21-24: print_tasks shapes ==

/// Row 21 — empty manager prints only the header.
#[test]
fn row21_print_empty() {
    differential("row21_print_empty", |api, side, rec| {
        open_logger(api, side, rec);
        unset_env("MAX_TASKS");
        let m = unsafe { (api.create_task_manager)() };
        unsafe { (api.print_tasks)(m) };
        rec.manager("m", m);
        unsafe {
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

/// Row 22 — exactly one task.
#[test]
fn row22_print_one() {
    differential("row22_print_one", |api, side, rec| {
        open_logger(api, side, rec);
        unset_env("MAX_TASKS");
        let m = unsafe { (api.create_task_manager)() };
        add(api, m, b"only one", -5);
        unsafe { (api.print_tasks)(m) };
        rec.manager("m", m);
        unsafe {
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

/// Row 23 — a full manager whose descriptions contain `%s`/`%d` and high bytes.
#[test]
fn row23_print_full_with_format_bait() {
    differential("row23_print_full", |api, side, rec| {
        open_logger(api, side, rec);
        set_env("MAX_TASKS", b"32");
        let m = unsafe { (api.create_task_manager)() };
        let baits: Vec<Vec<u8>> = vec![
            b"%s".to_vec(),
            b"%d %d %d".to_vec(),
            b"%n".to_vec(),
            b"100%".to_vec(),
            b"\xff\xfe\x80\x01tail".to_vec(),
            b"tab\there".to_vec(),
            b"cr\rhere".to_vec(),
        ];
        for (i, b) in baits.iter().enumerate() {
            add(api, m, b, i as i32);
        }
        let mut rng = Rng::new(0x2222_3333);
        let alpha = alphabet_wide();
        for i in 0..25 {
            let d = rng.bytes_upto(300, &alpha);
            add(api, m, &d, 1000 + i);
        }
        unsafe { (api.print_tasks)(m) };
        rec.manager("m", m);
        unsafe {
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

/// Row 24 — `print_tasks` called twice (no state change, output doubles).
#[test]
fn row24_print_twice() {
    differential("row24_print_twice", |api, side, rec| {
        open_logger(api, side, rec);
        set_env("MAX_TASKS", b"4");
        let m = unsafe { (api.create_task_manager)() };
        add(api, m, b"a", 1);
        add(api, m, b"b", 2);
        unsafe {
            (api.print_tasks)(m);
            (api.print_tasks)(m);
        }
        rec.manager("m", m);
        unsafe {
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

// ================================== rows 25-27: full low-level pipelines ==

/// Row 25 — the whole low-level pipeline, 40 randomised seeds.
#[test]
fn row25_full_low_level_pipeline() {
    differential("row25_pipeline", |api, side, rec| {
        open_logger(api, side, rec);
        let mut rng = Rng::new(0x4141_5151);
        let alpha = alphabet_wide();
        for seed in 0..40 {
            let cap = rng.range(0, 25);
            set_env("MAX_TASKS", cap.to_string().as_bytes());
            let m = unsafe { (api.create_task_manager)() };
            if m.is_null() {
                rec.note(format!("seed{seed}: NULL"));
                continue;
            }
            let n = rng.below(30);
            for _ in 0..n {
                let d = rng.bytes_upto(280, &alpha);
                add(api, m, &d, rng.i32());
            }
            unsafe { (api.print_tasks)(m) };
            rec.manager(&format!("seed{seed}"), m);
            unsafe { (api.destroy_task_manager)(m) };
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 26 — same pipeline but the manager is built *before* the logger, so the
/// early `log_info`/`log_warning` calls are swallowed by the NULL guard.
#[test]
fn row26_pipeline_manager_before_logger() {
    differential("row26_mgr_first", |api, side, rec| {
        set_env("MAX_TASKS", b"6");
        let m = unsafe { (api.create_task_manager)() };
        rec.manager("fresh", m);
        let mut rng = Rng::new(0x6161_7171);
        let alpha = alphabet_wide();
        for _ in 0..4 {
            let d = rng.bytes_upto(100, &alpha);
            add(api, m, &d, rng.i32());
        }
        // Now bring the logger up; only later lines should appear.
        open_logger(api, side, rec);
        for _ in 0..8 {
            let d = rng.bytes_upto(100, &alpha);
            add(api, m, &d, rng.i32());
        }
        unsafe { (api.print_tasks)(m) };
        rec.manager("final", m);
        unsafe {
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

/// Row 27 — pipeline with explicit `log_warning`/`log_error` interleaved.
#[test]
fn row27_pipeline_with_interleaved_logging() {
    differential("row27_interleaved_pipe", |api, side, rec| {
        open_logger(api, side, rec);
        set_env("MAX_TASKS", b"12");
        let m = unsafe { (api.create_task_manager)() };
        let mut rng = Rng::new(0x8181_9191);
        let alpha = alphabet_wide();
        for i in 0..20 {
            let d = rng.bytes_upto(120, &alpha);
            add(api, m, &d, i);
            let note = rng.bytes_upto(30, &alpha);
            let cn = cstr(&note);
            unsafe {
                if i % 2 == 0 {
                    (api.log_warning)(cn.as_ptr());
                } else {
                    (api.log_error)(cn.as_ptr());
                }
            }
        }
        unsafe { (api.print_tasks)(m) };
        rec.manager("m", m);
        unsafe {
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

// ======================================= rows 28-42: driver() input shapes ==

fn driver_row(label: &str, max_tasks: Option<&'static [u8]>, inputs: Vec<Vec<u8>>) {
    differential(label, |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        set_max_tasks(max_tasks);
        for (i, input) in inputs.iter().enumerate() {
            run_driver(api, input, rec, &format!("driver#{i}"));
        }
    });
}

/// Row 28 — the empty string: loop body never runs.
#[test]
fn row28_driver_empty_input() {
    driver_row("row28_empty", None, vec![b"".to_vec()]);
}

/// Row 29 — a single line with no trailing newline.
#[test]
fn row29_driver_single_line_no_newline() {
    driver_row("row29_one_line", None, vec![b"solitary task".to_vec()]);
}

/// Row 30 — a single line *with* a trailing newline.
#[test]
fn row30_driver_single_line_trailing_newline() {
    driver_row("row30_one_line_nl", None, vec![b"solitary task\n".to_vec()]);
}

/// Row 31 — newlines only, i.e. nothing but empty tasks.
#[test]
fn row31_driver_newlines_only() {
    driver_row(
        "row31_newlines",
        None,
        vec![b"\n".to_vec(), b"\n\n".to_vec(), b"\n\n\n".to_vec()],
    );
}

/// Row 32 — leading newline and consecutive interior newlines.
#[test]
fn row32_driver_leading_and_interior_newlines() {
    driver_row(
        "row32_odd_newlines",
        None,
        vec![
            b"\nfirst".to_vec(),
            b"\n\nsecond\n\n".to_vec(),
            b"a\n\nb\n\n\nc".to_vec(),
            b"\n\n\na".to_vec(),
        ],
    );
}

/// Row 33 — fewer lines than `max_tasks`.
#[test]
fn row33_driver_under_capacity() {
    driver_row("row33_under", None, vec![b"a\nb\nc\nd\ne".to_vec()]);
}

/// Row 34 — exactly `max_tasks` lines.
#[test]
fn row34_driver_exactly_capacity() {
    driver_row(
        "row34_exact",
        None,
        vec![b"1\n2\n3\n4\n5\n6\n7\n8\n9\n10".to_vec()],
    );
}

/// Row 35 — more lines than `max_tasks`: `[WARNING]` lines appear and
/// `priority` keeps incrementing for the rejected tasks too.
#[test]
fn row35_driver_over_capacity() {
    driver_row(
        "row35_over",
        None,
        vec![b"1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15\n".to_vec()],
    );
}

/// Row 36 — `MAX_TASKS=0`: every line is rejected.
#[test]
fn row36_driver_zero_capacity() {
    driver_row(
        "row36_zero_cap",
        Some(b"0"),
        vec![b"a\nb\nc".to_vec(), b"".to_vec()],
    );
}

/// Row 37 — lines around/over the 255-byte truncation boundary.
#[test]
fn row37_driver_long_lines() {
    let mut inputs = Vec::new();
    for len in [254usize, 255, 256, 257, 1000] {
        inputs.push(vec![b'x'; len]);
    }
    let mut mixed = Vec::new();
    mixed.extend_from_slice(b"short\n");
    mixed.extend_from_slice(&vec![b'A'; 300]);
    mixed.extend_from_slice(b"\ntiny\n");
    mixed.extend_from_slice(&vec![b'B'; 255]);
    inputs.push(mixed);
    driver_row("row37_long_lines", None, inputs);
}

/// Row 38 — 8-bit bytes, `\r`, tabs and printf-format bait in the payload.
#[test]
fn row38_driver_binary_and_format_bait() {
    driver_row(
        "row38_binary",
        None,
        vec![
            b"\xff\xfe\x80\x7f\x01\n\xc3\xa9\xe2\x82\xac\n".to_vec(),
            b"%s\n%d\n%n\n%%\n".to_vec(),
            b"crlf\r\nnext\r\n".to_vec(),
            b"tab\tsep\nmore\ttabs".to_vec(),
        ],
    );
}

/// Row 39 — 300 fully randomised inputs against randomised `MAX_TASKS`.
#[test]
fn row39_driver_randomised() {
    differential("row39_rand_driver", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        let mut rng = Rng::new(0xFACE_0FF1_CE);
        let alpha = alphabet_lines();
        for i in 0..300u32 {
            let cap = rng.range(0, 20);
            set_env("MAX_TASKS", cap.to_string().as_bytes());
            let len = match i % 5 {
                0 => 0,
                1 => rng.below(8) as usize,
                2 => rng.below(80) as usize,
                _ => rng.below(600) as usize,
            };
            let input = rng.bytes(len, &alpha);
            run_driver(api, &input, rec, &format!("driver#{i}(cap={cap})"));
        }
    });
}

/// Row 40 — `LOG_FILE` points at an existing file while running a randomised
/// input, so append semantics are checked through the full pipeline.
#[test]
fn row40_driver_appends_to_existing_log() {
    differential("row40_driver_append", |api, side, rec| {
        std::fs::write(side.log_path(), b"header line\n").unwrap();
        set_env_path("LOG_FILE", &side.log_path());
        set_env("MAX_TASKS", b"5");
        let mut rng = Rng::new(0x0BAD_C0DE);
        let alpha = alphabet_lines();
        for i in 0..20 {
            let input = rng.bytes_upto(200, &alpha);
            run_driver(api, &input, rec, &format!("driver#{i}"));
        }
    });
}

/// Row 41 — `driver` twice in one process. The C never resets `log_file` to
/// NULL in `finalize_logger`, so the second call must re-`fopen` and the log
/// must contain two complete sessions.
#[test]
fn row41_driver_twice_same_process() {
    driver_row(
        "row41_twice",
        None,
        vec![b"first run\nsecond line\n".to_vec(), b"second run\n".to_vec()],
    );
}

/// Row 42 — oversized inputs: one 64 KiB line, and 5000 short lines.
#[test]
fn row42_driver_oversized() {
    let mut many = Vec::new();
    for i in 0..5000 {
        many.extend_from_slice(format!("line-{i}\n").as_bytes());
    }
    driver_row(
        "row42_oversized",
        None,
        vec![vec![b'Z'; 64 * 1024], many, {
            let mut v = vec![b'q'; 70000];
            v.push(b'\n');
            v.extend_from_slice(&vec![b'w'; 70000]);
            v
        }],
    );
}

/// Extra: the `driver` symbol resolved through `dlsym` returning the exact
/// `EXIT_FAILURE`/success ints, with `LOG_FILE` unset so it uses `default.log`
/// relative to the cwd.
#[test]
fn row43_driver_with_default_log_in_cwd() {
    differential("row43_driver_default_log", |api, side, rec| {
        unset_env("LOG_FILE");
        unset_env("MAX_TASKS");
        let saved = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&side.dir).expect("chdir");
        let _ = &mut *rec;
        run_driver(api, b"alpha\nbeta\ngamma", rec, "driver");
        std::env::set_current_dir(saved).expect("restore cwd");
    });
}

/// Extra: interleave `driver` with direct low-level calls, proving the shared
/// `log_file` static is threaded through both entry paths identically.
///
/// Note the ordering constraint imposed by the C: `driver` ends with
/// `finalize_logger`, which `fclose`s `log_file` *without* resetting it to
/// NULL, so any low-level call made **after** `driver` returns is a
/// use-after-free. That case is genuine UB and is compared for crash-equivalence
/// in `errors.rs::e24_log_after_finalize` instead; here the low-level calls are
/// sandwiched *before* the first `driver` invocation, which is well-defined.
#[test]
fn row44_driver_interleaved_with_low_level() {
    differential("row44_mixed_entry", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        set_env("MAX_TASKS", b"3");

        // Low-level entry points first, sharing the module-level `log_file`
        // that `driver` will later re-open.
        open_logger(api, side, rec);
        let m = unsafe { (api.create_task_manager)() };
        rec.manager("m", m);
        add(api, m, b"before driver", 99);
        add(api, m, b"and another", -99);
        rec.manager("m2", m);
        unsafe {
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            // Leave `log_file` OPEN: `driver` will overwrite the static with a
            // second `fopen` of the same path (leaking the first stream), which
            // is exactly what the C does.
        }

        run_driver(api, b"one\ntwo\nthree\nfour", rec, "driver#0");
        // `driver` re-opens the logger itself, so a second call is fine.
        run_driver(api, b"again\n", rec, "driver#1");
    });
}

/// Extra: the description buffer must be zero-padded by `strncpy`, so a long
/// task followed by a short one in the *same slot index* of a *fresh* manager
/// yields identical bytes. Catches "reuse dirty bytes" translation errors.
#[test]
fn row45_description_zero_padding() {
    differential("row45_zero_pad", |api, side, rec| {
        open_logger(api, side, rec);
        set_env("MAX_TASKS", b"3");
        let mut rng = Rng::new(0xC0DE_1234);
        let alpha = alphabet_wide();
        for round in 0..20 {
            let m = unsafe { (api.create_task_manager)() };
            let long = rng.bytes(500, &alpha);
            add(api, m, &long, 1);
            let short = rng.bytes_upto(10, &alpha);
            add(api, m, &short, 2);
            add(api, m, b"", 3);
            rec.manager(&format!("round{round}"), m);
            unsafe {
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Extra: `log_*` called with a message that is exactly the buffer boundary of
/// the description, but through the logger (no truncation there — the logger
/// has no length limit at all).
#[test]
fn row46_logger_has_no_length_limit() {
    differential("row46_long_log", |api, side, rec| {
        open_logger(api, side, rec);
        let mut rng = Rng::new(0xAAAA_5555);
        let alpha = alphabet_wide();
        for len in [255usize, 256, 257, 4095, 4096, 4097, 65536] {
            let m = rng.bytes(len, &alpha);
            let c = cstr(&m);
            unsafe { (api.log_info)(c.as_ptr()) };
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Extra: a raw pointer cast sanity check — the two structs must have identical
/// layout, otherwise every other comparison would be meaningless.
#[test]
fn row47_struct_layout_matches_c() {
    assert_eq!(std::mem::size_of::<Task>(), 260, "sizeof(Task)");
    assert_eq!(std::mem::size_of::<TaskManager>(), 16, "sizeof(TaskManager)");
    assert_eq!(std::mem::align_of::<Task>(), 4, "alignof(Task)");
    assert_eq!(
        std::mem::offset_of!(Task, priority),
        256,
        "offsetof(Task, priority)"
    );
    assert_eq!(std::mem::offset_of!(TaskManager, max_tasks), 8);
    assert_eq!(std::mem::offset_of!(TaskManager, task_count), 12);
    let _: *const c_char = std::ptr::null();
}
