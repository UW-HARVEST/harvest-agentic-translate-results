//! Differential tests: run the C reference binary and the Rust binary as
//! subprocesses with identical stdin and require identical stdout, stderr and
//! exit status.
//!
//! The cases are derived from the branches actually present in
//! `c_src/src/main.c`: every `if`, every early `return`, every argument-count
//! guard, every capacity limit, every `strncmp` prefix suggestion, and the
//! `strcpy` overruns that corrupt the globals following each record array.

mod common;

use common::{assert_all, assert_same, script};

// ---------------------------------------------------------------------------
// main(): banner, prompt, fgets and EOF
// ---------------------------------------------------------------------------

#[test]
fn empty_and_blank_input() {
    assert_all(&[
        // fgets returns NULL immediately: banner, one prompt, exit 0.
        ("no input at all", Vec::new()),
        ("single newline", b"\n".to_vec()),
        ("several newlines", b"\n\n\n".to_vec()),
        // strtok finds no token, so `command` stays empty and nothing runs.
        ("spaces only", b"   \n".to_vec()),
        ("tabs only", b"\t\t\n".to_vec()),
        ("mixed blanks", b" \t \t \n".to_vec()),
        ("blank between commands", b"status\n\nwhoami\n   \n".to_vec()),
        // No trailing newline: fgets still returns the partial line.
        ("no trailing newline", b"status".to_vec()),
        ("only spaces, no newline", b"   ".to_vec()),
        // strcspn stops at '\n', so a CR stays part of the token.
        ("crlf line endings", b"status\r\nwhoami\r\n".to_vec()),
        ("bare carriage return", b"status\r".to_vec()),
        // The buffer is read as a C string, so an embedded NUL truncates it.
        ("nul inside command", b"sta\0tus\n".to_vec()),
        ("nul inside args", b"set a\0b c\nlistvars\n".to_vec()),
        ("nul first", b"\0status\n".to_vec()),
    ]);
}

#[test]
fn fgets_splits_long_lines() {
    // fgets reads at most MAX_INPUT - 1 == 255 bytes, so a longer line is
    // delivered as several "commands" without an intervening prompt skip.
    let mut cases: Vec<(&str, Vec<u8>)> = Vec::new();
    for (name, n) in [
        ("254", 254usize),
        ("255", 255),
        ("256", 256),
        ("257", 257),
        ("510", 510),
        ("511", 511),
        ("512", 512),
    ] {
        cases.push((name, format!("{}\n", "x".repeat(n)).into_bytes()));
    }
    // A split that lands in the middle of a token, so the tail becomes its own
    // command.
    cases.push((
        "split mid token",
        format!("{} status\n", "a".repeat(250)).into_bytes(),
    ));
    cases.push((
        "split produces valid command",
        format!("compare {} {}\n", "a".repeat(240), "b".repeat(240)).into_bytes(),
    ));
    assert_all(&cases);
}

#[test]
fn parse_command_truncation_and_arg_cap() {
    let long = "L".repeat(200);
    let cases: Vec<(&str, Vec<u8>)> = vec![
        // Tokens are truncated to MAX_COMMAND - 1 == 63 bytes.
        ("63 byte token", script(&[&format!("compare {} b", "a".repeat(63))])),
        ("64 byte token", script(&[&format!("compare {} b", "a".repeat(64))])),
        ("65 byte token", script(&[&format!("compare {} b", "a".repeat(65))])),
        ("long command name", script(&[&long])),
        // MAX_ARGS == 10: further tokens are dropped by parse_command.
        (
            "exactly 10 args",
            script(&["match p a1 a2 a3 a4 a5 a6 a7 a8 a9"]),
        ),
        (
            "11 args, last dropped",
            script(&["match p a1 a2 a3 a4 a5 a6 a7 a8 a9 a10"]),
        ),
        (
            "14 args",
            script(&["match p a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13"]),
        ),
        // Separators are only ' ' and '\t'; runs of them collapse.
        ("tab separated", script(&["adduser\talice\tpw\t4", "listusers"])),
        ("repeated separators", script(&["adduser   bob \t\t pw  7", "listusers"])),
        ("leading blanks", script(&["   status", "\t\t whoami"])),
        ("trailing blanks", script(&["status   ", "whoami\t"])),
    ];
    assert_all(&cases);
}

