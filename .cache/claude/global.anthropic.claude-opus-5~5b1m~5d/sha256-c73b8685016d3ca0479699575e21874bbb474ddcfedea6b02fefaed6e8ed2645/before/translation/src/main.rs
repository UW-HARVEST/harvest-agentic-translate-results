/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Faithful Rust translation of the C "command interpreter" driver.
//!
//! The translation reproduces the observable behaviour of the original C
//! program byte for byte, including its quirks:
//!
//!   * `fgets` semantics (at most `MAX_INPUT - 1` bytes per read, stopping
//!     after a newline, splitting over-long lines into several commands),
//!   * NUL-truncated C string views of the input buffer,
//!   * token truncation to `MAX_COMMAND - 1` bytes and the `MAX_ARGS` cap,
//!   * `strcmp`/`strncmp` returning the difference of the first differing
//!     bytes (as unsigned chars), the way glibc does,
//!   * `atoi` behaving like `(int)strtol(s, NULL, 10)` including saturation
//!     followed by truncation,
//!   * the fixed-size `strcpy` buffer overruns inside the global record
//!     arrays (a long user name spills into the password field, and so on).
//!
//! The record arrays are therefore modelled as flat byte buffers that mirror
//! the exact C struct layout, so that an overrunning `strcpy` corrupts
//! neighbouring fields exactly as it does in C.

use std::io::{self, BufReader, BufWriter, Read, Write};

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: i32 = 20;
const MAX_USERS: i32 = 10;
const MAX_VARIABLES: i32 = 20;

// ---------------------------------------------------------------------------
// C struct layouts (x86-64 System V)
// ---------------------------------------------------------------------------

// typedef struct { char name[32]; char password[32]; int permission_level; int logged_in; } user_t;
const U_SIZE: usize = 72;
const U_NAME: usize = 0;
const U_PASS: usize = 32;
const U_PERM: usize = 64;
const U_LOGGED: usize = 68;

// typedef struct { char filename[64]; char content[512]; char owner[32]; int permissions; } file_t;
const F_SIZE: usize = 612;
const F_NAME: usize = 0;
const F_CONTENT: usize = 64;
const F_OWNER: usize = 576;
const F_PERM: usize = 608;

// typedef struct { char name[32]; char value[128]; } variable_t;
const V_SIZE: usize = 160;
const V_NAME: usize = 0;
const V_VALUE: usize = 32;

/// Extra head room past the end of every emulated array so that an
/// overrunning `strcpy` on the last element cannot panic.  In C such a write
/// would silently smear over whatever global happens to sit next.
const SLACK: usize = 1024;

// ---------------------------------------------------------------------------
// Output: everything the program prints, byte exact.
// ---------------------------------------------------------------------------

struct Out {
    w: BufWriter<io::Stdout>,
}

impl Out {
    fn new() -> Out {
        Out {
            w: BufWriter::new(io::stdout()),
        }
    }

    /// printf("%s", ...) for raw (possibly non-UTF-8) bytes.
    fn s(&mut self, b: &[u8]) {
        let _ = self.w.write_all(b);
    }

    /// Literal text.
    fn t(&mut self, s: &str) {
        let _ = self.w.write_all(s.as_bytes());
    }

    /// printf("%d", ...)
    fn d(&mut self, v: i32) {
        let mut buf = [0u8; 12];
        let mut n = buf.len();
        let neg = v < 0;
        // Use i64 so that i32::MIN is representable when negated.
        let mut m = (v as i64).abs();
        loop {
            n -= 1;
            buf[n] = b'0' + (m % 10) as u8;
            m /= 10;
            if m == 0 {
                break;
            }
        }
        if neg {
            n -= 1;
            buf[n] = b'-';
        }
        let _ = self.w.write_all(&buf[n..]);
    }

    fn flush(&mut self) {
        let _ = self.w.flush();
    }
}

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

/// glibc `strcmp`: difference of the first differing bytes, compared as
/// `unsigned char`.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    c_strncmp(a, b, usize::MAX)
}

/// glibc `strncmp` over NUL-terminated views (`a`/`b` hold no interior NUL).
fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let mut i: usize = 0;
    while i < n {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/// `strstr(haystack, needle) != NULL`
fn c_strstr(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// C `atoi`, i.e. `(int)strtol(s, NULL, 10)` with glibc's saturation at
/// `LONG_MIN`/`LONG_MAX` followed by truncation to `int`.
fn c_atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    // Accumulate the magnitude, saturating the way strtol does.
    let limit: i128 = if neg {
        -(i64::MIN as i128)
    } else {
        i64::MAX as i128
    };
    let mut acc: i128 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        if !saturated {
            acc = acc * 10 + (s[i] - b'0') as i128;
            if acc > limit {
                acc = limit;
                saturated = true;
            }
        }
        i += 1;
    }
    let wide: i64 = if neg { (-acc) as i64 } else { acc as i64 };
    wide as i32
}

