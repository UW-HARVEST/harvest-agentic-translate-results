use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_long};
use std::process;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

#[derive(Clone)]
struct User {
    name: String,
    password: String,
    permission_level: i32,
    logged_in: bool,
}

#[derive(Clone)]
struct FileEntry {
    filename: String,
    content: String,
    owner: String,
    permissions: i32,
}

#[derive(Clone)]
struct Variable {
    name: String,
    value: String,
}

struct State {
    users: Vec<User>,
    current_user: Option<usize>,
    files: Vec<FileEntry>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

impl State {
    fn new() -> Self {
        Self {
            users: Vec::new(),
            current_user: None,
            files: Vec::new(),
            variables: Vec::new(),
            debug_mode: false,
            verbose_mode: false,
        }
    }

    fn current_user_logged_in(&self) -> Option<usize> {
        self.current_user
            .filter(|&idx| self.users.get(idx).map(|u| u.logged_in).unwrap_or(false))
    }
}

extern "C" {
    fn time(tloc: *mut c_long) -> c_long;
    fn ctime(timep: *const c_long) -> *mut c_char;
}

fn truncate_c_field(s: &str, max: usize) -> String {
    s.as_bytes()
        .iter()
        .copied()
        .take(max - 1)
        .map(char::from)
        .collect()
}

fn parse_command(input: &str) -> (String, Vec<String>) {
    let temp = truncate_c_field(input, MAX_INPUT);
    let mut tokens = temp.split(|ch| ch == ' ' || ch == '\t');
    let mut command = String::new();
    let mut args = Vec::new();

    for token in tokens.by_ref() {
        if !token.is_empty() {
            command = truncate_c_field(token, MAX_COMMAND);
            break;
        }
    }

    for token in tokens {
        if !token.is_empty() {
            if args.len() >= MAX_ARGS {
                break;
            }
            args.push(truncate_c_field(token, MAX_COMMAND));
        }
    }

    (command, args)
}

fn atoi_c(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
    {
        i += 1;
    }

    let mut sign = 1i64;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            sign = -1;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
        }
    }

    let mut value = 0i64;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }

    (value.wrapping_mul(sign)) as i32
}

fn strcmp_c(a: &str, b: &str) -> i32 {
    let aa = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    loop {
        let ca = aa.get(i).copied().unwrap_or(0);
        let cb = bb.get(i).copied().unwrap_or(0);
        if ca != cb || ca == 0 {
            return ca as i32 - cb as i32;
        }
        i += 1;
    }
}

fn strncmp_c(a: &str, b: &str, n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }

    let aa = a.as_bytes();
    let bb = b.as_bytes();
    for i in 0..(n as usize) {
        let ca = aa.get(i).copied().unwrap_or(0);
        let cb = bb.get(i).copied().unwrap_or(0);
        if ca != cb || ca == 0 {
            return ca as i32 - cb as i32;
        }
    }
    0
}

fn cmd_adduser(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        print!("Usage: adduser <username> <password> [permission_level]\n");
        return;
    }

    if state.users.len() >= MAX_USERS {
        print!("Error: Maximum users reached\n");
        return;
    }

    for user in &state.users {
        if user.name == args[0] {
            print!("Error: User '{}' already exists\n", args[0]);
            return;
        }
    }

    let permission_level = if args.len() >= 3 { atoi_c(&args[2]) } else { 1 };
    state.users.push(User {
        name: args[0].clone(),
        password: args[1].clone(),
        permission_level,
        logged_in: false,
    });

    print!(
        "User '{}' added with permission level {}\n",
        args[0], permission_level
    );
}

fn cmd_login(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        print!("Usage: login <username> <password>\n");
        return;
    }

    if let Some(idx) = state.current_user_logged_in() {
        print!(
            "Error: User '{}' already logged in. Use 'logout' first.\n",
            state.users[idx].name
        );
        return;
    }

    for i in 0..state.users.len() {
        if state.users[i].name == args[0] {
            if state.users[i].password == args[1] {
                state.users[i].logged_in = true;
                state.current_user = Some(i);
                print!("Login successful. Welcome, {}!\n", state.users[i].name);
                return;
            } else {
                print!("Error: Incorrect password\n");
                return;
            }
        }
    }

    print!("Error: User not found\n");
}

