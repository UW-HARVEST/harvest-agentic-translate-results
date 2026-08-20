//! Phase C -- error-path differential tests, one test per row of ERRORS.md.
//!
//! Each test builds the exact invalid input/condition, runs BOTH binaries and
//! asserts (a) identical stdout/stderr/exit status and (b) that the reference
//! binary really produced the documented rejection (so no row can pass by
//! accident).

mod common;

use common::*;

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = Vec::new();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

fn users(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(format!("adduser u{i} p{i} {}\n", i % 10).as_bytes());
    }
    v
}

fn files(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(format!("createfile f{i} c{i}\n").as_bytes());
    }
    v
}

fn vars(n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..n {
        v.extend_from_slice(format!("set v{i} val{i}\n").as_bytes());
    }
    v
}

// ---------------------------------------------------------------------------
// cmd_adduser
// ---------------------------------------------------------------------------

#[test]
fn e01_adduser_too_few_args() {
    diff_expect(
        "E01",
        b"adduser\nadduser onlyname\nadduser \t \n",
        b"Usage: adduser <username> <password> [permission_level]\n",
    );
}

#[test]
fn e02_adduser_max_users() {
    let v = cat(&[&users(10), b"adduser eleventh pw 1\nstatus\n"]);
    diff_expect("E02", &v, b"Error: Maximum users reached\n");
}

#[test]
fn e03_adduser_duplicate() {
    diff_expect(
        "E03",
        b"adduser dup pw\nadduser dup other 5\nlistusers\n",
        b"Error: User 'dup' already exists\n",
    );
}

#[test]
fn e04_adduser_full_beats_duplicate() {
    // ordering: the MAX_USERS check happens *before* the duplicate scan
    let v = cat(&[&users(10), b"adduser u0 p0 1\n"]);
    diff_expect("E04", &v, b"Error: Maximum users reached\n");
    diff_expect_absent("E04/no-dup-msg", &v, b"already exists");
}

// ---------------------------------------------------------------------------
// cmd_login
// ---------------------------------------------------------------------------

#[test]
fn e05_login_too_few_args() {
    diff_expect(
        "E05",
        b"login\nlogin justname\n",
        b"Usage: login <username> <password>\n",
    );
}

#[test]
fn e06_login_already_logged_in() {
    diff_expect(
        "E06",
        b"adduser a b\nlogin a b\nlogin a b\nlogin nosuchuser x\nlogin a wrong\n",
        b"Error: User 'a' already logged in. Use 'logout' first.\n",
    );
}

#[test]
fn e07_login_wrong_password() {
    diff_expect(
        "E07",
        b"adduser a b\nlogin a wrong\nlogin a B\nlogin a bb\nlogin a \n",
        b"Error: Incorrect password\n",
    );
}

#[test]
fn e08_login_user_not_found() {
    diff_expect("E08/empty-table", b"login ghost pw\n", b"Error: User not found\n");
    let v = cat(&[&users(3), b"login ghost pw\nlogin u9 p9\nlogin U0 p0\n"]);
    diff_expect("E08/populated", &v, b"Error: User not found\n");
}

#[test]
fn e09_login_first_match_wins() {
    // two users whose *name strings* are equal because of the name overflow:
    // only the first is ever consulted, so the second's password is unusable
    let name = vec![b'N'; 32];
    let mut v = Vec::new();
    v.extend_from_slice(b"adduser ");
    v.extend_from_slice(&name);
    v.extend_from_slice(b" same 1\n");
    v.extend_from_slice(b"adduser ");
    v.extend_from_slice(&name);
    v.extend_from_slice(b" other 1\n");
    v.extend_from_slice(b"listusers\n");
    let mut art = name.clone();
    art.extend_from_slice(b"same");
    v.extend_from_slice(b"login ");
    v.extend_from_slice(&art);
    v.extend_from_slice(b" other\n");
    v.extend_from_slice(b"login ");
    v.extend_from_slice(&art);
    v.extend_from_slice(b" same\nwhoami\n");
    diff_expect("E09", &v, b"Error: Incorrect password\n");
}

