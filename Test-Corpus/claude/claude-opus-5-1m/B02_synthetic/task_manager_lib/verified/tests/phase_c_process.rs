//! Phase C — out-of-process error-path differential tests.
//!
//! Some rows of `ERRORS.md` cannot be checked inside the test process:
//!
//!   * **rows 19, 20, 22, 23, 25, 30** — the C code has no null checks, so the
//!     process is killed by a signal. The observable is the *termination signal*
//!     (plus whatever reached the log/stdout before the fault), which requires a
//!     sacrificial process.
//!   * **rows 10, 11, 12, 29** — `malloc` returning NULL. Forced deterministically
//!     with `RLIMIT_AS`, which also requires a sacrificial address space.
//!   * **rows 5-8** in the *pristine* state: `static FILE *log_file == NULL` only
//!     holds in a brand-new process.
//!
//! This target runs with `harness = false` (see Cargo.toml) so that it owns its
//! stdout/stderr completely, and it re-executes **itself** as the child. Both
//! libraries are still loaded only through `libloading`.
//!
//! Child observations are encoded in the exit code, because after the heap has
//! been deliberately exhausted no allocation may happen:
//!
//! | exit code | meaning                        |
//! |-----------|--------------------------------|
//! | 70 / 71   | returned pointer NULL / non-NULL |
//! | 80/81/82/83 | returned int 0 / 1 / -1 / other |
//! | 90        | void scenario completed         |
//! | 65        | scenario precondition failed    |
//! | (signal)  | the library crashed             |

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const ENV_SCENARIO: &str = "DRIVER_ORACLE_SCENARIO";
const ENV_IMPL: &str = "DRIVER_ORACLE_IMPL";

// ===========================================================================
// Child side
// ===========================================================================

/// `libc::exit` rather than `std::process::exit`: it flushes glibc's streams —
/// which is where the library's log file and `stdout` live — and needs no
/// allocation, so it stays valid after the heap has been exhausted.
fn done(code: c_int) -> ! {
    unsafe { libc::exit(code) }
}

fn ptr_code<T>(p: *const T) -> c_int {
    if p.is_null() { 70 } else { 71 }
}

fn int_code(r: c_int) -> c_int {
    match r {
        0 => 80,
        1 => 81,
        -1 => 82,
        _ => 83,
    }
}

/// Current virtual-memory size in bytes, from `/proc/self/statm`.
fn vm_size_bytes() -> usize {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: usize = s
        .split_whitespace()
        .next()
        .and_then(|t| t.parse().ok())
        .expect("parse /proc/self/statm");
    pages * 4096
}

/// Clamp the address space to "currently mapped + slack" so any further large
/// allocation fails deterministically.
fn clamp_address_space(slack: usize) {
    let limit = vm_size_bytes() + slack;
    let rl = libc::rlimit {
        rlim_cur: limit as libc::rlim_t,
        rlim_max: limit as libc::rlim_t,
    };
    assert_eq!(
        unsafe { libc::setrlimit(libc::RLIMIT_AS, &rl) },
        0,
        "setrlimit(RLIMIT_AS) failed"
    );
}

/// Allocate and deliberately leak until a 16-byte `malloc` fails, so the very
/// first allocation inside `create_task_manager` returns NULL.
///
/// The blocks are threaded into a linked list and the head is passed through
/// `black_box`: LLVM is allowed to delete `malloc` calls whose result is never
/// used, which at `opt-level = 2` turned this into an infinite loop. Writing to
/// each block makes the allocations observably live.
fn exhaust_heap() {
    let mut head: *mut c_void = std::ptr::null_mut();
    loop {
        let p = unsafe { libc::malloc(16) };
        if p.is_null() {
            break;
        }
        unsafe { *(p as *mut *mut c_void) = head };
        head = p;
    }
    std::hint::black_box(head);
}

