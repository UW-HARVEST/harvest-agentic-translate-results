//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! `c_src` contains no `assert`, no error enum and no `RETURN_ERROR` macro: its
//! rejections are `-1` / `NULL` / `EXIT_FAILURE` sentinels, silent guards,
//! silent truncation, and — for every pointer parameter — no check at all.
//! The unchecked-pointer rows therefore compare *termination status* of a forked
//! child in addition to the output produced before the crash.
//!
//! MUST be run with `-- --test-threads=1` (fork + process-global fds/env).

mod common;

use common::*;
use std::os::raw::c_int;

fn init(api: &Api, rec: &mut Record) -> c_int {
    let rc = unsafe { (api.initialize_logger)() };
    rec.kv("initialize_logger", rc);
    rc
}

fn add(api: &Api, m: *mut TaskManager, desc: &[u8], prio: i32) {
    let c = cstr(desc);
    unsafe { (api.add_task)(m, c.as_ptr(), prio) };
}

// =========================================== rows 1-4: initialize_logger fails ==

/// Row 1 — `LOG_FILE` names a file inside a directory that does not exist:
/// `fopen` → ENOENT → `fprintf(stderr, ...)` + `return -1`.
#[test]
fn e01_initialize_logger_fopen_missing_dir() {
    differential("e01_missing_dir", |api, side, rec| {
        // `shared()` => identical path text on both sides, because the path is
        // echoed into stderr and would otherwise differ by construction.
        set_env_path("LOG_FILE", &side.shared("no/such/dir/app.log"));
        let rc = init(api, rec);
        assert_eq!(rc, -1, "C must reject an unopenable log path");
        // The failed open must leave the logger unusable, i.e. still a no-op.
        unsafe {
            (api.log_info)(c"must not appear".as_ptr());
            (api.log_warning)(c"must not appear".as_ptr());
            (api.log_error)(c"must not appear".as_ptr());
            (api.finalize_logger)();
        }
    });
}

/// Row 2 — `LOG_FILE=""` → `fopen("", "a")` → ENOENT.
#[test]
fn e02_initialize_logger_empty_path() {
    differential("e02_empty_path", |api, _side, rec| {
        set_env("LOG_FILE", b"");
        let rc = init(api, rec);
        assert_eq!(rc, -1);
    });
}

/// Row 3 — `LOG_FILE` is an existing directory → `fopen` → EISDIR.
#[test]
fn e03_initialize_logger_path_is_dir() {
    differential("e03_is_dir", |api, side, rec| {
        let d = side.shared("iamadir");
        std::fs::create_dir_all(&d).unwrap();
        set_env_path("LOG_FILE", &d);
        let rc = init(api, rec);
        assert_eq!(rc, -1);
    });
}

/// Row 4 — a path component is a regular file → `fopen` → ENOTDIR.
#[test]
fn e04_initialize_logger_enotdir() {
    differential("e04_enotdir", |api, side, rec| {
        let f = side.shared("iamafile");
        std::fs::write(&f, b"x").unwrap();
        set_env_path("LOG_FILE", &f.join("below.log"));
        let rc = init(api, rec);
        assert_eq!(rc, -1);
    });
}

// ==================================== rows 5-8: log_file == NULL guard paths ==

/// Rows 5, 6, 7 — all three `log_*` functions are silent no-ops while
/// `log_file` is NULL (fresh mapping ⇒ the static really is NULL).
#[test]
fn e05_log_fns_noop_when_uninitialised() {
    differential("e05_noop_logs", |api, _side, rec| {
        unsafe {
            (api.log_info)(c"info before init".as_ptr());
            (api.log_warning)(c"warning before init".as_ptr());
            (api.log_error)(c"error before init".as_ptr());
        }
        rec.note("no logger, no output");
        // and with a NULL message on top of a NULL stream
        unsafe {
            (api.log_info)(std::ptr::null());
            (api.log_warning)(std::ptr::null());
            (api.log_error)(std::ptr::null());
        }
    });
}