fn cmd_logout(state: &mut State) {
    let Some(idx) = state.current_user_logged_in() else {
        print!("Error: No user logged in\n");
        return;
    };

    print!("Goodbye, {}!\n", state.users[idx].name);
    state.users[idx].logged_in = false;
    state.current_user = None;
}

fn cmd_whoami(state: &State) {
    let Some(idx) = state.current_user_logged_in() else {
        print!("Not logged in\n");
        return;
    };

    print!("Current user: {}\n", state.users[idx].name);
    print!("Permission level: {}\n", state.users[idx].permission_level);
}

fn cmd_listusers(state: &State) {
    if state.users.is_empty() {
        print!("No users registered\n");
        return;
    }

    print!("Registered users:\n");
    for user in &state.users {
        print!(
            "  {} (level {}) {}\n",
            user.name,
            user.permission_level,
            if user.logged_in { "[logged in]" } else { "" }
        );
    }
}

fn cmd_createfile(state: &mut State, args: &[String]) {
    let Some(user_idx) = state.current_user_logged_in() else {
        print!("Error: Must be logged in\n");
        return;
    };

    if args.is_empty() {
        print!("Usage: createfile <filename> [content]\n");
        return;
    }

    if state.files.len() >= MAX_FILES {
        print!("Error: Maximum files reached\n");
        return;
    }

    for file in &state.files {
        if file.filename == args[0] {
            print!("Error: File '{}' already exists\n", args[0]);
            return;
        }
    }

    state.files.push(FileEntry {
        filename: args[0].clone(),
        content: if args.len() >= 2 {
            args[1].clone()
        } else {
            String::new()
        },
        owner: state.users[user_idx].name.clone(),
        permissions: 755,
    });

    print!("File '{}' created\n", args[0]);
}

fn cmd_readfile(state: &State, args: &[String]) {
    if args.is_empty() {
        print!("Usage: readfile <filename>\n");
        return;
    }

    for file in &state.files {
        if file.filename == args[0] {
            print!("=== {} ===\n", file.filename);
            print!("Owner: {}\n", file.owner);
            print!("Permissions: {}\n", file.permissions);
            print!("Content: {}\n", file.content);
            return;
        }
    }

    print!("Error: File '{}' not found\n", args[0]);
}

fn cmd_writefile(state: &mut State, args: &[String]) {
    let Some(user_idx) = state.current_user_logged_in() else {
        print!("Error: Must be logged in\n");
        return;
    };

    if args.len() < 2 {
        print!("Usage: writefile <filename> <content>\n");
        return;
    }

    for file in &mut state.files {
        if file.filename == args[0] {
            if file.owner == state.users[user_idx].name || state.users[user_idx].permission_level >= 5 {
                file.content = args[1].clone();
                print!("File '{}' updated\n", args[0]);
                return;
            } else {
                print!("Error: Permission denied\n");
                return;
            }
        }
    }

    print!("Error: File '{}' not found\n", args[0]);
}

fn cmd_deletefile(state: &mut State, args: &[String]) {
    let Some(user_idx) = state.current_user_logged_in() else {
        print!("Error: Must be logged in\n");
        return;
    };

    if args.is_empty() {
        print!("Usage: deletefile <filename>\n");
        return;
    }

    for i in 0..state.files.len() {
        if state.files[i].filename == args[0] {
            if state.files[i].owner == state.users[user_idx].name
                || state.users[user_idx].permission_level >= 9
            {
                state.files.remove(i);
                print!("File '{}' deleted\n", args[0]);
                return;
            } else {
                print!("Error: Permission denied\n");
                return;
            }
        }
    }

    print!("Error: File '{}' not found\n", args[0]);
}