// ---------------------------------------------------------------------------
// Flat memory emulating a C array of fixed-layout structs
// ---------------------------------------------------------------------------

struct Mem {
    b: Vec<u8>,
}

impl Mem {
    fn new(len: usize) -> Mem {
        Mem { b: vec![0u8; len] }
    }

    /// The NUL-terminated string starting at `off`.
    fn cstr(&self, off: usize) -> &[u8] {
        let end = match self.b[off..].iter().position(|&c| c == 0) {
            Some(p) => off + p,
            None => self.b.len(),
        };
        &self.b[off..end]
    }

    /// `strcpy(base + off, src)`
    fn strcpy(&mut self, off: usize, src: &[u8]) {
        let end = off + src.len();
        self.b[off..end].copy_from_slice(src);
        self.b[end] = 0;
    }

    fn set_u8(&mut self, off: usize, v: u8) {
        self.b[off] = v;
    }

    fn get_i32(&self, off: usize) -> i32 {
        let mut raw = [0u8; 4];
        raw.copy_from_slice(&self.b[off..off + 4]);
        i32::from_ne_bytes(raw)
    }

    fn set_i32(&mut self, off: usize, v: i32) {
        self.b[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }

    /// `dst_struct = src_struct` for structs of `len` bytes.
    fn copy_struct(&mut self, dst: usize, src: usize, len: usize) {
        self.b.copy_within(src..src + len, dst);
    }
}

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

struct State {
    users: Mem,
    user_count: i32,
    /// Index of `current_user` within `users`; `None` models a NULL pointer.
    current_user: Option<usize>,

    files: Mem,
    file_count: i32,

    variables: Mem,
    variable_count: i32,

    debug_mode: i32,
    verbose_mode: i32,
}

impl State {
    fn new() -> State {
        State {
            users: Mem::new(U_SIZE * MAX_USERS as usize + SLACK),
            user_count: 0,
            current_user: None,
            files: Mem::new(F_SIZE * MAX_FILES as usize + SLACK),
            file_count: 0,
            variables: Mem::new(V_SIZE * MAX_VARIABLES as usize + SLACK),
            variable_count: 0,
            debug_mode: 0,
            verbose_mode: 0,
        }
    }

    // -- helpers mirroring `current_user->field` -------------------------

    fn cu_name(&self) -> &[u8] {
        let i = self.current_user.expect("current_user is NULL");
        self.users.cstr(i * U_SIZE + U_NAME)
    }

    fn cu_perm(&self) -> i32 {
        let i = self.current_user.expect("current_user is NULL");
        self.users.get_i32(i * U_SIZE + U_PERM)
    }

    /// `current_user && current_user->logged_in`
    fn logged_in(&self) -> bool {
        match self.current_user {
            None => false,
            Some(i) => self.users.get_i32(i * U_SIZE + U_LOGGED) != 0,
        }
    }

    // ---------------------------------------------------------------
    // User management commands
    // ---------------------------------------------------------------

    fn cmd_adduser(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        let arg_count = args.len();
        if arg_count < 2 {
            o.t("Usage: adduser <username> <password> [permission_level]\n");
            return;
        }

        if self.user_count >= MAX_USERS {
            o.t("Error: Maximum users reached\n");
            return;
        }

        // Check if user already exists using strcmp
        for i in 0..self.user_count as usize {
            if c_strcmp(self.users.cstr(i * U_SIZE + U_NAME), &args[0]) == 0 {
                o.t("Error: User '");
                o.s(&args[0]);
                o.t("' already exists\n");
                return;
            }
        }

        let base = self.user_count as usize * U_SIZE;
        self.users.strcpy(base + U_NAME, &args[0]);
        self.users.strcpy(base + U_PASS, &args[1]);
        let level = if arg_count >= 3 { c_atoi(&args[2]) } else { 1 };
        self.users.set_i32(base + U_PERM, level);
        self.users.set_i32(base + U_LOGGED, 0);
        self.user_count += 1;

        o.t("User '");
        o.s(&args[0]);
        o.t("' added with permission level ");
        o.d(self.users.get_i32(base + U_PERM));
        o.t("\n");
    }

