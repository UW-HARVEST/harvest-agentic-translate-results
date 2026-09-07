//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input/condition the C rejects, calls
//! BOTH `.so`s, and asserts they return the SAME sentinel (`-1`, `NULL`,
//! `EXIT_FAILURE`) *and* emit the same `stderr` / log bytes — not merely
//! "both failed somehow".
//!
//! The genuinely-undefined rows (NULL `TaskManager*`, NULL `tasks`, double
//! `finalize_logger`) are executed in a forked child so the two
//! implementations' termination status can be compared without killing the
//! harness.

mod common;

use common::*;
use std::ffi::c_int;

fn app() -> LogCfg {
    LogCfg::InDir("app.log".into())
}

// =========================================================================
// E1 / E2 / E3 — initialize_logger: fopen failure
// =========================================================================

/// E1 — `LOG_FILE` names a path with a missing directory component.
#[test]
fn e01_initialize_logger_missing_directory() {
    let _g = lock();
    diff(
        "E1",
        LogCfg::Exact("/nonexistent-dir-xyz-e01/l.log".into()),
        None,
        |api, _| {
            let rc = unsafe { (api.initialize_logger)() };
            // A second call must fail identically and leave log_file NULL, so a
            // following log call is still a no-op.
            let rc2 = unsafe { (api.initialize_logger)() };
            unsafe { (api.log_info)(cs(b"should be dropped").as_ptr()) };
            format!("init={rc};init2={rc2}")
        },
    );
}

/// E2 — `LOG_FILE` is an existing directory (`fopen` → `EISDIR`).
#[test]
fn e02_initialize_logger_path_is_directory() {
    let _g = lock();
    diff("E2", LogCfg::Exact("/".into()), None, |api, _| {
        let rc = unsafe { (api.initialize_logger)() };
        format!("init={rc}")
    });
    // …and a nested existing directory too.
    diff("E2b", LogCfg::Exact("/usr/lib".into()), None, |api, _| {
        let rc = unsafe { (api.initialize_logger)() };
        format!("init={rc}")
    });
}

/// E3 — `LOG_FILE` is the empty string.
#[test]
fn e03_initialize_logger_empty_path() {
    let _g = lock();
    diff("E3", LogCfg::Exact(String::new()), None, |api, _| {
        let rc = unsafe { (api.initialize_logger)() };
        format!("init={rc}")
    });
}

// =========================================================================
// E4 / E5 / E6 / E7 — logger functions with log_file == NULL
// =========================================================================

/// E4 — `log_info` before `initialize_logger` (and after a failed one).
#[test]
fn e04_log_info_uninitialised() {
    let _g = lock();
    diff("E4", app(), None, |api, ctx| {
        unsafe { (api.log_info)(cs(b"dropped").as_ptr()) };
        unsafe { (api.log_info)(cs(b"").as_ptr()) };
        format!("log_exists={}", ctx.log_path.exists())
    });
}

/// E5 — `log_warning` with `log_file == NULL`.
#[test]
fn e05_log_warning_uninitialised() {
    let _g = lock();
    diff("E5", app(), None, |api, ctx| {
        unsafe { (api.log_warning)(cs(b"dropped").as_ptr()) };
        format!("log_exists={}", ctx.log_path.exists())
    });
}

/// E6 — `log_error` with `log_file == NULL`.
#[test]
fn e06_log_error_uninitialised() {
    let _g = lock();
    diff("E6", app(), None, |api, ctx| {
        unsafe { (api.log_error)(cs(b"dropped").as_ptr()) };
        format!("log_exists={}", ctx.log_path.exists())
    });
}

/// E7 — `finalize_logger` with `log_file == NULL`: must NOT write
/// `Logger finalized.` and must NOT `fclose`.
#[test]
fn e07_finalize_logger_uninitialised() {
    let _g = lock();
    diff("E7", app(), None, |api, ctx| {
        unsafe { (api.finalize_logger)() };
        unsafe { (api.finalize_logger)() };
        unsafe { (api.finalize_logger)() };
        format!("log_exists={}", ctx.log_path.exists())
    });
}

// =========================================================================
// E8 — create_task_manager: the TaskManager malloc failing (unreachable)
// =========================================================================

/// E8 — a 16-byte `malloc` never fails, so this branch cannot be provoked
/// through the public API. What *is* verifiable mechanically is that the branch
/// really exists in both libraries: its string literal must be compiled into
/// both `.so`s (a stub or a dropped branch would be caught here), and its
/// sibling branch E9 — which shares the `log_error` + bail-out shape — is
/// executed for real.
#[test]
fn e08_taskmanager_alloc_failure_branch_present() {
    let _g = lock();
    let needle = b"Failed to allocate memory for TaskManager.";
    for api in [c_api(), rust_api()] {
        assert!(
            contains(&so_bytes(api), needle),
            "{}: error-path literal {:?} is absent from {} — the branch was \
             dropped in translation",
            api.name,
            String::from_utf8_lossy(needle),
            api.path.display()
        );
    }
}