// ---------------------------------------------------------------------------
// cmd_logout / cmd_whoami / cmd_listusers
// ---------------------------------------------------------------------------

#[test]
fn e10_logout_not_logged_in() {
    diff_expect("E10", b"logout\nlogout x y\n", b"Error: No user logged in\n");
}

#[test]
fn e11_logout_after_logout() {
    diff_expect(
        "E11",
        b"adduser a b\nlogin a b\nlogout\nlogout\nwhoami\n",
        b"Error: No user logged in\n",
    );
    // logged_in cleared by an overflow while current_user still points there
    let pass = vec![b'p'; 40];
    let mut v = users(9);
    v.extend_from_slice(b"login u5 p5\nwhoami\n");
    v.extend_from_slice(b"adduser last ");
    v.extend_from_slice(&pass);
    v.extend_from_slice(b" 3\n");
    for i in 0..6 {
        v.extend_from_slice(format!("adduser n{i} q{i} 1\n").as_bytes());
    }
    v.extend_from_slice(b"whoami\nlogout\nstatus\n");
    diff_expect("E11/overflow-cleared", &v, b"Error: No user logged in\n");
}

#[test]
fn e12_whoami_not_logged_in() {
    diff_expect("E12", b"whoami\nadduser a b\nwhoami\n", b"Not logged in\n");
}

#[test]
fn e13_listusers_empty() {
    diff_expect("E13", b"listusers\nusers\n", b"No users registered\n");
}

// ---------------------------------------------------------------------------
// cmd_createfile
// ---------------------------------------------------------------------------

#[test]
fn e14_createfile_not_logged_in() {
    diff_expect(
        "E14",
        b"createfile f c\ntouch f\ncreatefile\nadduser a b\ncreatefile f c\n",
        b"Error: Must be logged in\n",
    );
    // the login check comes first, so a missing filename is not reported
    diff_expect_absent("E14/order", b"createfile\n", b"Usage: createfile");
}

#[test]
fn e15_createfile_no_filename() {
    diff_expect(
        "E15",
        b"adduser a b\nlogin a b\ncreatefile\ntouch\n",
        b"Usage: createfile <filename> [content]\n",
    );
}

#[test]
fn e16_createfile_max_files() {
    let v = cat(&[
        b"adduser a b 9\nlogin a b\n",
        &files(20),
        b"createfile f20 c\ntouch f21\nstatus\n",
    ]);
    diff_expect("E16", &v, b"Error: Maximum files reached\n");
}

#[test]
fn e17_createfile_duplicate() {
    diff_expect(
        "E17",
        b"adduser a b\nlogin a b\ncreatefile dup one\ncreatefile dup two\ntouch dup\nlistfiles\n",
        b"Error: File 'dup' already exists\n",
    );
}

#[test]
fn e18_createfile_full_beats_duplicate() {
    let v = cat(&[b"adduser a b 9\nlogin a b\n", &files(20), b"createfile f0 x\n"]);
    diff_expect("E18", &v, b"Error: Maximum files reached\n");
    diff_expect_absent("E18/no-dup-msg", &v, b"already exists");
}

// ---------------------------------------------------------------------------
// cmd_readfile
// ---------------------------------------------------------------------------

#[test]
fn e19_readfile_no_arg() {
    diff_expect("E19", b"readfile\ncat\n", b"Usage: readfile <filename>\n");
}

#[test]
fn e20_readfile_not_found() {
    diff_expect("E20/empty", b"readfile nope\ncat nope\n", b"Error: File 'nope' not found\n");
    let v = cat(&[
        b"adduser a b\nlogin a b\n",
        &files(3),
        b"readfile f9\ncat F0\nreadfile f\n",
    ]);
    diff_expect("E20/populated", &v, b"Error: File 'f9' not found\n");
}

#[test]
fn e21_readfile_without_login_succeeds() {
    // there is deliberately NO login check in cmd_readfile
    let v = b"adduser a b\nlogin a b\ncreatefile pub hello\nlogout\nreadfile pub\ncat pub\n";
    diff_expect("E21", v, b"=== pub ===\nOwner: a\nPermissions: 755\nContent: hello\n");
}