    fn cmd_login(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.len() < 2 {
            o.t("Usage: login <username> <password>\n");
            return;
        }

        if self.logged_in() {
            o.t("Error: User '");
            o.s(self.cu_name());
            o.t("' already logged in. Use 'logout' first.\n");
            return;
        }

        // Find user and verify password using strcmp
        for i in 0..self.user_count as usize {
            if c_strcmp(self.users.cstr(i * U_SIZE + U_NAME), &args[0]) == 0 {
                if c_strcmp(self.users.cstr(i * U_SIZE + U_PASS), &args[1]) == 0 {
                    self.users.set_i32(i * U_SIZE + U_LOGGED, 1);
                    self.current_user = Some(i);
                    o.t("Login successful. Welcome, ");
                    o.s(self.cu_name());
                    o.t("!\n");
                    return;
                } else {
                    o.t("Error: Incorrect password\n");
                    return;
                }
            }
        }

        o.t("Error: User not found\n");
    }

    fn cmd_logout(&mut self, o: &mut Out) {
        if !self.logged_in() {
            o.t("Error: No user logged in\n");
            return;
        }

        o.t("Goodbye, ");
        o.s(self.cu_name());
        o.t("!\n");
        let i = self.current_user.unwrap();
        self.users.set_i32(i * U_SIZE + U_LOGGED, 0);
        self.current_user = None;
    }

    fn cmd_whoami(&mut self, o: &mut Out) {
        if !self.logged_in() {
            o.t("Not logged in\n");
            return;
        }

        o.t("Current user: ");
        o.s(self.cu_name());
        o.t("\n");
        o.t("Permission level: ");
        o.d(self.cu_perm());
        o.t("\n");
    }

    fn cmd_listusers(&mut self, o: &mut Out) {
        if self.user_count == 0 {
            o.t("No users registered\n");
            return;
        }

        o.t("Registered users:\n");
        for i in 0..self.user_count as usize {
            o.t("  ");
            o.s(self.users.cstr(i * U_SIZE + U_NAME));
            o.t(" (level ");
            o.d(self.users.get_i32(i * U_SIZE + U_PERM));
            o.t(") ");
            if self.users.get_i32(i * U_SIZE + U_LOGGED) != 0 {
                o.t("[logged in]");
            }
            o.t("\n");
        }
    }

    // ---------------------------------------------------------------
    // File management commands
    // ---------------------------------------------------------------

    fn cmd_createfile(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if !self.logged_in() {
            o.t("Error: Must be logged in\n");
            return;
        }

        let arg_count = args.len();
        if arg_count < 1 {
            o.t("Usage: createfile <filename> [content]\n");
            return;
        }

        if self.file_count >= MAX_FILES {
            o.t("Error: Maximum files reached\n");
            return;
        }

        // Check if file exists using strcmp
        for i in 0..self.file_count as usize {
            if c_strcmp(self.files.cstr(i * F_SIZE + F_NAME), &args[0]) == 0 {
                o.t("Error: File '");
                o.s(&args[0]);
                o.t("' already exists\n");
                return;
            }
        }

        let base = self.file_count as usize * F_SIZE;
        self.files.strcpy(base + F_NAME, &args[0]);
        let owner = self.cu_name().to_vec();
        self.files.strcpy(base + F_OWNER, &owner);
        self.files.set_i32(base + F_PERM, 755);

        if arg_count >= 2 {
            self.files.strcpy(base + F_CONTENT, &args[1]);
        } else {
            // files[file_count].content[0] = '\0';
            self.files.set_u8(base + F_CONTENT, 0);
        }

        self.file_count += 1;
        o.t("File '");
        o.s(&args[0]);
        o.t("' created\n");
    }

    fn cmd_readfile(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Usage: readfile <filename>\n");
            return;
        }

        // Find file using strcmp
        for i in 0..self.file_count as usize {
            if c_strcmp(self.files.cstr(i * F_SIZE + F_NAME), &args[0]) == 0 {
                o.t("=== ");
                o.s(self.files.cstr(i * F_SIZE + F_NAME));
                o.t(" ===\n");
                o.t("Owner: ");
                o.s(self.files.cstr(i * F_SIZE + F_OWNER));
                o.t("\n");
                o.t("Permissions: ");
                o.d(self.files.get_i32(i * F_SIZE + F_PERM));
                o.t("\n");
                o.t("Content: ");
                o.s(self.files.cstr(i * F_SIZE + F_CONTENT));
                o.t("\n");
                return;
            }
        }