// ---------------------------------------------------------------------------
// User management
// ---------------------------------------------------------------------------

#[test]
fn adduser_branches() {
    assert_all(&[
        ("no args", script(&["adduser"])),
        ("one arg", script(&["adduser alice"])),
        ("two args, default level", script(&["adduser alice pw", "listusers"])),
        ("three args", script(&["adduser alice pw 5", "listusers"])),
        ("four args, extra ignored", script(&["adduser alice pw 5 junk", "listusers"])),
        ("duplicate user", script(&["adduser alice pw", "adduser alice other", "listusers"])),
        (
            "duplicate after several",
            script(&["adduser a p", "adduser b p", "adduser a p"]),
        ),
        // atoi on the permission level.
        (
            "atoi variants",
            script(&[
                "adduser u1 p abc",
                "adduser u2 p -3",
                "adduser u3 p +7",
                "adduser u4 p 0",
                "adduser u5 p 12abc",
                "adduser u6 p 0x10",
                "adduser u7 p 2147483647",
                "adduser u8 p 2147483648",
                "adduser u9 p -2147483649",
                "listusers",
                "status",
            ]),
        ),
        (
            "atoi huge saturating",
            script(&[
                "adduser u1 p 99999999999999999999",
                "adduser u2 p -99999999999999999999",
                "adduser u3 p 9223372036854775807",
                "adduser u4 p 9223372036854775808",
                "listusers",
            ]),
        ),
    ]);
}

#[test]
fn adduser_capacity() {
    // MAX_USERS == 10: the 11th and 12th must report the limit.
    let mut lines: Vec<String> = (1..=12).map(|i| format!("adduser u{i} p{i} {i}")).collect();
    lines.push("listusers".to_string());
    lines.push("status".to_string());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    assert_same("max users", &script(&refs));
}

#[test]
fn login_logout_whoami_branches() {
    assert_all(&[
        ("login no args", script(&["login"])),
        ("login one arg", script(&["login alice"])),
        ("login unknown user", script(&["login nobody x"])),
        ("login wrong password", script(&["adduser alice pw", "login alice bad"])),
        ("login ok", script(&["adduser alice pw", "login alice pw", "whoami"])),
        (
            "login twice",
            script(&["adduser alice pw", "adduser bob pw", "login alice pw", "login bob pw"]),
        ),
        (
            "login after logout",
            script(&[
                "adduser alice pw",
                "adduser bob pw 4",
                "login alice pw",
                "logout",
                "login bob pw",
                "whoami",
            ]),
        ),
        // The password check happens only for the first name match.
        (
            "second user same name never reached",
            script(&["adduser alice pw", "adduser alice other", "login alice other"]),
        ),
        ("logout when nobody logged in", script(&["logout"])),
        ("logout twice", script(&["adduser a p", "login a p", "logout", "logout"])),
        ("whoami not logged in", script(&["whoami"])),
        (
            "whoami negative level",
            script(&["adduser a p -42", "login a p", "whoami", "status"]),
        ),
        ("listusers empty", script(&["listusers"])),
        ("users alias empty", script(&["users"])),
        (
            "listusers with logged in marker",
            script(&["adduser a p 1", "adduser b p 2", "login b p", "listusers", "users"]),
        ),
    ]);
}

// ---------------------------------------------------------------------------
// File management
// ---------------------------------------------------------------------------

