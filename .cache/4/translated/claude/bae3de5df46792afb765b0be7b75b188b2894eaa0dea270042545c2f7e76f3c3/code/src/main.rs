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
//! The original C program keeps its state in fixed-size `char` arrays inside
//! plain C structs and copies into them with `strcpy`, which happily overruns
//! the individual members.  To reproduce the original byte-for-byte output --
//! including the overflow artifacts -- the global tables are modelled here as
//! flat byte buffers laid out exactly like the C structs.

use std::io::{BufRead, BufReader, Read, Write};

// ---------------------------------------------------------------------------
// #define constants
// ---------------------------------------------------------------------------

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

// ---------------------------------------------------------------------------
// struct layouts (x86-64 SysV, as produced by the reference compiler)
//
//   typedef struct { char name[32]; char password[32];
//                    int permission_level; int logged_in; } user_t;   // 72
//   typedef struct { char filename[64]; char content[512];
//                    char owner[32]; int permissions; } file_t;       // 612
//   typedef struct { char name[32]; char value[128]; } variable_t;    // 160
// ---------------------------------------------------------------------------

const USER_SIZE: usize = 72;
const U_NAME: usize = 0;
const U_PASSWORD: usize = 32;
const U_PERM_LEVEL: usize = 64;
const U_LOGGED_IN: usize = 68;

const FILE_SIZE: usize = 612;
const F_FILENAME: usize = 0;
const F_CONTENT: usize = 64;
const F_OWNER: usize = 576;
const F_PERMISSIONS: usize = 608;

const VAR_SIZE: usize = 160;
const V_NAME: usize = 0;
const V_VALUE: usize = 32;

/// Extra slack so that `strcpy` overruns past the end of a table (which the C
/// program performs into whatever `.bss` object follows) never panic.
const SLACK: usize = 512;

// ---------------------------------------------------------------------------
// C string helpers operating on flat byte buffers
// ---------------------------------------------------------------------------

/// `strcpy(buf + off, src)` -- writes `src` followed by a NUL terminator.
fn c_strcpy(buf: &mut [u8], off: usize, src: &[u8]) {
    let mut i = 0;
    while i < src.len() {
        if off + i >= buf.len() {
            return;
        }
        buf[off + i] = src[i];
        i += 1;
    }
    if off + i < buf.len() {
        buf[off + i] = 0;
    }
}

/// Reads the NUL-terminated string that starts at `buf + off`.
fn c_str(buf: &[u8], off: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = off;
    while i < buf.len() && buf[i] != 0 {
        out.push(buf[i]);
        i += 1;
    }
    out
}

/// Reads a little-endian `int` from `buf + off`.
fn c_int(buf: &[u8], off: usize) -> i32 {
    let mut b = [0u8; 4];
    for k in 0..4 {
        if off + k < buf.len() {
            b[k] = buf[off + k];
        }
    }
    i32::from_le_bytes(b)
}

/// Writes a little-endian `int` to `buf + off`.
fn c_set_int(buf: &mut [u8], off: usize, v: i32) {
    let b = v.to_le_bytes();
    for k in 0..4 {
        if off + k < buf.len() {
            buf[off + k] = b[k];
        }
    }
}

/// Copies `len` raw bytes from `src_off` to `dst_off` (struct assignment).
fn c_struct_copy(buf: &mut [u8], dst_off: usize, src_off: usize, len: usize) {
    for k in 0..len {
        if dst_off + k < buf.len() && src_off + k < buf.len() {
            buf[dst_off + k] = buf[src_off + k];
        }
    }
}

/// glibc `strcmp`: returns the difference of the first differing bytes,
/// interpreted as `unsigned char`.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let n = a.len().min(b.len());
    for i in 0..n {
        if a[i] != b[i] {
            return a[i] as i32 - b[i] as i32;
        }
    }
    if a.len() == b.len() {
        0
    } else if a.len() < b.len() {
        // a ran out: implicit NUL vs b[a.len()]
        -(b[a.len()] as i32)
    } else {
        a[b.len()] as i32
    }
}

/// glibc `strncmp` with the same "byte difference" return convention.
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
fn c_strstr_found(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|w| w == needle)
}