        o.t("Error: File '");
        o.s(&args[0]);
        o.t("' not found\n");
    }

    fn cmd_writefile(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if !self.logged_in() {
            o.t("Error: Must be logged in\n");
            return;
        }

        if args.len() < 2 {
            o.t("Usage: writefile <filename> <content>\n");
            return;
        }

        // Find file and check ownership
        for i in 0..self.file_count as usize {
            if c_strcmp(self.files.cstr(i * F_SIZE + F_NAME), &args[0]) == 0 {
                // Check if current user owns the file
                if c_strcmp(self.files.cstr(i * F_SIZE + F_OWNER), self.cu_name()) == 0
                    || self.cu_perm() >= 5
                {
                    self.files.strcpy(i * F_SIZE + F_CONTENT, &args[1]);
                    o.t("File '");
                    o.s(&args[0]);
                    o.t("' updated\n");
                    return;
                } else {
                    o.t("Error: Permission denied\n");
                    return;
                }
            }
        }

        o.t("Error: File '");
        o.s(&args[0]);
        o.t("' not found\n");
    }

    fn cmd_deletefile(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if !self.logged_in() {
            o.t("Error: Must be logged in\n");
            return;
        }

        if args.is_empty() {
            o.t("Usage: deletefile <filename>\n");
            return;
        }

        // Find and delete file
        for i in 0..self.file_count as usize {
            if c_strcmp(self.files.cstr(i * F_SIZE + F_NAME), &args[0]) == 0 {
                if c_strcmp(self.files.cstr(i * F_SIZE + F_OWNER), self.cu_name()) == 0
                    || self.cu_perm() >= 9
                {
                    // Shift remaining files
                    let mut j = i;
                    while j < self.file_count as usize - 1 {
                        self.files
                            .copy_struct(j * F_SIZE, (j + 1) * F_SIZE, F_SIZE);
                        j += 1;
                    }
                    self.file_count -= 1;
                    o.t("File '");
                    o.s(&args[0]);
                    o.t("' deleted\n");
                    return;
                } else {
                    o.t("Error: Permission denied\n");
                    return;
                }
            }
        }

        o.t("Error: File '");
        o.s(&args[0]);
        o.t("' not found\n");
    }

    fn cmd_listfiles(&mut self, o: &mut Out) {
        if self.file_count == 0 {
            o.t("No files\n");
            return;
        }

        o.t("Files:\n");
        for i in 0..self.file_count as usize {
            o.t("  ");
            o.s(self.files.cstr(i * F_SIZE + F_NAME));
            o.t(" (owner: ");
            o.s(self.files.cstr(i * F_SIZE + F_OWNER));
            o.t(", perm: ");
            o.d(self.files.get_i32(i * F_SIZE + F_PERM));
            o.t(")\n");
        }
    }

    // ---------------------------------------------------------------
    // Variable commands
    // ---------------------------------------------------------------

    fn cmd_set(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.len() < 2 {
            o.t("Usage: set <name> <value>\n");
            return;
        }

        // Check if variable exists
        for i in 0..self.variable_count as usize {
            if c_strcmp(self.variables.cstr(i * V_SIZE + V_NAME), &args[0]) == 0 {
                self.variables.strcpy(i * V_SIZE + V_VALUE, &args[1]);
                o.t("Variable '");
                o.s(&args[0]);
                o.t("' updated\n");
                return;
            }
        }

        // Create new variable
        if self.variable_count >= MAX_VARIABLES {
            o.t("Error: Maximum variables reached\n");
            return;
        }

        let base = self.variable_count as usize * V_SIZE;
        self.variables.strcpy(base + V_NAME, &args[0]);
        self.variables.strcpy(base + V_VALUE, &args[1]);
        self.variable_count += 1;
        o.t("Variable '");
        o.s(&args[0]);
        o.t("' set\n");
    }

    fn cmd_get(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Usage: get <name>\n");
            return;
        }

        for i in 0..self.variable_count as usize {
            if c_strcmp(self.variables.cstr(i * V_SIZE + V_NAME), &args[0]) == 0 {
                o.s(self.variables.cstr(i * V_SIZE + V_NAME));
                o.t(" = ");
                o.s(self.variables.cstr(i * V_SIZE + V_VALUE));
                o.t("\n");
                return;
            }
        }

