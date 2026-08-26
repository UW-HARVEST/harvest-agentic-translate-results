use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int, c_long, c_void};
use std::sync::Mutex;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

#[derive(Default)]
struct User {
    name: Vec<u8>,
    password: Vec<u8>,
    permission_level: i32,
    logged_in: bool,
}

#[derive(Default)]
struct File {
    filename: Vec<u8>,
    content: Vec<u8>,
    owner: Vec<u8>,
    permissions: i32,
}

#[derive(Default)]
struct Variable {
    name: Vec<u8>,
    value: Vec<u8>,
}

#[derive(Default)]
struct State {
    users: Vec<User>,
    current_user: Option<usize>,
    files: Vec<File>,
    variables: Vec<Variable>,
    debug_mode: bool,
    verbose_mode: bool,
}

static FFI_STATE: Mutex<Option<State>> = Mutex::new(None);

fn append(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(bytes);
}

fn append_between(out: &mut Vec<u8>, before: &[u8], value: &[u8], after: &[u8]) {
    append(out, before);
    append(out, value);
    append(out, after);
}

fn c_strcmp(left: &[u8], right: &[u8]) -> i32 {
    let common = left.len().min(right.len());
    for i in 0..common {
        if left[i] != right[i] {
            return i32::from(left[i]) - i32::from(right[i]);
        }
    }
    match left.len().cmp(&right.len()) {
        std::cmp::Ordering::Less => -i32::from(right[common]),
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => i32::from(left[common]),
    }
}

fn c_strncmp(left: &[u8], right: &[u8], n: i32) -> i32 {
    if n == 0 {
        return 0;
    }

    // A negative C int is converted to a large size_t. Both arrays are
    // NUL-terminated, so comparison still stops at the end of either value.
    let limit = if n < 0 { usize::MAX } else { n as usize };
    let common = left.len().min(right.len()).min(limit);
    for i in 0..common {
        if left[i] != right[i] {
            return i32::from(left[i]) - i32::from(right[i]);
        }
    }
    if common == limit {
        return 0;
    }
    match left.len().cmp(&right.len()) {
        std::cmp::Ordering::Less => -i32::from(right[common]),
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => i32::from(left[common]),
    }
}

fn c_atoi(value: &[u8]) -> i32 {
    unsafe extern "C" {
        fn atoi(value: *const c_char) -> i32;
    }

    let mut terminated = Vec::with_capacity(value.len() + 1);
    terminated.extend_from_slice(value);
    terminated.push(0);
    unsafe { atoi(terminated.as_ptr().cast()) }
}

fn bytes_until_nul(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..end]
}

fn overwritten_c_field(original: &[u8], capacity: usize, adjacent_field: &[u8]) -> Vec<u8> {
    if original.len() < capacity {
        original.to_vec()
    } else {
        let mut result = original[..capacity].to_vec();
        result.extend_from_slice(adjacent_field);
        result
    }
}

fn stored_user_fields(
    username: &[u8],
    password: &[u8],
    permission_level: i32,
) -> (Vec<u8>, Vec<u8>) {
    let stored_password = if password.len() < 32 {
        password.to_vec()
    } else {
        let mut result = password[..32].to_vec();
        result.extend_from_slice(bytes_until_nul(&permission_level.to_ne_bytes()));
        result
    };
    let stored_name = overwritten_c_field(username, 32, &stored_password);
    (stored_name, stored_password)
}

fn stored_file_owner(owner: &[u8]) -> Vec<u8> {
    overwritten_c_field(owner, 32, bytes_until_nul(&755i32.to_ne_bytes()))
}

fn parse_command_impl(input: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let input = &input[..input.len().min(MAX_INPUT - 1)];
    let mut tokens = input
        .split(|byte| *byte == b' ' || *byte == b'\t')
        .filter(|token| !token.is_empty());
    let command = match tokens.next() {
        Some(token) => token[..token.len().min(MAX_COMMAND - 1)].to_vec(),
        None => return (Vec::new(), Vec::new()),
    };
    let args = tokens
        .take(MAX_ARGS)
        .map(|token| token[..token.len().min(MAX_COMMAND - 1)].to_vec())
        .collect();
    (command, args)
}