// ---------------------------------------------------------------------------
// cmd_writefile
// ---------------------------------------------------------------------------

#[test]
fn e22_writefile_not_logged_in() {
    diff_expect(
        "E22",
        b"writefile f c\nwrite f c\nwritefile\n",
        b"Error: Must be logged in\n",
    );
    diff_expect_absent("E22/order", b"writefile\n", b"Usage: writefile");
}

#[test]
fn e23_writefile_too_few_args() {
    diff_expect(
        "E23",
        b"adduser a b\nlogin a b\nwritefile\nwritefile onlyname\nwrite f\n",
        b"Usage: writefile <filename> <content>\n",
    );
}

#[test]
fn e24_writefile_permission_denied() {
    for lvl in ["-3", "0", "1", "2", "3", "4"] {
        let v = cat(&[
            b"adduser owner pw 1\n",
            format!("adduser other pw2 {lvl}\n").as_bytes(),
            b"login owner pw\ncreatefile shared v1\nlogout\n",
            b"login other pw2\nwritefile shared v2\nreadfile shared\n",
        ]);
        diff_expect(&format!("E24/lvl{lvl}"), &v, b"Error: Permission denied\n");
    }
}

#[test]
fn e25_writefile_file_not_found() {
    let v = b"adduser a b 1\nlogin a b\ncreatefile f c\nwritefile ghost x\nwrite GHOST y\n";
    diff_expect("E25", v, b"Error: File 'ghost' not found\n");
}

#[test]
fn e26_writefile_level_boundary() {
    for (lvl, expect) in [("4", &b"Error: Permission denied\n"[..]), ("5", &b"File 'shared' updated\n"[..])] {
        let v = cat(&[
            b"adduser owner pw 1\n",
            format!("adduser other pw2 {lvl}\n").as_bytes(),
            b"login owner pw\ncreatefile shared v1\nlogout\n",
            b"login other pw2\nwritefile shared v2\nreadfile shared\n",
        ]);
        diff_expect(&format!("E26/lvl{lvl}"), &v, expect);
    }
}

// ---------------------------------------------------------------------------
// cmd_deletefile
// ---------------------------------------------------------------------------

#[test]
fn e27_deletefile_not_logged_in() {
    diff_expect("E27", b"deletefile f\nrm f\ndeletefile\n", b"Error: Must be logged in\n");
    diff_expect_absent("E27/order", b"deletefile\n", b"Usage: deletefile");
}

#[test]
fn e28_deletefile_no_arg() {
    diff_expect(
        "E28",
        b"adduser a b\nlogin a b\ndeletefile\nrm\n",
        b"Usage: deletefile <filename>\n",
    );
}

#[test]
fn e29_deletefile_permission_denied() {
    for lvl in ["0", "1", "5", "6", "7", "8"] {
        let v = cat(&[
            b"adduser owner pw 1\n",
            format!("adduser other pw2 {lvl}\n").as_bytes(),
            b"login owner pw\ncreatefile shared v1\nlogout\n",
            b"login other pw2\ndeletefile shared\nlistfiles\n",
        ]);
        diff_expect(&format!("E29/lvl{lvl}"), &v, b"Error: Permission denied\n");
    }
}

#[test]
fn e30_deletefile_not_found() {
    let v = b"adduser a b 9\nlogin a b\ncreatefile f c\ndeletefile ghost\nrm f\nrm f\n";
    diff_expect("E30", v, b"Error: File 'ghost' not found\n");
}

#[test]
fn e31_deletefile_level_boundary() {
    for (lvl, expect) in [("8", &b"Error: Permission denied\n"[..]), ("9", &b"File 'shared' deleted\n"[..])] {
        let v = cat(&[
            b"adduser owner pw 1\n",
            format!("adduser other pw2 {lvl}\n").as_bytes(),
            b"login owner pw\ncreatefile shared v1\nlogout\n",
            b"login other pw2\ndeletefile shared\nlistfiles\n",
        ]);
        diff_expect(&format!("E31/lvl{lvl}"), &v, expect);
    }
}