// =========================================================================
// E9 / E10 / E11 — create_task_manager: the tasks-array malloc failing
// =========================================================================

fn create_and_report(api: &Api, _ctx: &Ctx) -> String {
    unsafe {
        let rc = (api.initialize_logger)();
        let m = (api.create_task_manager)();
        let obs = format!("init={rc};returned_null={};{}", m.is_null(), dump_manager(m));
        if !m.is_null() {
            (api.destroy_task_manager)(m);
        }
        (api.finalize_logger)();
        obs
    }
}

/// E9 — `MAX_TASKS` huge → `malloc(max_tasks * 260)` fails → `NULL`.
#[test]
fn e09_create_task_manager_alloc_too_large() {
    let _g = lock();
    for v in ["2000000000", "2147483647", "100000000", "1000000000"] {
        diff(
            &format!("E9/MAX_TASKS={v}"),
            app(),
            Some(v),
            create_and_report,
        );
    }
}

/// E10 — `MAX_TASKS` negative → sign-extension + wrap → colossal size → `NULL`.
#[test]
fn e10_create_task_manager_negative_capacity() {
    let _g = lock();
    for v in ["-1", "-2", "-10", "-100000"] {
        diff(
            &format!("E10/MAX_TASKS={v}"),
            app(),
            Some(v),
            create_and_report,
        );
    }
}

/// E11 — `MAX_TASKS` = `INT_MIN` (and one step past it, which `atoi` saturates).
#[test]
fn e11_create_task_manager_int_min_capacity() {
    let _g = lock();
    for v in ["-2147483648", "-2147483649", "-99999999999999999999"] {
        diff(
            &format!("E11/MAX_TASKS={v}"),
            app(),
            Some(v),
            create_and_report,
        );
    }
}

// =========================================================================
// E12 / E13 / E14 — add_task rejections and truncation
// =========================================================================

/// E12 — one step past capacity: `task_count >= max_tasks` → warning, no-op.
#[test]
fn e12_add_task_past_capacity() {
    let _g = lock();
    for cap in [1usize, 2, 5, 10] {
        let s = cap.to_string();
        diff(
            &format!("E12/cap={cap}"),
            app(),
            Some(&s),
            move |api, _| unsafe {
                let rc = (api.initialize_logger)();
                let m = (api.create_task_manager)();
                let mut steps = String::new();
                // fill to capacity
                for i in 0..cap {
                    let d = cs(format!("fill-{i}").as_bytes());
                    (api.add_task)(m, d.as_ptr(), i as c_int);
                }
                let at_cap = (*m).task_count;
                // one step past, five times
                for i in 0..5 {
                    let d = cs(format!("overflow-{i}").as_bytes());
                    (api.add_task)(m, d.as_ptr(), 1000 + i);
                    steps += &format!("reject{i}: count={};", (*m).task_count);
                }
                (api.print_tasks)(m);
                let obs = format!(
                    "init={rc};at_cap={at_cap};{steps}\n{}",
                    dump_manager(m)
                );
                (api.destroy_task_manager)(m);
                (api.finalize_logger)();
                obs
            },
        );
    }
}

/// E13 — `max_tasks <= 0` so the very first `add_task` is already rejected.
#[test]
fn e13_add_task_zero_capacity() {
    let _g = lock();
    for v in ["0", "abc", "", "-0", "x9"] {
        diff(&format!("E13/MAX_TASKS={v:?}"), app(), Some(v), |api, _| unsafe {
            let rc = (api.initialize_logger)();
            let m = (api.create_task_manager)();
            if m.is_null() {
                (api.finalize_logger)();
                return format!("init={rc};manager=NULL");
            }
            for i in 0..3 {
                let d = cs(format!("rejected-{i}").as_bytes());
                (api.add_task)(m, d.as_ptr(), i);
            }
            (api.print_tasks)(m);
            let obs = format!("init={rc};{}", dump_manager(m));
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
            obs
        });
    }
}