type CArg = [c_char; MAX_COMMAND];

unsafe fn ffi_args(args: *const CArg, arg_count: c_int) -> Vec<Vec<u8>> {
    if arg_count <= 0 {
        return Vec::new();
    }

    (0..arg_count as usize)
        .map(|index| c_bytes(args.wrapping_add(index).cast()).to_vec())
        .collect()
}

unsafe fn c_bytes<'a>(value: *const c_char) -> &'a [u8] {
    unsafe extern "C" {
        fn strlen(value: *const c_char) -> usize;
    }

    std::slice::from_raw_parts(value.cast(), strlen(value))
}

fn with_ffi_state(action: impl FnOnce(&mut State, &mut Vec<u8>) -> bool) -> bool {
    let (keep_running, output) = {
        let mut state = FFI_STATE.lock().unwrap_or_else(|error| error.into_inner());
        let state = state.get_or_insert_with(State::default);
        let mut output = Vec::new();
        let keep_running = action(state, &mut output);
        (keep_running, output)
    };
    write_c_stdout(&output);
    keep_running
}

fn write_c_stdout(output: &[u8]) {
    unsafe extern "C" {
        static mut stdout: *mut c_void;
        fn fwrite(ptr: *const c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
        fn fflush(stream: *mut c_void) -> c_int;
    }

    if output.is_empty() {
        return;
    }

    unsafe {
        let stream = stdout;
        fwrite(output.as_ptr().cast(), 1, output.len(), stream);
        fflush(stream);
    }
}

impl State {
    fn cmd_adduser(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            append(
                out,
                b"Usage: adduser <username> <password> [permission_level]\n",
            );
            return;
        }
        if self.users.len() >= MAX_USERS {
            append(out, b"Error: Maximum users reached\n");
            return;
        }
        if self.users.iter().any(|user| user.name == args[0]) {
            append_between(out, b"Error: User '", &args[0], b"' already exists\n");
            return;
        }

        let permission_level = if args.len() >= 3 { c_atoi(&args[2]) } else { 1 };
        let (name, password) = stored_user_fields(&args[0], &args[1], permission_level);
        self.users.push(User {
            name,
            password,
            permission_level,
            logged_in: false,
        });
        append_between(out, b"User '", &args[0], b"' added with permission level ");
        let _ = writeln!(out, "{permission_level}");
    }

    fn cmd_login(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            append(out, b"Usage: login <username> <password>\n");
            return;
        }
        if let Some(index) = self.current_user {
            if self.users[index].logged_in {
                append_between(
                    out,
                    b"Error: User '",
                    &self.users[index].name,
                    b"' already logged in. Use 'logout' first.\n",
                );
                return;
            }
        }

        for index in 0..self.users.len() {
            if self.users[index].name == args[0] {
                if self.users[index].password == args[1] {
                    self.users[index].logged_in = true;
                    self.current_user = Some(index);
                    append_between(
                        out,
                        b"Login successful. Welcome, ",
                        &self.users[index].name,
                        b"!\n",
                    );
                } else {
                    append(out, b"Error: Incorrect password\n");
                }
                return;
            }
        }
        append(out, b"Error: User not found\n");
    }

    fn cmd_logout(&mut self, out: &mut Vec<u8>) {
        let Some(index) = self.current_user else {
            append(out, b"Error: No user logged in\n");
            return;
        };
        if !self.users[index].logged_in {
            append(out, b"Error: No user logged in\n");
            return;
        }
        append_between(out, b"Goodbye, ", &self.users[index].name, b"!\n");
        self.users[index].logged_in = false;
        self.current_user = None;
    }

    fn cmd_whoami(&self, out: &mut Vec<u8>) {
        let Some(index) = self.current_user else {
            append(out, b"Not logged in\n");
            return;
        };
        if !self.users[index].logged_in {
            append(out, b"Not logged in\n");
            return;
        }
        append_between(out, b"Current user: ", &self.users[index].name, b"\n");
        let _ = writeln!(
            out,
            "Permission level: {}",
            self.users[index].permission_level
        );
    }

    fn cmd_listusers(&self, out: &mut Vec<u8>) {
        if self.users.is_empty() {
            append(out, b"No users registered\n");
            return;
        }
        append(out, b"Registered users:\n");
        for user in &self.users {
            append_between(out, b"  ", &user.name, b" (level ");
            let _ = write!(out, "{}", user.permission_level);
            append(out, b") ");
            if user.logged_in {
                append(out, b"[logged in]");
            }
            append(out, b"\n");
        }
    }

    fn logged_in_user(&self) -> Option<&User> {
        self.current_user
            .and_then(|index| self.users.get(index))
            .filter(|user| user.logged_in)
    }

    fn cmd_createfile(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        let Some(user) = self.logged_in_user() else {
            append(out, b"Error: Must be logged in\n");
            return;
        };
        if args.is_empty() {
            append(out, b"Usage: createfile <filename> [content]\n");
            return;
        }
        if self.files.len() >= MAX_FILES {
            append(out, b"Error: Maximum files reached\n");
            return;
        }
        if self.files.iter().any(|file| file.filename == args[0]) {
            append_between(out, b"Error: File '", &args[0], b"' already exists\n");
            return;
        }

        let owner = stored_file_owner(&user.name);
        self.files.push(File {
            filename: args[0].clone(),
            content: args.get(1).cloned().unwrap_or_default(),
            owner,
            permissions: 755,
        });
        append_between(out, b"File '", &args[0], b"' created\n");
    }

    fn cmd_readfile(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            append(out, b"Usage: readfile <filename>\n");
            return;
        }
        if let Some(file) = self.files.iter().find(|file| file.filename == args[0]) {
            append_between(out, b"=== ", &file.filename, b" ===\n");
            append_between(out, b"Owner: ", &file.owner, b"\n");
            let _ = writeln!(out, "Permissions: {}", file.permissions);
            append_between(out, b"Content: ", &file.content, b"\n");
            return;
        }
        append_between(out, b"Error: File '", &args[0], b"' not found\n");
    }

    fn cmd_writefile(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        let Some(user) = self.logged_in_user() else {
            append(out, b"Error: Must be logged in\n");
            return;
        };
        if args.len() < 2 {
            append(out, b"Usage: writefile <filename> <content>\n");
            return;
        }
        let user_name = user.name.clone();
        let permission_level = user.permission_level;
        for file in &mut self.files {
            if file.filename == args[0] {
                if file.owner == user_name || permission_level >= 5 {
                    file.content = args[1].clone();
                    append_between(out, b"File '", &args[0], b"' updated\n");
                } else {
                    append(out, b"Error: Permission denied\n");
                }
                return;
            }
        }
        append_between(out, b"Error: File '", &args[0], b"' not found\n");
    }

    fn cmd_deletefile(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        let Some(user) = self.logged_in_user() else {
            append(out, b"Error: Must be logged in\n");
            return;
        };
        if args.is_empty() {
            append(out, b"Usage: deletefile <filename>\n");
            return;
        }
        let user_name = user.name.clone();
        let permission_level = user.permission_level;
        for index in 0..self.files.len() {
            if self.files[index].filename == args[0] {
                if self.files[index].owner == user_name || permission_level >= 9 {
                    self.files.remove(index);
                    append_between(out, b"File '", &args[0], b"' deleted\n");
                } else {
                    append(out, b"Error: Permission denied\n");
                }
                return;
            }
        }
        append_between(out, b"Error: File '", &args[0], b"' not found\n");
    }

    fn cmd_listfiles(&self, out: &mut Vec<u8>) {
        if self.files.is_empty() {
            append(out, b"No files\n");
            return;
        }
        append(out, b"Files:\n");
        for file in &self.files {
            append_between(out, b"  ", &file.filename, b" (owner: ");
            append_between(out, b"", &file.owner, b", perm: ");
            let _ = writeln!(out, "{})", file.permissions);
        }
    }

    fn cmd_set(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            append(out, b"Usage: set <name> <value>\n");
            return;
        }
        for variable in &mut self.variables {
            if variable.name == args[0] {
                variable.value = args[1].clone();
                append_between(out, b"Variable '", &args[0], b"' updated\n");
                return;
            }
        }
        if self.variables.len() >= MAX_VARIABLES {
            append(out, b"Error: Maximum variables reached\n");
            return;
        }
        self.variables.push(Variable {
            name: overwritten_c_field(&args[0], 32, &args[1]),
            value: args[1].clone(),
        });
        append_between(out, b"Variable '", &args[0], b"' set\n");
    }

    fn cmd_get(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            append(out, b"Usage: get <name>\n");
            return;
        }
        if let Some(variable) = self
            .variables
            .iter()
            .find(|variable| variable.name == args[0])
        {
            append_between(out, b"", &variable.name, b" = ");
            append_between(out, b"", &variable.value, b"\n");
            return;
        }
        append_between(out, b"Error: Variable '", &args[0], b"' not found\n");
    }

    fn cmd_unset(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            append(out, b"Usage: unset <name>\n");
            return;
        }
        for index in 0..self.variables.len() {
            if self.variables[index].name == args[0] {
                self.variables.remove(index);
                append_between(out, b"Variable '", &args[0], b"' unset\n");
                return;
            }
        }
        append_between(out, b"Error: Variable '", &args[0], b"' not found\n");
    }

    fn cmd_listvars(&self, out: &mut Vec<u8>) {
        if self.variables.is_empty() {
            append(out, b"No variables set\n");
            return;
        }
        append(out, b"Variables:\n");
        for variable in &self.variables {
            append_between(out, b"  ", &variable.name, b" = ");
            append_between(out, b"", &variable.value, b"\n");
        }
    }

    fn cmd_compare(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            append(out, b"Usage: compare <string1> <string2>\n");
            return;
        }
        let result = c_strcmp(&args[0], &args[1]);
        append_between(out, b"strcmp('", &args[0], b"', '");
        append_between(out, b"", &args[1], b"') = ");
        let _ = writeln!(out, "{result}");
        if result == 0 {
            append(out, b"Strings are equal\n");
        } else {
            append_between(out, b"'", &args[0], b"' ");
            append(out, if result < 0 { b"< " } else { b"> " });
            append_between(out, b"'", &args[1], b"'\n");
        }
    }

    fn cmd_compare_n(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 3 {
            append(out, b"Usage: compareN <string1> <string2> <n>\n");
            return;
        }
        let n = c_atoi(&args[2]);
        let result = c_strncmp(&args[0], &args[1], n);
        append_between(out, b"strncmp('", &args[0], b"', '");
        append_between(out, b"", &args[1], b"', ");
        let _ = writeln!(out, "{n}) = {result}");
        if result == 0 {
            let _ = writeln!(out, "First {n} characters are equal");
        } else {
            append_between(out, b"'", &args[0], b"' ");
            append(out, if result < 0 { b"< " } else { b"> " });
            append_between(out, b"'", &args[1], b"' (first ");
            let _ = writeln!(out, "{n} chars)");
        }
    }

    fn cmd_startswith(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            append(out, b"Usage: startswith <string> <prefix>\n");
            return;
        }
        append_between(out, b"'", &args[0], b"' ");
        if args[0].starts_with(&args[1]) {
            append(out, b"starts with ");
        } else {
            append(out, b"does not start with ");
        }
        append_between(out, b"'", &args[1], b"'\n");
    }

    fn cmd_match(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            append(out, b"Usage: match <pattern> <string1> [string2] ...\n");
            return;
        }
        append_between(out, b"Matching pattern '", &args[0], b"':\n");
        let mut matches = 0;
        for value in &args[1..] {
            append_between(out, b"  '", value, b"' - ");
            if value == &args[0] {
                append(out, b"EXACT MATCH\n");
                matches += 1;
            } else if args[0].is_empty()
                || value
                    .windows(args[0].len())
                    .any(|window| window == args[0].as_slice())
            {
                append(out, b"contains pattern\n");
                matches += 1;
            } else {
                append(out, b"no match\n");
            }
        }
        let _ = writeln!(out, "Total matches: {matches}");
    }

    fn cmd_help(&self, out: &mut Vec<u8>) {
        append(
            out,
            br#"
=== Command Interpreter Help ===
User Management:
  adduser <user> <pass> [level] - Add new user
  login <user> <pass>            - Login as user
  logout                         - Logout current user
  whoami                         - Show current user
  listusers                      - List all users

File Management:
  createfile <name> [content]    - Create file
  readfile <name>                - Read file
  writefile <name> <content>     - Write to file
  deletefile <name>              - Delete file
  listfiles                      - List all files

Variable Management:
  set <name> <value>             - Set variable
  get <name>                     - Get variable
  unset <name>                   - Unset variable
  listvars                       - List all variables

String Operations:
  compare <str1> <str2>          - Compare strings
  compareN <str1> <str2> <n>     - Compare first N chars
  startswith <str> <prefix>      - Check if starts with
  match <pattern> <str> ...      - Match pattern

System:
  debug [on|off]                 - Toggle debug mode
  verbose [on|off]               - Toggle verbose mode
  status                         - Show system status
  time                           - Show current time
  help                           - Show this help
  exit                           - Exit program
"#,
        );
    }

    fn cmd_debug(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            append(
                out,
                if self.debug_mode {
                    b"Debug mode: ON\n"
                } else {
                    b"Debug mode: OFF\n"
                },
            );
        } else if args[0] == b"on" {
            self.debug_mode = true;
            append(out, b"Debug mode enabled\n");
        } else if args[0] == b"off" {
            self.debug_mode = false;
            append(out, b"Debug mode disabled\n");
        } else {
            append(out, b"Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            append(
                out,
                if self.verbose_mode {
                    b"Verbose mode: ON\n"
                } else {
                    b"Verbose mode: OFF\n"
                },
            );
        } else if args[0] == b"on" {
            self.verbose_mode = true;
            append(out, b"Verbose mode enabled\n");
        } else if args[0] == b"off" {
            self.verbose_mode = false;
            append(out, b"Verbose mode disabled\n");
        } else {
            append(out, b"Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&self, out: &mut Vec<u8>) {
        append(out, b"\n=== System Status ===\n");
        let _ = writeln!(out, "Users: {}/{}", self.users.len(), MAX_USERS);
        let _ = writeln!(out, "Files: {}/{}", self.files.len(), MAX_FILES);
        let _ = writeln!(out, "Variables: {}/{}", self.variables.len(), MAX_VARIABLES);
        append(out, b"Current user: ");
        if let Some(user) = self.logged_in_user() {
            append(out, &user.name);
        } else {
            append(out, b"none");
        }
        append(out, b"\n");
        append(
            out,
            if self.debug_mode {
                b"Debug mode: ON\n"
            } else {
                b"Debug mode: OFF\n"
            },
        );
        append(
            out,
            if self.verbose_mode {
                b"Verbose mode: ON\n"
            } else {
                b"Verbose mode: OFF\n"
            },
        );
    }

    fn cmd_time(&self, out: &mut Vec<u8>) {
        unsafe extern "C" {
            fn time(timer: *mut c_long) -> c_long;
            fn ctime(timer: *const c_long) -> *mut c_char;
        }

        append(out, b"Current time: ");
        // Calling the same C routines preserves ctime's locale and timezone format.
        unsafe {
            let now = time(std::ptr::null_mut());
            let text = ctime(&now);
            if !text.is_null() {
                append(out, CStr::from_ptr(text).to_bytes());
            }
        }
    }

    fn process_command(&mut self, input: &[u8], out: &mut Vec<u8>) -> bool {
        let (command, args) = parse_command_impl(input);
        if command.is_empty() {
            return true;
        }
        if self.debug_mode {
            append_between(out, b"[DEBUG] Command: '", &command, b"', Args: ");
            let _ = writeln!(out, "{}", args.len());
        }

        match command.as_slice() {
            b"adduser" => self.cmd_adduser(&args, out),
            b"login" => self.cmd_login(&args, out),
            b"logout" => self.cmd_logout(out),
            b"whoami" => self.cmd_whoami(out),
            b"listusers" | b"users" => self.cmd_listusers(out),
            b"createfile" | b"touch" => self.cmd_createfile(&args, out),
            b"readfile" | b"cat" => self.cmd_readfile(&args, out),
            b"writefile" | b"write" => self.cmd_writefile(&args, out),
            b"deletefile" | b"rm" => self.cmd_deletefile(&args, out),
            b"listfiles" | b"ls" => self.cmd_listfiles(out),
            b"set" => self.cmd_set(&args, out),
            b"get" => self.cmd_get(&args, out),
            b"unset" => self.cmd_unset(&args, out),
            b"listvars" | b"vars" => self.cmd_listvars(out),
            b"compare" | b"cmp" => self.cmd_compare(&args, out),
            b"compareN" | b"cmpn" => self.cmd_compare_n(&args, out),
            b"startswith" => self.cmd_startswith(&args, out),
            b"match" => self.cmd_match(&args, out),
            b"debug" => self.cmd_debug(&args, out),
            b"verbose" => self.cmd_verbose(&args, out),
            b"status" => self.cmd_status(out),
            b"time" => self.cmd_time(out),
            b"help" | b"?" => self.cmd_help(out),
            b"exit" | b"quit" => {
                append(out, b"Goodbye!\n");
                return false;
            }
            _ if command.starts_with(b"add") => append(out, b"Did you mean 'adduser'?\n"),
            _ if command.starts_with(b"log") => append(out, b"Did you mean 'login' or 'logout'?\n"),
            _ if command.starts_with(b"list") => append(
                out,
                b"Did you mean 'listusers', 'listfiles', or 'listvars'?\n",
            ),
            _ if command.starts_with(b"create") => append(out, b"Did you mean 'createfile'?\n"),
            _ if command.starts_with(b"read") => append(out, b"Did you mean 'readfile'?\n"),
            _ if command.starts_with(b"write") => append(out, b"Did you mean 'writefile'?\n"),
            _ if command.starts_with(b"delete") => append(out, b"Did you mean 'deletefile'?\n"),
            _ => append_between(
                out,
                b"Unknown command: '",
                &command,
                b"'. Type 'help' for available commands.\n",
            ),
        }
        true
    }
}

