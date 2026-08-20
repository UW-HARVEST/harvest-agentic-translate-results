//! Phase B (CONFIGS.md rows 35-50) and Phase C (ERRORS.md rows 37-64):
//! differential tests of the translated *program*.
//!
//! `c_src/build/driver` (built from the unmodified C by CMake) and
//! `target/debug/driver` (the Rust translation of `main.c`) are fed identical
//! stdin scripts; stdout, stderr and the exit status are compared byte for
//! byte. This is what exercises `src/main.rs`, `src/cio.rs` and
//! `src/dag_lib.rs`.

mod common;

use common::Rng;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// running the two programs
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_driver() -> PathBuf {
    let p = crate_root().join("c_src/build/driver");
    if !p.exists() {
        let build = crate_root().join("c_src/build");
        std::fs::create_dir_all(&build).ok();
        let ok = Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
            && Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        assert!(ok, "failed to build the C driver with cmake");
    }
    p
}

fn rust_driver() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let debug = exe.parent().and_then(|p| p.parent()).expect("target/debug");
    let direct = debug.join("driver");
    if direct.exists() {
        return direct;
    }
    let p = crate_root().join("target/debug/driver");
    assert!(
        p.exists(),
        "{} does not exist - run `cargo build --offline` first",
        p.display()
    );
    p
}

struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    merged: Vec<u8>,
    status: String,
    /// `None` when the process was killed by a signal.
    code: Option<i32>,
    /// The fatal signal, if any.
    signal: Option<i32>,
}

fn temp_path(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("dagprog-{}-{}-{}", std::process::id(), n, tag))
}