fn child_main(which: &str, scenario: &str) -> ! {
    let path = match which {
        "c" => c_so_path(),
        "rust" => rust_so_path(),
        other => {
            eprintln!("oracle: unknown implementation {other:?}");
            done(64);
        }
    };
    let api = Api::load(if which == "c" { "C" } else { "RUST" }, &path);

    unsafe {
        match scenario {
            // Sanity check of the oracle plumbing itself.
            "ok_driver" => {
                let t = cstr(b"alpha\nbeta");
                done(int_code((api.driver)(t.as_ptr() as *const c_char)));
            }

            // ERRORS rows 5-8 in the pristine "never initialised" state.
            "fresh_log_before_init" => {
                let m = cstr(b"before init");
                let p = m.as_ptr() as *const c_char;
                (api.log_info)(p);
                (api.log_warning)(p);
                (api.log_error)(p);
                // must not fclose anything, must not log "Logger finalized."
                (api.finalize_logger)();
                (api.finalize_logger)();
                done(90);
            }

            // ERRORS row 19 — add_task(NULL, ...) dereferences manager.
            "null_add_task_manager" => {
                let d = cstr(b"task");
                (api.add_task)(std::ptr::null_mut(), d.as_ptr() as *const c_char, 1);
                done(90); // unreachable in C
            }

            // ERRORS row 20 — add_task(m, NULL, p) with room left: strncpy(NULL).
            "null_add_task_desc" => {
                if (api.initialize_logger)() != 0 {
                    done(65);
                }
                let m = (api.create_task_manager)();
                if m.is_null() || (*m).max_tasks < 1 {
                    done(65);
                }
                (api.add_task)(m, std::ptr::null(), 1);
                done(90); // unreachable in C
            }

            // ERRORS row 22 — print_tasks(NULL): "Tasks:\n" is emitted first.
            "null_print_manager" => {
                (api.print_tasks)(std::ptr::null());
                done(90); // unreachable in C
            }

            // ERRORS row 23 — manager->tasks == NULL while task_count > 0.
            "null_print_tasks_array" => {
                let m = libc::malloc(TASKMANAGER_SIZE) as *mut TaskManager;
                if m.is_null() {
                    done(65);
                }
                (*m).tasks = std::ptr::null_mut();
                (*m).max_tasks = 5;
                (*m).task_count = 3;
                (api.print_tasks)(m);
                done(90); // unreachable in C
            }

            // ERRORS row 25 — destroy_task_manager(NULL).
            "null_destroy" => {
                (api.destroy_task_manager)(std::ptr::null_mut());
                done(90); // unreachable in C
            }

            // ERRORS row 30 — driver(NULL): logger + manager are set up first,
            // then `while (*start != '\0')` faults.
            "null_driver" => {
                done(int_code((api.driver)(std::ptr::null()))); // unreachable in C
            }

            // ERRORS row 10 — malloc(sizeof(TaskManager)) itself returns NULL.
            "oom_create_manager" => {
                // Open the log *before* clamping so the FILE buffer already
                // exists; `log_error` then needs no further allocation.
                if (api.initialize_logger)() != 0 {
                    done(65);
                }
                clamp_address_space(4 << 20);
                exhaust_heap();
                done(ptr_code((api.create_task_manager)()));
            }

            // ERRORS rows 11/12 — the tasks-array allocation fails while the
            // 16-byte manager allocation still succeeds (MAX_TASKS from parent).
            "oom_create_tasks_array" => {
                if (api.initialize_logger)() != 0 {
                    done(65);
                }
                clamp_address_space(4 << 20);
                done(ptr_code((api.create_task_manager)()));
            }

            // ERRORS row 29 — driver's per-line malloc(length + 1) fails.
            //
            // A 32 MiB single line is built *before* the address space is clamped
            // to +2 MiB, so `fopen` and the 16 B + 2600 B manager allocations
            // still succeed while the 32 MiB line copy cannot.
            "oom_driver_task_line" => {
                const LINE: usize = 32 << 20;
                let buf = libc::malloc(LINE + 1) as *mut u8;
                if buf.is_null() {
                    done(65);
                }
                libc::memset(buf as *mut c_void, b'a' as c_int, LINE);
                *buf.add(LINE) = 0;
                clamp_address_space(2 << 20);
                done(int_code((api.driver)(buf as *const c_char)));
            }

            other => {
                eprintln!("oracle: unknown scenario {other:?}");
                done(64);
            }
        }
    }
}

// ===========================================================================
// Parent side
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
struct ChildResult {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// `None` when the library never created the log file at all.
    log: Option<Vec<u8>>,
}

fn run_child(
    which: &str,
    scenario: &str,
    max_tasks: Option<&str>,
    log_path: Option<&Path>,
) -> ChildResult {
    let exe: PathBuf = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(&exe);
    cmd.env(ENV_SCENARIO, scenario)
        .env(ENV_IMPL, which)
        // Make the library paths explicit so the child never has to guess.
        .env("C_DRIVER_SO", c_so_path())
        .env("RUST_DRIVER_SO", rust_so_path());
    match max_tasks {
        Some(v) => {
            cmd.env("MAX_TASKS", v);
        }
        None => {
            cmd.env_remove("MAX_TASKS");
        }
    }
    match log_path {
        Some(p) => {
            let _ = std::fs::remove_file(p);
            cmd.env("LOG_FILE", p);
        }
        None => {
            cmd.env_remove("LOG_FILE");
        }
    }

    let out = cmd.output().expect("spawn oracle child");
    ChildResult {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
        log: log_path.and_then(|p| std::fs::read(p).ok()),
    }
}