#[test]
fn e32_listfiles_empty() {
    diff_expect("E32", b"listfiles\nls\n", b"No files\n");
    let v = b"adduser a b 9\nlogin a b\ncreatefile f c\ndeletefile f\nlistfiles\n";
    diff_expect("E32/after-delete", v, b"No files\n");
}

// ---------------------------------------------------------------------------
// variables
// ---------------------------------------------------------------------------

#[test]
fn e33_set_too_few_args() {
    diff_expect("E33", b"set\nset onlyname\n", b"Usage: set <name> <value>\n");
}

#[test]
fn e34_set_max_variables() {
    let v = cat(&[&vars(20), b"set v20 x\nset another y\nstatus\n"]);
    diff_expect("E34", &v, b"Error: Maximum variables reached\n");
}

#[test]
fn e35_set_full_but_existing_updates() {
    // the capacity check is *after* the update scan
    let v = cat(&[&vars(20), b"set v7 updated\nget v7\nset v20 nope\n"]);
    diff_expect("E35", &v, b"Variable 'v7' updated\n");
    diff_expect("E35/still-rejects-new", &v, b"Error: Maximum variables reached\n");
}

#[test]
fn e36_get_no_arg() {
    diff_expect("E36", b"get\n", b"Usage: get <name>\n");
}

#[test]
fn e37_get_not_found() {
    diff_expect("E37/empty", b"get k\n", b"Error: Variable 'k' not found\n");
    let v = cat(&[&vars(3), b"get nope\nget V0\n"]);
    diff_expect("E37/populated", &v, b"Error: Variable 'nope' not found\n");
}

#[test]
fn e38_unset_no_arg() {
    diff_expect("E38", b"unset\n", b"Usage: unset <name>\n");
}

#[test]
fn e39_unset_not_found() {
    let v = cat(&[&vars(3), b"unset nope\nunset v0\nunset v0\nlistvars\n"]);
    diff_expect("E39", &v, b"Error: Variable 'nope' not found\n");
}

#[test]
fn e40_listvars_empty() {
    diff_expect("E40", b"listvars\nvars\n", b"No variables set\n");
    diff_expect("E40/after-unset", b"set k v\nunset k\nlistvars\n", b"No variables set\n");
}

// ---------------------------------------------------------------------------
// string commands
// ---------------------------------------------------------------------------

#[test]
fn e41_compare_too_few_args() {
    diff_expect("E41", b"compare\ncompare one\ncmp\ncmp x\n", b"Usage: compare <string1> <string2>\n");
}

#[test]
fn e42_comparen_too_few_args() {
    diff_expect(
        "E42",
        b"compareN\ncompareN a\ncompareN a b\ncmpn\ncmpn a\ncmpn a b\n",
        b"Usage: compareN <string1> <string2> <n>\n",
    );
}

#[test]
fn e43_comparen_non_numeric_count() {
    diff_expect(
        "E43",
        b"compareN a b abc\ncompareN a b x9\ncompareN a b +\ncompareN a b -\n",
        b"strncmp('a', 'b', 0) = 0\nFirst 0 characters are equal\n",
    );
}

#[test]
fn e44_comparen_negative_count() {
    diff_expect(
        "E44",
        b"compareN a b -1\ncompareN abc abd -5\ncompareN x x -1\n",
        b"strncmp('a', 'b', -1) = -1\n'a' < 'b' (first -1 chars)\n",
    );
}

#[test]
fn e45_comparen_overflowing_count() {
    diff_expect(
        "E45/int-max-plus-1",
        b"compareN a b 2147483648\n",
        b"strncmp('a', 'b', -2147483648) = -1\n",
    );
    diff_expect(
        "E45/long-max-plus",
        b"compareN a b 99999999999999999999\n",
        b"strncmp('a', 'b', -1) = -1\n",
    );
    diff_expect("E45/int-max", b"compareN a b 2147483647\n", b"strncmp('a', 'b', 2147483647) = -1\n");
    diff_case("E45/others", b"compareN a b -2147483648\ncompareN a b -2147483649\ncompareN a b 4294967296\n");
}