/// Runs `exe` with `input` on stdin. `merge` sends stdout and stderr to the same
/// file, which makes their interleaving observable.
fn run(exe: &PathBuf, input: &[u8], merge: bool) -> Outcome {
    let in_path = temp_path("in");
    let out_path = temp_path("out");
    let err_path = temp_path("err");
    {
        let mut f = std::fs::File::create(&in_path).expect("create stdin file");
        f.write_all(input).expect("write stdin file");
    }
    let stdin = std::fs::File::open(&in_path).expect("open stdin file");
    let out = std::fs::File::create(&out_path).expect("create stdout file");
    let err = if merge {
        out.try_clone().expect("clone stdout file")
    } else {
        std::fs::File::create(&err_path).expect("create stderr file")
    };
    let status = Command::new(exe)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));

    let read = |p: &PathBuf| std::fs::read(p).unwrap_or_default();
    let signal = std::os::unix::process::ExitStatusExt::signal(&status);
    let outcome = if merge {
        Outcome {
            stdout: Vec::new(),
            stderr: Vec::new(),
            merged: read(&out_path),
            status: format!("{status}"),
            code: status.code(),
            signal,
        }
    } else {
        Outcome {
            stdout: read(&out_path),
            stderr: read(&err_path),
            merged: Vec::new(),
            status: format!("{status}"),
            code: status.code(),
            signal,
        }
    };
    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);
    outcome
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn diff_report(label: &str, what: &str, a: &[u8], b: &[u8], input: &[u8]) -> String {
    let sa = show(a);
    let sb = show(b);
    let first = sa
        .bytes()
        .zip(sb.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or(sa.len().min(sb.len()));
    let from = first.saturating_sub(120);
    format!(
        "[{label}] {what} differs at byte {first}\n\
         --- C ----\n{}\n--- Rust ----\n{}\n--- stdin ----\n{}\n",
        &sa[from..(first + 200).min(sa.len())],
        &sb[from..(first + 200).min(sb.len())],
        show(&input[..input.len().min(1500)])
    )
}

/// Fatal signals a stack overrun can produce; which one the C gets depends on
/// the memory layout ASLR happens to pick, so the *same* input kills the C with
/// SIGSEGV on some runs and with SIGBUS on others (measured: 19 vs. 11 out of 30
/// runs). Only "died from one of these" is reproducible.
const CRASH_SIGNALS: [i32; 3] = [11 /* SIGSEGV */, 7 /* SIGBUS */, 6 /* SIGABRT */];

/// Compares the way the two processes ended. A normal exit must match exactly;
/// a fatal signal only has to be a fatal signal on both sides.
fn assert_same_ending(label: &str, c: &Outcome, r: &Outcome) {
    match c.signal {
        None => assert_eq!(
            (c.code, c.signal),
            (r.code, r.signal),
            "[{label}] exit status differs (C {} vs Rust {})",
            c.status,
            r.status
        ),
        Some(sig) => {
            assert!(
                CRASH_SIGNALS.contains(&sig),
                "[{label}] unexpected C signal {sig}"
            );
            let rsig = r.signal.unwrap_or_else(|| {
                panic!(
                    "[{label}] the C died from signal {sig} but Rust exited normally ({})",
                    r.status
                )
            });
            assert!(
                CRASH_SIGNALS.contains(&rsig),
                "[{label}] unexpected Rust signal {rsig}"
            );
        }
    }
}

/// Compares stdout, stderr and the exit status of the two programs.
fn assert_prog_same(label: &str, input: &[u8]) {
    let c = run(&c_driver(), input, false);
    let r = run(&rust_driver(), input, false);
    if c.stdout != r.stdout {
        panic!("{}", diff_report(label, "stdout", &c.stdout, &r.stdout, input));
    }
    if c.stderr != r.stderr {
        panic!("{}", diff_report(label, "stderr", &c.stderr, &r.stderr, input));
    }
    assert_same_ending(label, &c, &r);
}

/// Same, but with both streams pointed at one file so that the ordering of the
/// buffered stdout writes relative to the unbuffered stderr writes matters.
fn assert_prog_same_merged(label: &str, input: &[u8]) {
    let c = run(&c_driver(), input, true);
    let r = run(&rust_driver(), input, true);
    if c.merged != r.merged {
        panic!("{}", diff_report(label, "merged output", &c.merged, &r.merged, input));
    }
    assert_same_ending(label, &c, &r);
}

/// `main.c` prints the reference count *before* decrementing it, so this line
/// marks the point at which the next `delete_node()` releases the node's
/// storage while `graph->nodes[]` still points at it. Everything the C prints
/// after that depends on the allocator's book-keeping bytes, which are not
/// reproducible (verified: three runs of the same script print three different
/// city names for the freed node), so comparisons stop here.
const FREES_NEXT: &[u8] = b"Current ref count: 1\n";

fn free_boundary(stdout: &[u8]) -> Option<usize> {
    stdout
        .windows(FREES_NEXT.len())
        .position(|w| w == FREES_NEXT)
        .map(|p| p + FREES_NEXT.len())
}

/// `(rounds, truncated rounds, compared bytes, total C bytes)` of every
/// comparison made through `assert_prog_same_until_free`.
static STATS: std::sync::Mutex<(usize, usize, usize, usize)> = std::sync::Mutex::new((0, 0, 0, 0));

fn stats_add(truncated: bool, compared: usize, total: usize) {
    let mut g = STATS.lock().unwrap_or_else(|p| p.into_inner());
    g.0 += 1;
    if truncated {
        g.1 += 1;
    }
    g.2 += compared;
    g.3 += total;
}

fn stats_report(label: &str) {
    let g = STATS.lock().unwrap_or_else(|p| p.into_inner());
    println!(
        "[{label}] {} scripts, {} truncated at the C's use-after-free, \
         {}/{} stdout bytes compared ({:.1}%)",
        g.0,
        g.1,
        g.2,
        g.3,
        100.0 * g.2 as f64 / g.3.max(1) as f64
    );
}

/// Full comparison, unless the script makes the C release a node that is still
/// referenced by the graph - then only the deterministic prefix is compared.
fn assert_prog_same_until_free(label: &str, input: &[u8]) {
    let c = run(&c_driver(), input, false);
    let r = run(&rust_driver(), input, false);
    match free_boundary(&c.stdout) {
        None => {
            stats_add(false, c.stdout.len(), c.stdout.len());
            if c.stdout != r.stdout {
                panic!("{}", diff_report(label, "stdout", &c.stdout, &r.stdout, input));
            }
            if c.stderr != r.stderr {
                panic!("{}", diff_report(label, "stderr", &c.stderr, &r.stderr, input));
            }
            assert_same_ending(label, &c, &r);
        }
        Some(n) => {
            stats_add(true, n, c.stdout.len());
            assert!(
                r.stdout.len() >= n,
                "[{label}] Rust stopped after {} bytes, C is defined for {n}",
                r.stdout.len()
            );
            if c.stdout[..n] != r.stdout[..n] {
                panic!(
                    "{}",
                    diff_report(label, "stdout (defined prefix)", &c.stdout[..n], &r.stdout[..n], input)
                );
            }
        }
    }
}

/// Compares only the first `prefix` bytes of stdout/stderr; used where the C
/// runs into undefined behaviour (a node that has been `free()`d is still
/// reachable through `graph->nodes[]`).
fn assert_prog_same_prefix(label: &str, input: &[u8], prefix: usize) {
    let c = run(&c_driver(), input, false);
    let r = run(&rust_driver(), input, false);
    let n = prefix.min(c.stdout.len()).min(r.stdout.len());
    if c.stdout[..n] != r.stdout[..n] {
        panic!(
            "{}",
            diff_report(label, "stdout prefix", &c.stdout[..n], &r.stdout[..n], input)
        );
    }
    let m = prefix.min(c.stderr.len()).min(r.stderr.len());
    if c.stderr[..m] != r.stderr[..m] {
        panic!(
            "{}",
            diff_report(label, "stderr prefix", &c.stderr[..m], &r.stderr[..m], input)
        );
    }
}

// ---------------------------------------------------------------------------
// script builder
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Script(Vec<u8>);

impl Script {
    fn new() -> Script {
        Script(Vec::new())
    }
    /// A raw line (a `\n` is appended).
    fn line(&mut self, bytes: &[u8]) -> &mut Script {
        self.0.extend_from_slice(bytes);
        self.0.push(b'\n');
        self
    }
    fn n(&mut self, v: i64) -> &mut Script {
        self.line(v.to_string().as_bytes())
    }
    /// menu 1: add city
    fn add_city(&mut self, name: &[u8]) -> &mut Script {
        self.n(1).line(name)
    }
    /// menu 2: add route
    fn add_route(&mut self, from: &[u8], to: &[u8], distance: i64) -> &mut Script {
        self.n(2).line(from).line(to).n(distance)
    }
    /// menu 3: show all
    fn show_all(&mut self) -> &mut Script {
        self.n(3)
    }
    /// menu 4: details
    fn details(&mut self, name: &[u8]) -> &mut Script {
        self.n(4).line(name)
    }
    /// menu 5: shortest path
    fn path(&mut self, from: &[u8], to: &[u8]) -> &mut Script {
        self.n(5).line(from).line(to)
    }
    /// menu 6: shallow copy
    fn copy(&mut self, name: &[u8]) -> &mut Script {
        self.n(6).line(name)
    }
    /// menu 7: delete
    fn delete(&mut self, name: &[u8]) -> &mut Script {
        self.n(7).line(name)
    }
    /// menu 8: exit
    fn exit(&mut self) -> &mut Script {
        self.n(8)
    }
    fn bytes(&self) -> &[u8] {
        &self.0
    }
}

fn city(i: usize) -> Vec<u8> {
    format!("City{i}").into_bytes()
}

// ---------------------------------------------------------------------------
// CONFIGS rows 35-48
// ---------------------------------------------------------------------------

/// CONFIGS row 35
#[test]
fn cfg_prog_exit_immediately() {
    let mut s = Script::new();
    s.exit();
    assert_prog_same("cfg_prog_exit_immediately", s.bytes());
    assert_prog_same_merged("cfg_prog_exit_immediately merged", s.bytes());
}

/// CONFIGS row 36 / ERRORS row 37
#[test]
fn cfg_prog_empty_stdin() {
    assert_prog_same("cfg_prog_empty_stdin", b"");
    assert_prog_same("cfg_prog_empty_stdin nl", b"\n");
    assert_prog_same_merged("cfg_prog_empty_stdin merged", b"");
}

/// CONFIGS row 37
#[test]
fn cfg_prog_add_cities() {
    // one city
    let mut s = Script::new();
    s.add_city(b"Paris").show_all().exit();
    assert_prog_same("cfg_prog_add_cities one", s.bytes());

    // a few cities plus lookups
    let mut s = Script::new();
    for i in 0..3 {
        s.add_city(&city(i));
    }
    s.show_all().details(&city(1)).exit();
    assert_prog_same("cfg_prog_add_cities three", s.bytes());

    // name length boundaries around MAX_CITY_NAME - 1 = 63
    for len in [0usize, 1, 2, 62, 63, 64, 65, 66, 120, 254, 255, 256, 300] {
        let name: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let mut s = Script::new();
        s.add_city(&name);
        s.show_all();
        s.details(&name);
        s.details(&name[..name.len().min(63)].to_vec());
        s.exit();
        assert_prog_same(&format!("cfg_prog_add_cities len={len}"), s.bytes());
    }

    // fill the graph to MAX_NODES and beyond
    let mut s = Script::new();
    for i in 0..102 {
        s.add_city(&city(i));
    }
    s.show_all().exit();
    assert_prog_same("cfg_prog_add_cities full", s.bytes());
    assert_prog_same_merged("cfg_prog_add_cities full merged", s.bytes());
}

/// CONFIGS row 38
#[test]
fn cfg_prog_add_routes() {
    for d in [0i64, 1, 2, 1000, 2147483646, 2147483647] {
        let mut s = Script::new();
        s.add_city(b"A").add_city(b"B");
        s.add_route(b"A", b"B", d);
        s.show_all();
        s.path(b"A", b"B");
        s.exit();
        assert_prog_same(&format!("cfg_prog_add_routes d={d}"), s.bytes());
    }

    // ten out-edges (MAX_EDGES) and one too many
    let mut s = Script::new();
    s.add_city(b"hub");
    for i in 0..12 {
        s.add_city(&city(i));
    }
    for i in 0..12 {
        s.add_route(b"hub", &city(i), i as i64 + 1);
    }
    s.details(b"hub").exit();
    assert_prog_same("cfg_prog_add_routes max_edges", s.bytes());

    // self route and both directions
    let mut s = Script::new();
    s.add_city(b"A").add_city(b"B");
    s.add_route(b"A", b"A", 3);
    s.add_route(b"A", b"A", 4);
    s.add_route(b"A", b"B", 5);
    s.add_route(b"B", b"A", 6);
    s.show_all();
    s.path(b"A", b"B").path(b"B", b"A").path(b"A", b"A");
    s.exit();
    assert_prog_same("cfg_prog_add_routes self_and_reverse", s.bytes());
}

/// CONFIGS row 39
#[test]
fn cfg_prog_show_all() {
    for n in [0usize, 1, 2, 5, 20] {
        let mut s = Script::new();
        s.show_all();
        for i in 0..n {
            s.add_city(&city(i));
        }
        for i in 0..n {
            if i + 1 < n {
                s.add_route(&city(i), &city(i + 1), (i as i64 + 1) * 3);
            }
            if i % 4 == 0 && n > 2 {
                s.add_route(&city(i), &city((i + 2) % n), 7);
            }
        }
        s.show_all().exit();
        assert_prog_same(&format!("cfg_prog_show_all n={n}"), s.bytes());
    }
}

/// CONFIGS row 40 / ERRORS row 50
#[test]
fn cfg_prog_show_details() {
    let mut s = Script::new();
    s.details(b"nothing");
    for i in 0..5 {
        s.add_city(&city(i));
    }
    s.add_route(&city(0), &city(1), 1);
    s.add_route(&city(0), &city(2), 2);
    s.details(&city(0));
    s.details(&city(2));
    s.details(&city(4));
    s.details(b"missing");
    s.details(b"");
    s.exit();
    assert_prog_same("cfg_prog_show_details", s.bytes());
}

/// CONFIGS row 41
#[test]
fn cfg_prog_shortest_path() {
    // diamond with a tie plus a longer route
    let mut s = Script::new();
    for i in 0..5 {
        s.add_city(&city(i));
    }
    s.add_route(&city(0), &city(1), 5);
    s.add_route(&city(0), &city(2), 5);
    s.add_route(&city(1), &city(3), 5);
    s.add_route(&city(2), &city(3), 5);
    s.add_route(&city(3), &city(4), 1);
    s.path(&city(0), &city(0));
    s.path(&city(0), &city(1));
    s.path(&city(0), &city(3));
    s.path(&city(0), &city(4));
    s.path(&city(4), &city(0));
    s.path(&city(0), b"missing");
    s.path(b"missing", &city(0));
    s.exit();
    assert_prog_same("cfg_prog_shortest_path diamond", s.bytes());
    assert_prog_same_merged("cfg_prog_shortest_path diamond merged", s.bytes());

    // re-relaxation: the direct route is discovered first but is expensive
    let mut s = Script::new();
    for i in 0..4 {
        s.add_city(&city(i));
    }
    s.add_route(&city(0), &city(3), 100);
    s.add_route(&city(0), &city(1), 2);
    s.add_route(&city(1), &city(2), 2);
    s.add_route(&city(2), &city(3), 2);
    s.path(&city(0), &city(3));
    s.exit();
    assert_prog_same("cfg_prog_shortest_path relaxation", s.bytes());

    // unreachable / disconnected
    let mut s = Script::new();
    for i in 0..4 {
        s.add_city(&city(i));
    }
    s.add_route(&city(0), &city(1), 1);
    s.add_route(&city(2), &city(3), 1);
    s.path(&city(0), &city(2));
    s.path(&city(1), &city(0));
    s.exit();
    assert_prog_same("cfg_prog_shortest_path unreachable", s.bytes());
    assert_prog_same_merged("cfg_prog_shortest_path unreachable merged", s.bytes());

    // long chain (100 cities, the state array is exactly full)
    let mut s = Script::new();
    for i in 0..100 {
        s.add_city(&city(i));
    }
    for i in 0..99 {
        s.add_route(&city(i), &city(i + 1), 1);
    }
    s.path(&city(0), &city(99));
    s.path(&city(99), &city(0));
    s.exit();
    assert_prog_same("cfg_prog_shortest_path long_chain", s.bytes());
}

/// CONFIGS row 42
#[test]
fn cfg_prog_shallow_copy() {
    let mut s = Script::new();
    for i in 0..4 {
        s.add_city(&city(i));
    }
    s.add_route(&city(0), &city(1), 1);
    s.add_route(&city(1), &city(2), 1);
    s.add_route(&city(2), &city(0), 1); // cycle
    s.copy(&city(0));
    s.show_all();
    s.copy(&city(0));
    s.copy(&city(3));
    s.copy(b"missing");
    s.show_all();
    s.exit();
    assert_prog_same("cfg_prog_shallow_copy", s.bytes());
    assert_prog_same_merged("cfg_prog_shallow_copy merged", s.bytes());
}

/// CONFIGS row 43
#[test]
fn cfg_prog_delete_refcount_high() {
    let mut s = Script::new();
    for i in 0..3 {
        s.add_city(&city(i));
    }
    s.add_route(&city(0), &city(1), 1);
    s.add_route(&city(1), &city(2), 1);
    // three shallow copies -> ref_count 4 on every reachable node
    s.copy(&city(0));
    s.copy(&city(0));
    s.copy(&city(0));
    s.show_all();
    s.delete(&city(0));
    s.show_all();
    s.delete(&city(0));
    s.delete(&city(1));
    s.show_all();
    s.delete(b"missing");
    s.exit();
    assert_prog_same("cfg_prog_delete_refcount_high", s.bytes());
    assert_prog_same_merged("cfg_prog_delete_refcount_high merged", s.bytes());
}

/// CONFIGS row 44: the reference count reaches zero, so the C `free()`s a node
/// that `graph->nodes[]` still points at. Everything up to that point is fully
/// defined and is compared; see ERRORS.md.
#[test]
fn cfg_prog_delete_to_zero_then_reuse() {
    let mut s = Script::new();
    s.add_city(b"A").add_city(b"B");
    s.add_route(b"A", b"B", 1);
    s.show_all();
    s.delete(b"A");
    // from here on the C reads freed memory
    s.show_all();
    s.add_city(b"C");
    s.show_all();
    s.exit();
    // The deterministic part is everything before the second "3" command.
    let boundary = {
        let mut probe = Script::new();
        probe.add_city(b"A").add_city(b"B");
        probe.add_route(b"A", b"B", 1);
        probe.show_all();
        probe.delete(b"A");
        run(&c_driver(), probe.bytes(), false).stdout.len()
    };
    assert_prog_same_prefix("cfg_prog_delete_to_zero_then_reuse", s.bytes(), boundary);
}

/// CONFIGS row 45 / ERRORS rows 43, 45, 48, 53
#[test]
fn cfg_prog_stream_interleaving() {
    let mut s = Script::new();
    s.add_city(b"A");
    s.add_city(b"A"); // stderr: already exists, stdout: Failed to add city
    s.add_city(b"B");
    s.add_route(b"A", b"B", -5); // stderr: negative distance
    s.add_route(b"A", b"B", 1);
    s.add_route(b"A", b"B", 2); // stderr: edge already exists
    s.path(b"B", b"A"); // stderr: No path found + stdout: No path found
    s.copy(b"missing");
    s.exit();
    assert_prog_same("cfg_prog_stream_interleaving", s.bytes());
    assert_prog_same_merged("cfg_prog_stream_interleaving merged", s.bytes());
}

/// CONFIGS row 46: more than one 4096 byte stdio buffer of output.
#[test]
fn cfg_prog_large_output() {
    let mut s = Script::new();
    for i in 0..60 {
        s.add_city(&city(i));
    }
    for i in 0..59 {
        s.add_route(&city(i), &city(i + 1), i as i64 + 1);
    }
    for k in 0..6 {
        s.show_all();
        // stderr writes are unbuffered while stdout is not, so their relative
        // order depends on where the 4096 byte stdout buffer happens to flush
        s.add_city(&city(k)); // stderr: already exists
        s.add_route(&city(k), &city(k + 1), -1); // stderr: negative distance
        s.path(&city(59), &city(0)); // stderr: No path found
    }
    s.path(&city(0), &city(59));
    s.exit();
    assert_prog_same("cfg_prog_large_output", s.bytes());
    assert_prog_same_merged("cfg_prog_large_output merged", s.bytes());
}

/// CONFIGS row 47 / ERRORS rows 60, 61: the `MAX_INPUT` = 256 `fgets` boundary.
#[test]
fn cfg_prog_fgets_boundary() {
    for len in [253usize, 254, 255, 256, 257, 600] {
        // an over-long *menu* line: fgets splits it and the tail is read as the
        // next command
        let long: Vec<u8> = std::iter::repeat(b'7').take(len).collect();
        let mut input = Vec::new();
        input.extend_from_slice(&long);
        input.push(b'\n');
        input.extend_from_slice(b"8\n");
        assert_prog_same(&format!("cfg_prog_fgets_boundary menu len={len}"), &input);

        // an over-long *city name*
        let name: Vec<u8> = (0..len).map(|i| b'a' + (i % 26) as u8).collect();
        let mut s = Script::new();
        s.add_city(&name);
        s.show_all();
        s.exit();
        assert_prog_same(&format!("cfg_prog_fgets_boundary name len={len}"), s.bytes());

        // a long line where the tail happens to be a valid command
        let mut input = Vec::new();
        input.extend_from_slice(b"1\n");
        input.extend_from_slice(&vec![b'x'; len]);
        input.extend_from_slice(b"3\n8\n");
        assert_prog_same(&format!("cfg_prog_fgets_boundary tail len={len}"), &input);
    }
}

/// CONFIGS row 48 / ERRORS row 62
#[test]
fn cfg_prog_no_final_newline() {
    assert_prog_same("cfg_prog_no_final_newline 8", b"8");
    assert_prog_same("cfg_prog_no_final_newline 3", b"3");
    assert_prog_same("cfg_prog_no_final_newline city", b"1\nParis");
    assert_prog_same("cfg_prog_no_final_newline route", b"1\nA\n1\nB\n2\nA\nB");
    assert_prog_same("cfg_prog_no_final_newline path", b"1\nA\n5\nA");
    assert_prog_same("cfg_prog_no_final_newline choice", b"1\nA\n3");
}

// ---------------------------------------------------------------------------
// CONFIGS rows 49-50: randomised scripts
// ---------------------------------------------------------------------------

/// A model of the graph that is precise enough to keep the generated scripts
/// away from the C's use-after-free (see ERRORS.md row 59).
struct Model {
    stored: Vec<Vec<u8>>,
    succ: Vec<Vec<usize>>,
    ref_count: Vec<i32>,
}

impl Model {
    fn new() -> Model {
        Model {
            stored: Vec::new(),
            succ: Vec::new(),
            ref_count: Vec::new(),
        }
    }
    fn find(&self, name: &[u8]) -> Option<usize> {
        self.stored.iter().position(|s| s.as_slice() == name)
    }
    fn add_node(&mut self, name: &[u8]) {
        if self.stored.len() >= 100 || self.find(name).is_some() {
            return;
        }
        self.stored.push(name[..name.len().min(63)].to_vec());
        self.succ.push(Vec::new());
        self.ref_count.push(1);
    }
    fn add_edge(&mut self, from: &[u8], to: &[u8], distance: i64) {
        let (a, b) = match (self.find(from), self.find(to)) {
            (Some(a), Some(b)) => (a, b),
            _ => return,
        };
        if self.succ[a].len() >= 10 || distance < 0 || self.succ[a].contains(&b) {
            return;
        }
        self.succ[a].push(b);
    }
    fn shallow_copy(&mut self, name: &[u8]) {
        let start = match self.find(name) {
            Some(s) => s,
            None => return,
        };
        let mut seen = vec![false; self.stored.len()];
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            if seen[n] {
                continue;
            }
            seen[n] = true;
            self.ref_count[n] += 1;
            for &m in self.succ[n].iter().rev() {
                if !seen[m] {
                    stack.push(m);
                }
            }
        }
    }
}

