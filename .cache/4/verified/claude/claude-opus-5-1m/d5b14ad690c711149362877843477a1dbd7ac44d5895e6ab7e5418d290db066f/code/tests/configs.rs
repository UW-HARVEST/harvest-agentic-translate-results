//! Phase B -- valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Every test drives BOTH binaries (C reference + Rust translation) through the
//! process boundary and asserts byte-identical stdout/stderr/exit status.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// helpers shared by the rows
// ---------------------------------------------------------------------------

fn bytes(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// `adduser u0 p0 <lvl>` … for `n` users (levels cycle 0..9).
fn add_users(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(format!("adduser u{i} p{i} {}\n", i % 10).as_bytes());
    }
    v
}

fn add_files(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(format!("createfile f{i} c{i}\n").as_bytes());
    }
    v
}

fn add_vars(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(format!("set v{i} val{i}\n").as_bytes());
    }
    v
}

const MODE_PRELUDE: [&str; 4] = [
    "debug off\nverbose off\n",
    "debug on\nverbose off\n",
    "debug off\nverbose on\n",
    "debug on\nverbose on\n",
];

// ---------------------------------------------------------------------------
// C01 .. C10
// ---------------------------------------------------------------------------

#[test]
fn c01_empty_stdin() {
    diff_case("C01/empty", b"");
    diff_case("C01/one-newline", b"\n");
    diff_case("C01/two-newlines", b"\n\n");
}

#[test]
fn c02_no_trailing_newline() {
    diff_case("C02/status-no-nl", b"status");
    diff_case("C02/partial-cmd", b"adduser a b\nwhoami");
    diff_case("C02/spaces-no-nl", b"   ");
    diff_case("C02/single-char", b"x");
}

#[test]
fn c03_blank_lines_all_modes() {
    for (i, p) in MODE_PRELUDE.iter().enumerate() {
        let input = bytes(&[
            p.as_bytes(),
            b"\n",
            b" \n",
            b"\t\n",
            b"   \t \t \n",
            b" \t\n",
            b"status\n",
        ]);
        diff_case(&format!("C03/mode{i}"), &input);
    }
}

#[test]
fn c04_help_aliases_all_modes() {
    for (i, p) in MODE_PRELUDE.iter().enumerate() {
        diff_case(
            &format!("C04/mode{i}"),
            &bytes(&[p.as_bytes(), b"help\n?\nhelp extra args\n? x\n"]),
        );
    }
}

#[test]
fn c05_status_population_matrix() {
    let pops: [(usize, usize, usize); 5] = [(0, 0, 0), (1, 1, 1), (5, 7, 9), (10, 20, 20), (10, 0, 20)];
    for (mi, p) in MODE_PRELUDE.iter().enumerate() {
        for (ui, (nu, nf, nv)) in pops.iter().enumerate() {
            let mut v = Vec::new();
            v.extend_from_slice(p.as_bytes());
            v.extend_from_slice(&add_users(*nu));
            v.extend_from_slice(b"status\n");
            if *nu > 0 {
                v.extend_from_slice(b"login u0 p0\n");
            }
            v.extend_from_slice(&add_files(*nf));
            v.extend_from_slice(&add_vars(*nv));
            v.extend_from_slice(b"status\nlistusers\nlistfiles\nlistvars\n");
            v.extend_from_slice(b"logout\nstatus\n");
            diff_case(&format!("C05/mode{mi}/pop{ui}"), &v);
        }
    }
}

#[test]
fn c06_debug_toggle() {
    diff_case(
        "C06",
        b"debug\ndebug on\ndebug\nstatus\ndebug on\nwhoami\ndebug off\ndebug\nwhoami\n\
          debug ON\ndebug 1\ndebug onx\ndebug off extra\ndebug on extra\n",
    );
    // the [DEBUG] line prints the arg count -- sweep it
    let mut v = Vec::from(&b"debug on\n"[..]);
    for n in 0..12 {
        let mut line = String::from("match");
        for k in 0..n {
            line.push_str(&format!(" t{k}"));
        }
        line.push('\n');
        v.extend_from_slice(line.as_bytes());
    }
    diff_case("C06/argcount", &v);
}

#[test]
fn c07_verbose_toggle() {
    diff_case(
        "C07",
        b"verbose\nverbose on\nverbose\n\n \nstatus\nverbose off\nverbose\n\nstatus\n\
          verbose ON\nverbose 1\nverbose offx\nverbose on extra\n",
    );
}

#[test]
fn c08_debug_and_verbose_random_stream() {
    let mut rng = Rng::new(0x0808_0808);
    for it in 0..8 {
        let mut v = Vec::from(&b"debug on\nverbose on\n"[..]);
        for _ in 0..40 {
            let n = rng.range(0, 4);
            let mut line: Vec<u8> = rng.pick(&["status", "whoami", "listusers", "ls", "vars", "bogus", "logs", ""]).as_bytes().to_vec();
            for _ in 0..n {
                line.push(b' ');
                let t = rng.tok(1, 6);
                line.extend_from_slice(&t);
            }
            line.push(b'\n');
            v.extend_from_slice(&line);
        }
        diff_case(&format!("C08/it{it}"), &v);
    }
}

#[test]
fn c09_adduser_random_tables() {
    let mut rng = Rng::new(0x0909_0909);
    for it in 0..16 {
        let n = rng.range(1, 12);
        let mut v = Vec::new();
        for _ in 0..n {
            let name = rng.tok(1, 12);
            let pass = rng.tok(1, 12);
            let mut line = b"adduser ".to_vec();
            line.extend_from_slice(&name);
            line.push(b' ');
            line.extend_from_slice(&pass);
            if rng.chance(60) {
                line.extend_from_slice(format!(" {}", rng.range(0, 12)).as_bytes());
            }
            line.push(b'\n');
            v.extend_from_slice(&line);
        }
        v.extend_from_slice(b"listusers\nstatus\n");
        diff_case(&format!("C09/it{it}"), &v);
    }
}