        o.t("Error: Variable '");
        o.s(&args[0]);
        o.t("' not found\n");
    }

    fn cmd_unset(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Usage: unset <name>\n");
            return;
        }

        for i in 0..self.variable_count as usize {
            if c_strcmp(self.variables.cstr(i * V_SIZE + V_NAME), &args[0]) == 0 {
                let mut j = i;
                while j < self.variable_count as usize - 1 {
                    self.variables
                        .copy_struct(j * V_SIZE, (j + 1) * V_SIZE, V_SIZE);
                    j += 1;
                }
                self.variable_count -= 1;
                o.t("Variable '");
                o.s(&args[0]);
                o.t("' unset\n");
                return;
            }
        }

        o.t("Error: Variable '");
        o.s(&args[0]);
        o.t("' not found\n");
    }

    fn cmd_listvars(&mut self, o: &mut Out) {
        if self.variable_count == 0 {
            o.t("No variables set\n");
            return;
        }

        o.t("Variables:\n");
        for i in 0..self.variable_count as usize {
            o.t("  ");
            o.s(self.variables.cstr(i * V_SIZE + V_NAME));
            o.t(" = ");
            o.s(self.variables.cstr(i * V_SIZE + V_VALUE));
            o.t("\n");
        }
    }

    // ---------------------------------------------------------------
    // System commands
    // ---------------------------------------------------------------

    fn cmd_debug(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Debug mode: ");
            o.t(if self.debug_mode != 0 { "ON" } else { "OFF" });
            o.t("\n");
            return;
        }

        if c_strcmp(&args[0], &b"on"[..]) == 0 {
            self.debug_mode = 1;
            o.t("Debug mode enabled\n");
        } else if c_strcmp(&args[0], &b"off"[..]) == 0 {
            self.debug_mode = 0;
            o.t("Debug mode disabled\n");
        } else {
            o.t("Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Verbose mode: ");
            o.t(if self.verbose_mode != 0 { "ON" } else { "OFF" });
            o.t("\n");
            return;
        }

        if c_strcmp(&args[0], &b"on"[..]) == 0 {
            self.verbose_mode = 1;
            o.t("Verbose mode enabled\n");
        } else if c_strcmp(&args[0], &b"off"[..]) == 0 {
            self.verbose_mode = 0;
            o.t("Verbose mode disabled\n");
        } else {
            o.t("Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&mut self, o: &mut Out) {
        o.t("\n=== System Status ===\n");
        o.t("Users: ");
        o.d(self.user_count);
        o.t("/");
        o.d(MAX_USERS);
        o.t("\n");
        o.t("Files: ");
        o.d(self.file_count);
        o.t("/");
        o.d(MAX_FILES);
        o.t("\n");
        o.t("Variables: ");
        o.d(self.variable_count);
        o.t("/");
        o.d(MAX_VARIABLES);
        o.t("\n");
        o.t("Current user: ");
        if self.logged_in() {
            o.s(self.cu_name());
        } else {
            o.t("none");
        }
        o.t("\n");
        o.t("Debug mode: ");
        o.t(if self.debug_mode != 0 { "ON" } else { "OFF" });
        o.t("\n");
        o.t("Verbose mode: ");
        o.t(if self.verbose_mode != 0 { "ON" } else { "OFF" });
        o.t("\n");
    }
}

// ---------------------------------------------------------------------------
// String comparison commands (no global state)
// ---------------------------------------------------------------------------

fn cmd_compare(o: &mut Out, args: &[Vec<u8>]) {
    if args.len() < 2 {
        o.t("Usage: compare <string1> <string2>\n");
        return;
    }

    let result = c_strcmp(&args[0], &args[1]);

    o.t("strcmp('");
    o.s(&args[0]);
    o.t("', '");
    o.s(&args[1]);
    o.t("') = ");
    o.d(result);
    o.t("\n");

    if result == 0 {
        o.t("Strings are equal\n");
    } else if result < 0 {
        o.t("'");
        o.s(&args[0]);
        o.t("' < '");
        o.s(&args[1]);
        o.t("'\n");
    } else {
        o.t("'");
        o.s(&args[0]);
        o.t("' > '");
        o.s(&args[1]);
        o.t("'\n");
    }
}