/// glibc `atoi` == `(int)strtol(s, NULL, 10)`, including the LONG_MAX /
/// LONG_MIN saturation before the truncating cast to `int`.
fn c_atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let mut acc: u128 = 0;
    let mut overflow = false;
    let cutoff: u128 = if negative {
        1u128 << 63 // |LONG_MIN|
    } else {
        (1u128 << 63) - 1 // LONG_MAX
    };
    while i < s.len() && s[i].is_ascii_digit() {
        if !overflow {
            acc = acc * 10 + (s[i] - b'0') as u128;
            if acc > cutoff {
                overflow = true;
            }
        }
        i += 1;
    }
    let as_long: i64 = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };
    as_long as i32
}

// ---------------------------------------------------------------------------
// printf("%s"/"%d") emulation over raw bytes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum A<'a> {
    S(&'a [u8]),
    I(i32),
}

fn cfmt(fmt: &str, args: &[A]) -> Vec<u8> {
    let f = fmt.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(f.len() + 16);
    let mut ai = 0usize;
    let mut i = 0usize;
    while i < f.len() {
        if f[i] == b'%' && i + 1 < f.len() {
            match f[i + 1] {
                b's' => {
                    if let Some(A::S(s)) = args.get(ai) {
                        out.extend_from_slice(s);
                    }
                    ai += 1;
                    i += 2;
                }
                b'd' => {
                    if let Some(A::I(v)) = args.get(ai) {
                        out.extend_from_slice(v.to_string().as_bytes());
                    }
                    ai += 1;
                    i += 2;
                }
                b'%' => {
                    out.push(b'%');
                    i += 2;
                }
                _ => {
                    out.push(f[i]);
                    i += 1;
                }
            }
        } else {
            out.push(f[i]);
            i += 1;
        }
    }
    out
}

macro_rules! pf {
    ($st:expr, $fmt:expr) => {{
        $st.emit($fmt.as_bytes());
    }};
    ($st:expr, $fmt:expr, $($a:expr),+ $(,)?) => {{
        let __tmp = cfmt($fmt, &[$($a),+]);
        $st.emit(&__tmp);
    }};
}

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

struct State {
    users: Vec<u8>,
    user_count: usize,
    current_user: Option<usize>, // index into `users`

    files: Vec<u8>,
    file_count: usize,

    variables: Vec<u8>,
    variable_count: usize,

    debug_mode: i32,
    verbose_mode: i32,

    out: Vec<u8>,
}

impl State {
    fn new() -> State {
        State {
            users: vec![0u8; MAX_USERS * USER_SIZE + SLACK],
            user_count: 0,
            current_user: None,
            files: vec![0u8; MAX_FILES * FILE_SIZE + SLACK],
            file_count: 0,
            variables: vec![0u8; MAX_VARIABLES * VAR_SIZE + SLACK],
            variable_count: 0,
            debug_mode: 0,
            verbose_mode: 0,
            out: Vec::with_capacity(1 << 16),
        }
    }

    fn emit(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
        if self.out.len() >= (1 << 16) {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if !self.out.is_empty() {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            let _ = lock.write_all(&self.out);
            let _ = lock.flush();
            self.out.clear();
        }
    }

    // --- accessors -------------------------------------------------------

    fn user_name(&self, i: usize) -> Vec<u8> {
        c_str(&self.users, i * USER_SIZE + U_NAME)
    }
    fn user_password(&self, i: usize) -> Vec<u8> {
        c_str(&self.users, i * USER_SIZE + U_PASSWORD)
    }
    fn user_perm(&self, i: usize) -> i32 {
        c_int(&self.users, i * USER_SIZE + U_PERM_LEVEL)
    }
    fn user_logged_in(&self, i: usize) -> i32 {
        c_int(&self.users, i * USER_SIZE + U_LOGGED_IN)
    }

    fn file_name(&self, i: usize) -> Vec<u8> {
        c_str(&self.files, i * FILE_SIZE + F_FILENAME)
    }
    fn file_content(&self, i: usize) -> Vec<u8> {
        c_str(&self.files, i * FILE_SIZE + F_CONTENT)
    }
    fn file_owner(&self, i: usize) -> Vec<u8> {
        c_str(&self.files, i * FILE_SIZE + F_OWNER)
    }
    fn file_perm(&self, i: usize) -> i32 {
        c_int(&self.files, i * FILE_SIZE + F_PERMISSIONS)
    }

    fn var_name(&self, i: usize) -> Vec<u8> {
        c_str(&self.variables, i * VAR_SIZE + V_NAME)
    }
    fn var_value(&self, i: usize) -> Vec<u8> {
        c_str(&self.variables, i * VAR_SIZE + V_VALUE)
    }

    /// `current_user && current_user->logged_in`
    fn logged_in_user(&self) -> Option<usize> {
        match self.current_user {
            Some(i) if self.user_logged_in(i) != 0 => Some(i),
            _ => None,
        }
    }

    // --- user management -------------------------------------------------

    fn cmd_adduser(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: adduser <username> <password> [permission_level]\n");
            return;
        }

        if self.user_count >= MAX_USERS {
            pf!(self, "Error: Maximum users reached\n");
            return;
        }

        for i in 0..self.user_count {
            if c_strcmp(&self.user_name(i), &args[0]) == 0 {
                pf!(self, "Error: User '%s' already exists\n", A::S(&args[0]));
                return;
            }
        }

        let base = self.user_count * USER_SIZE;
        c_strcpy(&mut self.users, base + U_NAME, &args[0]);
        c_strcpy(&mut self.users, base + U_PASSWORD, &args[1]);
        let level = if arg_count >= 3 { c_atoi(&args[2]) } else { 1 };
        c_set_int(&mut self.users, base + U_PERM_LEVEL, level);
        c_set_int(&mut self.users, base + U_LOGGED_IN, 0);
        self.user_count += 1;

        let shown = self.user_perm(self.user_count - 1);
        pf!(
            self,
            "User '%s' added with permission level %d\n",
            A::S(&args[0]),
            A::I(shown)
        );
    }

