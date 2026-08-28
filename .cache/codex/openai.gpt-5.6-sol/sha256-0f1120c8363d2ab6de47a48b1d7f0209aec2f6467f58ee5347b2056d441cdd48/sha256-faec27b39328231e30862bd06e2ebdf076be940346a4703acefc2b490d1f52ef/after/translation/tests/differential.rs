use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn c_binary() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/driver")
}

fn run(binary: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));

    child
        .stdin
        .take()
        .expect("child stdin was not piped")
        .write_all(input)
        .expect("failed to write child stdin");
    child.wait_with_output().expect("failed to wait for child")
}

fn assert_matches(input: &[u8]) {
    let c = run(&c_binary(), input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_driver")), input);

    assert_eq!(
        rust.status,
        c.status,
        "exit status differs for input {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        rust.stdout,
        c.stdout,
        "stdout differs for input {:?}\nC:\n{}\nRust:\n{}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&rust.stdout)
    );
    assert_eq!(
        rust.stderr,
        c.stderr,
        "stderr differs for input {:?}\nC:\n{}\nRust:\n{}",
        String::from_utf8_lossy(input),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
}

#[test]
fn eof_empty_and_blank_input() {
    assert_matches(b"");
    assert_matches(b"\n \t \nstatus\n\n\t\n");
}

#[test]
fn user_management_branches() {
    assert_matches(
        b"adduser\n\
          adduser only-name\n\
          login\n\
          login missing password\n\
          whoami\n\
          logout\n\
          listusers\n\
          login nobody password\n\
          adduser alice secret\n\
          adduser alice other 9\n\
          adduser bob swordfish invalid\n\
          login alice wrong\n\
          login alice secret\n\
          login alice\n\
          login bob swordfish\n\
          whoami\n\
          listusers\n\
          logout\n\
          logout\n\
          login bob swordfish\n\
          whoami\n\
          logout\n",
    );
}

#[test]
fn maximum_users_and_validation_order() {
    let mut input = String::new();
    for index in 0..10 {
        input.push_str(&format!("adduser u{index} p{index} {index}\n"));
    }
    input.push_str("adduser overflow password\n");
    input.push_str("adduser u0 p0\n");
    input.push_str("listusers\n");
    assert_matches(input.as_bytes());
}

#[test]
fn file_usage_lookup_and_permissions() {
    assert_matches(
        b"createfile\n\
          writefile\n\
          deletefile\n\
          readfile\n\
          readfile absent\n\
          listfiles\n\
          adduser owner pass 1\n\
          adduser writer pass 5\n\
          adduser admin pass 9\n\
          login owner pass\n\
          createfile\n\
          createfile empty\n\
          createfile note original\n\
          createfile note duplicate\n\
          status\n\
          readfile\n\
          readfile absent\n\
          readfile empty\n\
          readfile note\n\
          writefile\n\
          writefile absent value\n\
          writefile note changed\n\
          readfile note\n\
          deletefile\n\
          deletefile absent\n\
          listfiles\n\
          logout\n\
          login writer pass\n\
          writefile note elevated\n\
          deletefile note\n\
          logout\n\
          login admin pass\n\
          deletefile note\n\
          logout\n\
          login owner pass\n\
          deletefile empty\n\
          listfiles\n\
          logout\n",
    );
}

#[test]
fn maximum_files_and_shift_after_delete() {
    let mut input = String::from("adduser owner pass\nlogin owner pass\n");
    for index in 0..20 {
        input.push_str(&format!("createfile f{index} c{index}\n"));
    }
    input.push_str("createfile overflow value\n");
    input.push_str("createfile f0 duplicate\n");
    input.push_str("deletefile f10\n");
    input.push_str("readfile f11\n");
    input.push_str("createfile replacement value\n");
    input.push_str("listfiles\n");
    assert_matches(input.as_bytes());
}

#[test]
fn variable_usage_lookup_update_and_shift() {
    assert_matches(
        b"set\n\
          set only-name\n\
          get\n\
          get absent\n\
          unset\n\
          unset absent\n\
          listvars\n\
          set alpha one\n\
          set beta two\n\
          set alpha updated\n\
          get alpha\n\
          listvars\n\
          unset alpha\n\
          get alpha\n\
          get beta\n\
          listvars\n\
          unset beta\n\
          listvars\n",
    );
}

#[test]
fn maximum_variables_and_update_before_capacity_error() {
    let mut input = String::new();
    for index in 0..20 {
        input.push_str(&format!("set v{index} value{index}\n"));
    }
    input.push_str("set v0 replacement\n");
    input.push_str("set overflow value\n");
    input.push_str("unset v10\n");
    input.push_str("get v11\n");
    input.push_str("set replacement value\n");
    input.push_str("listvars\n");
    assert_matches(input.as_bytes());
}

#[test]
fn comparison_and_matching_branches() {
    assert_matches(
        b"compare\n\
          compare one\n\
          compare same same\n\
          compare alpha beta\n\
          compare zeta beta\n\
          compareN\n\
          compareN one two\n\
          compareN prefix prelude 0\n\
          compareN prefix prelude 3\n\
          compareN alpha beta 5\n\
          compareN zeta beta 5\n\
          compareN alpha beta invalid\n\
          compareN alpha beta -1\n\
          startswith\n\
          startswith value\n\
          startswith prefix pre\n\
          startswith pre prefix\n\
          match\n\
          match needle\n\
          match needle needle hayneedle haystack\n\
          match x x ax no x bx cx dx ex fx ignored\n",
    );
}

#[test]
fn modes_status_and_debug_verbose_ordering() {
    assert_matches(
        b"debug\n\
          debug invalid\n\
          debug on\n\
          debug\n\
          status\n\
          verbose\n\
          verbose invalid\n\
          verbose on\n\
          status\n\
          \n\
          verbose off\n\
          verbose\n\
          debug off\n\
          debug\n\
          status\n",
    );
}

#[test]
fn help_aliases_and_command_aliases() {
    assert_matches(
        b"?\n\
          users\n\
          touch\n\
          cat absent\n\
          write\n\
          rm\n\
          ls\n\
          vars\n\
          cmp a b\n\
          cmpn abc abd 3\n\
          quit\n\
          status\n",
    );
}

#[test]
fn partial_command_suggestions_and_unknowns() {
    assert_matches(
        b"addition\n\
          logarithm\n\
          listing\n\
          create-other\n\
          reader\n\
          writer\n\
          delete-other\n\
          ad\n\
          unknown\n",
    );
}

#[test]
fn parser_limits_binary_bytes_and_line_chunking() {
    let mut input = Vec::new();
    input.extend_from_slice(b"\tcompare\tone\tone\tignored\r\n");
    input.extend_from_slice(b"status\r\n");
    input.extend_from_slice(b"compare ");
    input.extend(std::iter::repeat_n(b'a', 80));
    input.push(b' ');
    input.extend(std::iter::repeat_n(b'a', 80));
    input.push(b'\n');
    input.extend_from_slice(b"compare ");
    input.push(0xff);
    input.push(b' ');
    input.push(0x01);
    input.push(b'\n');
    input.extend_from_slice(b"status\0ignored\n");
    input.extend(std::iter::repeat_n(b'x', 300));
    assert_matches(&input);
}

#[test]
fn numeric_and_fixed_field_boundaries() {
    let long_user = "u".repeat(40);
    let long_variable = "v".repeat(40);
    let max_filename = "f".repeat(63);
    let max_content = "c".repeat(63);
    let input = format!(
        "adduser bounded pass 2147483648\n\
         login bounded pass\n\
         createfile {max_filename} {max_content}\n\
         readfile {max_filename}\n\
         status\n\
         logout\n\
         adduser {long_user} p\n\
         listusers\n\
         login {long_user} p\n\
         set {long_variable} value\n\
         listvars\n\
         get {long_variable}\n"
    );
    assert_matches(input.as_bytes());
}

#[test]
fn help_command() {
    assert_matches(b"help\n");
}

#[test]
fn exit_stops_without_another_prompt() {
    assert_matches(b"exit\nstatus\n");
}

#[test]
fn current_time_format_and_value() {
    let subsecond = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock predates the Unix epoch")
        .subsec_millis();
    if subsecond > 100 {
        thread::sleep(Duration::from_millis(u64::from(1_050 - subsecond)));
    }
    assert_matches(b"time\n");
}
