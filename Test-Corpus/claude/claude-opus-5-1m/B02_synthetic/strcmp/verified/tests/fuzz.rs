//! Broad randomized differential sweep (fixed seeds, so any failure is
//! reproducible).  This complements the per-row tests in `configs.rs` /
//! `errors.rs` by mixing *all* axes -- commands, aliases, token lengths around
//! the 32/40/41/63/64-byte overflow boundaries, wild bytes, separator runs,
//! oversized lines, embedded NULs and table-filling sequences -- in one stream.

mod common;

use common::*;

const CMDS: &[&str] = &[
    "adduser", "adduser", "login", "logout", "whoami", "listusers", "users", "createfile",
    "touch", "readfile", "cat", "writefile", "write", "deletefile", "rm", "listfiles", "ls",
    "set", "get", "unset", "listvars", "vars", "compare", "cmp", "compareN", "cmpn",
    "startswith", "match", "debug", "verbose", "status", "help", "?", "add", "log", "list",
    "create", "read", "write2", "delete", "bogus", "", "?", "listusers",
];

/// Token lengths that matter: 1, the 32-byte member size, the 40/41-byte
/// `user_count` boundary, the 36/37-byte `file_count` boundary and the 63/64
/// `strncpy` truncation.
const LENS: &[usize] = &[1, 2, 3, 5, 8, 16, 31, 32, 33, 35, 36, 37, 39, 40, 41, 42, 44, 62, 63, 64, 70];

fn gen(seed: u64, n_lines: usize) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    // a shared pool so that lookups hit as well as miss
    let pool: Vec<Vec<u8>> = (0..8)
        .map(|_| {
            let l = *rng.pick(LENS);
            if rng.chance(20) {
                rng.wild_token(l)
            } else {
                rng.token(l)
            }
        })
        .collect();
    let mut v: Vec<u8> = Vec::new();
    for _ in 0..n_lines {
        // occasionally emit a raw oddity instead of a command
        match rng.below(40) {
            0 => {
                v.extend_from_slice(b"\n");
                continue;
            }
            1 => {
                let n = rng.range(1, 8);
                v.extend_from_slice(&vec![b' '; n]);
                v.push(b'\n');
                continue;
            }
            2 => {
                v.extend_from_slice(b"\t\t \t\n");
                continue;
            }
            3 => {
                // oversized line -> fgets splits it
                let n = rng.range(200, 700);
                v.extend_from_slice(b"compare ");
                let t = rng.tok(1, 20);
                for _ in 0..n / 8 {
                    v.extend_from_slice(&t);
                    v.push(b' ');
                }
                v.push(b'\n');
                continue;
            }
            4 => {
                // embedded NUL
                let c = *rng.pick(CMDS);
                v.extend_from_slice(c.as_bytes());
                v.push(0);
                let t = rng.tok(1, 10);
                v.extend_from_slice(&t);
                v.push(b'\n');
                continue;
            }
            5 => {
                v.extend_from_slice(b"debug ");
                v.extend_from_slice(rng.pick(&["on", "off", "x"]).as_bytes());
                v.push(b'\n');
                v.extend_from_slice(b"verbose ");
                v.extend_from_slice(rng.pick(&["on", "off", "x"]).as_bytes());
                v.push(b'\n');
                continue;
            }
            6 => {
                // fill a table quickly
                let k = rng.range(1, 22);
                for i in 0..k {
                    match rng.below(3) {
                        0 => v.extend_from_slice(format!("adduser fu{i} fp{i} {i}\n").as_bytes()),
                        1 => v.extend_from_slice(format!("createfile ff{i} cc{i}\n").as_bytes()),
                        _ => v.extend_from_slice(format!("set fv{i} vv{i}\n").as_bytes()),
                    }
                }
                continue;
            }
            _ => {}
        }

        let cmd = *rng.pick(CMDS);
        let mut line: Vec<u8> = cmd.as_bytes().to_vec();
        let nargs = rng.range(0, 5);
        for _ in 0..nargs {
            // separator run
            if rng.chance(15) {
                let n = rng.range(1, 4);
                for _ in 0..n {
                    line.push(if rng.chance(50) { b' ' } else { b'\t' });
                }
            } else {
                line.push(b' ');
            }
            match rng.below(8) {
                0 | 1 | 2 => {
                    let k = rng.below(pool.len());
                    line.extend_from_slice(&pool[k]);
                }
                3 => {
                    let l = *rng.pick(LENS);
                    let t = rng.token(l);
                    line.extend_from_slice(&t);
                }
                4 => {
                    let l = *rng.pick(LENS);
                    let t = rng.wild_token(l);
                    line.extend_from_slice(&t);
                }
                5 => line.extend_from_slice(
                    rng.pick(&["on", "off", "0", "1", "4", "5", "8", "9", "-1", "63", "2147483648", "abc"])
                        .as_bytes(),
                ),
                6 => {
                    let k = rng.below(pool.len());
                    let mut t = pool[k].clone();
                    t.truncate(32.min(t.len()));
                    let k2 = rng.below(pool.len());
                    t.extend_from_slice(&pool[k2]);
                    t.truncate(63);
                    line.extend_from_slice(&t);
                }
                _ => {
                    let t = rng.tok(1, 6);
                    line.extend_from_slice(&t);
                }
            }
        }
        if rng.chance(6) {
            // trailing separators
            line.extend_from_slice(b"  \t");
        }
        line.push(b'\n');
        v.extend_from_slice(&line);
    }
    if rng.chance(25) {
        // drop the final newline
        if v.last() == Some(&b'\n') {
            v.pop();
        }
    }
    v
}

#[test]
fn fuzz_mixed_streams() {
    for seed in 0..120u64 {
        let input = gen(0xF0_0000 + seed, 45);
        diff_case(&format!("fuzz/seed{seed}"), &input);
    }
}

#[test]
fn fuzz_long_streams() {
    for seed in 0..30u64 {
        let input = gen(0xF1_0000 + seed, 200);
        diff_case(&format!("fuzz-long/seed{seed}"), &input);
    }
}

#[test]
fn fuzz_overflow_focused() {
    // every interesting length against a full user table, which is where the
    // reference program's overruns reach `user_count` / `current_user`
    let mut rng = Rng::new(0xBEEF_0001);
    for seed in 0..60u64 {
        let mut v = Vec::new();
        let pre = rng.below(11);
        for i in 0..pre {
            v.extend_from_slice(format!("adduser p{i} q{i} {i}\n").as_bytes());
        }
        for _ in 0..6 {
            let nl = *rng.pick(LENS);
            let pl = *rng.pick(LENS);
            let name = rng.token(nl);
            let pass = if rng.chance(30) { rng.wild_token(pl) } else { rng.token(pl) };
            v.extend_from_slice(b"adduser ");
            v.extend_from_slice(&name);
            v.push(b' ');
            v.extend_from_slice(&pass);
            v.extend_from_slice(format!(" {}\n", rng.below(30)).as_bytes());
            v.extend_from_slice(b"listusers\nstatus\n");
            v.extend_from_slice(b"login ");
            v.extend_from_slice(&name);
            v.push(b' ');
            v.extend_from_slice(&pass);
            v.push(b'\n');
            v.extend_from_slice(b"whoami\ncreatefile zz cc\nlistfiles\nlistvars\nlogout\n");
        }
        diff_case(&format!("fuzz-overflow/seed{seed}"), &v);
    }
}