#[test]
fn createfile_branches() {
    assert_all(&[
        // The login check comes before the argument check.
        ("createfile not logged in", script(&["createfile f"])),
        ("createfile no args not logged in", script(&["createfile"])),
        ("touch alias not logged in", script(&["touch f"])),
        (
            "createfile no args logged in",
            script(&["adduser a p", "login a p", "createfile"]),
        ),
        (
            "createfile without content",
            script(&["adduser a p", "login a p", "createfile f", "readfile f", "listfiles"]),
        ),
        (
            "createfile with content",
            script(&["adduser a p", "login a p", "createfile f hello", "readfile f"]),
        ),
        (
            "createfile extra args ignored",
            script(&["adduser a p", "login a p", "createfile f one two three", "readfile f"]),
        ),
        (
            "createfile duplicate",
            script(&["adduser a p", "login a p", "createfile f x", "createfile f y", "readfile f"]),
        ),
        (
            "touch alias",
            script(&["adduser a p", "login a p", "touch f body", "listfiles"]),
        ),
    ]);
}

#[test]
fn createfile_capacity() {
    // MAX_FILES == 20.
    let mut lines = vec!["adduser a p 1".to_string(), "login a p".to_string()];
    for i in 1..=22 {
        lines.push(format!("createfile f{i} c{i}"));
    }
    lines.push("listfiles".to_string());
    lines.push("status".to_string());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    assert_same("max files", &script(&refs));
}

#[test]
fn readfile_branches() {
    assert_all(&[
        // readfile needs no login at all.
        ("readfile no args", script(&["readfile"])),
        ("cat alias no args", script(&["cat"])),
        ("readfile empty store", script(&["readfile f"])),
        ("readfile missing", script(&["adduser a p", "login a p", "createfile g", "readfile f"])),
        (
            "readfile without login",
            script(&["adduser a p", "login a p", "createfile f body", "logout", "readfile f"]),
        ),
        (
            "cat alias",
            script(&["adduser a p", "login a p", "createfile f body", "cat f"]),
        ),
        (
            "readfile empty content",
            script(&["adduser a p", "login a p", "createfile f", "cat f"]),
        ),
    ]);
}

#[test]
fn writefile_branches() {
    assert_all(&[
        ("writefile not logged in", script(&["writefile f x"])),
        ("write alias not logged in", script(&["write f x"])),
        ("writefile no args", script(&["adduser a p", "login a p", "writefile"])),
        ("writefile one arg", script(&["adduser a p", "login a p", "writefile f"])),
        (
            "writefile missing file",
            script(&["adduser a p", "login a p", "writefile nope x"]),
        ),
        (
            "writefile as owner",
            script(&["adduser a p", "login a p", "createfile f old", "writefile f new", "cat f"]),
        ),
        // Non-owner with level < 5 is denied; level >= 5 is allowed.
        (
            "writefile denied",
            script(&[
                "adduser owner p 1",
                "adduser low p 4",
                "login owner p",
                "createfile f mine",
                "logout",
                "login low p",
                "writefile f hack",
                "cat f",
            ]),
        ),
        (
            "writefile allowed at level 5",
            script(&[
                "adduser owner p 1",
                "adduser mid p 5",
                "login owner p",
                "createfile f mine",
                "logout",
                "login mid p",
                "writefile f taken",
                "cat f",
            ]),
        ),
        (
            "write alias",
            script(&["adduser a p", "login a p", "createfile f old", "write f new", "cat f"]),
        ),
        (
            "writefile extra args ignored",
            script(&["adduser a p", "login a p", "createfile f", "writefile f one two", "cat f"]),
        ),
    ]);
}