#[test]
fn c10_permission_level_values() {
    let levels = [
        "-5", "-1", "0", "1", "4", "5", "8", "9", "10", "2147483647", "2147483648",
        "-2147483648", "-2147483649", "99999999999999999999", "abc", "+7", "-0", "7x", "x7",
        "0009", "1e3", "", " ", "+", "-",
    ];
    for (i, l) in levels.iter().enumerate() {
        let mut v = Vec::new();
        v.extend_from_slice(format!("adduser a{i} p {l}\n").as_bytes());
        v.extend_from_slice(b"listusers\n");
        v.extend_from_slice(format!("login a{i} p\n").as_bytes());
        v.extend_from_slice(b"whoami\ncreatefile ff cc\nlistfiles\nwritefile ff zz\ndeletefile ff\nlogout\n");
        diff_case(&format!("C10/level{i}"), &v);
    }
}

// ---------------------------------------------------------------------------
// C11 .. C20
// ---------------------------------------------------------------------------

#[test]
fn c11_max_users_both_list_aliases() {
    let mut v = add_users(10);
    v.extend_from_slice(b"listusers\nusers\nlogin u3 p3\nlistusers\nusers\n");
    v.extend_from_slice(b"adduser u10 p10 1\nstatus\nlogout\nlogin u9 p9\nusers\n");
    diff_case("C11", &v);
}

#[test]
fn c12_login_logout_random_sequences() {
    let mut rng = Rng::new(0x1212_1212);
    for it in 0..16 {
        let nu = rng.range(1, 6);
        let mut v = add_users(nu);
        for _ in 0..25 {
            match rng.below(6) {
                0 => {
                    let i = rng.below(nu + 2);
                    v.extend_from_slice(format!("login u{i} p{i}\n").as_bytes());
                }
                1 => {
                    let i = rng.below(nu + 2);
                    v.extend_from_slice(format!("login u{i} wrong\n").as_bytes());
                }
                2 => v.extend_from_slice(b"logout\n"),
                3 => v.extend_from_slice(b"whoami\n"),
                4 => v.extend_from_slice(b"listusers\n"),
                _ => v.extend_from_slice(b"status\n"),
            }
        }
        diff_case(&format!("C12/it{it}"), &v);
    }
}

#[test]
fn c13_whoami_each_state() {
    diff_case(
        "C13",
        b"whoami\nadduser a b 3\nwhoami\nlogin a b\nwhoami\nlogout\nwhoami\nlogin a b\nwhoami\n\
          login a b\nwhoami\nwhoami extra args\n",
    );
}

#[test]
fn c14_listusers_flags() {
    diff_case("C14/empty", b"listusers\nusers\n");
    diff_case("C14/one", b"adduser only pw\nlistusers\nlogin only pw\nlistusers\nlogout\nlistusers\n");
    let mut v = add_users(6);
    v.extend_from_slice(b"listusers\nlogin u2 p2\nlistusers\nlogout\nlogin u5 p5\nlistusers\n");
    diff_case("C14/many", &v);
}

#[test]
fn c15_createfile_aliases_random() {
    let mut rng = Rng::new(0x1515_1515);
    for it in 0..12 {
        let mut v = Vec::from(&b"adduser own pw 3\nlogin own pw\n"[..]);
        let n = rng.range(1, 22);
        for i in 0..n {
            let cmd = if rng.chance(50) { "createfile" } else { "touch" };
            let name = rng.tok(1, 10);
            let mut line = format!("{cmd} ").into_bytes();
            line.extend_from_slice(&name);
            if rng.chance(70) {
                line.push(b' ');
                let c = rng.tok(1, 20);
                line.extend_from_slice(&c);
            }
            line.push(b'\n');
            v.extend_from_slice(&line);
            if i % 5 == 0 {
                v.extend_from_slice(b"listfiles\n");
            }
        }
        v.extend_from_slice(b"listfiles\nls\nstatus\n");
        diff_case(&format!("C15/it{it}"), &v);
    }
}

#[test]
fn c16_readfile_aliases_states() {
    diff_case(
        "C16",
        b"cat nothing\nadduser o p\nlogin o p\ncreatefile f1 hello\nreadfile f1\ncat f1\n\
          writefile f1 world\nreadfile f1\ncat f1\ncreatefile f2\nreadfile f2\ncat f2\n\
          logout\nreadfile f1\ncat f2\nreadfile missing\ncat\nreadfile\n\
          login o p\ndeletefile f1\nreadfile f1\ncat f1\n",
    );
}

#[test]
fn c17_writefile_permission_matrix() {
    for lvl in ["-1", "0", "1", "4", "5", "6", "8", "9", "10", "100"] {
        let mut v = Vec::new();
        v.extend_from_slice(b"adduser owner pw 1\n");
        v.extend_from_slice(format!("adduser other pw2 {lvl}\n").as_bytes());
        v.extend_from_slice(b"login owner pw\ncreatefile shared original\nlogout\n");
        v.extend_from_slice(b"login other pw2\nwritefile shared changed\nreadfile shared\n");
        v.extend_from_slice(b"write shared again\ncat shared\nwritefile missing x\nwritefile shared\n");
        v.extend_from_slice(b"createfile mine ofother\nwritefile mine edit\nreadfile mine\nlogout\n");
        diff_case(&format!("C17/lvl{lvl}"), &v);
    }
}

#[test]
fn c18_deletefile_permission_and_shifting() {
    for lvl in ["0", "1", "5", "8", "9", "10"] {
        let mut v = Vec::new();
        v.extend_from_slice(b"adduser owner pw 1\n");
        v.extend_from_slice(format!("adduser other pw2 {lvl}\n").as_bytes());
        v.extend_from_slice(b"login owner pw\n");
        v.extend_from_slice(&add_files(20));
        v.extend_from_slice(b"listfiles\nlogout\nlogin other pw2\n");
        v.extend_from_slice(b"deletefile f0\nlistfiles\nrm f10\nls\nrm f19\nls\nrm missing\nrm\n");
        v.extend_from_slice(b"logout\nlogin owner pw\ndeletefile f5\nrm f1\nls\n");
        v.extend_from_slice(b"createfile f0 recreated\nls\ncat f0\n");
        diff_case(&format!("C18/lvl{lvl}"), &v);
    }
}

#[test]
fn c19_listfiles_populations() {
    diff_case("C19/empty", b"listfiles\nls\n");
    let mut v = Vec::from(&b"adduser o p 9\nlogin o p\n"[..]);
    v.extend_from_slice(b"listfiles\nls\ncreatefile one\nls\nlistfiles\n");
    v.extend_from_slice(&add_files(20));
    v.extend_from_slice(b"ls\nlistfiles\ncreatefile toomany\nrm f0\nrm f1\nls\ncreatefile again x\nls\n");
    diff_case("C19/full", &v);
}