struct Ctx {
    failures: Vec<String>,
    rows: usize,
}

impl Ctx {
    fn check(&mut self, label: &str, cond: bool, detail: impl FnOnce() -> String) {
        if !cond {
            self.failures.push(format!("[{label}] {}", detail()));
        }
    }

    /// Run one scenario against both libraries and require identical results.
    fn diff_scenario(
        &mut self,
        label: &str,
        scenario: &str,
        max_tasks: Option<&str>,
        use_log: bool,
    ) -> ChildResult {
        self.rows += 1;
        println!("--> {label}  (scenario={scenario}, MAX_TASKS={max_tasks:?})");

        let c_log = if use_log {
            Some(unique_path(&format!("proc_c_{scenario}.log")))
        } else {
            None
        };
        let r_log = if use_log {
            Some(unique_path(&format!("proc_r_{scenario}.log")))
        } else {
            None
        };

        let c = run_child("c", scenario, max_tasks, c_log.as_deref());
        let r = run_child("rust", scenario, max_tasks, r_log.as_deref());

        self.check(label, c.code == r.code, || {
            format!("exit code differs: C={:?} RUST={:?}", c.code, r.code)
        });
        self.check(label, c.signal == r.signal, || {
            format!(
                "termination signal differs: C={:?} RUST={:?}",
                c.signal, r.signal
            )
        });
        self.check(label, c.stdout == r.stdout, || {
            format!(
                "stdout differs:\n     C: {:?}\n  RUST: {:?}",
                String::from_utf8_lossy(&c.stdout),
                String::from_utf8_lossy(&r.stdout)
            )
        });
        self.check(label, c.stderr == r.stderr, || {
            format!(
                "stderr differs:\n     C: {:?}\n  RUST: {:?}",
                String::from_utf8_lossy(&c.stderr),
                String::from_utf8_lossy(&r.stderr)
            )
        });
        self.check(label, c.log == r.log, || {
            format!(
                "log file differs:\n     C: {:?}\n  RUST: {:?}",
                c.log.as_deref().map(String::from_utf8_lossy),
                r.log.as_deref().map(String::from_utf8_lossy),
            )
        });
        // Never accept a scenario that failed its own precondition.
        self.check(label, c.code != Some(65) && c.code != Some(64), || {
            format!("scenario precondition failed (exit {:?})", c.code)
        });
        c
    }
}

const SIGSEGV: i32 = 11;