fn cmd_listfiles(state: &State) {
    if state.files.is_empty() {
        print!("No files\n");
        return;
    }

    print!("Files:\n");
    for file in &state.files {
        print!(
            "  {} (owner: {}, perm: {})\n",
            file.filename, file.owner, file.permissions
        );
    }
}

fn cmd_set(state: &mut State, args: &[String]) {
    if args.len() < 2 {
        print!("Usage: set <name> <value>\n");
        return;
    }

    for variable in &mut state.variables {
        if variable.name == args[0] {
            variable.value = args[1].clone();
            print!("Variable '{}' updated\n", args[0]);
            return;
        }
    }

    if state.variables.len() >= MAX_VARIABLES {
        print!("Error: Maximum variables reached\n");
        return;
    }

    state.variables.push(Variable {
        name: args[0].clone(),
        value: args[1].clone(),
    });
    print!("Variable '{}' set\n", args[0]);
}

fn cmd_get(state: &State, args: &[String]) {
    if args.is_empty() {
        print!("Usage: get <name>\n");
        return;
    }

    for variable in &state.variables {
        if variable.name == args[0] {
            print!("{} = {}\n", variable.name, variable.value);
            return;
        }
    }

    print!("Error: Variable '{}' not found\n", args[0]);
}

fn cmd_unset(state: &mut State, args: &[String]) {
    if args.is_empty() {
        print!("Usage: unset <name>\n");
        return;
    }

    for i in 0..state.variables.len() {
        if state.variables[i].name == args[0] {
            state.variables.remove(i);
            print!("Variable '{}' unset\n", args[0]);
            return;
        }
    }

    print!("Error: Variable '{}' not found\n", args[0]);
}

fn cmd_listvars(state: &State) {
    if state.variables.is_empty() {
        print!("No variables set\n");
        return;
    }

    print!("Variables:\n");
    for variable in &state.variables {
        print!("  {} = {}\n", variable.name, variable.value);
    }
}

fn cmd_compare(args: &[String]) {
    if args.len() < 2 {
        print!("Usage: compare <string1> <string2>\n");
        return;
    }

    let result = strcmp_c(&args[0], &args[1]);
    print!("strcmp('{}', '{}') = {}\n", args[0], args[1], result);

    if result == 0 {
        print!("Strings are equal\n");
    } else if result < 0 {
        print!("'{}' < '{}'\n", args[0], args[1]);
    } else {
        print!("'{}' > '{}'\n", args[0], args[1]);
    }
}

fn cmd_compare_n(args: &[String]) {
    if args.len() < 3 {
        print!("Usage: compareN <string1> <string2> <n>\n");
        return;
    }

    let n = atoi_c(&args[2]);
    let result = strncmp_c(&args[0], &args[1], n);
    print!("strncmp('{}', '{}', {}) = {}\n", args[0], args[1], n, result);

    if result == 0 {
        print!("First {} characters are equal\n", n);
    } else if result < 0 {
        print!("'{}' < '{}' (first {} chars)\n", args[0], args[1], n);
    } else {
        print!("'{}' > '{}' (first {} chars)\n", args[0], args[1], n);
    }
}

fn cmd_startswith(args: &[String]) {
    if args.len() < 2 {
        print!("Usage: startswith <string> <prefix>\n");
        return;
    }

    let prefix_len = args[1].len() as i32;
    if strncmp_c(&args[0], &args[1], prefix_len) == 0 {
        print!("'{}' starts with '{}'\n", args[0], args[1]);
    } else {
        print!("'{}' does not start with '{}'\n", args[0], args[1]);
    }
}

fn cmd_match(args: &[String]) {
    if args.len() < 2 {
        print!("Usage: match <pattern> <string1> [string2] ...\n");
        return;
    }

    print!("Matching pattern '{}':\n", args[0]);
    let mut matches = 0;
    for arg in &args[1..] {
        if args[0] == *arg {
            print!("  '{}' - EXACT MATCH\n", arg);
            matches += 1;
        } else if arg.contains(&args[0]) {
            print!("  '{}' - contains pattern\n", arg);
            matches += 1;
        } else {
            print!("  '{}' - no match\n", arg);
        }
    }
    print!("Total matches: {}\n", matches);
}