#[test]
fn c20_variables_full_lifecycle() {
    let mut v = Vec::new();
    v.extend_from_slice(b"listvars\nvars\nget nope\nunset nope\nset\nset k\n");
    v.extend_from_slice(&add_vars(20));
    v.extend_from_slice(b"listvars\nvars\nset v0 updated\nget v0\nset v20 overflow\n");
    v.extend_from_slice(b"unset v0\nlistvars\nunset v10\nvars\nunset v19\nlistvars\n");
    v.extend_from_slice(b"set v20 nowfits\nlistvars\nget v20\nunset v20\nget v20\nstatus\n");
    diff_case("C20", &v);

    let mut rng = Rng::new(0x2020_2020);
    for it in 0..10 {
        let mut v = Vec::new();
        for _ in 0..30 {
            match rng.below(5) {
                0 => {
                    let n = rng.tok(1, 8);
                    let val = rng.tok(1, 12);
                    let mut l = b"set ".to_vec();
                    l.extend_from_slice(&n);
                    l.push(b' ');
                    l.extend_from_slice(&val);
                    l.push(b'\n');
                    v.extend_from_slice(&l);
                }
                1 => {
                    let n = rng.tok(1, 8);
                    let mut l = b"get ".to_vec();
                    l.extend_from_slice(&n);
                    l.push(b'\n');
                    v.extend_from_slice(&l);
                }
                2 => {
                    let n = rng.tok(1, 8);
                    let mut l = b"unset ".to_vec();
                    l.extend_from_slice(&n);
                    l.push(b'\n');
                    v.extend_from_slice(&l);
                }
                3 => v.extend_from_slice(b"listvars\n"),
                _ => v.extend_from_slice(b"vars\n"),
            }
        }
        diff_case(&format!("C20/rand{it}"), &v);
    }
}

// ---------------------------------------------------------------------------
// C21 .. C30
// ---------------------------------------------------------------------------

#[test]
fn c21_compare_random_pairs() {
    let mut rng = Rng::new(0x2121_2121);
    for it in 0..24 {
        let mut v = Vec::new();
        for _ in 0..40 {
            let cmd: &[u8] = if rng.chance(50) { b"compare " } else { b"cmp " };
            let a = if rng.chance(30) {
                rng.wild_tok(1, 64)
            } else {
                rng.tok(1, 20)
            };
            // frequently derive b from a so that equal/prefix/one-byte-off
            // cases are hit often
            let b = match rng.below(5) {
                0 => a.clone(),
                1 => {
                    let mut t = a.clone();
                    t.truncate(a.len().saturating_sub(1).max(1));
                    t
                }
                2 => {
                    let mut t = a.clone();
                    let i = rng.below(t.len());
                    t[i] = t[i].wrapping_add(1);
                    if t[i] == 0 || t[i] == b' ' || t[i] == b'\t' || t[i] == b'\n' {
                        t[i] = b'Q';
                    }
                    t
                }
                3 => {
                    let mut t = a.clone();
                    let e = rng.tok(1, 5);
                    t.extend_from_slice(&e);
                    t
                }
                _ => {
                    if rng.chance(30) {
                        rng.wild_tok(1, 64)
                    } else {
                        rng.tok(1, 20)
                    }
                }
            };
            let mut line = cmd.to_vec();
            line.extend_from_slice(&a);
            line.push(b' ');
            line.extend_from_slice(&b);
            line.push(b'\n');
            v.extend_from_slice(&line);
        }
        diff_case(&format!("C21/it{it}"), &v);
    }
}

#[test]
fn c22_comparen_random_pairs_and_counts() {
    let counts = [
        "0", "1", "2", "3", "5", "62", "63", "64", "100", "-1", "-5", "-63",
        "2147483647", "2147483648", "-2147483648", "99999999999999999999", "abc", "+4", "",
    ];
    let mut rng = Rng::new(0x2222_2222);
    for it in 0..20 {
        let mut v = Vec::new();
        for _ in 0..40 {
            let cmd: &[u8] = if rng.chance(50) { b"compareN " } else { b"cmpn " };
            let a = if rng.chance(25) {
                rng.wild_tok(1, 64)
            } else {
                rng.tok(1, 12)
            };
            let b = match rng.below(4) {
                0 => a.clone(),
                1 => {
                    let mut t = a.clone();
                    t.truncate(a.len().saturating_sub(1).max(1));
                    t
                }
                2 => {
                    let mut t = a.clone();
                    let i = rng.below(t.len());
                    t[i] = if t[i] == b'z' { b'a' } else { t[i] + 1 };
                    t
                }
                _ => rng.tok(1, 12),
            };
            let n = *rng.pick(&counts);
            let mut line = cmd.to_vec();
            line.extend_from_slice(&a);
            line.push(b' ');
            line.extend_from_slice(&b);
            if !n.is_empty() {
                line.push(b' ');
                line.extend_from_slice(n.as_bytes());
            }
            line.push(b'\n');
            v.extend_from_slice(&line);
        }
        diff_case(&format!("C22/it{it}"), &v);
    }
    // exhaustive small matrix
    let mut v = Vec::new();
    for a in ["", "a", "ab", "abc", "abd", "b", "A", "abcd"] {
        for b in ["", "a", "ab", "abc", "abd", "b", "A", "abcd"] {
            for n in ["0", "1", "2", "3", "4", "-1"] {
                if a.is_empty() || b.is_empty() {
                    continue;
                }
                v.extend_from_slice(format!("compareN {a} {b} {n}\n").as_bytes());
            }
        }
    }
    diff_case("C22/matrix", &v);
}

#[test]
fn c23_startswith_matrix() {
    let mut v = Vec::new();
    for a in ["a", "ab", "abc", "abcdef", "b", "A", "aB"] {
        for b in ["a", "ab", "abc", "abcdef", "b", "A", "aB"] {
            v.extend_from_slice(format!("startswith {a} {b}\n").as_bytes());
        }
    }
    v.extend_from_slice(b"startswith\nstartswith one\nstartswith one two three\n");
    diff_case("C23/matrix", &v);

    let mut rng = Rng::new(0x2323_2323);
    for it in 0..10 {
        let mut v = Vec::new();
        for _ in 0..40 {
            let a = if rng.chance(30) { rng.wild_tok(1, 64) } else { rng.tok(1, 16) };
            let b = if rng.chance(50) {
                let k = rng.range(1, a.len() + 2);
                a.iter().cloned().take(k).collect::<Vec<u8>>()
            } else if rng.chance(30) {
                rng.wild_tok(1, 64)
            } else {
                rng.tok(1, 16)
            };
            if b.is_empty() {
                continue;
            }
            let mut line = b"startswith ".to_vec();
            line.extend_from_slice(&a);
            line.push(b' ');
            line.extend_from_slice(&b);
            line.push(b'\n');
            v.extend_from_slice(&line);
        }
        diff_case(&format!("C23/rand{it}"), &v);
    }
}