fn cmd_compare_n(o: &mut Out, args: &[Vec<u8>]) {
    if args.len() < 3 {
        o.t("Usage: compareN <string1> <string2> <n>\n");
        return;
    }

    let n = c_atoi(&args[2]);
    // int -> size_t conversion, as in the C call.
    let result = c_strncmp(&args[0], &args[1], n as i64 as u64 as usize);

    o.t("strncmp('");
    o.s(&args[0]);
    o.t("', '");
    o.s(&args[1]);
    o.t("', ");
    o.d(n);
    o.t(") = ");
    o.d(result);
    o.t("\n");

    if result == 0 {
        o.t("First ");
        o.d(n);
        o.t(" characters are equal\n");
    } else if result < 0 {
        o.t("'");
        o.s(&args[0]);
        o.t("' < '");
        o.s(&args[1]);
        o.t("' (first ");
        o.d(n);
        o.t(" chars)\n");
    } else {
        o.t("'");
        o.s(&args[0]);
        o.t("' > '");
        o.s(&args[1]);
        o.t("' (first ");
        o.d(n);
        o.t(" chars)\n");
    }
}

fn cmd_startswith(o: &mut Out, args: &[Vec<u8>]) {
    if args.len() < 2 {
        o.t("Usage: startswith <string> <prefix>\n");
        return;
    }

    let prefix_len = args[1].len();

    if c_strncmp(&args[0], &args[1], prefix_len) == 0 {
        o.t("'");
        o.s(&args[0]);
        o.t("' starts with '");
        o.s(&args[1]);
        o.t("'\n");
    } else {
        o.t("'");
        o.s(&args[0]);
        o.t("' does not start with '");
        o.s(&args[1]);
        o.t("'\n");
    }
}

fn cmd_match(o: &mut Out, args: &[Vec<u8>]) {
    if args.len() < 2 {
        o.t("Usage: match <pattern> <string1> [string2] ...\n");
        return;
    }

    o.t("Matching pattern '");
    o.s(&args[0]);
    o.t("':\n");
    let mut matches = 0i32;

    for i in 1..args.len() {
        if c_strcmp(&args[0], &args[i]) == 0 {
            o.t("  '");
            o.s(&args[i]);
            o.t("' - EXACT MATCH\n");
            matches += 1;
        } else if c_strstr(&args[i], &args[0]) {
            o.t("  '");
            o.s(&args[i]);
            o.t("' - contains pattern\n");
            matches += 1;
        } else {
            o.t("  '");
            o.s(&args[i]);
            o.t("' - no match\n");
        }
    }

    o.t("Total matches: ");
    o.d(matches);
    o.t("\n");
}

fn cmd_help(o: &mut Out) {
    o.t("\n=== Command Interpreter Help ===\n");
    o.t("User Management:\n");
    o.t("  adduser <user> <pass> [level] - Add new user\n");
    o.t("  login <user> <pass>            - Login as user\n");
    o.t("  logout                         - Logout current user\n");
    o.t("  whoami                         - Show current user\n");
    o.t("  listusers                      - List all users\n");
    o.t("\nFile Management:\n");
    o.t("  createfile <name> [content]    - Create file\n");
    o.t("  readfile <name>                - Read file\n");
    o.t("  writefile <name> <content>     - Write to file\n");
    o.t("  deletefile <name>              - Delete file\n");
    o.t("  listfiles                      - List all files\n");
    o.t("\nVariable Management:\n");
    o.t("  set <name> <value>             - Set variable\n");
    o.t("  get <name>                     - Get variable\n");
    o.t("  unset <name>                   - Unset variable\n");
    o.t("  listvars                       - List all variables\n");
    o.t("\nString Operations:\n");
    o.t("  compare <str1> <str2>          - Compare strings\n");
    o.t("  compareN <str1> <str2> <n>     - Compare first N chars\n");
    o.t("  startswith <str> <prefix>      - Check if starts with\n");
    o.t("  match <pattern> <str> ...      - Match pattern\n");
    o.t("\nSystem:\n");
    o.t("  debug [on|off]                 - Toggle debug mode\n");
    o.t("  verbose [on|off]               - Toggle verbose mode\n");
    o.t("  status                         - Show system status\n");
    o.t("  time                           - Show current time\n");
    o.t("  help                           - Show this help\n");
    o.t("  exit                           - Exit program\n");
}

extern "C" {
    fn time(t: *mut i64) -> i64;
    fn ctime(t: *const i64) -> *const u8;
}