fn parent_main() {
    let mut ctx = Ctx {
        failures: Vec::new(),
        rows: 0,
    };

    // ---- plumbing sanity check --------------------------------------------
    let c = ctx.diff_scenario("sanity/ok_driver", "ok_driver", None, true);
    ctx.check("sanity/ok_driver", c.code == Some(80), || {
        format!("expected exit 80 (driver returned 0), got {:?}", c.code)
    });
    ctx.check("sanity/ok_driver", c.signal.is_none(), || {
        format!("must not crash, got signal {:?}", c.signal)
    });
    ctx.check(
        "sanity/ok_driver",
        c.stdout == b"Tasks:\n  [1] alpha (Priority: 1)\n  [2] beta (Priority: 2)\n",
        || format!("unexpected stdout {:?}", String::from_utf8_lossy(&c.stdout)),
    );

    // ---- ERRORS rows 5-8: pristine, never-initialised logger --------------
    let c = ctx.diff_scenario(
        "ERRORS 5,6,7,8/fresh_log_before_init",
        "fresh_log_before_init",
        None,
        true,
    );
    ctx.check("ERRORS 5-8", c.code == Some(90), || {
        format!("must complete normally, got {:?}/{:?}", c.code, c.signal)
    });
    ctx.check("ERRORS 5-8", c.stdout.is_empty() && c.stderr.is_empty(), || {
        "log_* before init must produce no output".to_string()
    });
    ctx.check("ERRORS 5-8", c.log.is_none(), || {
        format!(
            "no log file may be created, but got {:?}",
            c.log.as_deref().map(String::from_utf8_lossy)
        )
    });

    // ---- ERRORS rows 19, 20, 22, 23, 25, 30: null-pointer dereferences ----
    for (label, scenario, max_tasks, use_log) in [
        ("ERRORS 19/add_task(NULL,..)", "null_add_task_manager", None, false),
        ("ERRORS 20/add_task(m,NULL,..)", "null_add_task_desc", None, true),
        ("ERRORS 22/print_tasks(NULL)", "null_print_manager", None, false),
        (
            "ERRORS 23/print_tasks(tasks=NULL)",
            "null_print_tasks_array",
            None,
            false,
        ),
        ("ERRORS 25/destroy(NULL)", "null_destroy", None, false),
        ("ERRORS 30/driver(NULL)", "null_driver", None, true),
    ] {
        let c = ctx.diff_scenario(label, scenario, max_tasks, use_log);
        // The row is only really covered if the C reference actually faulted.
        ctx.check(label, c.signal == Some(SIGSEGV), || {
            format!(
                "the C reference was expected to die with SIGSEGV({SIGSEGV}); \
                 got code={:?} signal={:?} — the row was not actually triggered",
                c.code, c.signal
            )
        });
        // stdout is empty for both: glibc's stdout is fully buffered when
        // redirected, so `printf("Tasks:\n")` is lost when the process faults.
        ctx.check(label, c.stdout.is_empty(), || {
            format!("unexpected stdout {:?}", String::from_utf8_lossy(&c.stdout))
        });
    }

    // ---- ERRORS row 10: malloc(sizeof(TaskManager)) returns NULL ----------
    let c = ctx.diff_scenario("ERRORS 10/oom manager alloc", "oom_create_manager", None, true);
    ctx.check("ERRORS 10", c.code == Some(70), || {
        format!(
            "create_task_manager must return NULL (exit 70), got {:?}/{:?}",
            c.code, c.signal
        )
    });
    ctx.check(
        "ERRORS 10",
        c.log.as_deref().is_some_and(|l| {
            l.ends_with(b"[ERROR] Failed to allocate memory for TaskManager.\n")
        }),
        || {
            format!(
                "wrong error record: {:?}",
                c.log.as_deref().map(String::from_utf8_lossy)
            )
        },
    );

    // ---- ERRORS rows 11/12: tasks-array allocation returns NULL -----------
    for max in ["2000000000", "2147483647", "-1", "-2147483648"] {
        let label = format!("ERRORS 11,12/oom tasks alloc MAX_TASKS={max}");
        let c = ctx.diff_scenario(&label, "oom_create_tasks_array", Some(max), true);
        ctx.check(&label, c.code == Some(70), || {
            format!(
                "create_task_manager must return NULL (exit 70), got {:?}/{:?}",
                c.code, c.signal
            )
        });
        ctx.check(
            &label,
            c.log
                .as_deref()
                .is_some_and(|l| l.ends_with(b"[ERROR] Failed to allocate memory for tasks.\n")),
            || {
                format!(
                    "wrong error record: {:?}",
                    c.log.as_deref().map(String::from_utf8_lossy)
                )
            },
        );
    }

    // ---- ERRORS row 29: driver's per-line malloc(length + 1) fails --------
    let c = ctx.diff_scenario(
        "ERRORS 29/driver task-line malloc",
        "oom_driver_task_line",
        None,
        true,
    );
    ctx.check("ERRORS 29", c.code == Some(81), || {
        format!(
            "driver must return EXIT_FAILURE (exit 81), got {:?}/{:?}",
            c.code, c.signal
        )
    });
    ctx.check(
        "ERRORS 29",
        c.stderr == b"Error: Failed to allocate memory for task.\n",
        || format!("wrong stderr {:?}", String::from_utf8_lossy(&c.stderr)),
    );
    ctx.check(
        "ERRORS 29",
        c.log.as_deref() == Some(
            b"[INFO] Logger initialized.\n\
              [INFO] TaskManager created successfully.\n\
              [INFO] TaskManager destroyed successfully.\n\
              [INFO] Logger finalized.\n"
                .as_slice(),
        ),
        || {
            format!(
                "wrong log: {:?}",
                c.log.as_deref().map(String::from_utf8_lossy)
            )
        },
    );

    // ---- report ------------------------------------------------------------
    println!();
    if ctx.failures.is_empty() {
        println!(
            "phase_c_process: OK — {} out-of-process scenarios, C and Rust identical",
            ctx.rows
        );
    } else {
        eprintln!(
            "phase_c_process: {} FAILURE(S) out of {} scenarios:",
            ctx.failures.len(),
            ctx.rows
        );
        for f in &ctx.failures {
            eprintln!("  {f}");
        }
        std::process::exit(1);
    }
}

fn main() {
    match (std::env::var(ENV_SCENARIO), std::env::var(ENV_IMPL)) {
        (Ok(scenario), Ok(which)) => child_main(&which, &scenario),
        _ => parent_main(),
    }
}