#[test]
fn c24_match_random() {
    let mut rng = Rng::new(0x2424_2424);
    for it in 0..14 {
        let mut v = Vec::new();
        for _ in 0..25 {
            let pat = rng.tok(1, 6);
            let n = rng.range(0, 11);
            let mut line = b"match ".to_vec();
            line.extend_from_slice(&pat);
            for _ in 0..n {
                line.push(b' ');
                match rng.below(4) {
                    0 => line.extend_from_slice(&pat), // exact
                    1 => {
                        // contains
                        let pre = rng.tok(0, 4);
                        let post = rng.tok(0, 4);
                        line.extend_from_slice(&pre);
                        line.extend_from_slice(&pat);
                        line.extend_from_slice(&post);
                    }
                    2 => {
                        let t = rng.tok(1, 8);
                        line.extend_from_slice(&t);
                    }
                    _ => {
                        let t = rng.wild_tok(1, 10);
                        line.extend_from_slice(&t);
                    }
                }
            }
            line.push(b'\n');
            v.extend_from_slice(&line);
        }
        diff_case(&format!("C24/it{it}"), &v);
    }
}

const ALL_CMDS: &[&str] = &[
    "adduser", "login", "logout", "whoami", "listusers", "users", "createfile", "touch",
    "readfile", "cat", "writefile", "write", "deletefile", "rm", "listfiles", "ls", "set",
    "get", "unset", "listvars", "vars", "compare", "cmp", "compareN", "cmpn", "startswith",
    "match", "debug", "verbose", "status", "help", "?",
];

#[test]
fn c25_token_count_sweep_every_command() {
    for cmd in ALL_CMDS {
        let mut v = Vec::from(&b"adduser own pw 7\nlogin own pw\ncreatefile a b\nset k v\n"[..]);
        for n in 0..12usize {
            let mut line = String::from(*cmd);
            for k in 0..n {
                line.push_str(&format!(" a{k}"));
            }
            line.push('\n');
            v.extend_from_slice(line.as_bytes());
        }
        v.extend_from_slice(b"status\nlistusers\nlistfiles\nlistvars\n");
        diff_case(&format!("C25/{cmd}"), &v);
    }
}

#[test]
fn c26_token_length_sweep() {
    let lens = [1usize, 31, 32, 33, 62, 63, 64, 80];
    for cmd in ALL_CMDS {
        for &l in &lens {
            let t = vec![b'T'; l];
            let u = vec![b'U'; l];
            let w = vec![b'W'; l];
            let mut v = Vec::new();
            // one argument long, then all three long
            v.extend_from_slice(b"adduser own pw 7\nlogin own pw\n");
            v.extend_from_slice(&bytes(&[cmd.as_bytes(), b" ", &t, b" short 3\n"]));
            v.extend_from_slice(&bytes(&[cmd.as_bytes(), b" short ", &u, b" 3\n"]));
            v.extend_from_slice(&bytes(&[cmd.as_bytes(), b" ", &t, b" ", &u, b" ", &w, b"\n"]));
            v.extend_from_slice(b"status\nlistusers\nlistfiles\nlistvars\nwhoami\n");
            diff_case(&format!("C26/{cmd}/len{l}"), &v);
        }
    }
}

#[test]
fn c27_line_length_sweep() {
    for &l in &[1usize, 100, 253, 254, 255, 256, 257, 300, 600, 1024] {
        // a single oversized token
        let mut v = Vec::new();
        v.extend_from_slice(b"compare ");
        v.extend_from_slice(&vec![b'A'; l]);
        v.extend_from_slice(b" B\nstatus\n");
        diff_case(&format!("C27/onetoken{l}"), &v);

        // many small tokens filling the line
        let mut line = Vec::from(&b"match p"[..]);
        while line.len() < l {
            line.extend_from_slice(b" tok");
        }
        line.truncate(l);
        line.push(b'\n');
        line.extend_from_slice(b"status\n");
        diff_case(&format!("C27/manytokens{l}"), &line);

        // a long line of pure separators
        let mut sep = vec![b' '; l];
        sep.push(b'\n');
        sep.extend_from_slice(b"status\n");
        diff_case(&format!("C27/seps{l}"), &sep);
    }
}

#[test]
fn c28_arbitrary_byte_tokens() {
    let mut rng = Rng::new(0x2828_2828);
    for it in 0..16 {
        let mut v = Vec::new();
        for _ in 0..25 {
            let a = rng.wild_tok(1, 40);
            let b = rng.wild_tok(1, 40);
            let cmd = *rng.pick(&["compare", "startswith", "set", "adduser", "match", "get", "unset"]);
            let mut line = cmd.as_bytes().to_vec();
            line.push(b' ');
            line.extend_from_slice(&a);
            line.push(b' ');
            line.extend_from_slice(&b);
            line.push(b'\n');
            v.extend_from_slice(&line);
        }
        v.extend_from_slice(b"listusers\nlistvars\nstatus\n");
        diff_case(&format!("C28/it{it}"), &v);
    }
    // every byte value as a whole token
    let mut v = Vec::new();
    for b in 1u16..256 {
        let b = b as u8;
        if b == b' ' || b == b'\t' || b == b'\n' {
            continue;
        }
        v.extend_from_slice(&bytes(&[b"compare ", &[b], b" ", &[b'M'], b"\n"]));
        v.extend_from_slice(&bytes(&[b"set k", &[b], b" v\n"]));
    }
    v.extend_from_slice(b"listvars\n");
    diff_case("C28/allbytes", &v);
}