#[test]
fn e46_startswith_too_few_args() {
    diff_expect("E46", b"startswith\nstartswith one\n", b"Usage: startswith <string> <prefix>\n");
}

#[test]
fn e47_match_too_few_args() {
    diff_expect(
        "E47",
        b"match\nmatch pattern\n",
        b"Usage: match <pattern> <string1> [string2] ...\n",
    );
}

// ---------------------------------------------------------------------------
// modes
// ---------------------------------------------------------------------------

#[test]
fn e48_debug_no_arg_prints_state() {
    diff_expect("E48/off", b"debug\n", b"Debug mode: OFF\n");
    diff_expect("E48/on", b"debug on\ndebug\n", b"Debug mode: ON\n");
}

#[test]
fn e49_debug_invalid_arg() {
    diff_expect(
        "E49",
        b"debug ON\ndebug 1\ndebug onx\ndebug offf\ndebug true\ndebug -\n",
        b"Usage: debug [on|off]\n",
    );
}

#[test]
fn e50_verbose_no_arg_prints_state() {
    diff_expect("E50/off", b"verbose\n", b"Verbose mode: OFF\n");
    diff_expect("E50/on", b"verbose on\nverbose\n", b"Verbose mode: ON\n");
}

#[test]
fn e51_verbose_invalid_arg() {
    diff_expect(
        "E51",
        b"verbose OFF\nverbose 0\nverbose ono\nverbose yes\n",
        b"Usage: verbose [on|off]\n",
    );
}

// ---------------------------------------------------------------------------
// dispatch / main
// ---------------------------------------------------------------------------

#[test]
fn e52_empty_command_is_silent() {
    // with debug on, an empty command must NOT print the [DEBUG] line
    diff_expect_absent("E52/debug", b"debug on\n\n   \n\t\n  \t \n", b"[DEBUG] Command: ''");
    diff_expect_absent("E52/unknown", b"\n \n\t\n", b"Unknown command");
    diff_case("E52/verbose", b"verbose on\n\n   \n\t\nstatus\n");
}

#[test]
fn e53_prefix_add() {
    diff_expect("E53", b"add\nadd x\naddu\naddus\naddusers\nadduserx a b\n", b"Did you mean 'adduser'?\n");
}

#[test]
fn e54_prefix_log() {
    diff_expect("E54", b"log\nlogi\nlogins\nlogouts\nlogx\n", b"Did you mean 'login' or 'logout'?\n");
}

#[test]
fn e55_prefix_list() {
    diff_expect(
        "E55",
        b"list\nlistx\nlistuser\nlistfile\nlistvar\n",
        b"Did you mean 'listusers', 'listfiles', or 'listvars'?\n",
    );
}

#[test]
fn e56_prefix_create_read_write_delete() {
    diff_expect("E56/create", b"create\ncreatex\ncreatefiles\n", b"Did you mean 'createfile'?\n");
    diff_expect("E56/read", b"read\nreadx\nreadfiles\n", b"Did you mean 'readfile'?\n");
    diff_expect("E56/write", b"write2\nwritex\nwritefiles\n", b"Did you mean 'writefile'?\n");
    diff_expect("E56/delete", b"delete\ndeletex\ndeletefiles\n", b"Did you mean 'deletefile'?\n");
}

#[test]
fn e57_unknown_command() {
    diff_expect(
        "E57",
        b"bogus\nad\nlo\nlis\ncreat\nwrit\ndelet\nx\nADDUSER\n??\n",
        b"Unknown command: 'bogus'. Type 'help' for available commands.\n",
    );
    diff_expect("E57/short-ad", b"ad\n", b"Unknown command: 'ad'.");
    diff_expect("E57/short-lis", b"lis\n", b"Unknown command: 'lis'.");
    diff_expect("E57/short-writ", b"writ\n", b"Unknown command: 'writ'.");
    // non-ASCII command
    diff_case("E57/nonascii", b"\xff\xfe\x80\ncompare a b\n");
}