#[no_mangle]
pub unsafe extern "C" fn parse_command(
    input: *const c_char,
    command: *mut c_char,
    args: *mut CArg,
    arg_count: *mut c_int,
) {
    let input = c_bytes(input);
    let (parsed_command, parsed_args) = parse_command_impl(input);
    *arg_count = parsed_args.len() as c_int;

    if !parsed_command.is_empty() {
        std::ptr::write_bytes(command, 0, MAX_COMMAND);
        std::ptr::copy_nonoverlapping(
            parsed_command.as_ptr().cast(),
            command,
            parsed_command.len(),
        );
    }

    for (index, arg) in parsed_args.iter().enumerate() {
        let destination = (*args.add(index)).as_mut_ptr();
        std::ptr::write_bytes(destination, 0, MAX_COMMAND);
        std::ptr::copy_nonoverlapping(arg.as_ptr().cast(), destination, arg.len());
    }
}

macro_rules! export_args_command {
    ($export:ident, $method:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $export(args: *const CArg, arg_count: c_int) {
            let args = ffi_args(args, arg_count);
            with_ffi_state(|state, output| {
                state.$method(&args, output);
                true
            });
        }
    };
}

macro_rules! export_no_args_command {
    ($export:ident, $method:ident) => {
        #[no_mangle]
        pub extern "C" fn $export() {
            with_ffi_state(|state, output| {
                state.$method(output);
                true
            });
        }
    };
}