/// Row 8 — `finalize_logger` while `log_file` is NULL: no `fclose`, and in
/// particular *no* `[INFO] Logger finalized.` anywhere.
#[test]
fn e08_finalize_noop_when_uninitialised() {
    differential("e08_noop_finalize", |api, _side, rec| {
        unsafe {
            (api.finalize_logger)();
            (api.finalize_logger)();
            (api.finalize_logger)();
        }
        rec.note("three no-op finalizes");
    });
}

/// Row 9 — NULL `message` with the stream open: no null check in C, so glibc's
/// `%s` renders the literal `(null)`.
#[test]
fn e09_log_null_message() {
    differential("e09_null_message", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        unsafe {
            (api.log_info)(std::ptr::null());
            (api.log_warning)(std::ptr::null());
            (api.log_error)(std::ptr::null());
            (api.finalize_logger)();
        }
    });
}

// ============================== rows 10 & 21: documented-unreachable branches ==

fn c_src(file: &str) -> String {
    std::fs::read_to_string(workspace_root().join("c_src/src").join(file))
        .unwrap_or_else(|e| panic!("read c_src/src/{file}: {e}"))
}
fn rs_src(file: &str) -> String {
    std::fs::read_to_string(workspace_root().join("translation/src").join(file))
        .unwrap_or_else(|e| panic!("read translation/src/{file}: {e}"))
}

/// Assert `needles` all occur in `hay`, in this order.
fn ordered(hay: &str, needles: &[&str], what: &str) {
    let mut at = 0usize;
    for n in needles {
        match hay[at..].find(n) {
            Some(i) => at += i + n.len(),
            None => panic!("{what}: expected to find {n:?} after byte {at}"),
        }
    }
}

/// Row 10 — the `malloc(sizeof(TaskManager))` failure branch. A 16-byte
/// allocation cannot be made to fail from the public API without interposing
/// the allocator (which would change both libraries identically and prove
/// nothing), so the branch is verified structurally: identical message literal,
/// identical ordering, identical NULL sentinel. Its observable consequence — a
/// NULL manager propagating out of `driver` as `EXIT_FAILURE` — is exercised
/// live by `e11`/`e12`/`e20`.
#[test]
fn e10_manager_malloc_failure_unreachable() {
    let c = c_src("task_manager.c");
    ordered(
        &c,
        &[
            "TaskManager *manager = (TaskManager *)malloc(sizeof(TaskManager));",
            "if (!manager)",
            "log_error(\"Failed to allocate memory for TaskManager.\");",
            "return NULL;",
        ],
        "c_src/src/task_manager.c",
    );
    let r = rs_src("task_manager.rs");
    ordered(
        &r,
        &[
            "malloc(size_of::<TaskManager>())",
            "is_null()",
            "Failed to allocate memory for TaskManager.",
            "null_mut()",
        ],
        "translation/src/task_manager.rs",
    );
}

/// Row 21 — the `malloc(length + 1)` failure branch inside `driver`: same
/// argument as row 10. Verified structurally, including the exact `stderr`
/// literal and the `destroy` → `finalize` → `EXIT_FAILURE` ordering.
#[test]
fn e21_driver_task_malloc_failure_unreachable() {
    let c = c_src("driver.c");
    ordered(
        &c,
        &[
            "char *task = (char *)malloc(length + 1);",
            "if (!task)",
            "fprintf(stderr, \"Error: Failed to allocate memory for task.\\n\");",
            "destroy_task_manager(manager);",
            "finalize_logger();",
            "return EXIT_FAILURE;",
        ],
        "c_src/src/driver.c",
    );
    let r = rs_src("driver.rs");
    ordered(
        &r,
        &[
            "malloc(length.wrapping_add(1))",
            "task.is_null()",
            "Error: Failed to allocate memory for task.\\n",
            "destroy_task_manager(manager)",
            "finalize_logger()",
            "EXIT_FAILURE",
        ],
        "translation/src/driver.rs",
    );
}

// =============================== rows 11-12: tasks allocation failure => NULL ==

