//! Out-of-process differential oracle.
//!
//! `betagamma` and `compute_hash` compare raw heap ADDRESSES
//! (`mb1->data < mb2->data`, `mb1 < mb2`), so their results depend on the
//! process's allocator history, not only on the arguments.  Loading both
//! `.so`s into one process is therefore not a fair comparison: whichever
//! library runs second pops the chunks the first one just freed off glibc's
//! LIFO tcache and sees the opposite pointer ordering.
//!
//! This module re-executes the *test binary itself* twice — once with only the
//! C `.so` loaded, once with only the Rust `.so` loaded — so each library sees
//! an identical, pristine heap.  The two result streams must be byte-identical.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::Command;

pub const ROLE_ENV: &str = "DIFF_ROLE";
pub const IN_ENV: &str = "DIFF_IN";
pub const OUT_ENV: &str = "DIFF_OUT";
pub const CHILD_TEST: &str = "zz_oracle_child";

/// One scripted operation for the child to perform.
#[derive(Clone, Debug)]
pub enum Op {
    /// `betagamma(p1, p2, p3, p4)`
    Betagamma(i32, i32, i32, i32),
    /// allocate two blocks of `count` with `init1`/`init2`, then report
    /// `compute_hash` in both argument orders, both sizes and both element sums.
    Pipeline { count: usize, init1: i32, init2: i32 },
    /// allocate `n` blocks, report the full n x n `compute_hash` matrix.
    HashMatrix { n: usize, count: usize, init: i32 },
    /// `allocate_block(count, init)` then report size / null-ness / elements.
    Allocate { count: usize, init: i32 },
    /// `allocate_block` with a `count` that must fail, then `free_block(NULL)`.
    AllocFail { count: usize, init: i32 },
}

fn encode(ops: &[Op]) -> String {
    let mut s = String::new();
    for op in ops {
        match *op {
            Op::Betagamma(a, b, c, d) => writeln!(s, "B {} {} {} {}", a, b, c, d).unwrap(),
            Op::Pipeline {
                count,
                init1,
                init2,
            } => writeln!(s, "P {} {} {}", count, init1, init2).unwrap(),
            Op::HashMatrix { n, count, init } => {
                writeln!(s, "M {} {} {}", n, count, init).unwrap()
            }
            Op::Allocate { count, init } => writeln!(s, "A {} {}", count, init).unwrap(),
            Op::AllocFail { count, init } => writeln!(s, "F {} {}", count, init).unwrap(),
        }
    }
    s
}

fn tmp_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn run_child(role: &str, script: &PathBuf, tag: &str) -> String {
    let exe = std::env::current_exe().expect("current_exe");
    let out = tmp_dir().join(format!("diff_{}_{}_{}.out", tag, role, std::process::id()));
    let _ = std::fs::remove_file(&out);
    let status = Command::new(&exe)
        .args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"])
        .env(ROLE_ENV, role)
        .env(IN_ENV, script)
        .env(OUT_ENV, &out)
        .output()
        .expect("spawn oracle child");
    assert!(
        status.status.success(),
        "oracle child ({}) failed: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        role,
        status.status,
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    let s = std::fs::read_to_string(&out)
        .unwrap_or_else(|e| panic!("oracle child ({}) wrote no output: {}", role, e));
    let _ = std::fs::remove_file(&out);
    s
}

/// Run `ops` in a fresh process against each library and assert the two output
/// streams are byte-identical.
pub fn assert_same(tag: &str, ops: &[Op]) {
    let script_path = tmp_dir().join(format!("diff_{}_{}.script", tag, std::process::id()));
    {
        let mut f = std::fs::File::create(&script_path).expect("create script");
        f.write_all(encode(ops).as_bytes()).expect("write script");
    }
    let c = run_child("c", &script_path, tag);
    let r = run_child("rust", &script_path, tag);
    let _ = std::fs::remove_file(&script_path);

    // Guard against a vacuous pass: both children must actually have executed
    // every op (empty == empty would otherwise compare "equal").
    assert_eq!(
        c.lines().count(),
        ops.len(),
        "[{}] C child produced {} result lines for {} ops",
        tag,
        c.lines().count(),
        ops.len()
    );
    assert_eq!(
        r.lines().count(),
        ops.len(),
        "[{}] Rust child produced {} result lines for {} ops",
        tag,
        r.lines().count(),
        ops.len()
    );

    if c == r {
        return;
    }
    // Report the first differing line together with the op that produced it.
    let cl: Vec<&str> = c.lines().collect();
    let rl: Vec<&str> = r.lines().collect();
    for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
        if a != b {
            panic!(
                "[{}] out-of-process divergence at line {}\n  op   = {:?}\n  C    = {}\n  Rust = {}",
                tag,
                i,
                ops.get(i),
                a,
                b
            );
        }
    }
    panic!(
        "[{}] output length differs: C has {} lines, Rust has {} lines",
        tag,
        cl.len(),
        rl.len()
    );
}