fn cmd_help() {
    print!("\n=== Command Interpreter Help ===\n");
    print!("User Management:\n");
    print!("  adduser <user> <pass> [level] - Add new user\n");
    print!("  login <user> <pass>            - Login as user\n");
    print!("  logout                         - Logout current user\n");
    print!("  whoami                         - Show current user\n");
    print!("  listusers                      - List all users\n");
    print!("\nFile Management:\n");
    print!("  createfile <name> [content]    - Create file\n");
    print!("  readfile <name>                - Read file\n");
    print!("  writefile <name> <content>     - Write to file\n");
    print!("  deletefile <name>              - Delete file\n");
    print!("  listfiles                      - List all files\n");
    print!("\nVariable Management:\n");
    print!("  set <name> <value>             - Set variable\n");
    print!("  get <name>                     - Get variable\n");
    print!("  unset <name>                   - Unset variable\n");
    print!("  listvars                       - List all variables\n");
    print!("\nString Operations:\n");
    print!("  compare <str1> <str2>          - Compare strings\n");
    print!("  compareN <str1> <str2> <n>     - Compare first N chars\n");
    print!("  startswith <str> <prefix>      - Check if starts with\n");
    print!("  match <pattern> <str> ...      - Match pattern\n");
    print!("\nSystem:\n");
    print!("  debug [on|off]                 - Toggle debug mode\n");
    print!("  verbose [on|off]               - Toggle verbose mode\n");
    print!("  status                         - Show system status\n");
    print!("  time                           - Show current time\n");
    print!("  help                           - Show this help\n");
    print!("  exit                           - Exit program\n");
}

fn cmd_debug(state: &mut State, args: &[String]) {
    if args.is_empty() {
        print!("Debug mode: {}\n", if state.debug_mode { "ON" } else { "OFF" });
        return;
    }

    if args[0] == "on" {
        state.debug_mode = true;
        print!("Debug mode enabled\n");
    } else if args[0] == "off" {
        state.debug_mode = false;
        print!("Debug mode disabled\n");
    } else {
        print!("Usage: debug [on|off]\n");
    }
}

fn cmd_verbose(state: &mut State, args: &[String]) {
    if args.is_empty() {
        print!(
            "Verbose mode: {}\n",
            if state.verbose_mode { "ON" } else { "OFF" }
        );
        return;
    }

    if args[0] == "on" {
        state.verbose_mode = true;
        print!("Verbose mode enabled\n");
    } else if args[0] == "off" {
        state.verbose_mode = false;
        print!("Verbose mode disabled\n");
    } else {
        print!("Usage: verbose [on|off]\n");
    }
}

fn cmd_status(state: &State) {
    let current = state
        .current_user_logged_in()
        .map(|idx| state.users[idx].name.as_str())
        .unwrap_or("none");

    print!("\n=== System Status ===\n");
    print!("Users: {}/{}\n", state.users.len(), MAX_USERS);
    print!("Files: {}/{}\n", state.files.len(), MAX_FILES);
    print!("Variables: {}/{}\n", state.variables.len(), MAX_VARIABLES);
    print!("Current user: {}\n", current);
    print!("Debug mode: {}\n", if state.debug_mode { "ON" } else { "OFF" });
    print!(
        "Verbose mode: {}\n",
        if state.verbose_mode { "ON" } else { "OFF" }
    );
}

fn cmd_time() {
    unsafe {
        let now = time(std::ptr::null_mut());
        let text = ctime(&now as *const c_long);
        if !text.is_null() {
            let s = CStr::from_ptr(text).to_string_lossy();
            print!("Current time: {}", s);
        }
    }
}