#[test]
fn c29_embedded_nul_and_cr() {
    diff_case("C29/nul-mid", b"listvars\x00 ignored tail\nstatus\n");
    diff_case("C29/nul-first", b"\x00status\nstatus\n");
    diff_case("C29/nul-only", b"\x00\nstatus\n");
    diff_case("C29/nul-in-token", b"set k\x00ey value\nlistvars\n");
    diff_case("C29/crlf", b"status\r\nlistvars\r\nadduser a b 2\r\nlistusers\r\n");
    diff_case("C29/cr-only", b"status\rlistvars\r");
    diff_case("C29/nul-after-cmd", b"debug on\x00\nstatus\n");
    diff_case("C29/mixed", b"set a b\x00c\nget a\nlistvars\n\x00\n\n");
}

#[test]
fn c30_max_args_cutoff() {
    for n in [9usize, 10, 11, 12, 15] {
        for cmd in ["match", "adduser", "compare", "compareN", "set", "createfile", "bogus"] {
            let mut line = String::from(cmd);
            for k in 0..n {
                line.push_str(&format!(" t{k}"));
            }
            line.push('\n');
            let mut v = Vec::from(&b"debug on\n"[..]);
            v.extend_from_slice(line.as_bytes());
            v.extend_from_slice(b"listusers\nlistvars\nstatus\n");
            diff_case(&format!("C30/{cmd}/{n}"), &v);
        }
    }
}

// ---------------------------------------------------------------------------
// C31 .. C40  (the `strcpy` overflow shapes -- the C's dominant observable
// behaviour; every one of these was verified against the reference binary)
// ---------------------------------------------------------------------------

/// `adduser <name> <pass> [lvl]` with raw byte tokens.
fn adduser_line(name: &[u8], pass: &[u8], lvl: Option<&str>) -> Vec<u8> {
    let mut v = b"adduser ".to_vec();
    v.extend_from_slice(name);
    v.push(b' ');
    v.extend_from_slice(pass);
    if let Some(l) = lvl {
        v.push(b' ');
        v.extend_from_slice(l.as_bytes());
    }
    v.push(b'\n');
    v
}

fn login_line(name: &[u8], pass: &[u8]) -> Vec<u8> {
    let mut v = b"login ".to_vec();
    v.extend_from_slice(name);
    v.push(b' ');
    v.extend_from_slice(pass);
    v.push(b'\n');
    v
}

#[test]
fn c31_name_overflow_into_password() {
    for nlen in 30usize..=63 {
        for slot in [0usize, 1, 4, 8] {
            let mut v = add_users(slot);
            let name = vec![b'N'; nlen];
            v.extend_from_slice(&adduser_line(&name, b"pw12345", Some("6")));
            v.extend_from_slice(b"listusers\nstatus\n");
            // the artifact name is name[0..32] + password when nlen >= 32
            let mut artifact = name.clone();
            artifact.truncate(32);
            artifact.extend_from_slice(b"pw12345");
            v.extend_from_slice(&login_line(&artifact, b"pw12345"));
            v.extend_from_slice(b"whoami\nstatus\n");
            v.extend_from_slice(&login_line(&name, b"pw12345"));
            v.extend_from_slice(b"whoami\nlistusers\ncreatefile ff cc\nlistfiles\nlogout\n");
            diff_case(&format!("C31/nlen{nlen}/slot{slot}"), &v);
        }
    }
}

#[test]
fn c32_password_overflow_into_next_user() {
    for plen in 30usize..=63 {
        for slot in [0usize, 3, 8] {
            let mut v = add_users(slot);
            let pass = vec![b'P'; plen];
            v.extend_from_slice(&adduser_line(b"first", &pass, Some("4")));
            v.extend_from_slice(b"listusers\nstatus\n");
            // the next slot's name field now holds the tail of the password
            v.extend_from_slice(&adduser_line(b"second", b"sp", Some("2")));
            v.extend_from_slice(b"listusers\nstatus\n");
            v.extend_from_slice(&login_line(b"first", &pass));
            v.extend_from_slice(b"whoami\nlogout\nlogin second sp\nwhoami\nlogout\nlistusers\n");
            diff_case(&format!("C32/plen{plen}/slot{slot}"), &v);
        }
    }
}

#[test]
fn c33_last_slot_overflow_into_user_count() {
    // password exactly 40 bytes: the terminating NUL clears user_count's low
    // byte, so `users[user_count]` is re-evaluated as users[0] afterwards.
    let mut v = add_users(9);
    let pass = vec![b'p'; 40];
    v.extend_from_slice(&adduser_line(b"last", &pass, Some("3")));
    v.extend_from_slice(b"status\nlistusers\nusers\n");
    v.extend_from_slice(b"adduser again pw2 5\nstatus\nlistusers\n");
    v.extend_from_slice(b"login u0 p0\nwhoami\nlogout\n");
    v.extend_from_slice(&login_line(b"last", &pass));
    v.extend_from_slice(b"whoami\nstatus\n");
    diff_case("C33/len40", &v);

    // password 41 bytes whose last byte lands in user_count: in-range values
    for b in [1u8, 2, 5, 9, 10, 20, 100, 200, 223, 224] {
        let mut v = add_users(9);
        let mut pass = vec![b'p'; 40];
        pass.push(b);
        v.extend_from_slice(&adduser_line(b"last", &pass, Some("2")));
        v.extend_from_slice(b"status\n");
        v.extend_from_slice(b"listusers\n");
        v.extend_from_slice(b"adduser more pw 1\nstatus\nlogin u1 p1\nwhoami\nlogout\n");
        diff_case(&format!("C33/len41/byte{b}"), &v);
    }
}

#[test]
fn c34_owner_overflow_into_permissions_and_next_file() {
    // name is exactly 32 bytes, so the user's *name string* extends into the
    // password field: its length is 32 + password length.
    for plen in [1usize, 2, 3, 4, 5, 8, 16, 31] {
        let name = vec![b'N'; 32];
        let pass = vec![b'q'; plen];
        let mut artifact = name.clone();
        artifact.extend_from_slice(&pass);
        let mut v = adduser_line(&name, &pass, Some("9"));
        v.extend_from_slice(&login_line(&artifact, &pass));
        v.extend_from_slice(b"whoami\n");
        for i in 0..6 {
            v.extend_from_slice(format!("createfile g{i} content{i}\n").as_bytes());
            v.extend_from_slice(b"listfiles\n");
            v.extend_from_slice(format!("readfile g{i}\n").as_bytes());
        }
        v.extend_from_slice(b"status\nwritefile g0 newcontent\nreadfile g0\ndeletefile g1\nlistfiles\n");
        diff_case(&format!("C34/plen{plen}"), &v);
    }
}