// ---------------------------------------------------------------------------
// Child side
// ---------------------------------------------------------------------------

/// If the oracle env vars are present, act as the child: load the single
/// requested library, execute the script, write the results, and return `true`.
pub fn maybe_run_as_child() -> bool {
    let role = match std::env::var(ROLE_ENV) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let inp = std::env::var(IN_ENV).expect("DIFF_IN");
    let outp = std::env::var(OUT_ENV).expect("DIFF_OUT");
    let script = std::fs::read_to_string(&inp).expect("read script");

    // BOTH libraries are dlopen'ed in BOTH children, in the same order, so the
    // process' heap history up to (and during) the scripted ops is identical;
    // only WHICH library we call differs.  Calling just one keeps that library
    // from inheriting the other's freed chunks.
    let pair = super::libs();
    let lib: &super::Lib = match role.as_str() {
        "c" => &pair.c,
        "rust" => &pair.rs,
        other => panic!("unknown role {:?}", other),
    };
    let mut out = String::new();

    for line in script.lines() {
        let mut it = line.split_whitespace();
        let kind = match it.next() {
            Some(k) => k,
            None => continue,
        };
        let next_i32 = |it: &mut std::str::SplitWhitespace| -> i32 {
            it.next().expect("arg").parse().expect("i32")
        };
        match kind {
            "B" => {
                let (a, b, c, d) = (
                    next_i32(&mut it),
                    next_i32(&mut it),
                    next_i32(&mut it),
                    next_i32(&mut it),
                );
                let v = unsafe { (lib.betagamma)(a, b, c, d) };
                writeln!(out, "B {}", v).unwrap();
            }
            "P" => {
                let count: usize = it.next().unwrap().parse().unwrap();
                let i1 = next_i32(&mut it);
                let i2 = next_i32(&mut it);
                unsafe {
                    let m1 = (lib.allocate_block)(count, i1);
                    let m2 = (lib.allocate_block)(count, i2);
                    if m1.is_null() || m2.is_null() {
                        writeln!(out, "P null {} {}", m1.is_null(), m2.is_null()).unwrap();
                        (lib.free_block)(m1);
                        (lib.free_block)(m2);
                        continue;
                    }
                    let h12 = (lib.compute_hash)(m1, m2);
                    let h21 = (lib.compute_hash)(m2, m1);
                    let h11 = (lib.compute_hash)(m1, m1);
                    let mut s1: i32 = 0;
                    let mut s2: i32 = 0;
                    for k in 0..(*m1).size {
                        s1 = s1.wrapping_add(*(*m1).data.add(k));
                    }
                    for k in 0..(*m2).size {
                        s2 = s2.wrapping_add(*(*m2).data.add(k));
                    }
                    writeln!(
                        out,
                        "P {} {} {} {} {} {} {}",
                        h12,
                        h21,
                        h11,
                        (*m1).size,
                        (*m2).size,
                        s1,
                        s2
                    )
                    .unwrap();
                    (lib.free_block)(m1);
                    (lib.free_block)(m2);
                }
            }
            "M" => {
                let n: usize = it.next().unwrap().parse().unwrap();
                let count: usize = it.next().unwrap().parse().unwrap();
                let init = next_i32(&mut it);
                unsafe {
                    let mut v = Vec::with_capacity(n);
                    for k in 0..n {
                        let p = (lib.allocate_block)(count, init.wrapping_add(k as i32));
                        assert!(!p.is_null());
                        v.push(p);
                    }
                    write!(out, "M").unwrap();
                    for &a in &v {
                        for &b in &v {
                            write!(out, " {}", (lib.compute_hash)(a, b)).unwrap();
                        }
                    }
                    writeln!(out).unwrap();
                    for k in (0..n).rev() {
                        (lib.free_block)(v[k]);
                    }
                }
            }
            "A" => {
                let count: usize = it.next().unwrap().parse().unwrap();
                let init = next_i32(&mut it);
                unsafe {
                    let m = (lib.allocate_block)(count, init);
                    if m.is_null() {
                        writeln!(out, "A NULL").unwrap();
                        continue;
                    }
                    write!(out, "A {} {}", (*m).size, (*m).data.is_null()).unwrap();
                    for k in 0..(*m).size {
                        write!(out, " {}", *(*m).data.add(k)).unwrap();
                    }
                    writeln!(out).unwrap();
                    (lib.free_block)(m);
                }
            }
            "F" => {
                let count: usize = it.next().unwrap().parse().unwrap();
                let init = next_i32(&mut it);
                unsafe {
                    let m = (lib.allocate_block)(count, init);
                    writeln!(out, "F {}", m.is_null()).unwrap();
                    (lib.free_block)(m); // also exercises free_block(NULL)
                    (lib.free_block)(std::ptr::null_mut());
                }
            }
            other => panic!("bad op {:?}", other),
        }
    }

    std::fs::write(&outp, out).expect("write oracle output");
    true
}
