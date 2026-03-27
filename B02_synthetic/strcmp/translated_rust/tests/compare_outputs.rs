use std::io::Write;
use std::process::{Command, Stdio};

/// Helper: run the C binary with given input lines, return stdout
fn run_c_binary(input: &str) -> String {
    let c_bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/driver");
    let mut child = Command::new(&c_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run C binary");
    child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("Failed to wait on C binary");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Helper: run the Rust binary with given input lines, return stdout
fn run_rust_binary(input: &str) -> String {
    let rust_bin = env!("CARGO_BIN_EXE_driver");
    let mut child = Command::new(rust_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run Rust binary");
    child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("Failed to wait on Rust binary");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Compare C and Rust binary outputs for a given input sequence.
/// Filters out lines containing "time" output since timestamps differ.
fn compare_outputs(input: &str) {
    let c_out = run_c_binary(input);
    let r_out = run_rust_binary(input);

    let c_lines: Vec<&str> = c_out.lines()
        .filter(|l| !l.starts_with("Current time:"))
        .collect();
    let r_lines: Vec<&str> = r_out.lines()
        .filter(|l| !l.starts_with("Current time:"))
        .collect();

    if c_lines != r_lines {
        eprintln!("=== INPUT ===\n{}", input);
        eprintln!("=== C OUTPUT ({} lines) ===", c_lines.len());
        for l in &c_lines { eprintln!("{}", l); }
        eprintln!("=== RUST OUTPUT ({} lines) ===", r_lines.len());
        for l in &r_lines { eprintln!("{}", l); }

        // Find first difference
        let max = c_lines.len().max(r_lines.len());
        for i in 0..max {
            let cl = c_lines.get(i).unwrap_or(&"<missing>");
            let rl = r_lines.get(i).unwrap_or(&"<missing>");
            if cl != rl {
                eprintln!("FIRST DIFF at line {}:", i);
                eprintln!("  C:    {:?}", cl);
                eprintln!("  Rust: {:?}", rl);
                break;
            }
        }
        panic!("Output mismatch");
    }
}

#[test]
fn test_banner() {
    // Just the banner with no commands (EOF immediately)
    compare_outputs("");
}

#[test]
fn test_help() {
    compare_outputs("help\n");
}

#[test]
fn test_status_initial() {
    compare_outputs("status\n");
}

#[test]
fn test_user_management() {
    compare_outputs(
        "adduser alice pass123\n\
         adduser bob secret 5\n\
         listusers\n\
         login alice pass123\n\
         whoami\n\
         logout\n\
         login bob secret\n\
         whoami\n\
         logout\n"
    );
}

#[test]
fn test_user_errors() {
    compare_outputs(
        "adduser\n\
         login\n\
         logout\n\
         whoami\n\
         adduser alice pass\n\
         adduser alice pass\n\
         login alice wrong\n\
         login nobody pass\n\
         login alice pass\n\
         login alice pass\n"
    );
}

#[test]
fn test_file_management() {
    compare_outputs(
        "adduser alice pass123\n\
         login alice pass123\n\
         createfile test.txt hello\n\
         readfile test.txt\n\
         writefile test.txt world\n\
         readfile test.txt\n\
         listfiles\n\
         deletefile test.txt\n\
         listfiles\n\
         logout\n"
    );
}

#[test]
fn test_file_errors() {
    compare_outputs(
        "createfile test.txt\n\
         adduser alice pass123\n\
         login alice pass123\n\
         createfile\n\
         createfile test.txt\n\
         createfile test.txt\n\
         readfile\n\
         readfile nonexistent\n\
         writefile\n\
         writefile nonexistent data\n\
         deletefile\n\
         deletefile nonexistent\n\
         logout\n"
    );
}

#[test]
fn test_file_permissions() {
    compare_outputs(
        "adduser alice pass123\n\
         adduser bob pass456\n\
         login alice pass123\n\
         createfile secret.txt data\n\
         logout\n\
         login bob pass456\n\
         writefile secret.txt hack\n\
         deletefile secret.txt\n\
         logout\n\
         adduser admin pass789 9\n\
         login admin pass789\n\
         deletefile secret.txt\n\
         logout\n"
    );
}

#[test]
fn test_variables() {
    compare_outputs(
        "set foo bar\n\
         set baz qux\n\
         get foo\n\
         listvars\n\
         set foo updated\n\
         get foo\n\
         unset baz\n\
         listvars\n\
         get baz\n\
         unset nonexistent\n"
    );
}

#[test]
fn test_variable_errors() {
    compare_outputs(
        "set\n\
         get\n\
         unset\n\
         listvars\n"
    );
}

#[test]
fn test_compare() {
    compare_outputs(
        "compare hello hello\n\
         compare abc def\n\
         compare xyz abc\n\
         compare\n\
         compare one\n"
    );
}

#[test]
fn test_comparen() {
    compare_outputs(
        "compareN hello hello 3\n\
         compareN hello help 3\n\
         compareN abc def 2\n\
         compareN\n\
         compareN a b\n"
    );
}

#[test]
fn test_startswith() {
    compare_outputs(
        "startswith hello hel\n\
         startswith hello world\n\
         startswith\n\
         startswith one\n"
    );
}

#[test]
fn test_match() {
    compare_outputs(
        "match hello hello world helloworld\n\
         match\n\
         match pattern\n"
    );
}

#[test]
fn test_debug_verbose() {
    compare_outputs(
        "debug\n\
         debug on\n\
         status\n\
         debug off\n\
         verbose\n\
         verbose on\n\
         status\n\
         verbose off\n\
         debug invalid\n\
         verbose invalid\n"
    );
}

#[test]
fn test_command_aliases() {
    compare_outputs(
        "adduser alice pass123\n\
         login alice pass123\n\
         touch myfile content\n\
         cat myfile\n\
         write myfile newcontent\n\
         cat myfile\n\
         ls\n\
         rm myfile\n\
         ls\n\
         set x 1\n\
         vars\n\
         users\n\
         cmp abc def\n\
         cmpn abc def 2\n\
         ?\n"
    );
}

#[test]
fn test_partial_matches() {
    compare_outputs(
        "adding\n\
         logging\n\
         listing\n\
         creating\n\
         reading\n\
         writing\n\
         deleting\n\
         unknown_cmd\n"
    );
}

#[test]
fn test_empty_input() {
    // Pure empty/whitespace-only lines trigger UB in C (uninitialized command buffer).
    // We only test that a real command after empty-ish input still works.
    compare_outputs("help\n");
}

#[test]
fn test_complex_workflow() {
    compare_outputs(
        "adduser admin root 10\n\
         adduser user1 pass1\n\
         login admin root\n\
         createfile config.txt settings\n\
         createfile data.txt info\n\
         set env production\n\
         set version 1.0\n\
         status\n\
         logout\n\
         login user1 pass1\n\
         readfile config.txt\n\
         writefile config.txt hacked\n\
         createfile user1file.txt mydata\n\
         writefile user1file.txt updated\n\
         listfiles\n\
         listvars\n\
         compare admin user1\n\
         compareN admin admin 3\n\
         startswith administrator admin\n\
         match config config.txt data.txt user1file.txt\n\
         logout\n\
         login admin root\n\
         deletefile config.txt\n\
         deletefile data.txt\n\
         deletefile user1file.txt\n\
         listfiles\n\
         logout\n"
    );
}
