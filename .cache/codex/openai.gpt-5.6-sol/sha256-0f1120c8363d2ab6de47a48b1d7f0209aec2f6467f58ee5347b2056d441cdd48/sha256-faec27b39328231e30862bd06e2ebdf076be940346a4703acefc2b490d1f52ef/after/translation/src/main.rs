use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int, c_long};

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

const USER_SIZE: usize = 72;
const USER_PASSWORD: usize = 32;
const USER_PERMISSION: usize = 64;
const USER_LOGGED_IN: usize = 68;

const FILE_SIZE: usize = 612;
const FILE_CONTENT: usize = 64;
const FILE_OWNER: usize = 576;
const FILE_PERMISSIONS: usize = 608;

const VARIABLE_SIZE: usize = 160;
const VARIABLE_VALUE: usize = 32;

extern "C" {
    fn atoi(value: *const c_char) -> c_int;
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    fn strncmp(left: *const c_char, right: *const c_char, count: usize) -> c_int;
    fn time(timer: *mut c_long) -> c_long;
    fn ctime(timer: *const c_long) -> *mut c_char;
}

fn nul_terminated(value: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(value.len() + 1);
    result.extend_from_slice(value);
    result.push(0);
    result
}

fn c_atoi(value: &[u8]) -> i32 {
    let value = nul_terminated(value);
    unsafe { atoi(value.as_ptr().cast()) }
}

fn c_strcmp(left: &[u8], right: &[u8]) -> i32 {
    let left = nul_terminated(left);
    let right = nul_terminated(right);
    unsafe { strcmp(left.as_ptr().cast(), right.as_ptr().cast()) }
}

fn c_strncmp(left: &[u8], right: &[u8], count: usize) -> i32 {
    let left = nul_terminated(left);
    let right = nul_terminated(right);
    unsafe { strncmp(left.as_ptr().cast(), right.as_ptr().cast(), count) }
}

fn write_c_string(storage: &mut [u8], offset: usize, value: &[u8]) {
    for (index, byte) in value.iter().copied().chain(std::iter::once(0)).enumerate() {
        if let Some(slot) = storage.get_mut(offset + index) {
            *slot = byte;
        } else {
            break;
        }
    }
}

fn read_c_string(storage: &[u8], offset: usize) -> &[u8] {
    let tail = &storage[offset..];
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(tail.len());
    &tail[..length]
}

fn write_i32(storage: &mut [u8], offset: usize, value: i32) {
    if let Some(target) = storage.get_mut(offset..offset + 4) {
        target.copy_from_slice(&value.to_ne_bytes());
    }
}

fn read_i32(storage: &[u8], offset: usize) -> i32 {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(&storage[offset..offset + 4]);
    i32::from_ne_bytes(bytes)
}

fn push_dynamic(out: &mut Vec<u8>, prefix: &[u8], value: &[u8], suffix: &[u8]) {
    out.extend_from_slice(prefix);
    out.extend_from_slice(value);
    out.extend_from_slice(suffix);
}

fn parse_command(input: &[u8]) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
    let mut tokens = input
        .split(|byte| *byte == b' ' || *byte == b'\t')
        .filter(|token| !token.is_empty());
    let command = tokens
        .next()?
        .iter()
        .copied()
        .take(MAX_COMMAND - 1)
        .collect();
    let args = tokens
        .take(MAX_ARGS)
        .map(|token| token.iter().copied().take(MAX_COMMAND - 1).collect())
        .collect();
    Some((command, args))
}

struct State {
    users: Vec<u8>,
    user_count: usize,
    current_user: Option<usize>,
    files: Vec<u8>,
    file_count: usize,
    variables: Vec<u8>,
    variable_count: usize,
    debug_mode: bool,
    verbose_mode: bool,
}

impl State {
    fn new() -> Self {
        Self {
            users: vec![0; USER_SIZE * MAX_USERS],
            user_count: 0,
            current_user: None,
            files: vec![0; FILE_SIZE * MAX_FILES],
            file_count: 0,
            variables: vec![0; VARIABLE_SIZE * MAX_VARIABLES],
            variable_count: 0,
            debug_mode: false,
            verbose_mode: false,
        }
    }

    fn user_offset(index: usize) -> usize {
        index * USER_SIZE
    }

    fn user_name(&self, index: usize) -> &[u8] {
        read_c_string(&self.users, Self::user_offset(index))
    }