#[test]
fn deletefile_branches() {
    assert_all(&[
        ("deletefile not logged in", script(&["deletefile f"])),
        ("rm alias not logged in", script(&["rm f"])),
        ("deletefile no args", script(&["adduser a p", "login a p", "deletefile"])),
        (
            "deletefile missing",
            script(&["adduser a p", "login a p", "deletefile nope"]),
        ),
        (
            "deletefile as owner",
            script(&["adduser a p", "login a p", "createfile f", "deletefile f", "listfiles"]),
        ),
        // Non-owner needs level >= 9 (note: 5 is enough to write but not to delete).
        (
            "deletefile denied at level 5",
            script(&[
                "adduser owner p 1",
                "adduser mid p 5",
                "login owner p",
                "createfile f mine",
                "logout",
                "login mid p",
                "deletefile f",
                "listfiles",
            ]),
        ),
        (
            "deletefile allowed at level 9",
            script(&[
                "adduser owner p 1",
                "adduser high p 9",
                "login owner p",
                "createfile f mine",
                "logout",
                "login high p",
                "deletefile f",
                "listfiles",
            ]),
        ),
        // The shift loop: delete first, middle and last of three.
        (
            "delete first shifts",
            script(&[
                "adduser a p",
                "login a p",
                "createfile f1 c1",
                "createfile f2 c2",
                "createfile f3 c3",
                "deletefile f1",
                "listfiles",
                "cat f2",
                "cat f3",
            ]),
        ),
        (
            "delete middle shifts",
            script(&[
                "adduser a p",
                "login a p",
                "createfile f1 c1",
                "createfile f2 c2",
                "createfile f3 c3",
                "deletefile f2",
                "listfiles",
            ]),
        ),
        (
            "delete last no shift",
            script(&[
                "adduser a p",
                "login a p",
                "createfile f1 c1",
                "createfile f2 c2",
                "deletefile f2",
                "listfiles",
            ]),
        ),
        (
            "delete all then recreate",
            script(&[
                "adduser a p",
                "login a p",
                "createfile f1 c1",
                "deletefile f1",
                "listfiles",
                "createfile f1 again",
                "cat f1",
            ]),
        ),
        ("listfiles empty", script(&["listfiles"])),
        ("ls alias empty", script(&["ls"])),
        (
            "ls alias populated",
            script(&["adduser a p", "login a p", "createfile f x", "ls"]),
        ),
    ]);
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

#[test]
fn variable_branches() {
    assert_all(&[
        ("set no args", script(&["set"])),
        ("set one arg", script(&["set x"])),
        ("set new", script(&["set x 1", "listvars"])),
        ("set update", script(&["set x 1", "set x 2", "get x", "listvars"])),
        ("set extra args ignored", script(&["set x 1 2 3", "get x"])),
        ("get no args", script(&["get"])),
        ("get missing", script(&["get x"])),
        ("get after unset", script(&["set x 1", "unset x", "get x"])),
        ("unset no args", script(&["unset"])),
        ("unset missing", script(&["unset x"])),
        (
            "unset shifts",
            script(&["set a 1", "set b 2", "set c 3", "unset a", "listvars", "get b", "get c"]),
        ),
        (
            "unset middle",
            script(&["set a 1", "set b 2", "set c 3", "unset b", "listvars"]),
        ),
        (
            "unset last",
            script(&["set a 1", "set b 2", "unset b", "listvars"]),
        ),
        ("listvars empty", script(&["listvars"])),
        ("vars alias empty", script(&["vars"])),
        ("vars alias populated", script(&["set a 1", "vars"])),
    ]);
}

#[test]
fn variable_capacity() {
    // MAX_VARIABLES == 20; the existing-variable scan runs before the limit
    // check, so updating an existing name still works when full.
    let mut lines: Vec<String> = (1..=22).map(|i| format!("set v{i} val{i}")).collect();
    lines.push("set v1 updated".to_string());
    lines.push("listvars".to_string());
    lines.push("status".to_string());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    assert_same("max variables", &script(&refs));
}

// ---------------------------------------------------------------------------
// String comparison commands
// ---------------------------------------------------------------------------

#[test]
fn compare_branches() {
    assert_all(&[
        ("compare no args", script(&["compare"])),
        ("compare one arg", script(&["compare a"])),
        ("cmp alias no args", script(&["cmp"])),
        ("compare equal", script(&["compare abc abc"])),
        ("compare less", script(&["compare a b"])),
        ("compare greater", script(&["compare b a"])),
        // The printed value is the raw strcmp difference of unsigned chars.
        (
            "compare difference magnitudes",
            script(&[
                "compare a b",
                "compare b a",
                "compare a z",
                "compare z a",
                "compare A a",
                "compare a A",
                "compare abc abd",
                "compare abd abc",
                "compare ab abc",
                "compare abc ab",
                "cmp Z a",
                "cmp ~ !",
            ]),
        ),
        ("compare extra args ignored", script(&["compare a b c d"])),
    ]);
}

#[test]
fn compare_n_branches() {
    assert_all(&[
        ("compareN no args", script(&["compareN"])),
        ("compareN one arg", script(&["compareN a"])),
        ("compareN two args", script(&["compareN a b"])),
        ("cmpn alias short", script(&["cmpn a b"])),
        ("compareN n zero", script(&["compareN abc xyz 0"])),
        ("compareN equal prefix", script(&["compareN abc abd 2"])),
        ("compareN differing", script(&["compareN abc abd 3"])),
        ("compareN reversed", script(&["compareN abd abc 3"])),
        ("compareN n beyond length", script(&["compareN abc abc 99"])),
        // A negative n converts to a huge size_t, so strncmp compares fully.
        ("compareN negative n", script(&["compareN abc abd -1"])),
        ("compareN negative n equal", script(&["compareN abc abc -5"])),
        ("compareN n not a number", script(&["compareN abc abd xyz"])),
        ("compareN n int min", script(&["compareN abc abd -2147483648"])),
        ("compareN n saturating", script(&["compareN abc abd 99999999999999999999"])),
        ("cmpn alias", script(&["cmpn hello help 3", "cmpn hello help 4"])),
    ]);
}

#[test]
fn startswith_branches() {
    assert_all(&[
        ("startswith no args", script(&["startswith"])),
        ("startswith one arg", script(&["startswith hello"])),
        ("startswith yes", script(&["startswith hello he"])),
        ("startswith no", script(&["startswith hello xe"])),
        ("startswith equal", script(&["startswith hello hello"])),
        ("prefix longer than string", script(&["startswith he hello"])),
        ("startswith extra args", script(&["startswith hello he llo"])),
    ]);
}

#[test]
fn match_branches() {
    assert_all(&[
        ("match no args", script(&["match"])),
        ("match one arg", script(&["match pattern"])),
        ("match exact", script(&["match ab ab"])),
        ("match contains", script(&["match ab xaby"])),
        ("match none", script(&["match ab xyz"])),
        (
            "match mixture",
            script(&["match ab ab xaby xyz abc bab"]),
        ),
        // strstr with the pattern longer than the candidate.
        ("match pattern longer", script(&["match abcdef ab"])),
        ("match single char", script(&["match a a b aa ba"])),
    ]);
}

// ---------------------------------------------------------------------------
// System commands
// ---------------------------------------------------------------------------

#[test]
fn debug_and_verbose_branches() {
    assert_all(&[
        ("debug query off", script(&["debug"])),
        ("debug on", script(&["debug on", "debug", "status"])),
        ("debug off again", script(&["debug on", "debug off", "debug", "status"])),
        ("debug bad arg", script(&["debug maybe"])),
        ("debug bad arg keeps state", script(&["debug on", "debug maybe", "debug"])),
        ("debug case sensitive", script(&["debug ON", "debug OFF", "debug"])),
        // The [DEBUG] line is printed for every command once enabled.
        (
            "debug echo",
            script(&["debug on", "status", "compare a b", "adduser a p 1", "bogus", ""]),
        ),
        ("verbose query off", script(&["verbose"])),
        ("verbose on", script(&["verbose on", "verbose", "status"])),
        ("verbose off again", script(&["verbose on", "verbose off", "verbose"])),
        ("verbose bad arg", script(&["verbose maybe"])),
        // The [VERBOSE] line echoes the raw line, before parsing, including
        // blank lines and unknown commands.
        (
            "verbose echo",
            script(&["verbose on", "  spaced  out  ", "", "   ", "bogus", "status"]),
        ),
        ("debug and verbose together", script(&["debug on", "verbose on", "status", "x"])),
        ("verbose extra args", script(&["verbose on off"])),
        ("debug extra args", script(&["debug on off"])),
    ]);
}

#[test]
fn status_and_help_branches() {
    assert_all(&[
        ("status initial", script(&["status"])),
        (
            "status populated",
            script(&[
                "adduser a p 3",
                "login a p",
                "createfile f x",
                "set v 1",
                "debug on",
                "verbose on",
                "status",
            ]),
        ),
        (
            "status after logout keeps user",
            script(&["adduser a p", "login a p", "logout", "status"]),
        ),
        ("help", script(&["help"])),
        ("question mark alias", script(&["?"])),
        ("help twice", script(&["help", "?"])),
    ]);
}

#[test]
fn exit_branches() {
    assert_all(&[
        // exit(0) happens immediately; the remaining input is never read.
        ("exit", script(&["exit", "status"])),
        ("quit", script(&["quit", "status"])),
        ("exit with args", script(&["exit now please", "status"])),
        ("exit after work", script(&["adduser a p", "set v 1", "exit"])),
        ("exit with debug on", script(&["debug on", "exit"])),
        ("exit with verbose on", script(&["verbose on", "exit"])),
    ]);
}

// ---------------------------------------------------------------------------
// Dispatch: prefix suggestions and unknown commands
// ---------------------------------------------------------------------------

#[test]
fn prefix_suggestions_and_unknown() {
    assert_all(&[
        // Exact matches win over the strncmp prefix branches.
        (
            "exact wins over prefix",
            script(&[
                "adduser", "login", "logout", "listusers", "listfiles", "listvars", "createfile",
                "readfile", "writefile", "deletefile",
            ]),
        ),
        // strncmp(command, "...", n) == 0 suggestions.
        ("add prefix", script(&["add", "addu", "adduse", "addxyz"])),
        ("log prefix", script(&["log", "logi", "logo", "logxyz"])),
        ("list prefix", script(&["list", "listx", "listuser"])),
        ("create prefix", script(&["create", "createx", "createfil"])),
        ("read prefix", script(&["read", "readx", "readfil"])),
        ("write prefix", script(&["write2", "writex", "writefil"])),
        ("delete prefix", script(&["delete", "deletex", "deletefil"])),
        // Prefixes are checked in source order: "add" before "log" etc.
        ("shorter than prefix", script(&["ad", "lo", "lis", "creat", "rea", "writ", "delet"])),
        // Unknown commands.
        ("unknown", script(&["bogus", "Bogus", "BOGUS", "zzz", "x"])),
        ("unknown punctuation", script(&["!", "@", "-", "--help", "42"])),
        // Case sensitivity of every command name.
        ("case sensitive", script(&["HELP", "Status", "ADDUSER", "compareN", "comparen"])),
        // Aliases.
        (
            "all aliases",
            script(&["users", "vars", "ls", "cmp a b", "cmpn a b 1", "cat f", "rm f", "touch f"]),
        ),
    ]);
}

// ---------------------------------------------------------------------------
// Byte-level input handling
// ---------------------------------------------------------------------------

#[test]
fn non_ascii_and_high_bytes() {
    assert_all(&[
        ("utf8 args", "compare é é\nmatch é aéb\n".as_bytes().to_vec()),
        ("high bytes", b"compare \xff \x80\nmatch \x80 \x80\x81 z\n".to_vec()),
        // strcmp compares as unsigned char, so 0x80 > 0x7f.
        ("signedness of comparison", b"compare \x80 \x7f\ncompare \x7f \x80\n".to_vec()),
        ("del byte", b"compare \x7f a\n".to_vec()),
        ("high byte command", b"\xff\xfe\n".to_vec()),
        ("vertical tab is not a separator", b"set a\x0bb c\nlistvars\n".to_vec()),
        ("form feed is not a separator", b"compare a\x0cb c\n".to_vec()),
    ]);
}

// ---------------------------------------------------------------------------
// Fixed-buffer strcpy overruns inside the record structs
// ---------------------------------------------------------------------------

#[test]
fn field_overruns_within_structs() {
    // A token can be up to 63 bytes, but user_t::name and variable_t::name are
    // 32 bytes, so strcpy spills into the neighbouring field.
    let n40 = "N".repeat(40);
    let p40 = "P".repeat(40);
    let n63 = "A".repeat(63);
    let p63 = "B".repeat(63);
    assert_all(&[
        (
            "name overruns into password",
            script(&[
                &format!("adduser {n40} {p40} 7"),
                "listusers",
                "status",
            ]),
        ),
        (
            "name and password both maximal",
            script(&[&format!("adduser {n63} {p63} 5"), "listusers", "status"]),
        ),
        (
            "login with the overrun name",
            script(&[
                &format!("adduser {n40} short 7"),
                "listusers",
                // The stored name string is name[0..32] followed by the
                // password, so this is what login must be given.
                &format!("login {}short short", "N".repeat(32)),
                "whoami",
                "status",
            ]),
        ),
        (
            "variable name overruns into value",
            script(&[
                &format!("set {n40} {}", "W".repeat(63)),
                "listvars",
                &format!("get {n40}"),
                &format!("get {}{}", "N".repeat(32), "W".repeat(63)),
            ]),
        ),
        (
            "file name and content maximal",
            script(&[
                "adduser a p 1",
                "login a p",
                &format!("createfile {} {}", "F".repeat(63), "C".repeat(63)),
                "listfiles",
                &format!("readfile {}", "F".repeat(63)),
            ]),
        ),
        (
            "long owner name into file owner",
            script(&[
                &format!("adduser {n40} short 1"),
                &format!("login {}short short", "N".repeat(32)),
                "createfile f body",
                "listfiles",
                "readfile f",
            ]),
        ),
    ]);
}

// ---------------------------------------------------------------------------
// Overruns that walk off the end of an array and hit the following globals
// ---------------------------------------------------------------------------

/// `users` is immediately followed by `user_count` and `current_user` in
/// `.bss`, so a long password stored into `users[9]` corrupts them.  The C code
/// reloads `user_count` on the next statement, so the effect is visible - and
/// past a certain length the following store is wild and the process dies.
#[test]
fn users_array_overrun_corrupts_following_globals() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for len in [
        30usize, 35, 38, 39, // still inside the struct
        40, 41, // clobbers user_count with a still-usable value
        42, 43, 44, 47, 48, 55, 63, // clobbers it with a wild value
    ] {
        let mut lines: Vec<String> = (0..9).map(|i| format!("adduser u{i} p{i} {i}")).collect();
        lines.push(format!("adduser LAST {} 3", "P".repeat(len)));
        lines.push("status".to_string());
        lines.push("listusers".to_string());
        lines.push("whoami".to_string());
        lines.push("adduser extra pw 1".to_string());
        lines.push("status".to_string());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        cases.push((format!("users[9].password len {len}"), script(&refs)));
    }
    let borrowed: Vec<(&str, Vec<u8>)> = cases
        .iter()
        .map(|(n, i)| (n.as_str(), i.clone()))
        .collect();
    assert_all(&borrowed);
}