export_args_command!(cmd_adduser, cmd_adduser);
export_args_command!(cmd_login, cmd_login);
export_no_args_command!(cmd_logout, cmd_logout);
export_no_args_command!(cmd_whoami, cmd_whoami);
export_no_args_command!(cmd_listusers, cmd_listusers);
export_args_command!(cmd_createfile, cmd_createfile);
export_args_command!(cmd_readfile, cmd_readfile);
export_args_command!(cmd_writefile, cmd_writefile);
export_args_command!(cmd_deletefile, cmd_deletefile);
export_no_args_command!(cmd_listfiles, cmd_listfiles);
export_args_command!(cmd_set, cmd_set);
export_args_command!(cmd_get, cmd_get);
export_args_command!(cmd_unset, cmd_unset);
export_no_args_command!(cmd_listvars, cmd_listvars);
export_args_command!(cmd_compare, cmd_compare);
export_args_command!(cmd_compareN, cmd_compare_n);
export_args_command!(cmd_startswith, cmd_startswith);
export_args_command!(cmd_match, cmd_match);
export_no_args_command!(cmd_help, cmd_help);
export_args_command!(cmd_debug, cmd_debug);
export_args_command!(cmd_verbose, cmd_verbose);
export_no_args_command!(cmd_status, cmd_status);
export_no_args_command!(cmd_time, cmd_time);

