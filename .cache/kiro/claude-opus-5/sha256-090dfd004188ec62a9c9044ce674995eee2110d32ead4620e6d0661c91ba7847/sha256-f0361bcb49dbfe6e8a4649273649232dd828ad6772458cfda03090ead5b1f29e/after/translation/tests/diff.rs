//! Differential test driver: C `libpng.so` vs Rust `liblibpng.so`.
//!
//! Both libraries are loaded with `libloading` and driven *only* through their
//! exported `png_*` symbols, so the `#[no_mangle] extern "C"` wrappers are part
//! of what is under test.
//!
//! Structure
//! ---------
//! The binary has two modes:
//!
//! * **worker** (`diff worker <so-path> <case> <seed>`) — loads exactly one
//!   `.so`, runs exactly one case, writes a transcript of everything it observed
//!   to stdout and exits. Because it is a fresh process, libpng's `png_error`
//!   path (which must not return to its caller) is handled by simply
//!   terminating with a fixed status after recording the message — no `setjmp`
//!   from Rust and no risk of a stuck test.
//! * **driver** (default) — for every case and seed, spawns the worker twice,
//!   once per `.so`, and compares stdout and exit status byte-for-byte.
//!
//! This makes "the C and Rust libraries behaved identically" the literal
//! assertion, covering return values, out-parameters, emitted PNG bytes,
//! decoded pixels, warning/error text, callback order and termination.

#[path = "diff/api.rs"]
mod api;
#[macro_use]
#[path = "diff/support.rs"]
mod support;
#[path = "diff/cases.rs"]
mod cases;
#[path = "diff/types.rs"]
mod types;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

/// The reference C `libpng.so` is linked by `c_src/CMakeLists.txt` against zlib
/// only, so its references to `floor` and `pow` are left undefined (upstream
/// libpng's own build adds `-lm`). `c_src/` is read-only for this task, so
/// instead we pull libm into *this* binary: a `dlopen`ed object resolves
/// undefined symbols from the global scope, which includes the executable and
/// everything it is linked against. Without this the C `.so` fails to load with
/// "undefined symbol: floor" and every floating-point code path would be
/// untestable.
#[link(name = "m")]
extern "C" {
    fn floor(x: f64) -> f64;
    fn pow(x: f64, y: f64) -> f64;
}

/// Force the linker to retain the `DT_NEEDED` entry for libm.
fn anchor_libm() {
    let x = std::env::args().count() as f64;
    unsafe {
        let v = floor(x + 0.5) + pow(x, 2.0);
        // Consumed via black_box so `--gc-sections`/`--as-needed` cannot drop it.
        std::hint::black_box(v);
    }
}

fn c_so() -> PathBuf {
    repo_root().join("c_src/build/libpng.so")
}