/// Every `current_user && current_user->logged_in` test in the C has a third
/// outcome - a non-NULL `current_user` pointing at a record whose `logged_in`
/// is 0 - that `cmd_logout` alone can never produce, because it always NULLs the
/// pointer as well.  It becomes reachable through the `user_count` corruption:
/// with `user_count` reset to 0, `cmd_adduser`'s `users[user_count].logged_in = 0`
/// clears the flag on the record `current_user` still points at.
#[test]
fn dangling_current_user_with_cleared_flag() {
    let mut lines = vec![
        "adduser a p 1".to_string(),
        "login a p".to_string(),
        "whoami".to_string(),
    ];
    for i in 1..9 {
        lines.push(format!("adduser u{i} p{i} {i}"));
    }
    lines.push("status".to_string());
    // A 40-byte password overruns users[9].password by exactly enough to zero
    // the low byte of user_count, so the following field stores land on
    // users[0] - the record current_user points at.
    lines.push(format!("adduser LAST {} 3", "P".repeat(40)));
    // All of these now take the "pointer set but flag clear" path.
    for probe in [
        "status",
        "whoami",
        "logout",
        "createfile f x",
        "writefile f y",
        "deletefile f",
        "readfile f",
        "listusers",
        "listfiles",
    ] {
        lines.push(probe.to_string());
    }
    // login must also see the guard as false and go on to search the table.
    lines.push("login a p".to_string());
    lines.push("whoami".to_string());
    lines.push("status".to_string());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    assert_same("dangling current_user", &script(&refs));
}