#[test]
fn c35_last_file_slot_overflow_into_file_count() {
    // owner string length 36 -> the NUL clears file_count's low byte
    for plen in [4usize, 5] {
        let name = vec![b'N'; 32];
        let pass = vec![b'q'; plen];
        let mut artifact = name.clone();
        artifact.extend_from_slice(&pass);
        let mut v = adduser_line(&name, &pass, Some("9"));
        v.extend_from_slice(&login_line(&artifact, &pass));
        v.extend_from_slice(&add_files(20));
        v.extend_from_slice(b"status\nlistfiles\nreadfile f0\ncreatefile zz tail\nstatus\nlistfiles\n");
        diff_case(&format!("C35/plen{plen}"), &v);
    }
    // owner string length 37 with a small control byte at index 36: file_count
    // becomes that byte, which is still inside the mapping
    for b in [1u8, 2, 8, 20, 24] {
        let name = vec![b'N'; 32];
        let pass = vec![b'q', b'q', b'q', b'q', b];
        let mut artifact = name.clone();
        artifact.extend_from_slice(&pass);
        let mut v = adduser_line(&name, &pass, Some("9"));
        v.extend_from_slice(&login_line(&artifact, &pass));
        v.extend_from_slice(&add_files(20));
        v.extend_from_slice(b"status\n");
        diff_case(&format!("C35/byte{b}"), &v);
    }
}

#[test]
fn c36_exact_field_sizes() {
    let mut v = Vec::from(&b"adduser own pw 9\nlogin own pw\n"[..]);
    for l in [1usize, 31, 32, 33, 62, 63] {
        let fname = vec![b'F'; l];
        let content = vec![b'C'; l];
        let mut line = b"createfile ".to_vec();
        line.extend_from_slice(&fname);
        line.push(b' ');
        line.extend_from_slice(&content);
        line.push(b'\n');
        v.extend_from_slice(&line);
        let mut rl = b"readfile ".to_vec();
        rl.extend_from_slice(&fname);
        rl.push(b'\n');
        v.extend_from_slice(&rl);
        v.extend_from_slice(b"listfiles\n");
    }
    v.extend_from_slice(b"status\n");
    diff_case("C36", &v);
}

#[test]
fn c37_variable_name_overflow_into_value() {
    for nlen in 30usize..=63 {
        let name = vec![b'V'; nlen];
        let mut v = Vec::new();
        v.extend_from_slice(b"set pre preval\n");
        let mut line = b"set ".to_vec();
        line.extend_from_slice(&name);
        line.extend_from_slice(b" thevalue\n");
        v.extend_from_slice(&line);
        v.extend_from_slice(b"listvars\nvars\n");
        let mut artifact = name.clone();
        artifact.truncate(32);
        artifact.extend_from_slice(b"thevalue");
        let mut g = b"get ".to_vec();
        g.extend_from_slice(&artifact);
        g.push(b'\n');
        v.extend_from_slice(&g);
        let mut g2 = b"get ".to_vec();
        g2.extend_from_slice(&name);
        g2.push(b'\n');
        v.extend_from_slice(&g2);
        v.extend_from_slice(b"set post postval\nlistvars\nunset pre\nlistvars\nstatus\n");
        diff_case(&format!("C37/nlen{nlen}"), &v);
    }
}

#[test]
fn c38_exit_and_quit() {
    diff_case("C38/exit-first", b"exit\nstatus\nhelp\n");
    diff_case("C38/quit-first", b"quit\nstatus\n");
    diff_case("C38/exit-after-work", b"adduser a b\nlogin a b\ncreatefile f c\nexit\nls\n");
    diff_case("C38/exit-args", b"exit now please\nstatus\n");
    diff_case("C38/quit-args", b"quit 1 2 3\nstatus\n");
    diff_case("C38/exit-no-newline", b"adduser a b\nexit");
    // exit after >4096 bytes of output: the flush must happen
    let mut v = Vec::new();
    for _ in 0..5 {
        v.extend_from_slice(b"help\n");
    }
    v.extend_from_slice(b"exit\n");
    diff_case("C38/exit-after-flush", &v);
}

#[test]
fn c39_output_crossing_stdio_blocks() {
    for n in [1usize, 2, 3, 4, 5, 8, 13, 20] {
        let mut v = Vec::new();
        for _ in 0..n {
            v.extend_from_slice(b"help\n");
        }
        v.extend_from_slice(b"status\n");
        diff_case(&format!("C39/help{n}"), &v);
    }
    // fine-grained: pad the output in small steps around the 4096 boundary
    for pad in 0..40usize {
        let mut v = Vec::new();
        for _ in 0..3 {
            v.extend_from_slice(b"help\n");
        }
        for _ in 0..pad {
            v.extend_from_slice(b"whoami\n");
        }
        diff_case(&format!("C39/pad{pad}"), &v);
    }
}

#[test]
fn c40_partial_flush_then_segv() {
    // owner string of 40 bytes -> creating the 20th file corrupts file_count
    // with printable bytes -> `files[file_count].permissions` write leaves the
    // mapping -> SIGSEGV.  Only whole 4096-byte stdio blocks survive.
    for pump in 0..10usize {
        let name = vec![b'N'; 32];
        let pass = vec![b'q'; 8];
        let mut artifact = name.clone();
        artifact.extend_from_slice(&pass);
        let mut v = adduser_line(&name, &pass, Some("9"));
        v.extend_from_slice(&login_line(&artifact, &pass));
        for _ in 0..pump {
            v.extend_from_slice(b"help\n");
        }
        v.extend_from_slice(&add_files(20));
        v.extend_from_slice(b"status\nlistfiles\n");
        diff_case(&format!("C40/pump{pump}"), &v);
    }
    // same, but crash triggered through user_count (10th user, 44-byte password)
    for pump in 0..6usize {
        let mut v = Vec::new();
        for _ in 0..pump {
            v.extend_from_slice(b"help\n");
        }
        v.extend_from_slice(&add_users(9));
        let pass = vec![b'p'; 44];
        v.extend_from_slice(&adduser_line(b"last", &pass, Some("3")));
        v.extend_from_slice(b"status\nlistusers\n");
        diff_case(&format!("C40/user-pump{pump}"), &v);
    }
}

// ---------------------------------------------------------------------------
// C41 .. C50
// ---------------------------------------------------------------------------