fn process_command(state: &mut State, input: &str) {
    let (command, args) = parse_command(input);

    if command.is_empty() {
        return;
    }

    if state.debug_mode {
        print!("[DEBUG] Command: '{}', Args: {}\n", command, args.len());
    }

    if command == "adduser" {
        cmd_adduser(state, &args);
    } else if command == "login" {
        cmd_login(state, &args);
    } else if command == "logout" {
        cmd_logout(state);
    } else if command == "whoami" {
        cmd_whoami(state);
    } else if command == "listusers" || command == "users" {
        cmd_listusers(state);
    } else if command == "createfile" || command == "touch" {
        cmd_createfile(state, &args);
    } else if command == "readfile" || command == "cat" {
        cmd_readfile(state, &args);
    } else if command == "writefile" || command == "write" {
        cmd_writefile(state, &args);
    } else if command == "deletefile" || command == "rm" {
        cmd_deletefile(state, &args);
    } else if command == "listfiles" || command == "ls" {
        cmd_listfiles(state);
    } else if command == "set" {
        cmd_set(state, &args);
    } else if command == "get" {
        cmd_get(state, &args);
    } else if command == "unset" {
        cmd_unset(state, &args);
    } else if command == "listvars" || command == "vars" {
        cmd_listvars(state);
    } else if command == "compare" || command == "cmp" {
        cmd_compare(&args);
    } else if command == "compareN" || command == "cmpn" {
        cmd_compare_n(&args);
    } else if command == "startswith" {
        cmd_startswith(&args);
    } else if command == "match" {
        cmd_match(&args);
    } else if command == "debug" {
        cmd_debug(state, &args);
    } else if command == "verbose" {
        cmd_verbose(state, &args);
    } else if command == "status" {
        cmd_status(state);
    } else if command == "time" {
        cmd_time();
    } else if command == "help" || command == "?" {
        cmd_help();
    } else if command == "exit" || command == "quit" {
        print!("Goodbye!\n");
        let _ = io::stdout().flush();
        process::exit(0);
    } else if strncmp_c(&command, "add", 3) == 0 {
        print!("Did you mean 'adduser'?\n");
    } else if strncmp_c(&command, "log", 3) == 0 {
        print!("Did you mean 'login' or 'logout'?\n");
    } else if strncmp_c(&command, "list", 4) == 0 {
        print!("Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
    } else if strncmp_c(&command, "create", 6) == 0 {
        print!("Did you mean 'createfile'?\n");
    } else if strncmp_c(&command, "read", 4) == 0 {
        print!("Did you mean 'readfile'?\n");
    } else if strncmp_c(&command, "write", 5) == 0 {
        print!("Did you mean 'writefile'?\n");
    } else if strncmp_c(&command, "delete", 6) == 0 {
        print!("Did you mean 'deletefile'?\n");
    } else {
        print!(
            "Unknown command: '{}'. Type 'help' for available commands.\n",
            command
        );
    }
}

fn read_fgets_chunk<R: Read>(reader: &mut R) -> Option<String> {
    let mut chunk = Vec::new();
    let mut byte = [0u8; 1];

    while chunk.len() < MAX_INPUT - 1 {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                chunk.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if chunk.is_empty() {
        return None;
    }

    if let Some(newline_pos) = chunk.iter().position(|&b| b == b'\n') {
        chunk.truncate(newline_pos);
    }

    Some(String::from_utf8_lossy(&chunk).into_owned())
}

fn main() {
    print!("|----------------------------------------|\n");
    print!("|   COMMAND INTERPRETER                  |\n");
    print!("|   strcmp/strncmp demonstration         |\n");
    print!("|----------------------------------------|\n");
    print!("Type 'help' for available commands\n\n");

    let mut state = State::new();
    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    loop {
        print!("> ");
        let Some(line) = read_fgets_chunk(&mut stdin) else {
            break;
        };

        if state.verbose_mode {
            print!("[VERBOSE] Processing: '{}'\n", line);
        }

        process_command(&mut state, &line);
    }
}