/// Row 11 — `MAX_TASKS=-1`: `(size_t)(int)-1 * 260` wraps to a colossal
/// request, `malloc` fails, `log_error` + `free(manager)` + `return NULL`.
///
/// The *width* of that conversion is asserted structurally as well. In C,
/// `manager->max_tasks * sizeof(Task)` promotes the `int` to `size_t`, i.e.
/// **sign**-extension: `-1` becomes `2**64-1` and the product is `2**64-260`.
/// A translation using `as u32 as usize` (zero-extension) would request
/// "only" ~1.1 TB instead. Both fail on every realistic host — the two are
/// behaviourally equivalent in practice, which is exactly why the dynamic test
/// below cannot distinguish them and the conversion must be pinned in the
/// source.
#[test]
fn e11_sign_extension_of_max_tasks() {
    let c = c_src("task_manager.c");
    assert!(
        c.contains("malloc(manager->max_tasks * sizeof(Task))"),
        "the C multiplies the `int` field by `sizeof(Task)`, so the usual \
         arithmetic conversions sign-extend it"
    );
    let r = rs_src("task_manager.rs");
    assert!(
        r.contains("(*manager).max_tasks as isize as usize"),
        "the translation must sign-extend `max_tasks` (`as isize as usize`) to \
         match C's int -> size_t conversion; found:\n{}",
        r.lines()
            .filter(|l| l.contains("max_tasks") && l.contains("as "))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        !r.contains("max_tasks as u32") && !r.contains("max_tasks as usize"),
        "a zero-extending conversion would diverge from C on a host able to \
         satisfy a ~558 GB request"
    );
}