fn rust_so() -> PathBuf {
    // The cdylib lives next to this test binary's directory: target/<profile>/
    let exe = std::env::current_exe().expect("current_exe");
    // target/<profile>/deps/diff-<hash>  ->  target/<profile>/
    let mut d = exe.parent().unwrap().to_path_buf();
    if d.file_name().map(|f| f == "deps").unwrap_or(false) {
        d.pop();
    }
    let a = d.join("liblibpng.so");
    if a.exists() {
        return a;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/liblibpng.so")
}

struct Run {
    status: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_worker(exe: &Path, so: &Path, case: &str, seed: u64) -> Run {
    let out = Command::new(exe)
        .arg("worker")
        .arg(so)
        .arg(case)
        .arg(seed.to_string())
        .output()
        .expect("spawn worker");
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;
    Run {
        status: out.status.code(),
        signal,
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// First differing line, for a compact failure report.
fn first_diff(a: &[u8], b: &[u8]) -> String {
    let sa = show(a);
    let sb = show(b);
    let mut la = sa.lines();
    let mut lb = sb.lines();
    let mut n = 1;
    loop {
        match (la.next(), lb.next()) {
            (None, None) => return "(identical text, differing bytes)".into(),
            (x, y) if x == y => n += 1,
            (x, y) => {
                return format!(
                    "line {}:\n      C: {}\n   Rust: {}",
                    n,
                    x.unwrap_or("<eof>"),
                    y.unwrap_or("<eof>")
                )
            }
        }
    }
}

fn main() {
    anchor_libm();
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 5 && args[1] == "worker" {
        let so = &args[2];
        let (case, var) = match args[3].rsplit_once(':') {
            Some((c, v)) => (c.to_string(), v.parse::<u32>().unwrap_or(0)),
            None => (args[3].clone(), 0),
        };
        support::set_variant(var);
        let seed: u64 = args[4].parse().expect("seed");
        let api = match api::Api::load(so) {
            Ok(a) => a,
            Err(e) => {
                println!("LOAD-FAILED {}", e);
                support::finish(2);
            }
        };
        cases::run(&api, &case, seed);
        support::finish(0);
    }

    // ---- driver ----
    let exe = std::env::current_exe().expect("current_exe");
    let c = c_so();
    let r = rust_so();
    assert!(
        c.exists(),
        "C shared library not built: {} (run cmake in c_src/build)",
        c.display()
    );
    assert!(
        r.exists(),
        "Rust shared library not built: {} (run cargo build --release)",
        r.display()
    );

    let filter = std::env::var("DIFF_FILTER").unwrap_or_default();
    let registry = cases::registry();

    let mut failed: Vec<String> = Vec::new();
    let mut passed = 0usize;
    let mut checked_rows: BTreeMap<&'static str, usize> = BTreeMap::new();

    for spec in &registry {
        if !filter.is_empty() && !spec.name.contains(&filter) {
            continue;
        }
        let mut case_ok = true;
        'seeds: for i in 0..spec.seeds {
            let seed = 0x5eed_1b90u64
                .wrapping_mul(0x100_0000_01b3)
                .wrapping_add(i as u64)
                ^ (spec.name.bytes().fold(0u64, |a, b| {
                    a.rotate_left(5) ^ (b as u64)
                }));
            for v in 0..spec.variants.max(1) {
                let cname = format!("{}:{}", spec.name, v);
                let rc = run_worker(&exe, &c, &cname, seed);
                let rr = run_worker(&exe, &r, &cname, seed);

                let same = rc.status == rr.status
                    && rc.signal == rr.signal
                    && rc.stdout == rr.stdout;
                if !same {
                    case_ok = false;
                    failed.push(format!(
                        "FAIL {} seed={}\n   status C={:?}/{:?} Rust={:?}/{:?}\n   {}{}{}",
                        cname,
                        seed,
                        rc.status,
                        rc.signal,
                        rr.status,
                        rr.signal,
                        first_diff(&rc.stdout, &rr.stdout),
                        if rc.stderr.is_empty() {
                            String::new()
                        } else {
                            format!("\n   C stderr: {}", show(&rc.stderr).trim())
                        },
                        if rr.stderr.is_empty() {
                            String::new()
                        } else {
                            format!("\n   Rust stderr: {}", show(&rr.stderr).trim())
                        },
                    ));
                    break 'seeds; // one failing variant per case is enough to report
                }
            }
        }
        if case_ok {
            passed += 1;
            for row in spec.rows {
                *checked_rows.entry(row).or_insert(0) += 1;
            }
        }
        print!("{}", if case_ok { "." } else { "F" });
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    println!();

    println!("cases passed: {} / {}", passed, registry.len());
    let cfg: Vec<&&str> = checked_rows.keys().filter(|k| k.starts_with('B')).collect();
    let err: Vec<&&str> = checked_rows.keys().filter(|k| k.starts_with('C')).collect();
    println!(
        "CONFIGS.md rows covered: {}   ERRORS.md rows covered: {}",
        cfg.len(),
        err.len()
    );

    if !failed.is_empty() {
        eprintln!("\n{} case(s) diverged:\n", failed.len());
        for f in &failed {
            eprintln!("{}\n", f);
        }
        std::process::exit(1);
    }
}