    fn user_password(&self, index: usize) -> &[u8] {
        read_c_string(&self.users, Self::user_offset(index) + USER_PASSWORD)
    }

    fn user_permission(&self, index: usize) -> i32 {
        read_i32(&self.users, Self::user_offset(index) + USER_PERMISSION)
    }

    fn user_logged_in(&self, index: usize) -> bool {
        read_i32(&self.users, Self::user_offset(index) + USER_LOGGED_IN) != 0
    }

    fn set_user_logged_in(&mut self, index: usize, value: bool) {
        write_i32(
            &mut self.users,
            Self::user_offset(index) + USER_LOGGED_IN,
            i32::from(value),
        );
    }

    fn file_offset(index: usize) -> usize {
        index * FILE_SIZE
    }

    fn filename(&self, index: usize) -> &[u8] {
        read_c_string(&self.files, Self::file_offset(index))
    }

    fn file_content(&self, index: usize) -> &[u8] {
        read_c_string(&self.files, Self::file_offset(index) + FILE_CONTENT)
    }

    fn file_owner(&self, index: usize) -> &[u8] {
        read_c_string(&self.files, Self::file_offset(index) + FILE_OWNER)
    }

    fn file_permissions(&self, index: usize) -> i32 {
        read_i32(&self.files, Self::file_offset(index) + FILE_PERMISSIONS)
    }

    fn variable_offset(index: usize) -> usize {
        index * VARIABLE_SIZE
    }

    fn variable_name(&self, index: usize) -> &[u8] {
        read_c_string(&self.variables, Self::variable_offset(index))
    }

    fn variable_value(&self, index: usize) -> &[u8] {
        read_c_string(
            &self.variables,
            Self::variable_offset(index) + VARIABLE_VALUE,
        )
    }

    fn cmd_adduser(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            out.extend_from_slice(b"Usage: adduser <username> <password> [permission_level]\n");
            return;
        }
        if self.user_count >= MAX_USERS {
            out.extend_from_slice(b"Error: Maximum users reached\n");
            return;
        }
        for index in 0..self.user_count {
            if c_strcmp(self.user_name(index), &args[0]) == 0 {
                push_dynamic(out, b"Error: User '", &args[0], b"' already exists\n");
                return;
            }
        }

        let offset = Self::user_offset(self.user_count);
        write_c_string(&mut self.users, offset, &args[0]);
        write_c_string(&mut self.users, offset + USER_PASSWORD, &args[1]);
        let permission = if args.len() >= 3 { c_atoi(&args[2]) } else { 1 };
        write_i32(&mut self.users, offset + USER_PERMISSION, permission);
        write_i32(&mut self.users, offset + USER_LOGGED_IN, 0);
        self.user_count += 1;