/// E14 — over-long `description` is silently truncated to 255 bytes + NUL,
/// never an error. Compares all 260 raw bytes of each `Task`.
#[test]
fn e14_add_task_description_truncated() {
    let _g = lock();
    diff("E14", app(), Some("12"), |api, _| unsafe {
        let mut rng = Rng::new(0xE14);
        let rc = (api.initialize_logger)();
        let m = (api.create_task_manager)();
        for len in [255usize, 256, 257, 1024, 100_000] {
            let d = cs(&rng.ascii(len));
            (api.add_task)(m, d.as_ptr(), len as c_int);
        }
        (api.print_tasks)(m);
        let obs = format!("init={rc};{}", dump_manager(m));
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
        obs
    });
}

// =========================================================================
// E15 — NULL TaskManager* (undefined behaviour; compared via fork)
// =========================================================================

/// E15 — `add_task` / `print_tasks` / `destroy_task_manager` on a NULL manager.
/// Both implementations dereference NULL; the forked children must terminate
/// with the identical signal.
#[test]
fn e15_null_manager_pointer() {
    let _g = lock();
    diff_ub("E15/add_task", app(), Some("4"), |api, _| {
        let s = fork_status(|| {
            let d = cs(b"boom");
            unsafe { (api.add_task)(std::ptr::null_mut(), d.as_ptr(), 1) };
        });
        s
    });
    diff_ub("E15/print_tasks", app(), Some("4"), |api, _| {
        let s = fork_status(|| unsafe { (api.print_tasks)(std::ptr::null()) });
        s
    });
    diff_ub("E15/destroy", app(), Some("4"), |api, _| {
        let s = fork_status(|| unsafe { (api.destroy_task_manager)(std::ptr::null_mut()) });
        s
    });
}

// =========================================================================
// E16 — NULL message with the logger open
// =========================================================================

/// E16 — `log_*(NULL)`: glibc's `%s` renders `(null)`. Both sides call the same
/// `fprintf`, so the log bytes must match exactly.
#[test]
fn e16_log_null_message() {
    let _g = lock();
    diff("E16", app(), None, |api, _| unsafe {
        let rc = (api.initialize_logger)();
        (api.log_info)(std::ptr::null());
        (api.log_warning)(std::ptr::null());
        (api.log_error)(std::ptr::null());
        (api.finalize_logger)();
        format!("init={rc}")
    });
}

/// E16b — `add_task(manager, NULL, p)`: `strncpy` from NULL is UB; compared via
/// fork so the termination status of both sides is asserted equal.
#[test]
fn e16b_add_task_null_description() {
    let _g = lock();
    diff_ub("E16b", app(), Some("4"), |api, _| {
        let s = fork_status(|| unsafe {
            let m = (api.create_task_manager)();
            (api.add_task)(m, std::ptr::null(), 1);
        });
        s
    });
}

// =========================================================================
// E17 / E18 — driver bail-outs
// =========================================================================

/// E17 — `driver` when `initialize_logger` fails → `EXIT_FAILURE`, no stdout.
#[test]
fn e17_driver_logger_failure() {
    let _g = lock();
    for bad in [
        "/nonexistent-dir-xyz-e17/l.log",
        "/",
        "",
        "/usr/lib",
        "/proc/self/nonexistent/x",
    ] {
        diff(
            &format!("E17/LOG_FILE={bad:?}"),
            LogCfg::Exact(bad.into()),
            Some("10"),
            |api, _| {
                let input = cs(b"alpha\nbeta\ngamma");
                let rc = unsafe { (api.driver)(input.as_ptr()) };
                format!("driver={rc}")
            },
        );
    }
}

/// E18 — `driver` when `create_task_manager` returns NULL → `EXIT_FAILURE`,
/// nothing on stdout, and the log file left un-finalised (no
/// `Logger finalized.` line) — bug-for-bug.
#[test]
fn e18_driver_manager_failure() {
    let _g = lock();
    for v in ["-1", "-2147483648", "2000000000", "1000000000"] {
        diff(
            &format!("E18/MAX_TASKS={v}"),
            app(),
            Some(v),
            |api, _| {
                let input = cs(b"alpha\nbeta\ngamma");
                let rc = unsafe { (api.driver)(input.as_ptr()) };
                format!("driver={rc}")
            },
        );
    }
}

// =========================================================================
// E19 — driver's per-task malloc failing (unreachable)
// =========================================================================