fn random_script(rng: &mut Rng, commands: usize, allow_free: bool) -> Vec<u8> {
    let mut s = Script::new();
    let mut m = Model::new();
    let mut pool: Vec<Vec<u8>> = Vec::new();

    for _ in 0..commands {
        let known = |rng: &mut Rng, pool: &Vec<Vec<u8>>| -> Vec<u8> {
            if pool.is_empty() || rng.below(5) == 0 {
                rng.name(12)
            } else {
                pool[rng.below(pool.len())].clone()
            }
        };
        match rng.below(12) {
            0 | 1 | 2 => {
                let name = if rng.below(4) == 0 {
                    known(rng, &pool)
                } else {
                    let max_len = match rng.below(4) {
                        0 => 2,
                        1 => 8,
                        2 => 64,
                        _ => 70,
                    };
                    let mut n = rng.name(max_len);
                    if n.is_empty() && rng.bool() {
                        n.push(b'z');
                    }
                    n
                };
                s.add_city(&name);
                m.add_node(&name);
                if !pool.contains(&name) {
                    pool.push(name);
                }
            }
            3 | 4 => {
                let from = known(rng, &pool);
                let to = known(rng, &pool);
                let d = match rng.below(6) {
                    0 => 0,
                    1 => 1,
                    2 => rng.range_i32(0, 20) as i64,
                    3 => rng.range_i32(0, 1_000_000) as i64,
                    4 => -(rng.range_i32(1, 1000) as i64),
                    _ => 2147483647,
                };
                s.add_route(&from, &to, d);
                m.add_edge(&from, &to, d);
            }
            5 => {
                s.show_all();
            }
            6 => {
                let n = known(rng, &pool);
                s.details(&n);
            }
            7 | 8 => {
                let a = known(rng, &pool);
                let b = known(rng, &pool);
                s.path(&a, &b);
            }
            9 => {
                let n = known(rng, &pool);
                s.copy(&n);
                m.shallow_copy(&n);
            }
            10 => {
                let n = known(rng, &pool);
                let safe = match m.find(&n) {
                    Some(i) => m.ref_count[i] > 1,
                    None => true, // "not found" is harmless
                };
                if allow_free || safe {
                    if let Some(i) = m.find(&n) {
                        m.ref_count[i] -= 1;
                    }
                    s.delete(&n);
                }
            }
            _ => {
                // An invalid or out-of-range menu choice. Only values that do
                // not consume further lines are used here, so that the model
                // above stays in step with the command stream.
                match rng.below(5) {
                    0 => {
                        s.line(b"not-a-number");
                    }
                    1 => {
                        s.n([-3i64, -2, -1, 0, 9, 10, 11, 12][rng.below(8)]);
                    }
                    2 => {
                        s.line(b"");
                    }
                    3 => {
                        s.line(b"  12abc");
                    }
                    _ => {
                        s.n(2147483647);
                    }
                }
            }
        }
    }
    if rng.bool() {
        s.exit();
    }
    s.bytes().to_vec()
}