        push_dynamic(out, b"User '", &args[0], b"' added with permission level ");
        writeln!(out, "{}", self.user_permission(self.user_count - 1)).unwrap();
    }

    fn cmd_login(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            out.extend_from_slice(b"Usage: login <username> <password>\n");
            return;
        }
        if let Some(index) = self.current_user {
            if self.user_logged_in(index) {
                push_dynamic(
                    out,
                    b"Error: User '",
                    self.user_name(index),
                    b"' already logged in. Use 'logout' first.\n",
                );
                return;
            }
        }

        for index in 0..self.user_count {
            if c_strcmp(self.user_name(index), &args[0]) == 0 {
                if c_strcmp(self.user_password(index), &args[1]) == 0 {
                    self.set_user_logged_in(index, true);
                    self.current_user = Some(index);
                    push_dynamic(
                        out,
                        b"Login successful. Welcome, ",
                        self.user_name(index),
                        b"!\n",
                    );
                } else {
                    out.extend_from_slice(b"Error: Incorrect password\n");
                }
                return;
            }
        }
        out.extend_from_slice(b"Error: User not found\n");
    }

    fn cmd_logout(&mut self, out: &mut Vec<u8>) {
        let Some(index) = self.current_user else {
            out.extend_from_slice(b"Error: No user logged in\n");
            return;
        };
        if !self.user_logged_in(index) {
            out.extend_from_slice(b"Error: No user logged in\n");
            return;
        }

        push_dynamic(out, b"Goodbye, ", self.user_name(index), b"!\n");
        self.set_user_logged_in(index, false);
        self.current_user = None;
    }

    fn cmd_whoami(&self, out: &mut Vec<u8>) {
        let Some(index) = self.current_user else {
            out.extend_from_slice(b"Not logged in\n");
            return;
        };
        if !self.user_logged_in(index) {
            out.extend_from_slice(b"Not logged in\n");
            return;
        }
        push_dynamic(out, b"Current user: ", self.user_name(index), b"\n");
        writeln!(out, "Permission level: {}", self.user_permission(index)).unwrap();
    }

    fn cmd_listusers(&self, out: &mut Vec<u8>) {
        if self.user_count == 0 {
            out.extend_from_slice(b"No users registered\n");
            return;
        }
        out.extend_from_slice(b"Registered users:\n");
        for index in 0..self.user_count {
            push_dynamic(out, b"  ", self.user_name(index), b" (level ");
            write!(out, "{}) ", self.user_permission(index)).unwrap();
            if self.user_logged_in(index) {
                out.extend_from_slice(b"[logged in]");
            }
            out.push(b'\n');
        }
    }

    fn logged_in_user(&self) -> Option<usize> {
        self.current_user
            .filter(|index| self.user_logged_in(*index))
    }

    fn cmd_createfile(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        let Some(user) = self.logged_in_user() else {
            out.extend_from_slice(b"Error: Must be logged in\n");
            return;
        };
        if args.is_empty() {
            out.extend_from_slice(b"Usage: createfile <filename> [content]\n");
            return;
        }
        if self.file_count >= MAX_FILES {
            out.extend_from_slice(b"Error: Maximum files reached\n");
            return;
        }
        for index in 0..self.file_count {
            if c_strcmp(self.filename(index), &args[0]) == 0 {
                push_dynamic(out, b"Error: File '", &args[0], b"' already exists\n");
                return;
            }
        }

        let offset = Self::file_offset(self.file_count);
        let owner = self.user_name(user).to_vec();
        write_c_string(&mut self.files, offset, &args[0]);
        write_c_string(&mut self.files, offset + FILE_OWNER, &owner);
        write_i32(&mut self.files, offset + FILE_PERMISSIONS, 755);
        write_c_string(
            &mut self.files,
            offset + FILE_CONTENT,
            args.get(1).map_or(&[], Vec::as_slice),
        );
        self.file_count += 1;
        push_dynamic(out, b"File '", &args[0], b"' created\n");
    }

    fn cmd_readfile(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            out.extend_from_slice(b"Usage: readfile <filename>\n");
            return;
        }
        for index in 0..self.file_count {
            if c_strcmp(self.filename(index), &args[0]) == 0 {
                push_dynamic(out, b"=== ", self.filename(index), b" ===\n");
                push_dynamic(out, b"Owner: ", self.file_owner(index), b"\n");
                writeln!(out, "Permissions: {}", self.file_permissions(index)).unwrap();
                push_dynamic(out, b"Content: ", self.file_content(index), b"\n");
                return;
            }
        }
        push_dynamic(out, b"Error: File '", &args[0], b"' not found\n");
    }

    fn cmd_writefile(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        let Some(user) = self.logged_in_user() else {
            out.extend_from_slice(b"Error: Must be logged in\n");
            return;
        };
        if args.len() < 2 {
            out.extend_from_slice(b"Usage: writefile <filename> <content>\n");
            return;
        }
        for index in 0..self.file_count {
            if c_strcmp(self.filename(index), &args[0]) == 0 {
                let owns_file = c_strcmp(self.file_owner(index), self.user_name(user)) == 0;
                if owns_file || self.user_permission(user) >= 5 {
                    write_c_string(
                        &mut self.files,
                        Self::file_offset(index) + FILE_CONTENT,
                        &args[1],
                    );
                    push_dynamic(out, b"File '", &args[0], b"' updated\n");
                } else {
                    out.extend_from_slice(b"Error: Permission denied\n");
                }
                return;
            }
        }
        push_dynamic(out, b"Error: File '", &args[0], b"' not found\n");
    }

    fn cmd_deletefile(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        let Some(user) = self.logged_in_user() else {
            out.extend_from_slice(b"Error: Must be logged in\n");
            return;
        };
        if args.is_empty() {
            out.extend_from_slice(b"Usage: deletefile <filename>\n");
            return;
        }
        for index in 0..self.file_count {
            if c_strcmp(self.filename(index), &args[0]) == 0 {
                let owns_file = c_strcmp(self.file_owner(index), self.user_name(user)) == 0;
                if owns_file || self.user_permission(user) >= 9 {
                    let start = Self::file_offset(index);
                    let end = Self::file_offset(self.file_count);
                    self.files.copy_within(start + FILE_SIZE..end, start);
                    self.file_count -= 1;
                    push_dynamic(out, b"File '", &args[0], b"' deleted\n");
                } else {
                    out.extend_from_slice(b"Error: Permission denied\n");
                }
                return;
            }
        }
        push_dynamic(out, b"Error: File '", &args[0], b"' not found\n");
    }

    fn cmd_listfiles(&self, out: &mut Vec<u8>) {
        if self.file_count == 0 {
            out.extend_from_slice(b"No files\n");
            return;
        }
        out.extend_from_slice(b"Files:\n");
        for index in 0..self.file_count {
            push_dynamic(out, b"  ", self.filename(index), b" (owner: ");
            out.extend_from_slice(self.file_owner(index));
            write!(out, ", perm: {})\n", self.file_permissions(index)).unwrap();
        }
    }

    fn cmd_set(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            out.extend_from_slice(b"Usage: set <name> <value>\n");
            return;
        }
        for index in 0..self.variable_count {
            if c_strcmp(self.variable_name(index), &args[0]) == 0 {
                write_c_string(
                    &mut self.variables,
                    Self::variable_offset(index) + VARIABLE_VALUE,
                    &args[1],
                );
                push_dynamic(out, b"Variable '", &args[0], b"' updated\n");
                return;
            }
        }
        if self.variable_count >= MAX_VARIABLES {
            out.extend_from_slice(b"Error: Maximum variables reached\n");
            return;
        }

        let offset = Self::variable_offset(self.variable_count);
        write_c_string(&mut self.variables, offset, &args[0]);
        write_c_string(&mut self.variables, offset + VARIABLE_VALUE, &args[1]);
        self.variable_count += 1;
        push_dynamic(out, b"Variable '", &args[0], b"' set\n");
    }

    fn cmd_get(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            out.extend_from_slice(b"Usage: get <name>\n");
            return;
        }
        for index in 0..self.variable_count {
            if c_strcmp(self.variable_name(index), &args[0]) == 0 {
                push_dynamic(out, b"", self.variable_name(index), b" = ");
                out.extend_from_slice(self.variable_value(index));
                out.push(b'\n');
                return;
            }
        }
        push_dynamic(out, b"Error: Variable '", &args[0], b"' not found\n");
    }

    fn cmd_unset(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            out.extend_from_slice(b"Usage: unset <name>\n");
            return;
        }
        for index in 0..self.variable_count {
            if c_strcmp(self.variable_name(index), &args[0]) == 0 {
                let start = Self::variable_offset(index);
                let end = Self::variable_offset(self.variable_count);
                self.variables
                    .copy_within(start + VARIABLE_SIZE..end, start);
                self.variable_count -= 1;
                push_dynamic(out, b"Variable '", &args[0], b"' unset\n");
                return;
            }
        }
        push_dynamic(out, b"Error: Variable '", &args[0], b"' not found\n");
    }

    fn cmd_listvars(&self, out: &mut Vec<u8>) {
        if self.variable_count == 0 {
            out.extend_from_slice(b"No variables set\n");
            return;
        }
        out.extend_from_slice(b"Variables:\n");
        for index in 0..self.variable_count {
            push_dynamic(out, b"  ", self.variable_name(index), b" = ");
            out.extend_from_slice(self.variable_value(index));
            out.push(b'\n');
        }
    }

    fn cmd_compare(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            out.extend_from_slice(b"Usage: compare <string1> <string2>\n");
            return;
        }
        let result = c_strcmp(&args[0], &args[1]);
        push_dynamic(out, b"strcmp('", &args[0], b"', '");
        out.extend_from_slice(&args[1]);
        writeln!(out, "') = {result}").unwrap();
        if result == 0 {
            out.extend_from_slice(b"Strings are equal\n");
        } else {
            out.push(b'\'');
            out.extend_from_slice(&args[0]);
            out.extend_from_slice(if result < 0 { b"' < '" } else { b"' > '" });
            out.extend_from_slice(&args[1]);
            out.extend_from_slice(b"'\n");
        }
    }

    fn cmd_compare_n(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 3 {
            out.extend_from_slice(b"Usage: compareN <string1> <string2> <n>\n");
            return;
        }
        let count = c_atoi(&args[2]);
        let result = c_strncmp(&args[0], &args[1], count as usize);
        push_dynamic(out, b"strncmp('", &args[0], b"', '");
        out.extend_from_slice(&args[1]);
        write!(out, "', {count}) = {result}\n").unwrap();
        if result == 0 {
            writeln!(out, "First {count} characters are equal").unwrap();
        } else {
            out.push(b'\'');
            out.extend_from_slice(&args[0]);
            out.extend_from_slice(if result < 0 { b"' < '" } else { b"' > '" });
            out.extend_from_slice(&args[1]);
            writeln!(out, "' (first {count} chars)").unwrap();
        }
    }

    fn cmd_startswith(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            out.extend_from_slice(b"Usage: startswith <string> <prefix>\n");
            return;
        }
        out.push(b'\'');
        out.extend_from_slice(&args[0]);
        if c_strncmp(&args[0], &args[1], args[1].len()) == 0 {
            out.extend_from_slice(b"' starts with '");
        } else {
            out.extend_from_slice(b"' does not start with '");
        }
        out.extend_from_slice(&args[1]);
        out.extend_from_slice(b"'\n");
    }

    fn cmd_match(&self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.len() < 2 {
            out.extend_from_slice(b"Usage: match <pattern> <string1> [string2] ...\n");
            return;
        }
        push_dynamic(out, b"Matching pattern '", &args[0], b"':\n");
        let mut matches = 0;
        for value in &args[1..] {
            out.extend_from_slice(b"  '");
            out.extend_from_slice(value);
            if c_strcmp(&args[0], value) == 0 {
                out.extend_from_slice(b"' - EXACT MATCH\n");
                matches += 1;
            } else if args[0].is_empty()
                || value
                    .windows(args[0].len())
                    .any(|window| window == args[0].as_slice())
            {
                out.extend_from_slice(b"' - contains pattern\n");
                matches += 1;
            } else {
                out.extend_from_slice(b"' - no match\n");
            }
        }
        writeln!(out, "Total matches: {matches}").unwrap();
    }

    fn cmd_help(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(
            b"\n=== Command Interpreter Help ===\n"
                .iter()
                .chain(b"User Management:\n")
                .chain(b"  adduser <user> <pass> [level] - Add new user\n")
                .chain(b"  login <user> <pass>            - Login as user\n")
                .chain(b"  logout                         - Logout current user\n")
                .chain(b"  whoami                         - Show current user\n")
                .chain(b"  listusers                      - List all users\n")
                .chain(b"\nFile Management:\n")
                .chain(b"  createfile <name> [content]    - Create file\n")
                .chain(b"  readfile <name>                - Read file\n")
                .chain(b"  writefile <name> <content>     - Write to file\n")
                .chain(b"  deletefile <name>              - Delete file\n")
                .chain(b"  listfiles                      - List all files\n")
                .chain(b"\nVariable Management:\n")
                .chain(b"  set <name> <value>             - Set variable\n")
                .chain(b"  get <name>                     - Get variable\n")
                .chain(b"  unset <name>                   - Unset variable\n")
                .chain(b"  listvars                       - List all variables\n")
                .chain(b"\nString Operations:\n")
                .chain(b"  compare <str1> <str2>          - Compare strings\n")
                .chain(b"  compareN <str1> <str2> <n>     - Compare first N chars\n")
                .chain(b"  startswith <str> <prefix>      - Check if starts with\n")
                .chain(b"  match <pattern> <str> ...      - Match pattern\n")
                .chain(b"\nSystem:\n")
                .chain(b"  debug [on|off]                 - Toggle debug mode\n")
                .chain(b"  verbose [on|off]               - Toggle verbose mode\n")
                .chain(b"  status                         - Show system status\n")
                .chain(b"  time                           - Show current time\n")
                .chain(b"  help                           - Show this help\n")
                .chain(b"  exit                           - Exit program\n")
                .copied()
                .collect::<Vec<_>>()
                .as_slice(),
        );
    }

    fn cmd_debug(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            out.extend_from_slice(if self.debug_mode {
                b"Debug mode: ON\n"
            } else {
                b"Debug mode: OFF\n"
            });
        } else if args[0] == b"on" {
            self.debug_mode = true;
            out.extend_from_slice(b"Debug mode enabled\n");
        } else if args[0] == b"off" {
            self.debug_mode = false;
            out.extend_from_slice(b"Debug mode disabled\n");
        } else {
            out.extend_from_slice(b"Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, args: &[Vec<u8>], out: &mut Vec<u8>) {
        if args.is_empty() {
            out.extend_from_slice(if self.verbose_mode {
                b"Verbose mode: ON\n"
            } else {
                b"Verbose mode: OFF\n"
            });
        } else if args[0] == b"on" {
            self.verbose_mode = true;
            out.extend_from_slice(b"Verbose mode enabled\n");
        } else if args[0] == b"off" {
            self.verbose_mode = false;
            out.extend_from_slice(b"Verbose mode disabled\n");
        } else {
            out.extend_from_slice(b"Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"\n=== System Status ===\n");
        writeln!(out, "Users: {}/{}", self.user_count, MAX_USERS).unwrap();
        writeln!(out, "Files: {}/{}", self.file_count, MAX_FILES).unwrap();
        writeln!(out, "Variables: {}/{}", self.variable_count, MAX_VARIABLES).unwrap();
        out.extend_from_slice(b"Current user: ");
        if let Some(index) = self.logged_in_user() {
            out.extend_from_slice(self.user_name(index));
        } else {
            out.extend_from_slice(b"none");
        }
        out.push(b'\n');
        out.extend_from_slice(if self.debug_mode {
            b"Debug mode: ON\n"
        } else {
            b"Debug mode: OFF\n"
        });
        out.extend_from_slice(if self.verbose_mode {
            b"Verbose mode: ON\n"
        } else {
            b"Verbose mode: OFF\n"
        });
    }

    fn cmd_time(&self, out: &mut Vec<u8>) {
        let now = unsafe { time(std::ptr::null_mut()) };
        let text = unsafe { ctime(&now) };
        out.extend_from_slice(b"Current time: ");
        if !text.is_null() {
            out.extend_from_slice(unsafe { CStr::from_ptr(text) }.to_bytes());
        }
    }

    fn process_command(&mut self, input: &[u8], out: &mut Vec<u8>) -> bool {
        let Some((command, args)) = parse_command(input) else {
            return false;
        };
        if self.debug_mode {
            push_dynamic(out, b"[DEBUG] Command: '", &command, b"', Args: ");
            writeln!(out, "{}", args.len()).unwrap();
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
                out.extend_from_slice(b"Goodbye!\n");
                return true;
            }
            _ if command.starts_with(b"add") => out.extend_from_slice(b"Did you mean 'adduser'?\n"),
            _ if command.starts_with(b"log") => {
                out.extend_from_slice(b"Did you mean 'login' or 'logout'?\n")
            }
            _ if command.starts_with(b"list") => {
                out.extend_from_slice(b"Did you mean 'listusers', 'listfiles', or 'listvars'?\n")
            }
            _ if command.starts_with(b"create") => {
                out.extend_from_slice(b"Did you mean 'createfile'?\n")
            }
            _ if command.starts_with(b"read") => {
                out.extend_from_slice(b"Did you mean 'readfile'?\n")
            }
            _ if command.starts_with(b"write") => {
                out.extend_from_slice(b"Did you mean 'writefile'?\n")
            }
            _ if command.starts_with(b"delete") => {
                out.extend_from_slice(b"Did you mean 'deletefile'?\n")
            }
            _ => {
                push_dynamic(
                    out,
                    b"Unknown command: '",
                    &command,
                    b"'. Type 'help' for available commands.\n",
                );
            }
        }
        false
    }
}

fn fgets_like<R: Read>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut result = Vec::with_capacity(MAX_INPUT);
    let mut byte = [0; 1];
    while result.len() < MAX_INPUT - 1 {
        match reader.read(&mut byte) {
            Ok(0) => return Ok((!result.is_empty()).then_some(result)),
            Ok(_) => {
                result.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(Some(result))
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut state = State::new();

    output
        .write_all(
            b"|----------------------------------------|\n\
|   COMMAND INTERPRETER                  |\n\
|   strcmp/strncmp demonstration         |\n\
|----------------------------------------|\n\
Type 'help' for available commands\n\n",
        )
        .ok();

    loop {
        output.write_all(b"> ").ok();
        output.flush().ok();

        let Ok(Some(mut line)) = fgets_like(&mut input) else {
            break;
        };
        let visible_end = line
            .iter()
            .position(|byte| *byte == 0 || *byte == b'\n')
            .unwrap_or(line.len());
        line.truncate(visible_end);

        let mut response = Vec::new();
        if state.verbose_mode {
            push_dynamic(&mut response, b"[VERBOSE] Processing: '", &line, b"'\n");
        }
        let exit = state.process_command(&line, &mut response);
        output.write_all(&response).ok();
        if exit {
            break;
        }
    }
}