/// `files` is immediately followed by `file_count`.  `createfile` copies
/// `current_user->name` into the 32-byte `owner` field, so a user whose stored
/// name string is long enough overruns `files[19]` into `file_count`.
#[test]
fn files_array_overrun_corrupts_file_count() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // A 32-byte name field with no room for the terminator means the stored
    // name string continues into the password, giving a name of length
    // 32 + password_len.
    for pw_len in [1usize, 2, 3, 4, 5, 6, 8, 12] {
        let name = "N".repeat(32);
        let pw = "p".repeat(pw_len);
        let effective_name = format!("{name}{pw}");
        let mut lines = vec![
            format!("adduser {name} {pw} 1"),
            format!("login {effective_name} {pw}"),
            "whoami".to_string(),
        ];
        for i in 1..=20 {
            lines.push(format!("createfile f{i} c{i}"));
        }
        lines.push("status".to_string());
        lines.push("listfiles".to_string());
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        cases.push((format!("files[19].owner via password len {pw_len}"), script(&refs)));
    }
    let borrowed: Vec<(&str, Vec<u8>)> = cases
        .iter()
        .map(|(n, i)| (n.as_str(), i.clone()))
        .collect();
    assert_all(&borrowed);
}

/// The last `variables` element cannot reach past the array (32 + 63 + 1 fits
/// inside the 160-byte struct), so a full table with maximal names and values
/// must stay well behaved.
#[test]
fn variables_array_maximal_entries() {
    let mut lines: Vec<String> = Vec::new();
    for i in 0..20 {
        let name = format!("{}{i:02}", "V".repeat(30));
        let value = format!("{}{i:02}", "W".repeat(61));
        lines.push(format!("set {name} {value}"));
    }
    lines.push("listvars".to_string());
    lines.push("status".to_string());
    lines.push(format!("get {}00", "V".repeat(30)));
    lines.push(format!("unset {}00", "V".repeat(30)));
    lines.push("listvars".to_string());
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    assert_same("maximal variables", &script(&refs));
}

