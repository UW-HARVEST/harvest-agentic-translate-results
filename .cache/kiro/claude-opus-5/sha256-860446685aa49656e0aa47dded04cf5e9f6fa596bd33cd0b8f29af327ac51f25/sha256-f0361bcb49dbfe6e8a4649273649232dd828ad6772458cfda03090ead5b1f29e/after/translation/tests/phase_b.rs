//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call goes through `dlopen`/`dlsym` on the two `.so` files; the Rust
//! crate is never called directly.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::ptr;

// --------------------------------------------------------------- op program --

#[derive(Clone, Debug)]
pub enum Op {
    InitLogger,
    FinalizeLogger,
    LogInfo(Vec<u8>),
    LogWarning(Vec<u8>),
    LogError(Vec<u8>),
    LogInfoNull,
    Create,
    AddTask(Vec<u8>, i32),
    PrintTasks,
    Snapshot,
    Destroy,
    Driver(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rec {
    Ret(c_int),
    ManagerNull(bool),
    Fields { max_tasks: c_int, task_count: c_int },
    Tasks(Vec<Vec<u8>>),
}

unsafe fn interp(lib: &Lib, ops: &[Op]) -> Vec<Rec> {
    let mut recs = Vec::new();
    let mut mgr: *mut TaskManager = ptr::null_mut();
    for op in ops {
        match op {
            Op::InitLogger => recs.push(Rec::Ret((lib.initialize_logger())())),
            Op::FinalizeLogger => (lib.finalize_logger())(),
            Op::LogInfo(m) => (lib.log_info())(cstr(m).as_ptr()),
            Op::LogWarning(m) => (lib.log_warning())(cstr(m).as_ptr()),
            Op::LogError(m) => (lib.log_error())(cstr(m).as_ptr()),
            Op::LogInfoNull => (lib.log_info())(ptr::null()),
            Op::Create => {
                mgr = (lib.create_task_manager())();
                recs.push(Rec::ManagerNull(mgr.is_null()));
            }
            Op::AddTask(desc, prio) => {
                if !mgr.is_null() {
                    (lib.add_task())(mgr, cstr(desc).as_ptr(), *prio);
                }
            }
            Op::PrintTasks => {
                if !mgr.is_null() {
                    (lib.print_tasks())(mgr);
                }
            }
            Op::Snapshot => {
                recs.push(Rec::ManagerNull(mgr.is_null()));
                if !mgr.is_null() {
                    let (max_tasks, task_count, tasks) = snapshot(mgr);
                    recs.push(Rec::Fields {
                        max_tasks,
                        task_count,
                    });
                    recs.push(Rec::Tasks(tasks));
                }
            }
            Op::Destroy => {
                if !mgr.is_null() {
                    (lib.destroy_task_manager())(mgr);
                    mgr = ptr::null_mut();
                }
            }
            Op::Driver(input) => {
                let s = cstr(input);
                recs.push(Rec::Ret((lib.driver())(s.as_ptr() as *const c_char)));
            }
        }
    }
    recs
}

struct Run {
    recs: Vec<Rec>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    log: Vec<u8>,
}

/// Runs `ops` against one implementation with the given environment, capturing
/// stdout and the resulting log file.
fn exec(lib: &Lib, log_path: Option<&Path>, max_tasks: Option<&str>, ops: &[Op]) -> Run {
    match log_path {
        Some(p) => set_env("LOG_FILE", p.to_str().unwrap()),
        None => unset_env("LOG_FILE"),
    }
    match max_tasks {
        Some(v) => set_env("MAX_TASKS", v),
        None => unset_env("MAX_TASKS"),
    }
    let (recs, stdout, stderr) = capture_out_err(|| unsafe { interp(lib, ops) });
    let log = match log_path {
        Some(p) => read_log(&p.to_path_buf()),
        None => Vec::new(),
    };
    Run {
        recs,
        stdout,
        stderr,
        log,
    }
}

/// The core differential assertion: same ops, same env, separate log files.
fn diff(ctx: &str, tag: &str, max_tasks: Option<&str>, ops: &[Op]) {
    let (libs, _g) = libs();
    let logs = fresh_logs(tag);
    let c = exec(&libs.c, Some(&logs.c), max_tasks, ops);
    let r = exec(&libs.rust, Some(&logs.rust), max_tasks, ops);
    compare(ctx, &c, &r);
    let _ = std::fs::remove_file(&logs.c);
    let _ = std::fs::remove_file(&logs.rust);
}

fn compare(ctx: &str, c: &Run, r: &Run) {
    if c.recs != r.recs {
        panic!("return values / struct state differ [{ctx}]\n  C   : {:?}\n  Rust: {:?}", c.recs, r.recs);
    }
    assert_bytes_eq("stdout", ctx, &c.stdout, &r.stdout);
    assert_bytes_eq("stderr", ctx, &c.stderr, &r.stderr);
    assert_bytes_eq("log file", ctx, &c.log, &r.log);
}

fn b(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

// =========================================================== C1 .. C5 (log) ==

#[test]
fn c1_init_finalize_writable() {
    diff("C1", "c1", None, &[Op::InitLogger, Op::FinalizeLogger]);
}

#[test]
fn c2_log_file_unset_default_log() {
    // LOG_FILE unset => "default.log" relative to the CWD. Give each
    // implementation its own CWD so the two default.log files can be compared.
    let (libs, _g) = libs();
    let base = tmp_path("c2");
    let dc = base.join("c");
    let dr = base.join("rust");
    std::fs::create_dir_all(&dc).unwrap();
    std::fs::create_dir_all(&dr).unwrap();
    // Pre-existing content proves append mode.
    std::fs::write(dc.join("default.log"), b"PRE\n").unwrap();
    std::fs::write(dr.join("default.log"), b"PRE\n").unwrap();

    let ops = [
        Op::InitLogger,
        Op::LogInfo(b("hello")),
        Op::LogWarning(b("warn")),
        Op::LogError(b("err")),
        Op::FinalizeLogger,
    ];
    let saved = cwd();
    cd(&dc);
    let c = exec(&libs.c, None, None, &ops);
    cd(&dr);
    let r = exec(&libs.rust, None, None, &ops);
    cd(&saved);
    assert_bytes_eq("stderr", "C2", &c.stderr, &r.stderr);

    let lc = read_log(&dc.join("default.log"));
    let lr = read_log(&dr.join("default.log"));
    if c.recs != r.recs {
        panic!("C2 recs differ: {:?} vs {:?}", c.recs, r.recs);
    }
    assert_bytes_eq("stdout", "C2", &c.stdout, &r.stdout);
    assert_bytes_eq("default.log", "C2", &lc, &lr);
    assert!(lc.starts_with(b"PRE\n"), "append mode expected, got {}", show(&lc));
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn c3_double_init() {
    diff(
        "C3",
        "c3",
        None,
        &[
            Op::InitLogger,
            Op::LogInfo(b("between")),
            Op::InitLogger,
            Op::LogInfo(b("after")),
            Op::FinalizeLogger,
        ],
    );
}

#[test]
fn c4_log_severities_random_ascii() {
    let mut rng = Rng::new(0xC4_0001);
    for it in 0..200u32 {
        let mut ops = vec![Op::InitLogger];
        let n = rng.range(1, 6);
        for _ in 0..n {
            let len = rng.range(0, 512) as usize;
            let msg = rng.ascii(len);
            ops.push(match rng.below(3) {
                0 => Op::LogInfo(msg),
                1 => Op::LogWarning(msg),
                _ => Op::LogError(msg),
            });
        }
        ops.push(Op::FinalizeLogger);
        diff(&format!("C4 it={it}"), "c4", None, &ops);
    }
}

#[test]
fn c5_log_arbitrary_bytes() {
    let mut rng = Rng::new(0xC5_0001);
    for it in 0..200u32 {
        let mut ops = vec![Op::InitLogger];
        for _ in 0..rng.range(1, 4) {
            let len = rng.range(0, 300) as usize;
            let msg: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
            ops.push(match rng.below(3) {
                0 => Op::LogInfo(msg),
                1 => Op::LogWarning(msg),
                _ => Op::LogError(msg),
            });
        }
        ops.push(Op::FinalizeLogger);
        diff(&format!("C5 it={it}"), "c5", None, &ops);
    }
}

// ==================================================== C6 .. C10 (create/destroy) ==

#[test]
fn c6_create_default_max_tasks() {
    diff(
        "C6",
        "c6",
        None,
        &[Op::InitLogger, Op::Create, Op::Snapshot, Op::Destroy, Op::FinalizeLogger],
    );
}

#[test]
fn c7_create_random_max_tasks() {
    let mut rng = Rng::new(0xC7_0001);
    for it in 0..200u32 {
        let m = rng.range(1, 4096).to_string();
        diff(
            &format!("C7 it={it} MAX_TASKS={m}"),
            "c7",
            Some(&m),
            &[Op::InitLogger, Op::Create, Op::Snapshot, Op::Destroy, Op::FinalizeLogger],
        );
    }
}

#[test]
fn c8_create_max_tasks_zero() {
    diff(
        "C8",
        "c8",
        Some("0"),
        &[Op::InitLogger, Op::Create, Op::Snapshot, Op::Destroy, Op::FinalizeLogger],
    );
}

#[test]
fn c9_create_max_tasks_parsing() {
    for v in [
        "", "abc", "7x", " 9", "+3", "-0", "0x10", "2147483647", "  12  ", "9.9", "007",
    ] {
        diff(
            &format!("C9 MAX_TASKS={v:?}"),
            "c9",
            Some(v),
            &[Op::InitLogger, Op::Create, Op::Snapshot, Op::Destroy, Op::FinalizeLogger],
        );
    }
}

#[test]
fn c10_create_destroy_lifecycle() {
    diff(
        "C10",
        "c10",
        Some("5"),
        &[
            Op::InitLogger,
            Op::Create,
            Op::Snapshot,
            Op::PrintTasks,
            Op::Destroy,
            Op::FinalizeLogger,
        ],
    );
}

// ========================================================= C11 .. C17 (add_task) ==

#[test]
fn c11_add_single_task_random() {
    let mut rng = Rng::new(0xC11_0001);
    for it in 0..200u32 {
        let desc = rng.ascii_between(1, 60);
        let prio = rng.i32();
        diff(
            &format!("C11 it={it}"),
            "c11",
            Some("10"),
            &[
                Op::InitLogger,
                Op::Create,
                Op::AddTask(desc, prio),
                Op::Snapshot,
                Op::PrintTasks,
                Op::Destroy,
                Op::FinalizeLogger,
            ],
        );
    }
}

#[test]
fn c12_add_n_tasks_random() {
    let mut rng = Rng::new(0xC12_0001);
    for it in 0..200u32 {
        let n = rng.range(1, 10);
        let mut ops = vec![Op::InitLogger, Op::Create];
        for _ in 0..n {
            ops.push(Op::AddTask(
                rng.ascii_between(0, 80),
                rng.i32(),
            ));
        }
        ops.extend([Op::Snapshot, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger]);
        diff(&format!("C12 it={it} n={n}"), "c12", Some("10"), &ops);
    }
}

#[test]
fn c13_capacity_one() {
    for n in [1usize, 2, 3] {
        let mut ops = vec![Op::InitLogger, Op::Create];
        for i in 0..n {
            ops.push(Op::AddTask(b(&format!("task{i}")), i as i32 + 1));
        }
        ops.extend([Op::Snapshot, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger]);
        diff(&format!("C13 n={n}"), "c13", Some("1"), &ops);
    }
}

#[test]
fn c14_capacity_three_overflow() {
    let mut ops = vec![Op::InitLogger, Op::Create];
    for i in 0..5 {
        ops.push(Op::AddTask(b(&format!("t{i}")), 100 - i));
    }
    ops.extend([Op::Snapshot, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger]);
    diff("C14", "c14", Some("3"), &ops);
}

#[test]
fn c15_desc_truncation_boundary() {
    for len in [0usize, 1, 253, 254, 255, 256, 257, 512, 1024] {
        let desc: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        diff(
            &format!("C15 len={len}"),
            "c15",
            Some("4"),
            &[
                Op::InitLogger,
                Op::Create,
                Op::AddTask(desc, 42),
                Op::Snapshot,
                Op::PrintTasks,
                Op::Destroy,
                Op::FinalizeLogger,
            ],
        );
    }
}

#[test]
fn c16_desc_arbitrary_bytes() {
    let mut rng = Rng::new(0xC16_0001);
    for it in 0..200u32 {
        let len = rng.range(1, 400) as usize;
        let desc: Vec<u8> = (0..len).map(|_| rng.range(1, 255) as u8).collect();
        diff(
            &format!("C16 it={it} len={len}"),
            "c16",
            Some("4"),
            &[
                Op::InitLogger,
                Op::Create,
                Op::AddTask(desc, rng.i32()),
                Op::Snapshot,
                Op::PrintTasks,
                Op::Destroy,
                Op::FinalizeLogger,
            ],
        );
    }
}

#[test]
fn c17_priority_values() {
    let mut fixed: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    let mut rng = Rng::new(0xC17_0001);
    for _ in 0..200 {
        fixed.push(rng.i32());
    }
    for (k, p) in fixed.iter().enumerate() {
        diff(
            &format!("C17 k={k} priority={p}"),
            "c17",
            Some("2"),
            &[
                Op::InitLogger,
                Op::Create,
                Op::AddTask(b("p"), *p),
                Op::Snapshot,
                Op::PrintTasks,
                Op::Destroy,
                Op::FinalizeLogger,
            ],
        );
    }
}

// ======================================================= C18 .. C22 (print/pipeline) ==

#[test]
fn c18_print_empty() {
    diff(
        "C18",
        "c18",
        Some("10"),
        &[Op::InitLogger, Op::Create, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger],
    );
}

#[test]
fn c19_print_one() {
    diff(
        "C19",
        "c19",
        Some("10"),
        &[
            Op::InitLogger,
            Op::Create,
            Op::AddTask(b("only one"), 7),
            Op::PrintTasks,
            Op::Destroy,
            Op::FinalizeLogger,
        ],
    );
}

#[test]
fn c20_print_many_with_specifiers() {
    let mut rng = Rng::new(0xC20_0001);
    let nasty = [
        &b"%s"[..],
        &b"%d %d %d"[..],
        &b"%n"[..],
        &b"%%"[..],
        &b"100%"[..],
        &b"a\tb"[..],
        &b"\x7f\x01"[..],
    ];
    for it in 0..100u32 {
        let mut ops = vec![Op::InitLogger, Op::Create];
        for i in 0..10 {
            let desc = if it % 2 == 0 {
                nasty[(i as usize + it as usize) % nasty.len()].to_vec()
            } else {
                rng.ascii_between(0, 120)
            };
            ops.push(Op::AddTask(desc, rng.i32()));
        }
        ops.extend([Op::Snapshot, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger]);
        diff(&format!("C20 it={it}"), "c20", Some("10"), &ops);
    }
}

#[test]
fn c21_print_after_capacity_guard() {
    let mut ops = vec![Op::InitLogger, Op::Create];
    for i in 0..12 {
        ops.push(Op::AddTask(b(&format!("over{i}")), i));
    }
    ops.extend([Op::Snapshot, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger]);
    diff("C21", "c21", Some("4"), &ops);
}

#[test]
fn c22_full_lowlevel_pipeline_random() {
    let mut rng = Rng::new(0xC22_0001);
    for it in 0..100u32 {
        let max = rng.range(1, 16);
        let n = rng.range(0, 24);
        let mut ops = vec![Op::InitLogger, Op::Create];
        for _ in 0..n {
            ops.push(Op::AddTask(
                rng.ascii_between(0, 300),
                rng.i32(),
            ));
        }
        ops.extend([Op::Snapshot, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger]);
        diff(
            &format!("C22 it={it} max={max} n={n}"),
            "c22",
            Some(&max.to_string()),
            &ops,
        );
    }
}

// ============================================================ C23 .. C33 (driver) ==

#[test]
fn c23_driver_single_line_no_newline() {
    diff("C23", "c23", None, &[Op::Driver(b("write the report"))]);
}

#[test]
fn c24_driver_single_line_trailing_newline() {
    diff("C24", "c24", None, &[Op::Driver(b("write the report\n"))]);
}

#[test]
fn c25_driver_multiline_no_trailing() {
    let mut rng = Rng::new(0xC25_0001);
    for it in 0..200u32 {
        let n = rng.range(1, 9);
        let lines: Vec<Vec<u8>> = (0..n)
            .map(|_| {
                let l = rng.range(0, 40) as usize;
                rng.ascii(l).into_iter().filter(|&c| c != b'\n').collect()
            })
            .collect();
        let input = lines.join(&b'\n');
        diff(&format!("C25 it={it} n={n}"), "c25", None, &[Op::Driver(input)]);
    }
}

#[test]
fn c26_driver_multiline_trailing_newline() {
    let mut rng = Rng::new(0xC26_0001);
    for it in 0..200u32 {
        let n = rng.range(1, 9);
        let lines: Vec<Vec<u8>> = (0..n)
            .map(|_| rng.ascii_between(0, 40))
            .collect();
        let mut input = lines.join(&b'\n');
        input.push(b'\n');
        diff(&format!("C26 it={it} n={n}"), "c26", None, &[Op::Driver(input)]);
    }
}

#[test]
fn c27_driver_empty_lines() {
    for s in [
        "\n", "\n\n", "\n\n\n", "a\n\nb", "\na", "a\n", "\n\na\n\n", "  \n  \n",
    ] {
        diff(&format!("C27 {s:?}"), "c27", None, &[Op::Driver(b(s))]);
    }
}

#[test]
fn c28_driver_long_lines() {
    let mut rng = Rng::new(0xC28_0001);
    for it in 0..100u32 {
        let n = rng.range(1, 4);
        let lines: Vec<Vec<u8>> = (0..n)
            .map(|_| rng.ascii_between(240, 600))
            .collect();
        let input = lines.join(&b'\n');
        diff(&format!("C28 it={it}"), "c28", None, &[Op::Driver(input)]);
    }
}

#[test]
fn c29_driver_max_tasks_smaller_than_lines() {
    let input = b("l1\nl2\nl3\nl4\nl5\nl6\nl7");
    diff("C29", "c29", Some("3"), &[Op::Driver(input)]);
}

#[test]
fn c30_driver_max_tasks_zero() {
    diff("C30", "c30", Some("0"), &[Op::Driver(b("a\nb\nc"))]);
}

#[test]
fn c31_driver_random_cross_product() {
    let mut rng = Rng::new(0xC31_0001);
    for it in 0..150u32 {
        let max = rng.range(1, 20);
        let n = rng.range(0, 30) as usize;
        let mut input: Vec<u8> = Vec::new();
        for i in 0..n {
            if i > 0 {
                input.push(b'\n');
            }
            input.extend(rng.ascii_between(0, 300));
        }
        if rng.below(2) == 0 {
            input.push(b'\n');
        }
        diff(
            &format!("C31 it={it} max={max} n={n}"),
            "c31",
            Some(&max.to_string()),
            &[Op::Driver(input)],
        );
    }
}

#[test]
fn c32_driver_twice_appends() {
    diff(
        "C32",
        "c32",
        Some("4"),
        &[Op::Driver(b("first run\nsecond")), Op::Driver(b("third\nfourth"))],
    );
}

#[test]
fn c33_driver_arbitrary_bytes() {
    let mut rng = Rng::new(0xC33_0001);
    for it in 0..150u32 {
        let n = rng.range(1, 6) as usize;
        let mut input: Vec<u8> = Vec::new();
        for i in 0..n {
            if i > 0 {
                input.push(b'\n');
            }
            input.extend(rng.bytes_no_nl_between(0, 300));
        }
        diff(&format!("C33 it={it}"), "c33", None, &[Op::Driver(input)]);
    }
}

#[test]
fn c34_large_allocatable_max_tasks_high_indices() {
    // Exercises `tasks[i]` at the far end of a large allocation — the only place
    // where an index/offset computation could overflow or diverge.
    for max in ["20000", "100000"] {
        let n: usize = max.parse::<usize>().unwrap() + 3; // fill exactly, then overflow
        let mut ops = vec![Op::InitLogger, Op::Create];
        for i in 0..n {
            ops.push(Op::AddTask(b(&format!("t{i}")), i as i32 - 1));
        }
        ops.extend([Op::Snapshot, Op::PrintTasks, Op::Destroy, Op::FinalizeLogger]);
        diff(&format!("C34 MAX_TASKS={max} n={n}"), "c34", Some(max), &ops);
    }
}

#[test]
fn c35_driver_very_long_input() {
    // Long single line (well past the 255 truncation bound) and a long line count.
    let mut rng = Rng::new(0xC35_0001);
    let one_long = rng.ascii(20_000);
    diff("C35 one 20k line", "c35", Some("4"), &[Op::Driver(one_long)]);

    let mut many: Vec<u8> = Vec::new();
    for i in 0..500 {
        if i > 0 {
            many.push(b'\n');
        }
        many.extend(rng.ascii_between(0, 400));
    }
    diff("C35 500 lines", "c35", Some("64"), &[Op::Driver(many)]);
}

// Silence unused-import warnings for items only used by other test files.
#[allow(unused)]
fn _unused(_: PathBuf, _: *const c_char) {}