/// `time_t now = time(NULL); printf("Current time: %s", ctime(&now));`
fn cmd_time(o: &mut Out) {
    let now: i64 = unsafe { time(std::ptr::null_mut()) };
    let p = unsafe { ctime(&now as *const i64) };
    o.t("Current time: ");
    if p.is_null() {
        // glibc printf renders a NULL "%s" argument as "(null)".
        o.t("(null)");
    } else {
        let mut len = 0usize;
        // SAFETY: `ctime` returns a NUL-terminated static buffer.
        while unsafe { *p.add(len) } != 0 {
            len += 1;
        }
        let bytes = unsafe { std::slice::from_raw_parts(p, len) };
        o.s(bytes);
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

fn is_delim(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

/// `parse_command`: `strtok` on " \t", truncating every token to
/// `MAX_COMMAND - 1` bytes and keeping at most `MAX_ARGS` arguments.
///
/// When the input holds no token at all, the C code leaves `process_command`'s
/// uninitialised `command` buffer untouched and then tests
/// `strlen(command) == 0`, which is undefined behaviour.  The reference build
/// (the supplied CMakeLists.txt sets no CMAKE_BUILD_TYPE, so the compiler runs
/// unoptimised) always observes an empty buffer there, because the
/// `printf("> ")` call in `main` reuses and clears that stack region on every
/// iteration; blank input lines are therefore no-ops.  This translation
/// reproduces that observable behaviour by returning an empty command.
/// (Only at -O2 and above does the stale previous command survive in that
/// buffer and get re-executed.)
fn parse_command(input: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    // strncpy(temp, input, MAX_INPUT - 1); temp[MAX_INPUT - 1] = '\0';
    let temp: &[u8] = if input.len() > MAX_INPUT - 1 {
        &input[..MAX_INPUT - 1]
    } else {
        input
    };

    let mut tokens: Vec<&[u8]> = Vec::new();
    let mut i = 0usize;
    while i < temp.len() {
        while i < temp.len() && is_delim(temp[i]) {
            i += 1;
        }
        if i >= temp.len() {
            break;
        }
        let start = i;
        while i < temp.len() && !is_delim(temp[i]) {
            i += 1;
        }
        tokens.push(&temp[start..i]);
    }

    let truncate = |t: &[u8]| -> Vec<u8> {
        if t.len() > MAX_COMMAND - 1 {
            t[..MAX_COMMAND - 1].to_vec()
        } else {
            t.to_vec()
        }
    };

    let mut cmd: Vec<u8> = Vec::new();
    let mut args: Vec<Vec<u8>> = Vec::new();

    if let Some(first) = tokens.first() {
        cmd = truncate(first);
        for t in tokens.iter().skip(1) {
            if args.len() >= MAX_ARGS {
                break;
            }
            args.push(truncate(t));
        }
    }

    (cmd, args)
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

fn process_command(state: &mut State, o: &mut Out, input: &[u8]) {
    let (command, args) = parse_command(input);
    let arg_count = args.len() as i32;

    if command.is_empty() {
        return;
    }

    if state.debug_mode != 0 {
        o.t("[DEBUG] Command: '");
        o.s(&command);
        o.t("', Args: ");
        o.d(arg_count);
        o.t("\n");
    }

    let c = &command[..];

    // Command routing using strcmp and strncmp
    // User commands
    if c_strcmp(c, &b"adduser"[..]) == 0 {
        state.cmd_adduser(o, &args);
    } else if c_strcmp(c, &b"login"[..]) == 0 {
        state.cmd_login(o, &args);
    } else if c_strcmp(c, &b"logout"[..]) == 0 {
        state.cmd_logout(o);
    } else if c_strcmp(c, &b"whoami"[..]) == 0 {
        state.cmd_whoami(o);
    } else if c_strcmp(c, &b"listusers"[..]) == 0 || c_strcmp(c, &b"users"[..]) == 0 {
        state.cmd_listusers(o);
    }
    // File commands
    else if c_strcmp(c, &b"createfile"[..]) == 0 || c_strcmp(c, &b"touch"[..]) == 0 {
        state.cmd_createfile(o, &args);
    } else if c_strcmp(c, &b"readfile"[..]) == 0 || c_strcmp(c, &b"cat"[..]) == 0 {
        state.cmd_readfile(o, &args);
    } else if c_strcmp(c, &b"writefile"[..]) == 0 || c_strcmp(c, &b"write"[..]) == 0 {
        state.cmd_writefile(o, &args);
    } else if c_strcmp(c, &b"deletefile"[..]) == 0 || c_strcmp(c, &b"rm"[..]) == 0 {
        state.cmd_deletefile(o, &args);
    } else if c_strcmp(c, &b"listfiles"[..]) == 0 || c_strcmp(c, &b"ls"[..]) == 0 {
        state.cmd_listfiles(o);
    }
    // Variable commands
    else if c_strcmp(c, &b"set"[..]) == 0 {
        state.cmd_set(o, &args);
    } else if c_strcmp(c, &b"get"[..]) == 0 {
        state.cmd_get(o, &args);
    } else if c_strcmp(c, &b"unset"[..]) == 0 {
        state.cmd_unset(o, &args);
    } else if c_strcmp(c, &b"listvars"[..]) == 0 || c_strcmp(c, &b"vars"[..]) == 0 {
        state.cmd_listvars(o);
    }
    // String comparison commands
    else if c_strcmp(c, &b"compare"[..]) == 0 || c_strcmp(c, &b"cmp"[..]) == 0 {
        cmd_compare(o, &args);
    } else if c_strcmp(c, &b"compareN"[..]) == 0 || c_strcmp(c, &b"cmpn"[..]) == 0 {
        cmd_compare_n(o, &args);
    } else if c_strcmp(c, &b"startswith"[..]) == 0 {
        cmd_startswith(o, &args);
    } else if c_strcmp(c, &b"match"[..]) == 0 {
        cmd_match(o, &args);
    }
    // System commands
    else if c_strcmp(c, &b"debug"[..]) == 0 {
        state.cmd_debug(o, &args);
    } else if c_strcmp(c, &b"verbose"[..]) == 0 {
        state.cmd_verbose(o, &args);
    } else if c_strcmp(c, &b"status"[..]) == 0 {
        state.cmd_status(o);
    } else if c_strcmp(c, &b"time"[..]) == 0 {
        cmd_time(o);
    } else if c_strcmp(c, &b"help"[..]) == 0 || c_strcmp(c, &b"?"[..]) == 0 {
        cmd_help(o);
    } else if c_strcmp(c, &b"exit"[..]) == 0 || c_strcmp(c, &b"quit"[..]) == 0 {
        o.t("Goodbye!\n");
        o.flush();
        std::process::exit(0);
    }
    // Check for partial matches using strncmp
    else if c_strncmp(c, &b"add"[..], 3) == 0 {
        o.t("Did you mean 'adduser'?\n");
    } else if c_strncmp(c, &b"log"[..], 3) == 0 {
        o.t("Did you mean 'login' or 'logout'?\n");
    } else if c_strncmp(c, &b"list"[..], 4) == 0 {
        o.t("Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
    } else if c_strncmp(c, &b"create"[..], 6) == 0 {
        o.t("Did you mean 'createfile'?\n");
    } else if c_strncmp(c, &b"read"[..], 4) == 0 {
        o.t("Did you mean 'readfile'?\n");
    } else if c_strncmp(c, &b"write"[..], 5) == 0 {
        o.t("Did you mean 'writefile'?\n");
    } else if c_strncmp(c, &b"delete"[..], 6) == 0 {
        o.t("Did you mean 'deletefile'?\n");
    } else {
        o.t("Unknown command: '");
        o.s(c);
        o.t("'. Type 'help' for available commands.\n");
    }
}

// ---------------------------------------------------------------------------
// stdin handling
// ---------------------------------------------------------------------------

/// `fgets(buf, size, stdin)`: read at most `size - 1` bytes, stopping right
/// after a newline.  Returns `None` for the NULL return (EOF/error with
/// nothing read).
fn c_fgets<R: Read>(r: &mut R, size: usize) -> Option<Vec<u8>> {
    let mut v: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    while v.len() + 1 < size {
        match r.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                v.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn main() {
    let mut o = Out::new();
    let mut state = State::new();
    let mut stdin = BufReader::new(io::stdin());

    o.t("|----------------------------------------|\n");
    o.t("|   COMMAND INTERPRETER                  |\n");
    o.t("|   strcmp/strncmp demonstration         |\n");
    o.t("|----------------------------------------|\n");
    o.t("Type 'help' for available commands\n\n");

    loop {
        o.t("> ");
        // Keep the prompt visible before blocking on input, matching the
        // line-buffered behaviour of C's stdout on a terminal.  The emitted
        // byte stream is unaffected.
        o.flush();

        let raw = match c_fgets(&mut stdin, MAX_INPUT) {
            Some(v) => v,
            None => break,
        };

        // The buffer is only ever looked at as a C string: it ends at the
        // first NUL byte.
        let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        let s = &raw[..end];

        // input[strcspn(input, "\n")] = 0;
        let end = s.iter().position(|&c| c == b'\n').unwrap_or(s.len());
        let input = s[..end].to_vec();

        if state.verbose_mode != 0 {
            o.t("[VERBOSE] Processing: '");
            o.s(&input);
            o.t("'\n");
        }

        process_command(&mut state, &mut o, &input);
    }

    o.flush();
}
