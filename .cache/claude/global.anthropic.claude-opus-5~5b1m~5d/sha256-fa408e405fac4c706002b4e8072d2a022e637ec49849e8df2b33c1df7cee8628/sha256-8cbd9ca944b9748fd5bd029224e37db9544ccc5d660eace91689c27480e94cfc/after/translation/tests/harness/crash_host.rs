//! Comparator for the ERRORS.md rows that terminate the process.
//!
//! Those rows (null `label`, cyclic tree, `INT_MIN / -1`, `INT_MIN % -1`) cannot
//! be asserted in-process. They are also not comparable from a *Rust* host,
//! because the Rust runtime installs a stack-overflow guard-page handler that
//! turns `SIGSEGV` into an abort — an artefact of the harness, not of the
//! library. So both libraries are driven from the **same tiny C host** which
//! `dlopen`s whichever `.so` it is given. Then the exit status / terminating
//! signal / stdout of the two children are compared directly.

#![allow(dead_code)]

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const HOST_C: &str = r#"
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*F4)(int, int, int, int);
typedef int (*ADD)(int, int, int, const char *);
typedef int (*SUM)(int);

int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: host <so> <case>\n"); return 98; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 99; }
    const char *c = argv[2];
    int *node_count = (int *)dlsym(h, "node_count");
    if (!node_count) { fprintf(stderr, "dlsym node_count: %s\n", dlerror()); return 97; }

    if (!strcmp(c, "ok")) {
        F4 f = (F4)dlsym(h, "divide_op");
        printf("%d %d %d\n", f(-2147483647 - 1, 1, 0, 0), f(7, -2, 0, 0), f(9, 0, 0, 0));
    } else if (!strcmp(c, "div_intmin")) {
        F4 f = (F4)dlsym(h, "divide_op");
        printf("no-trap %d\n", f(-2147483647 - 1, -1, 0, 0));
    } else if (!strcmp(c, "rem_intmin")) {
        F4 f = (F4)dlsym(h, "modulo_op");
        printf("no-trap %d\n", f(-2147483647 - 1, -1, 0, 0));
    } else if (!strcmp(c, "null_label")) {
        ADD a = (ADD)dlsym(h, "add_tree_node");
        *node_count = 0;
        printf("no-trap %d\n", a(1, 1, -1, NULL));
    } else if (!strcmp(c, "cycle")) {
        /* Two nodes both with id 1: the second one's parent lookup finds the
           first, so node_table[0].left_child_id becomes 1 -> a self cycle. */
        ADD a = (ADD)dlsym(h, "add_tree_node");
        SUM s = (SUM)dlsym(h, "calculate_tree_sum");
        *node_count = 0;
        a(1, 5, -1, "root");
        a(1, 7, 1, "dup");
        printf("no-trap %d\n", s(1));
    } else {
        fprintf(stderr, "unknown case %s\n", c);
        return 96;
    }
    fflush(stdout);
    return 0;
}
"#;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

static HOST: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Compile the C host once into the cargo target dir. `None` if no C compiler.
fn host() -> Option<&'static Path> {
    HOST.get_or_init(|| {
        let out_dir = crate_root().join("target/crash_host");
        std::fs::create_dir_all(&out_dir).ok()?;
        let src = out_dir.join("host.c");
        std::fs::write(&src, HOST_C).ok()?;
        let exe = out_dir.join("host");
        for cc in ["cc", "gcc", "clang"] {
            let st = Command::new(cc)
                .arg(&src)
                .arg("-o")
                .arg(&exe)
                .arg("-ldl")
                .status();
            if matches!(st, Ok(s) if s.success()) {
                return Some(exe);
            }
        }
        None
    })
    .as_deref()
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
}

fn run(host: &Path, so: &Path, case: &str) -> Outcome {
    let out = Command::new(host)
        .arg(so)
        .arg(case)
        .output()
        .unwrap_or_else(|e| panic!("cannot run C host {}: {e}", host.display()));
    assert_ne!(out.status.code(), Some(99), "C host could not dlopen {}", so.display());
    assert_ne!(out.status.code(), Some(98), "C host usage error");
    assert_ne!(out.status.code(), Some(97), "C host could not dlsym node_count");
    assert_ne!(out.status.code(), Some(96), "C host: unknown case `{case}`");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
    }
}

/// Assert the C `.so` and the Rust `.so`, driven from the identical C host,
/// terminate the same way (same exit code *or* the same signal) and print the
/// same stdout bytes.
#[track_caller]
pub fn assert_same_termination(case: &str) {
    let Some(h) = host() else {
        eprintln!("SKIP crash_host case `{case}`: no C compiler available");
        return;
    };
    let c_so = super::c_so_path();
    let r_so = super::rust_so_path();
    // touch the pair first so the staleness guard runs
    let _ = super::pair();

    let c = run(h, &c_so, case);
    let r = run(h, &r_so, case);

    assert_eq!(
        c.signal, r.signal,
        "case `{case}`: terminating signal differs — C={:?} Rust={:?}\n\
         C stdout   = {:?}\nRust stdout = {:?}",
        c.signal,
        r.signal,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    assert_eq!(c.code, r.code, "case `{case}`: exit code differs");
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "case `{case}`: stdout differs"
    );
    eprintln!("crash_host `{case}`: code={:?} signal={:?} (identical)", c.code, c.signal);
}
