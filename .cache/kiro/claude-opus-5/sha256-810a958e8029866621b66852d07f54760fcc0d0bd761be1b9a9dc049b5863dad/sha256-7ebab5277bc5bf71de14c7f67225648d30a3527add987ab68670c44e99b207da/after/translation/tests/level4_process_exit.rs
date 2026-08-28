//! Level 4: what reaches the log file when the process exits *without*
//! `finalize_logger()` having closed the stream.
//!
//! `driver()` returns `EXIT_FAILURE` straight after `create_task_manager()`
//! fails, skipping `finalize_logger()`, and `initialize_logger()` overwrites
//! `log_file` without closing the old stream.  In C those streams are still
//! flushed by glibc's `_IO_cleanup` at exit; the translation has to match.
//!
//! Each scenario runs in a fresh child process (this same test binary, re-run
//! with `--exact exit_child`) with only one of the two libraries loaded.

mod harness;

use harness::{cstr, show, Api};
use std::path::PathBuf;

const LIB_VAR: &str = "DRIVER_EXIT_LIB";
const SCENARIO_VAR: &str = "DRIVER_EXIT_SCENARIO";

fn scenario(api: &Api, name: &str) {
    unsafe {
        match name {
            // Buffered entries, stream never closed.
            "no_finalize" => {
                assert_eq!((api.initialize_logger)(), 0);
                let m = cstr(b"never finalized");
                (api.log_info)(m.as_ptr());
                (api.log_warning)(m.as_ptr());
            }
            // Two sessions on the same path; the first FILE * is leaked.
            "double_init" => {
                assert_eq!((api.initialize_logger)(), 0);
                let a = cstr(b"first session");
                (api.log_info)(a.as_ptr());
                assert_eq!((api.initialize_logger)(), 0);
                let b = cstr(b"second session");
                (api.log_info)(b.as_ptr());
            }
            "triple_init" => {
                for i in 0..3 {
                    assert_eq!((api.initialize_logger)(), 0);
                    let m = cstr(format!("session {i}").as_bytes());
                    (api.log_info)(m.as_ptr());
                }
            }
            // Re-init after a failed open: log_file is NULL in between.
            "init_fail_then_init" => {
                let good = std::env::var("LOG_FILE").unwrap();
                harness::env_set("LOG_FILE", Some("/tmp"));
                assert_eq!((api.initialize_logger)(), -1);
                harness::env_set("LOG_FILE", Some(&good));
                assert_eq!((api.initialize_logger)(), 0);
                let m = cstr(b"recovered");
                (api.log_info)(m.as_ptr());
            }
            // driver() bails out before finalize_logger(), leaving the banner
            // and the two allocation-failure lines buffered.
            "driver_create_fails" => {
                harness::env_set("MAX_TASKS", Some("-1"));
                let s = cstr(b"a\nb\n");
                let r = (api.driver)(s.as_ptr());
                println!("driver -> {r}");
            }
            // A clean driver() run: nothing should be left over at exit.
            "driver_ok" => {
                harness::env_set("MAX_TASKS", Some("3"));
                let s = cstr(b"one\ntwo\nthree\nfour\n");
                let r = (api.driver)(s.as_ptr());
                println!("driver -> {r}");
            }
            // Enough volume that part of the log is flushed by the buffer
            // filling up and the rest only by the exit-time flush.
            "partial_flush" => {
                assert_eq!((api.initialize_logger)(), 0);
                let m = cstr(&vec![b'p'; 93]);
                for _ in 0..50 {
                    (api.log_info)(m.as_ptr());
                }
            }
            other => panic!("unknown scenario {other:?}"),
        }
    }
}

/// The child half of every scenario.  A no-op during a normal test run.
#[test]
fn exit_child() {
    let (Ok(which), Ok(name)) = (std::env::var(LIB_VAR), std::env::var(SCENARIO_VAR)) else {
        return;
    };
    let api = harness::load_single(&which);
    scenario(api, &name);
    // Deliberately returns without flushing anything: process exit must do it.
}

struct ChildResult {
    status: String,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    log: Vec<u8>,
}

fn run_child(which: &str, name: &str, log_path: &PathBuf, cwd: &PathBuf) -> ChildResult {
    let _ = std::fs::remove_file(log_path);
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "exit_child", "--nocapture", "--test-threads=1"])
        .env(LIB_VAR, which)
        .env(SCENARIO_VAR, name)
        .env("LOG_FILE", log_path)
        .env_remove("MAX_TASKS")
        .current_dir(cwd)
        .output()
        .expect("spawn child");
    ChildResult {
        status: format!("{:?}", out.status.code()),
        // Strip libtest's own chatter, keep only what the scenario printed.
        stdout: out
            .stdout
            .split(|&b| b == b'\n')
            .filter(|l| l.starts_with(b"driver ->") || l.starts_with(b"Tasks:") || l.starts_with(b"  ["))
            .flat_map(|l| l.iter().copied().chain(std::iter::once(b'\n')))
            .collect(),
        stderr: out.stderr,
        log: std::fs::read(log_path).unwrap_or_default(),
    }
}

fn compare_at_exit(name: &str) {
    let _guard = harness::lock();
    harness::ensure_built();
    let cwd = harness::scratch_dir();
    std::fs::create_dir_all(&cwd).unwrap();
    let log_path = cwd.join(format!("exit-{name}.log"));

    let c = run_child("c", name, &log_path, &cwd);
    let r = run_child("rust", name, &log_path, &cwd);

    let mut problems = Vec::new();
    if c.status != r.status {
        problems.push(format!("exit status: C {} vs Rust {}", c.status, r.status));
    }
    if c.stdout != r.stdout {
        problems.push(format!(
            "stdout: C {:?} vs Rust {:?}",
            show(&c.stdout),
            show(&r.stdout)
        ));
    }
    if c.log != r.log {
        problems.push(format!(
            "log at exit: C {:?} vs Rust {:?}",
            show(&c.log),
            show(&r.log)
        ));
    }
    assert!(
        problems.is_empty(),
        "exit-time scenario `{name}` diverged:\n{}\nC stderr: {}\nRust stderr: {}",
        problems.join("\n"),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
}

#[test]
fn exit_flush_no_finalize() {
    compare_at_exit("no_finalize");
}

#[test]
fn exit_flush_double_init() {
    compare_at_exit("double_init");
}

#[test]
fn exit_flush_triple_init() {
    compare_at_exit("triple_init");
}

#[test]
fn exit_flush_init_fail_then_init() {
    compare_at_exit("init_fail_then_init");
}

#[test]
fn exit_flush_driver_create_fails() {
    compare_at_exit("driver_create_fails");
}

#[test]
fn exit_flush_driver_ok() {
    compare_at_exit("driver_ok");
}

#[test]
fn exit_flush_partial() {
    compare_at_exit("partial_flush");
}