#[no_mangle]
pub unsafe extern "C" fn process_command(input: *const c_char) {
    let input = c_bytes(input);
    let keep_running = with_ffi_state(|state, output| state.process_command(input, output));
    if !keep_running {
        unsafe extern "C" {
            fn exit(status: c_int) -> !;
        }
        exit(0);
    }
}

fn c_fgets<R: Read>(reader: &mut R, input: &mut Vec<u8>) -> io::Result<bool> {
    input.clear();
    let mut byte = [0u8; 1];
    while input.len() < MAX_INPUT - 1 {
        match reader.read(&mut byte) {
            Ok(0) => return Ok(!input.is_empty()),
            Ok(_) => {
                input.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(true)
}

fn run_interpreter() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let stdout = io::stdout();
    let mut stdout = io::BufWriter::new(stdout.lock());
    let mut input = Vec::with_capacity(MAX_INPUT);

    stdout.write_all(
        b"|----------------------------------------|\n\
|   COMMAND INTERPRETER                  |\n\
|   strcmp/strncmp demonstration         |\n\
|----------------------------------------|\n\
Type 'help' for available commands\n\n",
    )?;

    loop {
        stdout.write_all(b"> ")?;
        if !c_fgets(&mut stdin, &mut input)? {
            break;
        }

        let c_string_end = input
            .iter()
            .position(|byte| *byte == 0 || *byte == b'\n')
            .unwrap_or(input.len());
        let line = &input[..c_string_end];
        let (keep_running, output) = {
            let mut state = FFI_STATE.lock().unwrap_or_else(|error| error.into_inner());
            let state = state.get_or_insert_with(State::default);
            let mut output = Vec::new();
            if state.verbose_mode {
                append_between(&mut output, b"[VERBOSE] Processing: '", line, b"'\n");
            }
            let keep_running = state.process_command(line, &mut output);
            (keep_running, output)
        };
        stdout.write_all(&output)?;
        if !keep_running {
            break;
        }
    }

    stdout.flush()
}

#[export_name = "main"]
pub extern "C" fn c_main() -> c_int {
    match run_interpreter() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}