/// E19 — the extracted-task allocation is at most `strlen(tasks) + 1`, so this
/// branch cannot be provoked through the public API. As with E8, its presence in
/// both libraries is asserted mechanically via its string literal.
#[test]
fn e19_driver_task_alloc_failure_branch_present() {
    let _g = lock();
    for needle in [
        &b"Error: Failed to allocate memory for task.\n"[..],
        b"Failed to allocate memory for tasks.",
        b"Failed to open log file: %s\n",
        b"Cannot add task: Maximum task limit reached.",
        b"TaskManager created successfully.",
        b"Task added successfully.",
        b"TaskManager destroyed successfully.",
        b"Logger initialized.",
        b"Logger finalized.",
        b"  [%d] %s (Priority: %d)\n",
        b"[INFO] %s\n",
        b"[WARNING] %s\n",
        b"[ERROR] %s\n",
        b"MAX_TASKS",
        b"LOG_FILE",
        b"default.log",
    ] {
        for api in [c_api(), rust_api()] {
            assert!(
                contains(&so_bytes(api), needle),
                "{}: literal {:?} missing from {}",
                api.name,
                String::from_utf8_lossy(needle),
                api.path.display()
            );
        }
    }
}

// =========================================================================
// E20 — driver(NULL)
// =========================================================================

/// E20 — `driver(NULL)` dereferences NULL after the logger and manager are set
/// up. Compared via fork.
#[test]
fn e20_driver_null_input() {
    let _g = lock();
    diff_ub("E20", app(), Some("10"), |api, _| {
        let s = fork_status(|| unsafe {
            (api.driver)(std::ptr::null());
        });
        s
    });
}

// =========================================================================
// E21 — driver("")
// =========================================================================

/// E21 — the empty-input early exit: prints only `Tasks:\n`, returns 0.
#[test]
fn e21_driver_empty_string() {
    let _g = lock();
    diff("E21", app(), Some("10"), |api, _| {
        let input = cs(b"");
        let rc = unsafe { (api.driver)(input.as_ptr()) };
        format!("driver={rc}")
    });
    // and with capacity 0 / negative, where creation itself fails
    diff("E21b", app(), Some("0"), |api, _| {
        let input = cs(b"");
        format!("driver={}", unsafe { (api.driver)(input.as_ptr()) })
    });
}

// =========================================================================
// E22 — double finalize_logger (UB; compared via fork)
// =========================================================================

/// E22 — the C never resets `log_file` to `NULL`, so a second
/// `finalize_logger` `fprintf`s to and `fclose`s an already-closed `FILE*`.
///
/// This is a use-after-free *inside glibc*: its outcome is genuinely
/// nondeterministic (observed both `exited(0)` and `SIGABRT` from the same C
/// binary, depending on whether the freed `FILE` was recycled). There is
/// therefore no stable behaviour to differentially assert, and `ERRORS.md`
/// records this row as inspection-only.
///
/// What IS asserted mechanically here is the property that matters: the Rust
/// mirrors the C's missing reset exactly. Neither `finalize_logger` clears its
/// static after `fclose`, so both reach the same use-after-free state — and
/// neither one "fixes" the bug, which would be an observable divergence for any
/// caller that closes twice.
#[test]
fn e22_double_finalize_logger_state_mirrors_c() {
    let _g = lock();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let c = std::fs::read_to_string(root.join("c_src/src/logger.c")).expect("read logger.c");
    let c_body = between(&c, "void finalize_logger()", "\n}");
    assert!(
        c_body.contains("fclose(log_file)"),
        "C finalize_logger no longer closes the stream: {c_body:?}"
    );
    assert!(
        !c_body.contains("log_file = NULL"),
        "C finalize_logger now resets log_file — ERRORS.md/E22 must be revisited"
    );

    let r = std::fs::read_to_string(root.join("translation/src/logger.rs")).expect("read logger.rs");
    let r_body = between(&r, "pub extern \"C\" fn finalize_logger()", "\n}\n");
    assert!(
        r_body.contains("fclose(stream)"),
        "Rust finalize_logger no longer closes the stream: {r_body:?}"
    );
    assert!(
        !r_body.contains("LOG_FILE = ptr::null_mut()") && !r_body.contains("LOG_FILE = core::ptr::null_mut()"),
        "Rust finalize_logger resets LOG_FILE to NULL but the C does not — \
         a caller that finalizes twice would observe different behaviour.\nbody: {r_body:?}"
    );

    // The reachable half of the row IS asserted differentially: `finalize_logger`
    // on a never-opened logger must be a silent no-op on both sides (E7), and a
    // *single* finalize must produce identical bytes on both sides (C13).
}

fn between<'a>(hay: &'a str, start: &str, end: &str) -> &'a str {
    let i = hay.find(start).unwrap_or_else(|| panic!("marker {start:?} not found"));
    let rest = &hay[i..];
    let j = rest.find(end).unwrap_or(rest.len());
    &rest[..j]
}

// =========================================================================
// Generic FFI-boundary boundaries (ERRORS.md, closing section)
// =========================================================================