#[test]
fn e58_prefix_checked_after_exact_matches() {
    // "list" is a prefix branch but "listusers" is an exact match earlier
    diff_expect_absent("E58/listusers", b"adduser a b\nlistusers\n", b"Did you mean");
    diff_expect_absent("E58/adduser", b"adduser a b\n", b"Did you mean");
    diff_expect_absent("E58/write", b"adduser a b\nlogin a b\ncreatefile f c\nwrite f x\n", b"Did you mean");
    diff_expect("E58/lister", b"lister\n", b"Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
}

#[test]
fn e59_eof_paths() {
    diff_expect_status("E59/empty", b"", "exit:0");
    diff_expect_status("E59/no-newline", b"status", "exit:0");
    diff_expect_status("E59/newline-only", b"\n", "exit:0");
    diff_case("E59/trailing-prompt", b"adduser a b\n");
}

#[test]
fn e60_token_truncation_at_63() {
    let t64 = vec![b'T'; 64];
    let t80 = vec![b'T'; 80];
    let v = cat(&[
        b"compare ",
        &t64,
        b" ",
        &t80,
        b"\nset ",
        &t64,
        b" ",
        &t80,
        b"\nlistvars\nadduser ",
        &t80,
        b" pw\nlistusers\n",
    ]);
    // 63 T's, not 64/80
    let mut expect = b"strcmp('".to_vec();
    expect.extend_from_slice(&vec![b'T'; 63]);
    expect.extend_from_slice(b"', '");
    expect.extend_from_slice(&vec![b'T'; 63]);
    expect.extend_from_slice(b"') = 0\n");
    diff_expect("E60", &v, &expect);
}

#[test]
fn e61_line_split_at_255() {
    // 260-byte line: the first 255 bytes form one command, the rest another
    let mut v = Vec::new();
    v.extend_from_slice(b"compare ");
    v.extend_from_slice(&vec![b'A'; 247]);
    v.extend_from_slice(&vec![b'B'; 13]);
    v.push(b'\n');
    v.extend_from_slice(b"status\n");
    diff_expect("E61", &v, b"Usage: compare <string1> <string2>\n");
    let mut expect = b"Unknown command: '".to_vec();
    expect.extend_from_slice(&vec![b'B'; 13]);
    expect.extend_from_slice(b"'. Type 'help' for available commands.\n");
    diff_expect("E61/tail", &v, &expect);
}

#[test]
fn e62_embedded_nul_truncates_line() {
    diff_expect("E62", b"status\x00 garbage tail\nlistvars\n", b"=== System Status ===\n");
    diff_expect_absent("E62/no-extra", b"status\x00 garbage\n", b"Unknown command");
    // the NUL cuts the line, so `compare` sees a single argument
    diff_expect(
        "E62/token",
        b"compare ab\x00cd ab\nlistvars\n",
        b"Usage: compare <string1> <string2>\n",
    );
    diff_case("E62/first-byte", b"\x00status\nstatus\n");
}

// ---------------------------------------------------------------------------
// undefined-behaviour rejections (U01 .. U08)
// ---------------------------------------------------------------------------

#[test]
fn u01_name_overflow_into_password() {
    let name = vec![b'N'; 40];
    let v = cat(&[b"adduser ", &name, b" pw12 6\nlistusers\nstatus\n"]);
    let mut expect = b"  ".to_vec();
    expect.extend_from_slice(&vec![b'N'; 32]);
    expect.extend_from_slice(b"pw12 (level 6) \n");
    diff_expect("U01", &v, &expect);
}

#[test]
fn u02_password_overflow_into_next_slot() {
    let pass = vec![b'P'; 39];
    let v = cat(&[
        b"adduser first ",
        &pass,
        b" 4\nlistusers\nadduser second sp 2\nlistusers\nstatus\n",
    ]);
    // the second slot's name field holds the tail of the first password until
    // the next `adduser` rewrites it
    diff_expect("U02", &v, b"Registered users:\n");
    for plen in 32usize..=39 {
        let pass = vec![b'P'; plen];
        let v = cat(&[
            b"adduser first ",
            &pass,
            b" 4\nlistusers\nadduser second sp 2\nlistusers\nlogin second sp\nwhoami\nstatus\n",
        ]);
        diff_case(&format!("U02/plen{plen}"), &v);
    }
}

#[test]
fn u03_user_count_zeroed_by_40_byte_password() {
    let pass = vec![b'p'; 40];
    let v = cat(&[&users(9), b"adduser last ", &pass, b" 3\nstatus\nlistusers\n"]);
    diff_expect("U03", &v, b"Users: 1/10\n");
    diff_expect("U03/one-row", &v, b"  u0 (level 3) \n");
}

#[test]
fn u04_user_count_set_to_password_byte() {
    for b in [1u8, 5, 100, 224] {
        let mut pass = vec![b'p'; 40];
        pass.push(b);
        let v = cat(&[&users(9), b"adduser last ", &pass, b" 2\nstatus\n"]);
        diff_expect(
            &format!("U04/byte{b}"),
            &v,
            format!("Users: {}/10\n", b as u32 + 1).as_bytes(),
        );
    }
}

#[test]
fn u05_user_count_overflow_segv() {
    for len in [42usize, 43, 44, 48, 56, 63] {
        let pass = vec![b'p'; len];
        let v = cat(&[&users(9), b"adduser last ", &pass, b" 3\nstatus\nlistusers\n"]);
        diff_expect_status(&format!("U05/len{len}"), &v, "signal:11");
    }
    // byte >= 225 in the 41-byte case
    for b in [225u8, 240, 255] {
        let mut pass = vec![b'p'; 40];
        pass.push(b);
        let v = cat(&[&users(9), b"adduser last ", &pass, b" 3\nstatus\n"]);
        diff_expect_status(&format!("U05/byte{b}"), &v, "signal:11");
    }
}

#[test]
fn u06_file_count_corruption() {
    // owner string length 36 -> file_count zeroed
    let name = vec![b'N'; 32];
    let pass = vec![b'q'; 4];
    let mut art = name.clone();
    art.extend_from_slice(&pass);
    let v = cat(&[
        b"adduser ",
        &name,
        b" ",
        &pass,
        b" 9\nlogin ",
        &art,
        b" ",
        &pass,
        b"\n",
        &files(20),
        b"status\nlistfiles\n",
    ]);
    diff_expect("U06/len36", &v, b"Files: 1/20\n");

    // owner string length 40 -> file_count = 'qqqq' -> out of range write
    let pass8 = vec![b'q'; 8];
    let mut art8 = name.clone();
    art8.extend_from_slice(&pass8);
    let v2 = cat(&[
        b"adduser ",
        &name,
        b" ",
        &pass8,
        b" 9\nlogin ",
        &art8,
        b" ",
        &pass8,
        b"\n",
        &files(20),
        b"status\n",
    ]);
    diff_expect_status("U06/len40", &v2, "signal:11");
}

#[test]
fn u07_variable_scan_off_the_mapping_segv() {
    // The 10th user's 41-byte password sets user_count = 224, so
    // `users[224].permission_level = atoi(argv)` writes straight into
    // `variable_count` (0x40afe0 == &users[224].permission_level).  Variable
    // commands then scan variables[0..variable_count] and walk off the end of
    // the writable mapping.  `listvars` reads `.value` too, so it dies one
    // element earlier than the name-only scans.
    let vc_script = |vc: usize, tail: &[u8]| -> Vec<u8> {
        let mut pass = vec![b'p'; 40];
        pass.push(224);
        cat(&[
            &users(9),
            b"adduser last ",
            &pass,
            format!(" {vc}\n").as_bytes(),
            tail,
        ])
    };

    // far out of range: every variable command dies
    for vc in [26usize, 30, 60, 200] {
        for tail in [
            &b"status\nlistvars\n"[..],
            &b"get zz\n"[..],
            &b"set zz yy\n"[..],
            &b"unset zz\n"[..],
        ] {
            diff_expect_status(
                &format!("U07/far{vc}/{}", tail.len()),
                &vc_script(vc, tail),
                "signal:11",
            );
        }
    }

    // boundary: variables[20].name is the last object inside the mapping, its
    // .value is not -> listvars dies at 21, the name-only scans at 22
    diff_expect_status("U07/listvars21", &vc_script(21, b"listvars\n"), "signal:11");
    diff_expect_status("U07/get21", &vc_script(21, b"get zz\n"), "exit:0");
    diff_expect_status("U07/unset21", &vc_script(21, b"unset zz\n"), "exit:0");
    diff_expect_status("U07/set21", &vc_script(21, b"set zz yy\n"), "exit:0");
    diff_expect_status("U07/get22", &vc_script(22, b"get zz\n"), "signal:11");
    diff_expect_status("U07/unset22", &vc_script(22, b"unset zz\n"), "signal:11");
    diff_expect_status("U07/set22", &vc_script(22, b"set zz yy\n"), "signal:11");
    diff_expect_status("U07/listvars20", &vc_script(20, b"listvars\n"), "exit:0");

    // fully in-range corrupted counts stay alive
    for vc in [1usize, 5, 19, 20] {
        diff_expect_status(
            &format!("U07/alive{vc}"),
            &vc_script(vc, b"status\nlistvars\nget zz\nset zz yy\nunset zz\nlistvars\n"),
            "exit:0",
        );
    }
}

#[test]
fn u08_negative_user_count_writes_padding() {
    // password bytes 40..43 = 0xff -> user_count = -1.  The name/password
    // `strcpy`s used the *old* count, and the two `int` writes land in the
    // 24-byte padding below `users`, which is harmless: the program survives
    // and reports `Users: 0/10` after the `++`.
    let mut pass = vec![b'p'; 40];
    pass.extend_from_slice(&[0xff, 0xff, 0xff, 0xff]);
    let v = cat(&[
        &users(9),
        b"adduser last ",
        &pass,
        b" 3\nstatus\nlistusers\nadduser more pw 1\nstatus\n",
    ]);
    diff_expect("U08", &v, b"Users: 0/10\n");
    diff_expect_status("U08/survives", &v, "exit:0");
}

#[test]
fn u09_negative_count_clobbers_got_segv() {
    // password bytes 40..43 = fe ff ff ff -> user_count = -2, ++ -> -1, so the
    // *next* adduser `strcpy`s into users[-1], i.e. into .got.plt: the next
    // libc call jumps to the clobbered slot.
    let mut pass = vec![b'p'; 40];
    pass.extend_from_slice(&[0xfe, 0xff, 0xff, 0xff]);
    let v = cat(&[
        &users(9),
        b"adduser last ",
        &pass,
        b" 3\nstatus\nadduser more pw 1\nstatus\nlistusers\n",
    ]);
    diff_expect_status("U09", &v, "signal:11");
}

#[test]
fn u10_negative_count_clobbers_dynamic_segv() {
    // user_count = -5, ++ -> -4: users[-4] lies in .dynamic/.fini_array, so
    // the program dies before its buffered output reaches the pipe.
    let mut pass = vec![b'p'; 40];
    pass.extend_from_slice(&[0xfb, 0xff, 0xff, 0xff]);
    let v = cat(&[
        &users(9),
        b"adduser last ",
        &pass,
        b" 3\nstatus\nadduser more pw 1\nstatus\n",
    ]);
    diff_expect_status("U10", &v, "signal:11");
}

#[test]
fn u11_corrupted_file_count_shift_stays_in_range() {
    // owner string of 37 bytes with a control byte -> file_count = 24, then
    // `file_count++` -> 25.  deletefile's shift loop copies garbage but never
    // leaves the mapping, so the program survives.
    let name = vec![b'N'; 32];
    let pass = vec![b'q', b'q', b'q', b'q', 24];
    let mut art = name.clone();
    art.extend_from_slice(&pass);
    let v = cat(&[
        b"adduser ",
        &name,
        b" ",
        &pass,
        b" 9\nlogin ",
        &art,
        b" ",
        &pass,
        b"\n",
        &files(20),
        b"status\ndeletefile f0\nlistfiles\nstatus\n",
    ]);
    diff_expect_status("U11", &v, "exit:0");
}