// ---------------------------------------------------------------------------
// Longer mixed sessions
// ---------------------------------------------------------------------------

#[test]
fn long_mixed_session() {
    let lines: Vec<&str> = vec![
        "help",
        "status",
        "adduser alice secret",
        "adduser bob pw 5",
        "adduser carol pw 9",
        "adduser dave pw abc",
        "listusers",
        "login alice secret",
        "whoami",
        "createfile notes hello",
        "createfile notes again",
        "readfile notes",
        "writefile notes updated",
        "readfile notes",
        "listfiles",
        "logout",
        "login bob pw",
        "writefile notes bobwrote",
        "readfile notes",
        "deletefile notes",
        "logout",
        "login carol pw",
        "deletefile notes",
        "listfiles",
        "set greeting hi",
        "get greeting",
        "set greeting bye",
        "listvars",
        "unset greeting",
        "listvars",
        "compare alpha beta",
        "compareN alpha alpine 2",
        "startswith alphabet alpha",
        "match al alpha beta alps",
        "debug on",
        "status",
        "verbose on",
        "status",
        "debug off",
        "verbose off",
        "status",
        "exit",
    ];
    assert_same("long mixed session", &script(&lines));
}

/// Output large enough to cross glibc's stdio buffer several times, so that the
/// flushing boundaries are exercised too.
#[test]
fn large_output_volume() {
    let mut lines: Vec<String> = Vec::new();
    for _ in 0..40 {
        lines.push("help".to_string());
        lines.push("status".to_string());
    }
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    assert_same("large output", &script(&refs));
}