#[test]
fn e11_create_negative_max_tasks() {
    differential("e11_negative_max", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        for v in [
            &b"-1"[..],
            b"-2",
            b"-10",
            b"-1000",
            b"-2147483648",
            b"-2147483647",
        ] {
            set_env("MAX_TASKS", v);
            let m = unsafe { (api.create_task_manager)() };
            rec.note(format!(
                "MAX_TASKS={:?} -> {}",
                String::from_utf8_lossy(v),
                if m.is_null() { "NULL" } else { "non-NULL" }
            ));
            assert!(m.is_null(), "C returns NULL for a wrapped size request");
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 12 — `MAX_TASKS=2147483647` → a 558 GB request. Whether the allocator
/// satisfies it is a property of the host, not of the translation: both sides
/// must agree either way, which is exactly what the harness asserts.
#[test]
fn e12_create_huge_max_tasks() {
    differential("e12_huge_max", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        for v in [&b"2147483647"[..], b"2147483646", b"100000000"] {
            set_env("MAX_TASKS", v);
            let m = unsafe { (api.create_task_manager)() };
            rec.manager(&format!("MAX_TASKS={:?}", String::from_utf8_lossy(v)), m);
            if !m.is_null() {
                unsafe { (api.destroy_task_manager)(m) };
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

// ================================== row 13: add_task capacity rejection ==

/// Row 13 — `task_count >= max_tasks`: `log_warning` and a silent return that
/// must not touch `task_count` or the `tasks` array.
#[test]
fn e13_add_task_when_full() {
    differential("e13_full", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        for cap in [0usize, 1, 3] {
            set_env("MAX_TASKS", cap.to_string().as_bytes());
            let m = unsafe { (api.create_task_manager)() };
            for i in 0..cap {
                add(api, m, format!("ok-{i}").as_bytes(), i as i32);
            }
            rec.manager(&format!("cap{cap}/full"), m);
            for i in 0..4 {
                add(api, m, format!("rejected-{i}").as_bytes(), 9000 + i);
                rec.manager(&format!("cap{cap}/reject{i}"), m);
            }
            unsafe {
                (api.print_tasks)(m);
                (api.destroy_task_manager)(m);
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 14 — silent truncation at 255 bytes plus the forced NUL at index 255.
#[test]
fn e14_add_task_truncates_long_description() {
    differential("e14_truncate", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        set_env("MAX_TASKS", b"8");
        let m = unsafe { (api.create_task_manager)() };
        let mut rng = Rng::new(0x1357_9BDF);
        let alpha = alphabet_wide();
        for i in 0..8 {
            let d = rng.bytes(256 + i * 137, &alpha);
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

// ============================ rows 15-18, 22-24: unchecked pointers (fork) ==

/// Row 15 — `add_task(NULL, ...)`: the C reads `(*manager).task_count` with no
/// null check. Compared by termination status of a forked child.
#[test]
fn e15_add_task_null_manager() {
    differential_forked("e15_null_manager", |api, side| {
        set_env_path("LOG_FILE", &side.log_path());
        unsafe { (api.initialize_logger)() };
        let c = cstr(b"boom");
        unsafe { (api.add_task)(std::ptr::null_mut(), c.as_ptr(), 1) };
    });
}

/// Row 16 — `add_task(m, NULL, p)`: `strncpy(dst, NULL, 255)`.
#[test]
fn e16_add_task_null_description() {
    differential_forked("e16_null_desc", |api, side| {
        set_env_path("LOG_FILE", &side.log_path());
        unsafe { (api.initialize_logger)() };
        unset_env("MAX_TASKS");
        let m = unsafe { (api.create_task_manager)() };
        unsafe { (api.add_task)(m, std::ptr::null(), 7) };
    });
}

/// Row 17 — `print_tasks(NULL)`: `printf("Tasks:\n")` happens *first*, then the
/// null dereference. (The buffered header is lost with the crashing process on
/// both sides, which the harness confirms by comparing the captured stdout.)
#[test]
fn e17_print_tasks_null_manager() {
    differential_forked("e17_null_print", |api, _side| {
        unsafe { (api.print_tasks)(std::ptr::null()) };
    });
}

/// Row 18 — `destroy_task_manager(NULL)`: `free((*manager).tasks)`.
#[test]
fn e18_destroy_null_manager() {
    differential_forked("e18_null_destroy", |api, side| {
        set_env_path("LOG_FILE", &side.log_path());
        unsafe { (api.initialize_logger)() };
        unsafe { (api.destroy_task_manager)(std::ptr::null_mut()) };
    });
}

/// Row 22 — `driver(NULL)`: `*start` on a null pointer, *after* the logger and
/// the manager were already created.
#[test]
fn e22_driver_null_input() {
    differential_forked("e22_null_driver", |api, side| {
        set_env_path("LOG_FILE", &side.log_path());
        unset_env("MAX_TASKS");
        let d = api.driver();
        unsafe { d(std::ptr::null()) };
    });
}

/// The statuses glibc can produce for a use-after-`fclose`. Identical list for
/// both sides — the assertion below is symmetric.
const UAF_ALLOWED: &[&str] = &[
    "exited(0)",          // the freed FILE happened to survive untouched
    "signal(6)+core",     // glibc's "double free detected in tcache" -> abort
    "signal(6)",
    "signal(11)+core",    // clobbered vtable / _flags -> segfault
    "signal(11)",
];

/// Assert both sides' outcomes come from `UAF_ALLOWED`, and report both lists so
/// a genuine divergence (e.g. a *Rust panic message* where C faults silently) is
/// still caught — a Rust-side abort with a panic message would appear as
/// `signal(6)` but is caught by the stderr comparison in `e24b` below.
fn assert_same_uaf_class(row: &str, c: &[String], r: &[String]) {
    for (which, list) in [("C", c), ("Rust", r)] {
        for st in list {
            assert!(
                UAF_ALLOWED.contains(&st.as_str()),
                "{row}: {which} produced unexpected status {st:?}; \
                 observed C={c:?} Rust={r:?}"
            );
        }
    }
    assert_eq!(c.len(), r.len(), "{row}: run counts must match");
}

/// Row 23 — `finalize_logger` twice. The C never resets `log_file` after
/// `fclose`, so the second call `fprintf`s and then `fclose`s a **freed**
/// `FILE`. This is undefined behaviour whose manifestation is allocator-state
/// dependent: running the *same, unmodified C library* twice in a row yields
/// `exited(0)` on one run and `free(): double free detected in tcache 2` +
/// SIGABRT on the next. An exact status comparison is therefore impossible in
/// principle, so this row is pinned down two ways:
///
/// 1. structurally — neither implementation clears `log_file`, and both do
///    `log_info` *then* `fclose`, so the second call takes the identical path;
/// 2. dynamically — over many forked runs, both sides' statuses are drawn from
///    the same allowed set (in particular, neither ever produces a *clean*
///    diagnostic that the other does not).
#[test]
fn e23_double_finalize() {
    // (1) structural: the C's `finalize_logger` never resets the static.
    let c = c_src("logger.c");
    ordered(
        &c,
        &[
            "void finalize_logger() {",
            "if (log_file) {",
            "log_info(\"Logger finalized.\");",
            "fclose(log_file);",
        ],
        "c_src/src/logger.c",
    );
    assert!(
        !c[c.find("void finalize_logger").unwrap()..].contains("log_file = NULL"),
        "C's finalize_logger does NOT null the static; the translation must not either"
    );
    let r = rs_src("logger.rs");
    ordered(
        &r,
        &[
            "pub extern \"C\" fn finalize_logger()",
            "is_null()",
            "Logger finalized.",
            "fclose(stream)",
        ],
        "translation/src/logger.rs",
    );
    let fin = &r[r.find("fn finalize_logger").unwrap()..];
    assert!(
        !fin.contains("LOG_FILE = ptr::null_mut()") && !fin.contains("LOG_FILE ="),
        "translation must mirror the C and leave LOG_FILE dangling: {fin}"
    );

    // (2) dynamic: same outcome class on both sides.
    let (cs, rs) = forked_statuses("e23_double_finalize", 12, |api, side| {
        set_env_path("LOG_FILE", &side.log_path());
        unsafe {
            (api.initialize_logger)();
            (api.log_info)(c"one".as_ptr());
            (api.finalize_logger)();
            (api.finalize_logger)();
        }
    });
    assert_same_uaf_class("row23", &cs, &rs);
}

/// Row 24 — any `log_*` after `finalize_logger` (dangling, non-NULL
/// `log_file`). Same undefined-behaviour caveat as row 23.
#[test]
fn e24_log_after_finalize() {
    let (cs, rs) = forked_statuses("e24_log_after_finalize", 12, |api, side| {
        set_env_path("LOG_FILE", &side.log_path());
        unsafe {
            (api.initialize_logger)();
            (api.finalize_logger)();
            (api.log_info)(c"after close".as_ptr());
            (api.log_warning)(c"after close".as_ptr());
            (api.log_error)(c"after close".as_ptr());
        }
    });
    assert_same_uaf_class("row24", &cs, &rs);
}

/// Row 24b — the guard in every logger function is `if (log_file)`, i.e. a
/// *pointer* test, never a "is initialised" flag, and nothing in `logger.c`
/// assigns to `log_file` except `initialize_logger`. A defensive translation
/// that reset the static in `finalize_logger` would turn rows 23/24 into silent
/// no-ops — a behavioural change that the nondeterministic UB manifestation can
/// never reliably reveal. So this property is asserted **structurally**, where
/// it is deterministic, for both implementations at once.
#[test]
fn e24b_dangling_log_file_is_still_treated_as_open() {
    let c = c_src("logger.c");
    // In the C, exactly one *statement* writes the static (the declaration
    // `static FILE *log_file = NULL;` is the initialiser, not an assignment).
    let c_writes = c
        .lines()
        .filter(|l| l.contains("log_file = ") && !l.trim_start().starts_with("static"))
        .count();
    assert_eq!(
        c_writes, 1,
        "expected exactly one assignment to `log_file` in logger.c (in \
         initialize_logger); found {c_writes}"
    );
    assert!(
        c.contains("log_file = fopen(log_file_path, \"a\")"),
        "the single assignment must be the fopen in initialize_logger"
    );
    // All four consumers guard on the pointer itself.
    for f in [
        "void log_info(",
        "void log_warning(",
        "void log_error(",
        "void finalize_logger(",
    ] {
        let body = &c[c.find(f).unwrap_or_else(|| panic!("missing {f}"))..];
        let body = &body[..body.find("\n}").unwrap_or(body.len())];
        assert!(
            body.contains("if (log_file)"),
            "{f} must guard on the raw pointer: {body}"
        );
        assert!(
            !body.contains("log_file ="),
            "{f} must not assign to log_file: {body}"
        );
    }

    // The translation must mirror all of that exactly.
    let r = rs_src("logger.rs");
    let r_writes = r
        .lines()
        .filter(|l| l.contains("LOG_FILE = ") && !l.trim_start().starts_with("static"))
        .count();
    assert_eq!(
        r_writes, 1,
        "the translation must also write LOG_FILE exactly once (in \
         initialize_logger); found {r_writes}"
    );
    assert!(
        r.contains("LOG_FILE = cstd::fopen(log_file_path, c\"a\".as_ptr())"),
        "the single assignment must be the fopen in initialize_logger"
    );
    for f in [
        "fn log_info(",
        "fn log_warning(",
        "fn log_error(",
        "fn finalize_logger(",
    ] {
        let body = &r[r.find(f).unwrap_or_else(|| panic!("missing {f}"))..];
        let body = &body[..body.find("\n}").unwrap_or(body.len())];
        assert!(
            body.contains("is_null()"),
            "{f} must guard on the raw pointer: {body}"
        );
        assert!(
            !body.contains("LOG_FILE ="),
            "{f} must not assign to LOG_FILE: {body}"
        );
    }

    // And the well-defined half is checked dynamically: with `log_file` still
    // NULL (fresh mapping) every one of them is a guaranteed clean no-op, which
    // proves the guard is read at all.
    differential("e24b_guard_is_live", |api, _side, rec| {
        unsafe {
            (api.log_info)(c"x".as_ptr());
            (api.log_warning)(c"x".as_ptr());
            (api.log_error)(c"x".as_ptr());
            (api.finalize_logger)();
        }
        rec.note("all four were no-ops");
    });
}

// ============================================ rows 19-20: driver early exits ==

/// Row 19 — `driver` when `initialize_logger` fails: `EXIT_FAILURE`, no
/// manager, no `Tasks:` header, no `finalize_logger`.
#[test]
fn e19_driver_logger_failure() {
    differential("e19_driver_no_log", |api, side, rec| {
        set_env_path("LOG_FILE", &side.shared("nope/deeper/app.log"));
        unset_env("MAX_TASKS");
        let d = api.driver();
        for input in [&b""[..], b"a", b"a\nb\nc\n"] {
            let c = cstr(input);
            let rc = unsafe { d(c.as_ptr()) };
            rec.kv(&format!("driver({:?})", String::from_utf8_lossy(input)), rc);
            assert_eq!(rc, 1, "EXIT_FAILURE");
        }
    });
}

/// Row 20 — `driver` when `create_task_manager` returns NULL: `EXIT_FAILURE`
/// and — deliberately — the logger is left open (no `Logger finalized.` line).
#[test]
fn e20_driver_manager_failure() {
    differential("e20_driver_no_mgr", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        set_env("MAX_TASKS", b"-1");
        let d = api.driver();
        for input in [&b""[..], b"a", b"a\nb\nc\n"] {
            let c = cstr(input);
            let rc = unsafe { d(c.as_ptr()) };
            rec.kv(&format!("driver({:?})", String::from_utf8_lossy(input)), rc);
            assert_eq!(rc, 1, "EXIT_FAILURE");
        }
        // Each failed call leaked an open stream, exactly as the C does; the
        // harness's fflush makes their buffered bytes comparable.
    });
}

// ======================================== rows 25-26: atoi-derived max_tasks ==

/// Row 25 — non-numeric `MAX_TASKS` → `atoi` = 0 → permanently full manager.
#[test]
fn e25_max_tasks_non_numeric() {
    differential("e25_non_numeric", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        for v in [
            &b"abc"[..],
            b"",
            b"   ",
            b"+",
            b"-",
            b"?",
            b"\n",
            b"nan",
            b"null",
            b"0",
            b"-0",
            b"+0",
        ] {
            set_env("MAX_TASKS", v);
            let m = unsafe { (api.create_task_manager)() };
            rec.manager(&format!("mgr({:?})", String::from_utf8_lossy(v)), m);
            if !m.is_null() {
                add(api, m, b"should be rejected", 1);
                rec.manager(&format!("after({:?})", String::from_utf8_lossy(v)), m);
                unsafe {
                    (api.print_tasks)(m);
                    (api.destroy_task_manager)(m);
                }
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

/// Row 26 — `MAX_TASKS` values that overflow `int`; whatever `atoi` produces,
/// both sides must produce the same `max_tasks` and the same success/failure.
#[test]
fn e26_max_tasks_int_overflow() {
    differential("e26_int_overflow", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        for v in [
            &b"2147483648"[..],
            b"4294967296",
            b"99999999999999",
            b"-99999999999999",
            b"9223372036854775807",
            b"9223372036854775808",
            b"-9223372036854775808",
            b"0x10",
            b"010",
        ] {
            set_env("MAX_TASKS", v);
            let m = unsafe { (api.create_task_manager)() };
            rec.manager(&format!("mgr({:?})", String::from_utf8_lossy(v)), m);
            if !m.is_null() {
                unsafe { (api.destroy_task_manager)(m) };
            }
        }
        unsafe { (api.finalize_logger)() };
    });
}

// ====================================== row 27: full-range int across the FFI ==

/// Row 27 — the public API has no enum parameters; its only `int` inputs are
/// `priority` and the `TaskManager` counters, and the C validates *neither*.
/// Every representable `int` (including the "no valid variant" extremes an
/// out-of-range C enum value would produce) must round-trip identically.
#[test]
fn e27_priority_out_of_range() {
    differential("e27_priority_range", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        set_env("MAX_TASKS", b"64");
        let m = unsafe { (api.create_task_manager)() };
        let mut vals = vec![
            0i32,
            -1,
            1,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            i32::MAX - 1,
            -2147483648,
            0x7fff_ffff,
            u32::MAX as i32,
            0x8000_0000u32 as i32,
        ];
        let mut rng = Rng::new(0xFEED_FACE);
        while vals.len() < 64 {
            vals.push(rng.i32());
        }
        for (i, p) in vals.iter().enumerate() {
            add(api, m, format!("p{i}").as_bytes(), *p);
        }
        rec.manager("m", m);
        unsafe {
            (api.print_tasks)(m);
            (api.destroy_task_manager)(m);
            (api.finalize_logger)();
        }
    });
}

// ============================== rows 28-30: generic boundary inputs ==

/// Row 28 — zero-length input.
#[test]
fn e28_driver_empty_input() {
    differential("e28_empty", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        unset_env("MAX_TASKS");
        let d = api.driver();
        let c = cstr(b"");
        let rc = unsafe { d(c.as_ptr()) };
        rec.kv("driver", rc);
        assert_eq!(rc, 0);
    });
}

/// Row 29 — oversized input: a 64 KiB single line and 5000 lines.
#[test]
fn e29_driver_oversized_input() {
    differential("e29_oversized", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        unset_env("MAX_TASKS");
        let d = api.driver();
        let big = vec![b'x'; 64 * 1024];
        let c = cstr(&big);
        rec.kv("driver(64KiB)", unsafe { d(c.as_ptr()) });
        let mut many = Vec::new();
        for i in 0..5000 {
            many.extend_from_slice(format!("l{i}\n").as_bytes());
        }
        let c2 = cstr(&many);
        rec.kv("driver(5000 lines)", unsafe { d(c2.as_ptr()) });
    });
}

/// Row 30 — one step either side of the 255-byte description limit.
#[test]
fn e30_description_length_boundary() {
    differential("e30_len_boundary", |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        set_env("MAX_TASKS", b"8");
        let m = unsafe { (api.create_task_manager)() };
        for (i, len) in [253usize, 254, 255, 256, 257].iter().enumerate() {
            let d: Vec<u8> = (0..*len).map(|k| b'a' + (k % 26) as u8).collect();
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

// ============ extra: hostile caller-built TaskManager (out-of-range indices) ==

/// The `TaskManager` layout is public, so a real consumer can hand the library
/// counters the C never validates. `add_task` gates only on
/// `task_count >= max_tasks`, so a *negative* `task_count` slips through and
/// indexes **before** the array. The buffer here is padded so those writes stay
/// inside memory the test owns, letting the resulting bytes be compared.
fn hostile(label: &str, task_count: c_int, max_tasks: c_int) {
    const SLACK: usize = 16; // Task slots of head-room on each side
    const TASK: usize = 260;
    differential(label, |api, side, rec| {
        set_env_path("LOG_FILE", &side.log_path());
        assert_eq!(init(api, rec), 0);
        let mut buf = vec![0u8; TASK * (SLACK * 2 + 8)];
        let base = unsafe { buf.as_mut_ptr().add(TASK * SLACK) } as *mut Task;
        let mut m = TaskManager {
            tasks: base,
            max_tasks,
            task_count,
        };
        add(api, &mut m as *mut TaskManager, b"hostile-payload", 0x5A5A_5A5A);
        rec.note(format!(
            "task_count {task_count} -> {} (max_tasks {max_tasks})",
            m.task_count
        ));
        // `print_tasks` loops `for (i = 0; i < task_count; i++)` with no upper
        // bound of its own, so only render when the (possibly attacker-supplied)
        // count stays inside the buffer this test owns.
        if (0..=8).contains(&m.task_count) {
            unsafe { (api.print_tasks)(&m as *const TaskManager) };
        } else {
            rec.note("print_tasks skipped: task_count out of owned range");
        }
        rec.note(format!("buf={}", hex(&buf)));
        unsafe { (api.finalize_logger)() };
    });
}

#[test]
fn e31_negative_task_count_writes_before_array() {
    hostile("e31_neg_count", -1, 10);
}

#[test]
fn e32_negative_count_and_zero_cap() {
    hostile("e32_neg_count_zero_cap", -5, 0);
}

#[test]
fn e33_int_max_count_int_min_cap_is_rejected() {
    hostile("e33_max_min", i32::MAX, i32::MIN);
}

#[test]
fn e34_equal_extremes_are_rejected() {
    hostile("e34_max_max", i32::MAX, i32::MAX);
}

#[test]
fn e35_zero_count_negative_cap_is_rejected() {
    hostile("e35_zero_neg", 0, -1);
}

/// One step past the last valid index: `task_count == max_tasks - 1` with
/// `max_tasks == INT_MAX` passes the gate and then indexes ~558 GB past the
/// array. Both sides must fault identically.
#[test]
fn e36_out_of_range_index_faults_identically() {
    differential_forked("e36_oob_index", |api, side| {
        set_env_path("LOG_FILE", &side.log_path());
        unsafe { (api.initialize_logger)() };
        set_env("MAX_TASKS", b"4");
        let m = unsafe { (api.create_task_manager)() };
        unsafe {
            (*m).max_tasks = i32::MAX;
            (*m).task_count = i32::MAX - 1;
        }
        let c = cstr(b"way out of range");
        unsafe { (api.add_task)(m, c.as_ptr(), 1) };
    });
}

/// Generic boundary: a path longer than `PATH_MAX` → `fopen` → ENAMETOOLONG.
#[test]
fn e37_initialize_logger_path_too_long() {
    differential("e37_name_too_long", |api, _side, rec| {
        let long = format!("/{}", "a".repeat(5000));
        set_env("LOG_FILE", long.as_bytes());
        let rc = init(api, rec);
        assert_eq!(rc, -1);
    });
}

/// Generic boundary: a path that exists but is not writable → EACCES.
#[test]
fn e38_initialize_logger_permission_denied() {
    differential("e38_eacces", |api, side, rec| {
        let f = side.shared("readonly.log");
        // `shared()` is the same path for both sides, so creation must be
        // idempotent: the C run already made it read-only.
        if !f.exists() {
            std::fs::write(&f, b"").unwrap();
        }
        let mut perms = std::fs::metadata(&f).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o444);
        std::fs::set_permissions(&f, perms).unwrap();
        set_env_path("LOG_FILE", &f);
        let rc = init(api, rec);
        rec.kv("rc", rc);
    });
}