/// CONFIGS row 49
#[test]
fn cfg_prog_random_scripts() {
    let mut rng = Rng::new(0x1234_5678_9ABC);
    for round in 0..300 {
        let input = random_script(&mut rng, 80, false);
        assert_prog_same_until_free(&format!("cfg_prog_random_scripts round={round}"), &input);
        if round % 10 == 0 && free_boundary(&run(&c_driver(), &input, false).stdout).is_none() {
            assert_prog_same_merged(
                &format!("cfg_prog_random_scripts merged round={round}"),
                &input,
            );
        }
    }
    stats_report("cfg_prog_random_scripts");
}

/// CONFIGS row 50: scripts that may drive a reference count to zero. The C then
/// keeps using a freed node, which is undefined behaviour; the comparison
/// therefore stops at the first `delete` that frees something, but everything
/// before it (usually the bulk of the script) is compared exactly.
#[test]
fn cfg_prog_random_scripts_with_delete() {
    let mut rng = Rng::new(0xFEED_FACE);
    let mut with_ub = 0usize;
    for round in 0..200 {
        let input = random_script(&mut rng, 60, true);
        if free_boundary(&run(&c_driver(), &input, false).stdout).is_some() {
            with_ub += 1;
        }
        assert_prog_same_until_free(
            &format!("cfg_prog_random_scripts_with_delete round={round}"),
            &input,
        );
    }
    // the point of this row is to actually reach the freeing deletes
    assert!(with_ub > 0, "no round released a node");
    stats_report("cfg_prog_random_scripts_with_delete");
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows 38-64 that are not already covered above
// ---------------------------------------------------------------------------

/// ERRORS rows 38, 40, 41
#[test]
fn err_prog_invalid_input() {
    let cases: Vec<&[u8]> = vec![
        b"abc\n8\n",
        b"\n8\n",
        b"+\n8\n",
        b"-\n8\n",
        b"x1\n8\n",
        b".5\n8\n",
        b"  \n8\n",
        b"\t\n8\n",
        b"one\ntwo\nthree\n8\n",
        // accepted by %d despite the surroundings
        b"  3xyz\n8\n",
        b"+3\n8\n",
        b"-3\n8\n",
        b"03\n8\n",
        b"3 4\n8\n",
        b"8abc\n",
        // out of int range
        b"99999999999999\n8\n",
        b"-99999999999999\n8\n",
        b"2147483648\n8\n",
        b"4294967297\n8\n",
        b"0x8\n8\n",
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_prog_same(&format!("err_prog_invalid_input case={i}"), c);
    }
}

/// ERRORS rows 39, 64: every choice around the valid 1..8 range.
#[test]
fn err_prog_invalid_choice() {
    for v in -3i64..=12 {
        let mut input = Vec::new();
        input.extend_from_slice(v.to_string().as_bytes());
        input.push(b'\n');
        // keep the session alive for the non-terminating choices
        input.extend_from_slice(b"missing\nmissing\n0\n8\n");
        assert_prog_same(&format!("err_prog_invalid_choice v={v}"), &input);
    }
    for v in [i32::MIN as i64, i32::MAX as i64, -2147483649, 2147483648] {
        let mut input = Vec::new();
        input.extend_from_slice(v.to_string().as_bytes());
        input.extend_from_slice(b"\n8\n");
        assert_prog_same(&format!("err_prog_invalid_choice extreme v={v}"), &input);
    }
}

/// ERRORS rows 42, 44, 49, 51, 54, 57: EOF at every prompt.
#[test]
fn err_prog_eof_at_prompts() {
    let cases: Vec<&[u8]> = vec![
        b"1\n",              // after "Enter city name"
        b"2\n",              // after "Enter from city"
        b"2\nA\n",           // after "Enter to city"
        b"2\nA\nB\n",        // after "Enter distance"
        b"4\n",              // details
        b"5\n",              // start city
        b"5\nA\n",           // end city
        b"6\n",              // shallow copy
        b"7\n",              // delete
        b"3\n",              // show all then EOF
        b"1\nA\n2\nA\n",     // partially filled route
        b"1\nA\n5\nA\n",     // partially filled path
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_prog_same(&format!("err_prog_eof_at_prompts case={i}"), c);
        assert_prog_same_merged(&format!("err_prog_eof_at_prompts merged case={i}"), c);
    }
}

/// ERRORS rows 43, 45, 46, 47, 48
#[test]
fn err_prog_route_and_city_failures() {
    let mut s = Script::new();
    // duplicate city -> "Failed to add city"
    s.add_city(b"A");
    s.add_city(b"A");
    // route with an unparsable distance
    s.n(2).line(b"A").line(b"A").line(b"abc");
    // from city missing (note: the distance is read first)
    s.add_route(b"nope", b"A", 5);
    // to city missing
    s.add_route(b"A", b"nope", 5);
    // both missing
    s.add_route(b"nope", b"nope2", 5);
    // negative distance -> add_edge fails
    s.add_route(b"A", b"A", -1);
    // duplicate edge
    s.add_route(b"A", b"A", 1);
    s.add_route(b"A", b"A", 1);
    s.show_all();
    s.exit();
    assert_prog_same("err_prog_route_and_city_failures", s.bytes());
    assert_prog_same_merged("err_prog_route_and_city_failures merged", s.bytes());
}

/// ERRORS rows 52, 53, 55, 58
#[test]
fn err_prog_missing_cities() {
    let mut s = Script::new();
    s.add_city(b"A").add_city(b"B");
    s.path(b"A", b"B"); // no route -> No path found
    s.path(b"nope", b"B");
    s.path(b"A", b"nope");
    s.details(b"nope");
    s.copy(b"nope");
    s.delete(b"nope");
    s.exit();
    assert_prog_same("err_prog_missing_cities", s.bytes());
    assert_prog_same_merged("err_prog_missing_cities merged", s.bytes());
}

/// ERRORS row 63
#[test]
fn err_prog_empty_city_name() {
    let mut s = Script::new();
    s.add_city(b"");
    s.show_all();
    s.details(b"");
    s.add_city(b"");
    s.add_route(b"", b"", 0);
    s.path(b"", b"");
    s.copy(b"");
    s.show_all();
    s.exit();
    assert_prog_same("err_prog_empty_city_name", s.bytes());
    assert_prog_same_merged("err_prog_empty_city_name merged", s.bytes());
}

/// Names containing bytes that are not valid UTF-8 must survive unchanged.
#[test]
fn err_prog_non_utf8_names() {
    let names: Vec<Vec<u8>> = vec![
        vec![0xff],
        vec![0x80, 0x81, 0x82],
        vec![0xc3, 0x28],
        vec![0xe2, 0x82],
        vec![0xf0, 0x9f, 0x92, 0xa9],
        vec![0x01, 0x02, 0x7f],
        b"tab\there".to_vec(),
        b"a b c".to_vec(),
    ];
    let mut s = Script::new();
    for n in &names {
        s.add_city(n);
    }
    for n in &names {
        s.details(n);
    }
    s.add_route(&names[0], &names[1], 3);
    s.path(&names[0], &names[1]);
    s.copy(&names[0]);
    s.show_all();
    s.exit();
    assert_prog_same("err_prog_non_utf8_names", s.bytes());
    assert_prog_same_merged("err_prog_non_utf8_names merged", s.bytes());
}

/// A NUL byte inside a line: `fgets` keeps it, but every `str*` function stops
/// there, so the city name is the part before the NUL.
#[test]
fn err_prog_embedded_nul() {
    let mut input = Vec::new();
    input.extend_from_slice(b"1\nAB");
    input.push(0);
    input.extend_from_slice(b"CD\n3\n4\nAB\n");
    input.extend_from_slice(b"8\n");
    assert_prog_same("err_prog_embedded_nul", &input);

    // NUL in the menu line
    let mut input = Vec::new();
    input.extend_from_slice(b"3");
    input.push(0);
    input.extend_from_slice(b"8\n8\n");
    assert_prog_same("err_prog_embedded_nul menu", &input);
}

/// Carriage returns (as produced by CRLF input) are part of the city name.
#[test]
fn err_prog_crlf() {
    assert_prog_same("err_prog_crlf", b"1\r\nParis\r\n3\r\n8\r\n");
    assert_prog_same("err_prog_crlf mixed", b"1\nParis\r\n4\nParis\n4\nParis\r\n3\n8\n");
}

/// CONFIGS row 51 / ERRORS row 65: distances close to `INT_MAX` make the
/// Dijkstra relaxation `state[i].distance + edge.distance` overflow, which can
/// give a node itself as its own predecessor. The reconstruction loop then runs
/// past the end of `node_t *path[MAX_NODES]` and the process dies from a fatal
/// signal. Everything the C managed to flush before dying is deterministic and
/// is compared; the signal itself is not (see `CRASH_SIGNALS`).
#[test]
fn cfg_prog_overflow_stack_overrun() {
    let mut rng = Rng::new(0x0F1E2D3C);
    let mut crashed = 0usize;
    for round in 0..200 {
        let n = 2 + rng.below(5);
        let mut s = Script::new();
        for i in 0..n {
            s.add_city(&city(i));
        }
        for i in 0..n {
            for _ in 0..rng.below(4) {
                let j = rng.below(n);
                let d = [
                    2147483647i64,
                    2147483646,
                    2147483000,
                    1073741824,
                    2000000000,
                    5,
                    1,
                    0,
                ][rng.below(8)];
                s.add_route(&city(i), &city(j), d);
            }
        }
        for a in 0..n {
            for b in 0..n {
                s.path(&city(a), &city(b));
            }
        }
        s.exit();
        let input = s.bytes().to_vec();
        let c = run(&c_driver(), &input, false);
        if c.signal.is_some() {
            crashed += 1;
        }
        assert_prog_same(
            &format!("cfg_prog_overflow_stack_overrun round={round}"),
            &input,
        );
    }
    // the row is pointless unless the overrun is actually reached
    assert!(crashed > 0, "no round reached the stack overrun");
    println!("[cfg_prog_overflow_stack_overrun] {crashed}/200 scripts crashed the C");
}