#[test]
fn c41_time_command() {
    // the timestamp itself is normalized (it can tick between the two runs),
    // its *length* and the surrounding bytes are compared exactly
    diff_case("C41/plain", b"time\ntime\n");
    diff_case("C41/args", b"time now\ntime 1 2 3\n");
    diff_case("C41/mixed", b"debug on\nverbose on\ntime\nstatus\ntime\n");
}

#[test]
fn c42_prefix_and_unknown_commands() {
    let cmds = [
        "add", "addu", "addusers", "adduserx", "ad", "a", "log", "logi", "logins", "logouts",
        "lo", "list", "listx", "listuser", "lis", "create", "createx", "creat", "read", "readx",
        "rea", "write", "writex", "writ", "delete", "deletex", "delet", "bogus", "x", "Adduser",
        "LOGIN", "?x", "??", "exitx", "quitx", "statusx", "helpx", "timex", "setx", "getx",
        "unsetx", "matchx", "compare2", "cmpx", "startswithx", "debugx", "verbosex",
    ];
    let mut v = Vec::from(&b"adduser keep pw 3\nset kv val\n"[..]);
    for c in cmds {
        v.extend_from_slice(format!("{c}\n").as_bytes());
        v.extend_from_slice(format!("{c} one two\n").as_bytes());
    }
    v.extend_from_slice(b"listusers\nlistvars\nstatus\n");
    diff_case("C42", &v);
    // with debug on the [DEBUG] line must appear for these too
    let mut v2 = Vec::from(&b"debug on\n"[..]);
    for c in cmds {
        v2.extend_from_slice(format!("{c} a b c\n").as_bytes());
    }
    diff_case("C42/debug", &v2);
}

#[test]
fn c43_alias_equivalence() {
    // Same state, then the two alias spellings back to back: their outputs must
    // be identical to each other in C, and identical to Rust's.
    let pairs = [
        ("listusers", "users"),
        ("createfile", "touch"),
        ("readfile", "cat"),
        ("writefile", "write"),
        ("deletefile", "rm"),
        ("listfiles", "ls"),
        ("listvars", "vars"),
        ("compare", "cmp"),
        ("compareN", "cmpn"),
        ("help", "?"),
    ];
    for (a, b) in pairs {
        let mut v = Vec::from(&b"adduser own pw 9\nlogin own pw\ncreatefile ff cc\nset k v\n"[..]);
        v.extend_from_slice(format!("{a}\n{b}\n").as_bytes());
        v.extend_from_slice(format!("{a} ff cc 2\n{b} ff cc 2\n").as_bytes());
        v.extend_from_slice(format!("{a} x\n{b} x\n").as_bytes());
        v.extend_from_slice(b"listusers\nlistfiles\nlistvars\nstatus\n");
        diff_case(&format!("C43/{a}-{b}"), &v);
    }
    diff_case("C43/quit", b"adduser a b\nquit\n");
    diff_case("C43/exit", b"adduser a b\nexit\n");
}

const SOUP_CMDS: &[&str] = &[
    "adduser", "login", "logout", "whoami", "listusers", "users", "createfile", "touch",
    "readfile", "cat", "writefile", "write", "deletefile", "rm", "listfiles", "ls", "set",
    "get", "unset", "listvars", "vars", "compare", "cmp", "compareN", "cmpn", "startswith",
    "match", "debug", "verbose", "status", "help", "?", "add", "log", "list", "create", "read",
    "write2", "delete", "bogus", "",
];

fn soup(seed: u64, n_cmds: usize, long_tokens: bool) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    let names: Vec<Vec<u8>> = (0..6)
        .map(|_| {
            if long_tokens {
                let l = rng.range(28, 64);
                rng.token(l)
            } else {
                rng.tok(1, 10)
            }
        })
        .collect();
    let mut v = Vec::new();
    for _ in 0..n_cmds {
        let cmd = *rng.pick(SOUP_CMDS);
        let mut line = cmd.as_bytes().to_vec();
        let nargs = rng.range(0, 4);
        for _ in 0..nargs {
            line.push(b' ');
            match rng.below(6) {
                0 | 1 => {
                    let k = rng.below(names.len());
                    line.extend_from_slice(&names[k]);
                }
                2 => {
                    let t = if long_tokens { rng.tok(28, 64) } else { rng.tok(1, 10) };
                    line.extend_from_slice(&t);
                }
                3 => line.extend_from_slice(rng.pick(&["on", "off", "ON", "1", "0"]).as_bytes()),
                4 => line.extend_from_slice(
                    rng.pick(&["0", "1", "4", "5", "9", "-1", "63", "2147483648", "zz"])
                        .as_bytes(),
                ),
                _ => {
                    let t = if long_tokens { rng.wild_tok(28, 64) } else { rng.wild_tok(1, 12) };
                    line.extend_from_slice(&t);
                }
            }
        }
        line.push(b'\n');
        v.extend_from_slice(&line);
    }
    v
}

#[test]
fn c44_random_command_soup() {
    for seed in 0..12u64 {
        let input = soup(0x4444_0000 + seed, 70, false);
        diff_case(&format!("C44/seed{seed}"), &input);
    }
}

#[test]
fn c45_random_command_soup_long_tokens() {
    for seed in 0..12u64 {
        let input = soup(0x4545_0000 + seed, 70, true);
        diff_case(&format!("C45/seed{seed}"), &input);
    }
}

#[test]
fn c46_mode_cross_product_all_commands() {
    for (mi, p) in MODE_PRELUDE.iter().enumerate() {
        let mut v = Vec::from(p.as_bytes());
        v.extend_from_slice(b"adduser own pw 9\nlogin own pw\ncreatefile ff cc\nset k v\n");
        for cmd in ALL_CMDS {
            v.extend_from_slice(format!("{cmd}\n").as_bytes());
            v.extend_from_slice(format!("{cmd} ff cc 3\n").as_bytes());
        }
        v.extend_from_slice(b"status\n");
        diff_case(&format!("C46/mode{mi}"), &v);
    }
}