    fn cmd_login(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: login <username> <password>\n");
            return;
        }

        if let Some(cu) = self.logged_in_user() {
            let name = self.user_name(cu);
            pf!(
                self,
                "Error: User '%s' already logged in. Use 'logout' first.\n",
                A::S(&name)
            );
            return;
        }

        for i in 0..self.user_count {
            if c_strcmp(&self.user_name(i), &args[0]) == 0 {
                if c_strcmp(&self.user_password(i), &args[1]) == 0 {
                    c_set_int(&mut self.users, i * USER_SIZE + U_LOGGED_IN, 1);
                    self.current_user = Some(i);
                    let name = self.user_name(i);
                    pf!(self, "Login successful. Welcome, %s!\n", A::S(&name));
                    return;
                } else {
                    pf!(self, "Error: Incorrect password\n");
                    return;
                }
            }
        }

        pf!(self, "Error: User not found\n");
    }

    fn cmd_logout(&mut self) {
        let cu = match self.logged_in_user() {
            Some(i) => i,
            None => {
                pf!(self, "Error: No user logged in\n");
                return;
            }
        };

        let name = self.user_name(cu);
        pf!(self, "Goodbye, %s!\n", A::S(&name));
        c_set_int(&mut self.users, cu * USER_SIZE + U_LOGGED_IN, 0);
        self.current_user = None;
    }

    fn cmd_whoami(&mut self) {
        let cu = match self.logged_in_user() {
            Some(i) => i,
            None => {
                pf!(self, "Not logged in\n");
                return;
            }
        };

        let name = self.user_name(cu);
        let perm = self.user_perm(cu);
        pf!(self, "Current user: %s\n", A::S(&name));
        pf!(self, "Permission level: %d\n", A::I(perm));
    }

    fn cmd_listusers(&mut self) {
        if self.user_count == 0 {
            pf!(self, "No users registered\n");
            return;
        }

        pf!(self, "Registered users:\n");
        for i in 0..self.user_count {
            let name = self.user_name(i);
            let perm = self.user_perm(i);
            let flag: &[u8] = if self.user_logged_in(i) != 0 {
                b"[logged in]"
            } else {
                b""
            };
            pf!(
                self,
                "  %s (level %d) %s\n",
                A::S(&name),
                A::I(perm),
                A::S(flag)
            );
        }
    }

    // --- file management -------------------------------------------------

    fn cmd_createfile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        let cu = match self.logged_in_user() {
            Some(i) => i,
            None => {
                pf!(self, "Error: Must be logged in\n");
                return;
            }
        };

        if arg_count < 1 {
            pf!(self, "Usage: createfile <filename> [content]\n");
            return;
        }

        if self.file_count >= MAX_FILES {
            pf!(self, "Error: Maximum files reached\n");
            return;
        }

        for i in 0..self.file_count {
            if c_strcmp(&self.file_name(i), &args[0]) == 0 {
                pf!(self, "Error: File '%s' already exists\n", A::S(&args[0]));
                return;
            }
        }

        let base = self.file_count * FILE_SIZE;
        let owner = self.user_name(cu);
        c_strcpy(&mut self.files, base + F_FILENAME, &args[0]);
        c_strcpy(&mut self.files, base + F_OWNER, &owner);
        c_set_int(&mut self.files, base + F_PERMISSIONS, 755);

        if arg_count >= 2 {
            c_strcpy(&mut self.files, base + F_CONTENT, &args[1]);
        } else if base + F_CONTENT < self.files.len() {
            self.files[base + F_CONTENT] = 0;
        }

        self.file_count += 1;
        pf!(self, "File '%s' created\n", A::S(&args[0]));
    }

    fn cmd_readfile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            pf!(self, "Usage: readfile <filename>\n");
            return;
        }

        for i in 0..self.file_count {
            if c_strcmp(&self.file_name(i), &args[0]) == 0 {
                let name = self.file_name(i);
                let owner = self.file_owner(i);
                let perm = self.file_perm(i);
                let content = self.file_content(i);
                pf!(self, "=== %s ===\n", A::S(&name));
                pf!(self, "Owner: %s\n", A::S(&owner));
                pf!(self, "Permissions: %d\n", A::I(perm));
                pf!(self, "Content: %s\n", A::S(&content));
                return;
            }
        }

        pf!(self, "Error: File '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_writefile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        let cu = match self.logged_in_user() {
            Some(i) => i,
            None => {
                pf!(self, "Error: Must be logged in\n");
                return;
            }
        };

        if arg_count < 2 {
            pf!(self, "Usage: writefile <filename> <content>\n");
            return;
        }

        for i in 0..self.file_count {
            if c_strcmp(&self.file_name(i), &args[0]) == 0 {
                let cu_name = self.user_name(cu);
                if c_strcmp(&self.file_owner(i), &cu_name) == 0 || self.user_perm(cu) >= 5 {
                    c_strcpy(&mut self.files, i * FILE_SIZE + F_CONTENT, &args[1]);
                    pf!(self, "File '%s' updated\n", A::S(&args[0]));
                    return;
                } else {
                    pf!(self, "Error: Permission denied\n");
                    return;
                }
            }
        }

        pf!(self, "Error: File '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_deletefile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        let cu = match self.logged_in_user() {
            Some(i) => i,
            None => {
                pf!(self, "Error: Must be logged in\n");
                return;
            }
        };

        if arg_count < 1 {
            pf!(self, "Usage: deletefile <filename>\n");
            return;
        }

        for i in 0..self.file_count {
            if c_strcmp(&self.file_name(i), &args[0]) == 0 {
                let cu_name = self.user_name(cu);
                if c_strcmp(&self.file_owner(i), &cu_name) == 0 || self.user_perm(cu) >= 9 {
                    for j in i..self.file_count - 1 {
                        c_struct_copy(
                            &mut self.files,
                            j * FILE_SIZE,
                            (j + 1) * FILE_SIZE,
                            FILE_SIZE,
                        );
                    }
                    self.file_count -= 1;
                    pf!(self, "File '%s' deleted\n", A::S(&args[0]));
                    return;
                } else {
                    pf!(self, "Error: Permission denied\n");
                    return;
                }
            }
        }

        pf!(self, "Error: File '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_listfiles(&mut self) {
        if self.file_count == 0 {
            pf!(self, "No files\n");
            return;
        }

        pf!(self, "Files:\n");
        for i in 0..self.file_count {
            let name = self.file_name(i);
            let owner = self.file_owner(i);
            let perm = self.file_perm(i);
            pf!(
                self,
                "  %s (owner: %s, perm: %d)\n",
                A::S(&name),
                A::S(&owner),
                A::I(perm)
            );
        }
    }

    // --- variables -------------------------------------------------------

    fn cmd_set(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: set <name> <value>\n");
            return;
        }

        for i in 0..self.variable_count {
            if c_strcmp(&self.var_name(i), &args[0]) == 0 {
                c_strcpy(&mut self.variables, i * VAR_SIZE + V_VALUE, &args[1]);
                pf!(self, "Variable '%s' updated\n", A::S(&args[0]));
                return;
            }
        }

        if self.variable_count >= MAX_VARIABLES {
            pf!(self, "Error: Maximum variables reached\n");
            return;
        }

        let base = self.variable_count * VAR_SIZE;
        c_strcpy(&mut self.variables, base + V_NAME, &args[0]);
        c_strcpy(&mut self.variables, base + V_VALUE, &args[1]);
        self.variable_count += 1;
        pf!(self, "Variable '%s' set\n", A::S(&args[0]));
    }

    fn cmd_get(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            pf!(self, "Usage: get <name>\n");
            return;
        }

        for i in 0..self.variable_count {
            if c_strcmp(&self.var_name(i), &args[0]) == 0 {
                let name = self.var_name(i);
                let value = self.var_value(i);
                pf!(self, "%s = %s\n", A::S(&name), A::S(&value));
                return;
            }
        }

        pf!(self, "Error: Variable '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_unset(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            pf!(self, "Usage: unset <name>\n");
            return;
        }

        for i in 0..self.variable_count {
            if c_strcmp(&self.var_name(i), &args[0]) == 0 {
                for j in i..self.variable_count - 1 {
                    c_struct_copy(
                        &mut self.variables,
                        j * VAR_SIZE,
                        (j + 1) * VAR_SIZE,
                        VAR_SIZE,
                    );
                }
                self.variable_count -= 1;
                pf!(self, "Variable '%s' unset\n", A::S(&args[0]));
                return;
            }
        }

        pf!(self, "Error: Variable '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_listvars(&mut self) {
        if self.variable_count == 0 {
            pf!(self, "No variables set\n");
            return;
        }

        pf!(self, "Variables:\n");
        for i in 0..self.variable_count {
            let name = self.var_name(i);
            let value = self.var_value(i);
            pf!(self, "  %s = %s\n", A::S(&name), A::S(&value));
        }
    }

    // --- string comparison ----------------------------------------------

    fn cmd_compare(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: compare <string1> <string2>\n");
            return;
        }

        let result = c_strcmp(&args[0], &args[1]);

        pf!(
            self,
            "strcmp('%s', '%s') = %d\n",
            A::S(&args[0]),
            A::S(&args[1]),
            A::I(result)
        );

        if result == 0 {
            pf!(self, "Strings are equal\n");
        } else if result < 0 {
            pf!(self, "'%s' < '%s'\n", A::S(&args[0]), A::S(&args[1]));
        } else {
            pf!(self, "'%s' > '%s'\n", A::S(&args[0]), A::S(&args[1]));
        }
    }

    fn cmd_compare_n(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 3 {
            pf!(self, "Usage: compareN <string1> <string2> <n>\n");
            return;
        }

        let n = c_atoi(&args[2]);
        // `int` -> `size_t` conversion (sign extension on LP64).
        let result = c_strncmp(&args[0], &args[1], n as isize as usize);

        pf!(
            self,
            "strncmp('%s', '%s', %d) = %d\n",
            A::S(&args[0]),
            A::S(&args[1]),
            A::I(n),
            A::I(result)
        );

        if result == 0 {
            pf!(self, "First %d characters are equal\n", A::I(n));
        } else if result < 0 {
            pf!(
                self,
                "'%s' < '%s' (first %d chars)\n",
                A::S(&args[0]),
                A::S(&args[1]),
                A::I(n)
            );
        } else {
            pf!(
                self,
                "'%s' > '%s' (first %d chars)\n",
                A::S(&args[0]),
                A::S(&args[1]),
                A::I(n)
            );
        }
    }

    fn cmd_startswith(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: startswith <string> <prefix>\n");
            return;
        }

        let prefix_len = args[1].len();

        if c_strncmp(&args[0], &args[1], prefix_len) == 0 {
            pf!(
                self,
                "'%s' starts with '%s'\n",
                A::S(&args[0]),
                A::S(&args[1])
            );
        } else {
            pf!(
                self,
                "'%s' does not start with '%s'\n",
                A::S(&args[0]),
                A::S(&args[1])
            );
        }
    }

    fn cmd_match(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: match <pattern> <string1> [string2] ...\n");
            return;
        }

        pf!(self, "Matching pattern '%s':\n", A::S(&args[0]));
        let mut matches = 0i32;

        for i in 1..arg_count {
            if c_strcmp(&args[0], &args[i]) == 0 {
                pf!(self, "  '%s' - EXACT MATCH\n", A::S(&args[i]));
                matches += 1;
            } else if c_strstr_found(&args[i], &args[0]) {
                pf!(self, "  '%s' - contains pattern\n", A::S(&args[i]));
                matches += 1;
            } else {
                pf!(self, "  '%s' - no match\n", A::S(&args[i]));
            }
        }

        pf!(self, "Total matches: %d\n", A::I(matches));
    }

    // --- system ----------------------------------------------------------

    fn cmd_help(&mut self) {
        pf!(self, "\n=== Command Interpreter Help ===\n");
        pf!(self, "User Management:\n");
        pf!(self, "  adduser <user> <pass> [level] - Add new user\n");
        pf!(self, "  login <user> <pass>            - Login as user\n");
        pf!(self, "  logout                         - Logout current user\n");
        pf!(self, "  whoami                         - Show current user\n");
        pf!(self, "  listusers                      - List all users\n");
        pf!(self, "\nFile Management:\n");
        pf!(self, "  createfile <name> [content]    - Create file\n");
        pf!(self, "  readfile <name>                - Read file\n");
        pf!(self, "  writefile <name> <content>     - Write to file\n");
        pf!(self, "  deletefile <name>              - Delete file\n");
        pf!(self, "  listfiles                      - List all files\n");
        pf!(self, "\nVariable Management:\n");
        pf!(self, "  set <name> <value>             - Set variable\n");
        pf!(self, "  get <name>                     - Get variable\n");
        pf!(self, "  unset <name>                   - Unset variable\n");
        pf!(self, "  listvars                       - List all variables\n");
        pf!(self, "\nString Operations:\n");
        pf!(self, "  compare <str1> <str2>          - Compare strings\n");
        pf!(self, "  compareN <str1> <str2> <n>     - Compare first N chars\n");
        pf!(self, "  startswith <str> <prefix>      - Check if starts with\n");
        pf!(self, "  match <pattern> <str> ...      - Match pattern\n");
        pf!(self, "\nSystem:\n");
        pf!(self, "  debug [on|off]                 - Toggle debug mode\n");
        pf!(self, "  verbose [on|off]               - Toggle verbose mode\n");
        pf!(self, "  status                         - Show system status\n");
        pf!(self, "  time                           - Show current time\n");
        pf!(self, "  help                           - Show this help\n");
        pf!(self, "  exit                           - Exit program\n");
    }

    fn cmd_debug(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            let s: &[u8] = if self.debug_mode != 0 { b"ON" } else { b"OFF" };
            pf!(self, "Debug mode: %s\n", A::S(s));
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.debug_mode = 1;
            pf!(self, "Debug mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.debug_mode = 0;
            pf!(self, "Debug mode disabled\n");
        } else {
            pf!(self, "Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            let s: &[u8] = if self.verbose_mode != 0 { b"ON" } else { b"OFF" };
            pf!(self, "Verbose mode: %s\n", A::S(s));
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.verbose_mode = 1;
            pf!(self, "Verbose mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.verbose_mode = 0;
            pf!(self, "Verbose mode disabled\n");
        } else {
            pf!(self, "Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&mut self) {
        pf!(self, "\n=== System Status ===\n");
        let (uc, fc, vc) = (
            self.user_count as i32,
            self.file_count as i32,
            self.variable_count as i32,
        );
        pf!(self, "Users: %d/%d\n", A::I(uc), A::I(MAX_USERS as i32));
        pf!(self, "Files: %d/%d\n", A::I(fc), A::I(MAX_FILES as i32));
        pf!(
            self,
            "Variables: %d/%d\n",
            A::I(vc),
            A::I(MAX_VARIABLES as i32)
        );
        let cur = match self.logged_in_user() {
            Some(i) => self.user_name(i),
            None => b"none".to_vec(),
        };
        pf!(self, "Current user: %s\n", A::S(&cur));
        let d: &[u8] = if self.debug_mode != 0 { b"ON" } else { b"OFF" };
        pf!(self, "Debug mode: %s\n", A::S(d));
        let v: &[u8] = if self.verbose_mode != 0 { b"ON" } else { b"OFF" };
        pf!(self, "Verbose mode: %s\n", A::S(v));
    }

    fn cmd_time(&mut self) {
        let now: i64 = unsafe { libc_time(std::ptr::null_mut()) };
        let p = unsafe { libc_ctime(&now as *const i64) };
        let text: Vec<u8> = if p.is_null() {
            b"(null)".to_vec()
        } else {
            let mut v = Vec::new();
            let mut i: isize = 0;
            unsafe {
                while *p.offset(i) != 0 {
                    v.push(*p.offset(i) as u8);
                    i += 1;
                }
            }
            v
        };
        pf!(self, "Current time: %s", A::S(&text));
    }

    // --- dispatch --------------------------------------------------------

    fn process_command(&mut self, input: &[u8]) {
        let (command, args) = parse_command(input);
        let arg_count = args.len();

        if command.is_empty() {
            return;
        }

        if self.debug_mode != 0 {
            pf!(
                self,
                "[DEBUG] Command: '%s', Args: %d\n",
                A::S(&command),
                A::I(arg_count as i32)
            );
        }

        let c = &command[..];

        // User commands
        if c_strcmp(c, b"adduser") == 0 {
            self.cmd_adduser(&args, arg_count);
        } else if c_strcmp(c, b"login") == 0 {
            self.cmd_login(&args, arg_count);
        } else if c_strcmp(c, b"logout") == 0 {
            self.cmd_logout();
        } else if c_strcmp(c, b"whoami") == 0 {
            self.cmd_whoami();
        } else if c_strcmp(c, b"listusers") == 0 || c_strcmp(c, b"users") == 0 {
            self.cmd_listusers();
        }
        // File commands
        else if c_strcmp(c, b"createfile") == 0 || c_strcmp(c, b"touch") == 0 {
            self.cmd_createfile(&args, arg_count);
        } else if c_strcmp(c, b"readfile") == 0 || c_strcmp(c, b"cat") == 0 {
            self.cmd_readfile(&args, arg_count);
        } else if c_strcmp(c, b"writefile") == 0 || c_strcmp(c, b"write") == 0 {
            self.cmd_writefile(&args, arg_count);
        } else if c_strcmp(c, b"deletefile") == 0 || c_strcmp(c, b"rm") == 0 {
            self.cmd_deletefile(&args, arg_count);
        } else if c_strcmp(c, b"listfiles") == 0 || c_strcmp(c, b"ls") == 0 {
            self.cmd_listfiles();
        }
        // Variable commands
        else if c_strcmp(c, b"set") == 0 {
            self.cmd_set(&args, arg_count);
        } else if c_strcmp(c, b"get") == 0 {
            self.cmd_get(&args, arg_count);
        } else if c_strcmp(c, b"unset") == 0 {
            self.cmd_unset(&args, arg_count);
        } else if c_strcmp(c, b"listvars") == 0 || c_strcmp(c, b"vars") == 0 {
            self.cmd_listvars();
        }
        // String comparison commands
        else if c_strcmp(c, b"compare") == 0 || c_strcmp(c, b"cmp") == 0 {
            self.cmd_compare(&args, arg_count);
        } else if c_strcmp(c, b"compareN") == 0 || c_strcmp(c, b"cmpn") == 0 {
            self.cmd_compare_n(&args, arg_count);
        } else if c_strcmp(c, b"startswith") == 0 {
            self.cmd_startswith(&args, arg_count);
        } else if c_strcmp(c, b"match") == 0 {
            self.cmd_match(&args, arg_count);
        }
        // System commands
        else if c_strcmp(c, b"debug") == 0 {
            self.cmd_debug(&args, arg_count);
        } else if c_strcmp(c, b"verbose") == 0 {
            self.cmd_verbose(&args, arg_count);
        } else if c_strcmp(c, b"status") == 0 {
            self.cmd_status();
        } else if c_strcmp(c, b"time") == 0 {
            self.cmd_time();
        } else if c_strcmp(c, b"help") == 0 || c_strcmp(c, b"?") == 0 {
            self.cmd_help();
        } else if c_strcmp(c, b"exit") == 0 || c_strcmp(c, b"quit") == 0 {
            pf!(self, "Goodbye!\n");
            self.flush();
            std::process::exit(0);
        }
        // Check for partial matches using strncmp
        else if c_strncmp(c, b"add", 3) == 0 {
            pf!(self, "Did you mean 'adduser'?\n");
        } else if c_strncmp(c, b"log", 3) == 0 {
            pf!(self, "Did you mean 'login' or 'logout'?\n");
        } else if c_strncmp(c, b"list", 4) == 0 {
            pf!(self, "Did you mean 'listusers', 'listfiles', or 'listvars'?\n");
        } else if c_strncmp(c, b"create", 6) == 0 {
            pf!(self, "Did you mean 'createfile'?\n");
        } else if c_strncmp(c, b"read", 4) == 0 {
            pf!(self, "Did you mean 'readfile'?\n");
        } else if c_strncmp(c, b"write", 5) == 0 {
            pf!(self, "Did you mean 'writefile'?\n");
        } else if c_strncmp(c, b"delete", 6) == 0 {
            pf!(self, "Did you mean 'deletefile'?\n");
        } else {
            pf!(
                self,
                "Unknown command: '%s'. Type 'help' for available commands.\n",
                A::S(c)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// libc bindings needed for the (locale/timezone dependent) `time` command
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "time"]
    fn libc_time(t: *mut i64) -> i64;
    #[link_name = "ctime"]
    fn libc_ctime(t: *const i64) -> *const std::os::raw::c_char;
}

// ---------------------------------------------------------------------------
// Tokenizer (strtok on " \t")
// ---------------------------------------------------------------------------

fn parse_command(input: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    // char temp[MAX_INPUT]; strncpy(temp, input, MAX_INPUT - 1);
    // temp[MAX_INPUT - 1] = '\0';
    let limit = input.len().min(MAX_INPUT - 1);
    let temp = &input[..limit];

    let mut cmd: Vec<u8> = Vec::new();
    let mut args: Vec<Vec<u8>> = Vec::new();

    let mut tokens = temp
        .split(|&c| c == b' ' || c == b'\t')
        .filter(|t| !t.is_empty());

    if let Some(tok) = tokens.next() {
        let n = tok.len().min(MAX_COMMAND - 1);
        cmd.extend_from_slice(&tok[..n]);

        for tok in tokens {
            if args.len() >= MAX_ARGS {
                break;
            }
            let n = tok.len().min(MAX_COMMAND - 1);
            args.push(tok[..n].to_vec());
        }
    }

    (cmd, args)
}

// ---------------------------------------------------------------------------
// fgets emulation
// ---------------------------------------------------------------------------

/// `fgets(buf, size, stdin)` -- returns the bytes read (newline included) or
/// `None` at end-of-file with nothing read.
fn fgets<R: BufRead>(r: &mut R, size: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    loop {
        if out.len() + 1 >= size {
            return Some(out);
        }
        let (chunk, had_nl) = {
            let buf = match r.fill_buf() {
                Ok(b) => b,
                Err(_) => {
                    return if out.is_empty() { None } else { Some(out) };
                }
            };
            if buf.is_empty() {
                return if out.is_empty() { None } else { Some(out) };
            }
            let remaining = size - 1 - out.len();
            let take = buf.len().min(remaining);
            let slice = &buf[..take];
            match slice.iter().position(|&c| c == b'\n') {
                Some(p) => (slice[..=p].to_vec(), true),
                None => (slice.to_vec(), false),
            }
        };
        let n = chunk.len();
        out.extend_from_slice(&chunk);
        r.consume(n);
        if had_nl {
            return Some(out);
        }
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let mut st = State::new();

    pf!(st, "|----------------------------------------|\n");
    pf!(st, "|   COMMAND INTERPRETER                  |\n");
    pf!(st, "|   strcmp/strncmp demonstration         |\n");
    pf!(st, "|----------------------------------------|\n");
    pf!(st, "Type 'help' for available commands\n\n");

    let stdin = std::io::stdin();
    let mut reader = BufReader::new(StdinRead(stdin));

    loop {
        pf!(st, "> ");

        let raw = match fgets(&mut reader, MAX_INPUT) {
            Some(v) => v,
            None => break,
        };

        // input[strcspn(input, "\n")] = 0;
        let cut = raw
            .iter()
            .position(|&c| c == b'\n' || c == 0)
            .unwrap_or(raw.len());
        let input = &raw[..cut];

        if st.verbose_mode != 0 {
            pf!(st, "[VERBOSE] Processing: '%s'\n", A::S(input));
        }

        let owned = input.to_vec();
        st.process_command(&owned);
    }

    st.flush();
}

/// Thin `Read` adapter so `BufReader` owns the stdin handle.
struct StdinRead(std::io::Stdin);

impl Read for StdinRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.lock().read(buf)
    }
}