/// The whole 32-bit `int` domain is legal for `priority` — this library declares
/// no `enum`, so the "value with no valid variant" case is the `int` extremes.
/// Every value must be stored verbatim and rendered identically by `%d`.
#[test]
fn generic_int_domain_across_ffi() {
    let _g = lock();
    diff("generic/int-domain", app(), Some("64"), |api, _| unsafe {
        let mut rng = Rng::new(0x1017_u64);
        let mut vals: Vec<c_int> = vec![
            c_int::MIN,
            c_int::MIN + 1,
            -65537,
            -65536,
            -256,
            -1,
            0,
            1,
            255,
            256,
            65535,
            65536,
            c_int::MAX - 1,
            c_int::MAX,
        ];
        while vals.len() < 60 {
            vals.push(rng.next_i32());
        }
        let rc = (api.initialize_logger)();
        let m = (api.create_task_manager)();
        for (i, p) in vals.iter().enumerate() {
            let d = cs(format!("v{i}").as_bytes());
            (api.add_task)(m, d.as_ptr(), *p);
        }
        (api.print_tasks)(m);
        let obs = format!("init={rc};{}", dump_manager(m));
        (api.destroy_task_manager)(m);
        (api.finalize_logger)();
        obs
    });
}

/// `MAX_TASKS` values one step past every interesting boundary.
#[test]
fn generic_max_tasks_boundaries() {
    let _g = lock();
    for v in [
        "0",
        "1",
        "-1",
        "2147483646",
        "2147483647",
        "2147483648",
        "-2147483647",
        "-2147483648",
        "-2147483649",
        "99999999999999999999",
        "+2147483647",
        "8261127",
        "8261128",
        "8261129",
    ] {
        diff(
            &format!("generic/MAX_TASKS={v}"),
            app(),
            Some(v),
            |api, _| unsafe {
                let rc = (api.initialize_logger)();
                let m = (api.create_task_manager)();
                let mut obs = format!("init={rc};null={};", m.is_null());
                if !m.is_null() {
                    obs += &format!("max_tasks={};", (*m).max_tasks);
                    let d = cs(b"probe");
                    (api.add_task)(m, d.as_ptr(), 1);
                    obs += &format!("count_after_add={};", (*m).task_count);
                    (api.print_tasks)(m);
                    (api.destroy_task_manager)(m);
                }
                (api.finalize_logger)();
                obs
            },
        );
    }
}

/// The `manager->max_tasks * sizeof(Task)` conversion.
///
/// In C the `int` operand is converted to `size_t` by *sign extension* followed
/// by reinterpretation, and the product wraps modulo 2^64. A zero-extending
/// translation (`as u32 as usize`) is **observationally equivalent through the
/// public API on any realistic host**: across the entire negative `int32` range
/// the two computations yield 558 GB … 18.4 EB and 1.1 TB … 18.4 EB
/// respectively, so `malloc` returns `NULL` for both (verified for every
/// boundary value by the differential rows E10/E11 below).
///
/// Because no input can distinguish them at run time, the semantics are pinned
/// at the source level instead, so a future edit cannot silently introduce a
/// conversion that *would* differ on a machine with >558 GB of usable address
/// space.
#[test]
fn size_conversion_semantics_mirror_c() {
    let _g = lock();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let c = std::fs::read_to_string(root.join("c_src/src/task_manager.c")).expect("read C");
    assert!(
        c.contains("malloc(manager->max_tasks * sizeof(Task))"),
        "the C allocation expression changed — revisit this row"
    );

    let r = std::fs::read_to_string(root.join("translation/src/task_manager.rs")).expect("read Rust");
    assert!(
        r.contains("as isize as usize).wrapping_mul(size_of::<Task>())"),
        "Rust must reproduce C's int -> size_t conversion (sign-extend then \
         reinterpret) and the modulo-2^64 product; found neither \
         `as isize as usize` nor `wrapping_mul` in task_manager.rs"
    );

    // …and differentially: every negative / oversized capacity must still yield
    // NULL from both libraries, exhaustively over the boundaries.
    let mut rng = Rng::new(0x5127);
    let mut vals: Vec<String> = vec![
        "-1".into(),
        "-2".into(),
        "-1000".into(),
        "-2000000000".into(),
        "-2147483647".into(),
        "-2147483648".into(),
    ];
    for _ in 0..12 {
        let v = -(1i64 + (rng.next_u32() as i64 % 2_147_483_647));
        vals.push(v.to_string());
    }
    for v in &vals {
        diff(
            &format!("size-conv/MAX_TASKS={v}"),
            app(),
            Some(v),
            create_and_report,
        );
    }
}