#[test]
fn c47_repeated_file_deletions() {
    let mut rng = Rng::new(0x4747_4747);
    for it in 0..8 {
        let mut v = Vec::from(&b"adduser own pw 9\nlogin own pw\n"[..]);
        v.extend_from_slice(&add_files(20));
        let mut alive: Vec<usize> = (0..20).collect();
        while !alive.is_empty() {
            let k = rng.below(alive.len());
            let f = alive.remove(k);
            let cmd = if rng.chance(50) { "deletefile" } else { "rm" };
            v.extend_from_slice(format!("{cmd} f{f}\n").as_bytes());
            if rng.chance(40) {
                v.extend_from_slice(b"listfiles\n");
            }
            if rng.chance(30) {
                let g = rng.below(20);
                v.extend_from_slice(format!("readfile f{g}\n").as_bytes());
            }
        }
        v.extend_from_slice(b"listfiles\nstatus\ncreatefile new after\nls\n");
        diff_case(&format!("C47/it{it}"), &v);
    }
}

#[test]
fn c48_repeated_variable_unsets() {
    let mut rng = Rng::new(0x4848_4848);
    for it in 0..8 {
        let mut v = add_vars(20);
        let mut alive: Vec<usize> = (0..20).collect();
        while !alive.is_empty() {
            let k = rng.below(alive.len());
            let x = alive.remove(k);
            v.extend_from_slice(format!("unset v{x}\n").as_bytes());
            if rng.chance(40) {
                v.extend_from_slice(b"listvars\n");
            }
            if rng.chance(30) {
                let g = rng.below(20);
                v.extend_from_slice(format!("get v{g}\n").as_bytes());
            }
            if rng.chance(20) {
                let s = rng.below(25);
                v.extend_from_slice(format!("set w{s} nv\n").as_bytes());
            }
        }
        v.extend_from_slice(b"listvars\nstatus\n");
        diff_case(&format!("C48/it{it}"), &v);
    }
}

#[test]
fn c49_max_length_name_and_password() {
    for slot in [0usize, 1, 8] {
        let name = vec![b'N'; 63];
        let pass = vec![b'P'; 63];
        let mut v = add_users(slot);
        v.extend_from_slice(&adduser_line(&name, &pass, Some("7")));
        v.extend_from_slice(b"listusers\nstatus\n");
        let mut artifact = vec![b'N'; 32];
        artifact.extend_from_slice(&pass);
        v.extend_from_slice(&login_line(&artifact, &pass));
        v.extend_from_slice(b"whoami\nlistusers\ncreatefile ff cc\nlistfiles\nreadfile ff\n");
        v.extend_from_slice(&login_line(&name, &pass));
        v.extend_from_slice(b"whoami\nlogout\nwhoami\nadduser after pw 1\nlistusers\nstatus\n");
        diff_case(&format!("C49/slot{slot}"), &v);
    }
}

#[test]
fn c50_login_state_on_corrupted_flags() {
    // 9 users, log in as u5, then the 10th user's 40-byte password resets
    // user_count to 0 -> subsequent adduser calls rewrite slots 0.. and clear
    // users[5].logged_in while current_user still points at it.
    let pass = vec![b'p'; 40];
    for extra in 0..8usize {
        let mut v = add_users(9);
        v.extend_from_slice(b"login u5 p5\nwhoami\n");
        v.extend_from_slice(&adduser_line(b"last", &pass, Some("3")));
        v.extend_from_slice(b"status\nwhoami\nlistusers\n");
        for i in 0..extra {
            v.extend_from_slice(format!("adduser n{i} q{i} {i}\n").as_bytes());
            v.extend_from_slice(b"whoami\nstatus\n");
        }
        v.extend_from_slice(b"whoami\nlogout\nwhoami\ncreatefile ff cc\nlistfiles\n");
        v.extend_from_slice(b"login u0 p0\nwhoami\nlogout\nlistusers\nstatus\n");
        diff_case(&format!("C50/extra{extra}"), &v);
    }
}

// ---------------------------------------------------------------------------
// C51 -- stdio buffering mode (glibc line-buffers a terminal, fully buffers a
// pipe; the difference is observable when the program dies with SIGSEGV)
// ---------------------------------------------------------------------------

/// Runs `bin` under `script(1)`, i.e. with a pseudo-terminal on stdin *and*
/// stdout, feeding `input`.  Returns (status, output) or None when `script` is
/// unavailable.
fn run_under_pty(bin: &std::path::Path, input: &[u8]) -> Option<(String, Vec<u8>)> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = match Command::new("script")
        .arg("-q")
        .arg("-e")
        .arg("-c")
        .arg(bin.to_str().unwrap())
        .arg("/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TZ", "UTC")
        .env("LC_ALL", "C")
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return None,
    };
    let mut sink = child.stdin.take().unwrap();
    let data = input.to_vec();
    let w = std::thread::spawn(move || {
        let _ = sink.write_all(&data);
        let _ = sink.flush();
    });
    let out = child.wait_with_output().ok()?;
    let _ = w.join();
    Some((format!("{:?}", out.status.code()), out.stdout))
}

#[test]
fn c51_terminal_line_buffering() {
    let inputs: Vec<Vec<u8>> = vec![
        b"help\nstatus\n".to_vec(),
        b"adduser a b 3\nlogin a b\ncreatefile f c\nls\ncat f\nexit\n".to_vec(),
        b"compare abc abd\nlistvars\nbogus\n".to_vec(),
        {
            // SIGSEGV: with a terminal the completed lines have already been
            // written, so much more output survives than through a pipe
            let mut v = add_users(9);
            let pass = vec![b'p'; 44];
            v.extend_from_slice(&adduser_line(b"last", &pass, Some("3")));
            v.extend_from_slice(b"status\nlistusers\n");
            v
        },
        {
            let mut v = Vec::new();
            for _ in 0..3 {
                v.extend_from_slice(b"help\n");
            }
            v.extend_from_slice(&add_users(9));
            let pass = vec![b'p'; 44];
            v.extend_from_slice(&adduser_line(b"last", &pass, Some("3")));
            v.extend_from_slice(b"status\n");
            v
        },
    ];
    for (i, input) in inputs.iter().enumerate() {
        let c = match run_under_pty(&c_bin(), input) {
            Some(x) => x,
            None => {
                eprintln!("skipping C51: script(1) unavailable");
                return;
            }
        };
        let r = run_under_pty(&rust_bin(), input).unwrap();
        assert_eq!(
            c.1.len(),
            r.1.len(),
            "C51/#{i}: pty output length differs (C={} RUST={})",
            c.1.len(),
            r.1.len()
        );
        assert_eq!(
            normalize_time(&c.1),
            normalize_time(&r.1),
            "C51/#{i}: pty output differs"
        );
        assert_eq!(c.0, r.0, "C51/#{i}: status differs");
    }
}
