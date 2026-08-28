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
//!   * glibc's stdio buffering discipline (line buffered on a terminal, fully
//!     buffered in `st_blksize` units otherwise), so that output still in the
//!     buffer when the process dies is lost exactly as in C,
//!   * the fixed-size `strcpy` buffer overruns in the global record arrays.
//!
//! ## Why the whole `.bss` is emulated
//!
//! `cmd_adduser` and friends `strcpy` caller-controlled tokens (up to 63 bytes
//! after `parse_command` truncates them) into 32-byte struct fields.  Once the
//! record arrays are full, those overruns walk off the end of the array and
//! land on the *scalar globals that follow it in `.bss`* -- `user_count`,
//! `current_user`, `file_count` -- which the C code then reloads on the very
//! next statement.  The result is wild but perfectly deterministic for a given
//! link layout: writing a 40-byte password into `users[9].password` zeroes
//! `user_count`, a 41-byte one sets it to 80, and a 42-byte one sets it to
//! 20560 and the following store faults.
//!
//! To reproduce that, this program models the reference build's writable
//! segment as one flat byte array at the real link addresses, keeps every
//! counter *in* that array (so an overrun corrupts it), and reloads each
//! counter from memory exactly where the unoptimised C reloads it.  Accesses
//! that leave the mapped window terminate the process with `SIGSEGV`, matching
//! the C.
//!
//! The addresses below come from the reference build produced by the supplied
//! `CMakeLists.txt` (no `CMAKE_BUILD_TYPE`, i.e. `-O0`, non-PIE):
//!
//! ```text
//! 0x4070a0  users            720 bytes   (10 x 72)
//! 0x407370  user_count         4
//! 0x407378  current_user       8
//! 0x407380  files          12240 bytes   (20 x 612)
//! 0x40a350  file_count         4
//! 0x40a360  variables       3200 bytes   (20 x 160)
//! 0x40afe0  variable_count     4
//! 0x40afe4  debug_mode         4
//! 0x40afe8  verbose_mode       4
//! ```

use std::io::Read;

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: i32 = 20;
const MAX_USERS: i32 = 10;
const MAX_VARIABLES: i32 = 20;

// ---------------------------------------------------------------------------
// Emulated address space (the reference build's RW LOAD segment)
// ---------------------------------------------------------------------------

/// First byte of the writable mapping: the RW `LOAD` segment starts at
/// `0x406de8`, so the kernel maps from page `0x406000`.
const WIN_LO: u64 = 0x406000;
/// One past the last writable byte: the segment ends at `0x40aff0`, so the
/// last mapped page ends at `0x40b000`.
const WIN_HI: u64 = 0x40b000;

const A_USERS: u64 = 0x4070a0;
const A_USER_COUNT: u64 = 0x407370;
const A_CURRENT_USER: u64 = 0x407378;
const A_FILES: u64 = 0x407380;
const A_FILE_COUNT: u64 = 0x40a350;
const A_VARIABLES: u64 = 0x40a360;
const A_VARIABLE_COUNT: u64 = 0x40afe0;
const A_DEBUG_MODE: u64 = 0x40afe4;
const A_VERBOSE_MODE: u64 = 0x40afe8;

// typedef struct { char name[32]; char password[32]; int permission_level; int logged_in; } user_t;
const U_SIZE: u64 = 72;
const U_NAME: u64 = 0;
const U_PASS: u64 = 32;
const U_PERM: u64 = 64;
const U_LOGGED: u64 = 68;

// typedef struct { char filename[64]; char content[512]; char owner[32]; int permissions; } file_t;
const F_SIZE: u64 = 612;
const F_NAME: u64 = 0;
const F_CONTENT: u64 = 64;
const F_OWNER: u64 = 576;
const F_PERM: u64 = 608;

// typedef struct { char name[32]; char value[128]; } variable_t;
const V_SIZE: u64 = 160;
const V_NAME: u64 = 0;
const V_VALUE: u64 = 32;

fn a_user(i: i32) -> u64 {
    A_USERS.wrapping_add((i as i64 as u64).wrapping_mul(U_SIZE))
}

fn a_file(i: i32) -> u64 {
    A_FILES.wrapping_add((i as i64 as u64).wrapping_mul(F_SIZE))
}

fn a_var(i: i32) -> u64 {
    A_VARIABLES.wrapping_add((i as i64 as u64).wrapping_mul(V_SIZE))
}

// ---------------------------------------------------------------------------
// Raw libc bindings
// ---------------------------------------------------------------------------

extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn isatty(fd: i32) -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
    fn _exit(status: i32) -> !;
    fn time(t: *mut i64) -> i64;
    fn ctime(t: *const i64) -> *const u8;
}

const SIGSEGV: i32 = 11;
const SIG_DFL: usize = 0;

/// Die the way the C process dies on an out-of-bounds access: terminated by
/// `SIGSEGV`, with whatever is still sitting in the stdio buffer discarded.
///
/// The Rust runtime installs its own `SIGSEGV` handler for stack-overflow
/// reporting, so the default disposition is restored first; otherwise the
/// process would abort (`SIGABRT`) with a "stack overflow" message instead.
fn segv() -> ! {
    unsafe {
        signal(SIGSEGV, SIG_DFL);
        raise(SIGSEGV);
        // Unreachable in practice; keeps the `!` return type honest.
        _exit(128 + SIGSEGV)
    }
}

// ---------------------------------------------------------------------------
// Output: glibc stdio emulation over fd 1.
// ---------------------------------------------------------------------------

/// Everything the program prints, byte exact, through a buffer that behaves
/// like glibc's: line buffered when stdout is a terminal, otherwise fully
/// buffered in `st_blksize`-sized chunks.  This matters because a `SIGSEGV`
/// throws the buffer away, so the bytes that actually reach the pipe are only
/// the whole chunks flushed before the fault.
struct Out {
    buf: Vec<u8>,
    cap: usize,
    line_buffered: bool,
}

/// `st_blksize` of fd 1, which is the buffer size glibc picks in
/// `_IO_file_doallocate`; `BUFSIZ` when it cannot be determined.
fn stdout_blksize() -> usize {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::FromRawFd;
    let f = unsafe { std::fs::File::from_raw_fd(1) };
    let n = f.metadata().map(|m| m.blksize() as usize).unwrap_or(0);
    // Do not let the File close fd 1.
    std::mem::forget(f);
    if n == 0 {
        8192
    } else {
        n
    }
}

impl Out {
    fn new() -> Out {
        let tty = unsafe { isatty(1) } == 1;
        Out {
            buf: Vec::new(),
            cap: stdout_blksize(),
            line_buffered: tty,
        }
    }

    fn raw(bytes: &[u8]) {
        let mut off = 0usize;
        while off < bytes.len() {
            let n = unsafe { write(1, bytes[off..].as_ptr(), bytes.len() - off) };
            if n > 0 {
                off += n as usize;
            } else if n == 0 {
                break;
            } else {
                // EINTR retries; anything else is unrecoverable, and C's
                // printf would silently fail too.
                let e = std::io::Error::last_os_error();
                if e.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
        }
    }

    fn emit(&mut self, n: usize) {
        Out::raw(&self.buf[..n]);
        self.buf.drain(..n);
    }

    fn put(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.line_buffered {
            if let Some(p) = self.buf.iter().rposition(|&c| c == b'\n') {
                self.emit(p + 1);
            }
            if self.buf.len() >= self.cap {
                let n = self.buf.len();
                self.emit(n);
            }
        } else {
            while self.buf.len() >= self.cap {
                let n = self.cap;
                self.emit(n);
            }
        }
    }

    /// printf("%s", ...) for raw (possibly non-UTF-8) bytes.
    fn s(&mut self, b: &[u8]) {
        self.put(b);
    }

    /// Literal text.
    fn t(&mut self, s: &str) {
        self.put(s.as_bytes());
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
        let tail = buf[n..].to_vec();
        self.put(&tail);
    }

    fn flush(&mut self) {
        let n = self.buf.len();
        if n > 0 {
            self.emit(n);
        }
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
// The emulated writable segment
// ---------------------------------------------------------------------------

struct Bss {
    b: Vec<u8>,
}

impl Bss {
    fn new() -> Bss {
        Bss {
            b: vec![0u8; (WIN_HI - WIN_LO) as usize],
        }
    }

    /// Translate an address to an index, faulting when `len` bytes starting
    /// there are not inside the mapped window.
    fn at(&self, addr: u64, len: u64) -> usize {
        if addr < WIN_LO {
            segv();
        }
        match addr.checked_add(len) {
            Some(end) if end <= WIN_HI => (addr - WIN_LO) as usize,
            _ => segv(),
        }
    }

    fn read_i32(&self, addr: u64) -> i32 {
        let o = self.at(addr, 4);
        let mut raw = [0u8; 4];
        raw.copy_from_slice(&self.b[o..o + 4]);
        i32::from_le_bytes(raw)
    }

    fn write_i32(&mut self, addr: u64, v: i32) {
        let o = self.at(addr, 4);
        self.b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn read_u64(&self, addr: u64) -> u64 {
        let o = self.at(addr, 8);
        let mut raw = [0u8; 8];
        raw.copy_from_slice(&self.b[o..o + 8]);
        u64::from_le_bytes(raw)
    }

    fn write_u64(&mut self, addr: u64, v: u64) {
        let o = self.at(addr, 8);
        self.b[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// The NUL-terminated string starting at `addr`, as an owned copy.
    /// Running off the end of the mapping faults, as the read would in C.
    fn cstr(&self, addr: u64) -> Vec<u8> {
        let start = self.at(addr, 1);
        match self.b[start..].iter().position(|&c| c == 0) {
            Some(p) => self.b[start..start + p].to_vec(),
            None => segv(),
        }
    }

    /// `strcpy(addr, src)`
    fn strcpy(&mut self, addr: u64, src: &[u8]) {
        let o = self.at(addr, src.len() as u64 + 1);
        self.b[o..o + src.len()].copy_from_slice(src);
        self.b[o + src.len()] = 0;
    }

    fn write_u8(&mut self, addr: u64, v: u8) {
        let o = self.at(addr, 1);
        self.b[o] = v;
    }

    /// `*dst = *src` for structs of `len` bytes.
    fn copy_struct(&mut self, dst: u64, src: u64, len: u64) {
        let d = self.at(dst, len);
        let s = self.at(src, len);
        self.b.copy_within(s..s + len as usize, d);
    }
}

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

struct State {
    m: Bss,
}

impl State {
    fn new() -> State {
        State { m: Bss::new() }
    }

    // -- the scalar globals, always (re)loaded from memory ---------------

    fn user_count(&self) -> i32 {
        self.m.read_i32(A_USER_COUNT)
    }
    fn set_user_count(&mut self, v: i32) {
        self.m.write_i32(A_USER_COUNT, v)
    }
    fn file_count(&self) -> i32 {
        self.m.read_i32(A_FILE_COUNT)
    }
    fn set_file_count(&mut self, v: i32) {
        self.m.write_i32(A_FILE_COUNT, v)
    }
    fn variable_count(&self) -> i32 {
        self.m.read_i32(A_VARIABLE_COUNT)
    }
    fn set_variable_count(&mut self, v: i32) {
        self.m.write_i32(A_VARIABLE_COUNT, v)
    }
    fn debug_mode(&self) -> i32 {
        self.m.read_i32(A_DEBUG_MODE)
    }
    fn set_debug_mode(&mut self, v: i32) {
        self.m.write_i32(A_DEBUG_MODE, v)
    }
    fn verbose_mode(&self) -> i32 {
        self.m.read_i32(A_VERBOSE_MODE)
    }
    fn set_verbose_mode(&mut self, v: i32) {
        self.m.write_i32(A_VERBOSE_MODE, v)
    }

    /// `current_user`, as a raw pointer value (0 == NULL).
    fn cu(&self) -> u64 {
        self.m.read_u64(A_CURRENT_USER)
    }
    fn set_cu(&mut self, p: u64) {
        self.m.write_u64(A_CURRENT_USER, p)
    }

    // -- helpers mirroring `current_user->field` -------------------------

    fn cu_name(&self) -> Vec<u8> {
        self.m.cstr(self.cu().wrapping_add(U_NAME))
    }

    fn cu_perm(&self) -> i32 {
        self.m.read_i32(self.cu().wrapping_add(U_PERM))
    }

    /// `current_user && current_user->logged_in`
    fn logged_in(&self) -> bool {
        let p = self.cu();
        p != 0 && self.m.read_i32(p.wrapping_add(U_LOGGED)) != 0
    }

    // ---------------------------------------------------------------
    // User management commands
    // ---------------------------------------------------------------

    fn cmd_adduser(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        let arg_count = args.len() as i32;
        if arg_count < 2 {
            o.t("Usage: adduser <username> <password> [permission_level]\n");
            return;
        }

        if self.user_count() >= MAX_USERS {
            o.t("Error: Maximum users reached\n");
            return;
        }

        // Check if user already exists using strcmp
        let mut i: i32 = 0;
        while i < self.user_count() {
            if c_strcmp(&self.m.cstr(a_user(i) + U_NAME), &args[0]) == 0 {
                o.t("Error: User '");
                o.s(&args[0]);
                o.t("' already exists\n");
                return;
            }
            i += 1;
        }

        // Each `users[user_count]` below is a fresh load of the global, which
        // matters because the preceding strcpy may have clobbered it.
        let uc = self.user_count();
        self.m.strcpy(a_user(uc) + U_NAME, &args[0]);

        let uc = self.user_count();
        self.m.strcpy(a_user(uc) + U_PASS, &args[1]);

        let level = if arg_count >= 3 { c_atoi(&args[2]) } else { 1 };
        let uc = self.user_count();
        self.m.write_i32(a_user(uc) + U_PERM, level);

        let uc = self.user_count();
        self.m.write_i32(a_user(uc) + U_LOGGED, 0);

        let uc = self.user_count();
        self.set_user_count(uc.wrapping_add(1));

        o.t("User '");
        o.s(&args[0]);
        o.t("' added with permission level ");
        let uc = self.user_count();
        let perm = self.m.read_i32(a_user(uc.wrapping_sub(1)) + U_PERM);
        o.d(perm);
        o.t("\n");
    }

    fn cmd_login(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.len() < 2 {
            o.t("Usage: login <username> <password>\n");
            return;
        }

        if self.logged_in() {
            o.t("Error: User '");
            let n = self.cu_name();
            o.s(&n);
            o.t("' already logged in. Use 'logout' first.\n");
            return;
        }

        // Find user and verify password using strcmp
        let mut i: i32 = 0;
        while i < self.user_count() {
            if c_strcmp(&self.m.cstr(a_user(i) + U_NAME), &args[0]) == 0 {
                if c_strcmp(&self.m.cstr(a_user(i) + U_PASS), &args[1]) == 0 {
                    self.m.write_i32(a_user(i) + U_LOGGED, 1);
                    let p = a_user(i);
                    self.set_cu(p);
                    o.t("Login successful. Welcome, ");
                    let n = self.cu_name();
                    o.s(&n);
                    o.t("!\n");
                    return;
                } else {
                    o.t("Error: Incorrect password\n");
                    return;
                }
            }
            i += 1;
        }

        o.t("Error: User not found\n");
    }

    fn cmd_logout(&mut self, o: &mut Out) {
        if !self.logged_in() {
            o.t("Error: No user logged in\n");
            return;
        }

        o.t("Goodbye, ");
        let n = self.cu_name();
        o.s(&n);
        o.t("!\n");
        let p = self.cu();
        self.m.write_i32(p.wrapping_add(U_LOGGED), 0);
        self.set_cu(0);
    }

    fn cmd_whoami(&mut self, o: &mut Out) {
        if !self.logged_in() {
            o.t("Not logged in\n");
            return;
        }

        o.t("Current user: ");
        let n = self.cu_name();
        o.s(&n);
        o.t("\n");
        o.t("Permission level: ");
        let p = self.cu_perm();
        o.d(p);
        o.t("\n");
    }

    fn cmd_listusers(&mut self, o: &mut Out) {
        if self.user_count() == 0 {
            o.t("No users registered\n");
            return;
        }

        o.t("Registered users:\n");
        let mut i: i32 = 0;
        while i < self.user_count() {
            o.t("  ");
            let n = self.m.cstr(a_user(i) + U_NAME);
            o.s(&n);
            o.t(" (level ");
            let perm = self.m.read_i32(a_user(i) + U_PERM);
            o.d(perm);
            o.t(") ");
            if self.m.read_i32(a_user(i) + U_LOGGED) != 0 {
                o.t("[logged in]");
            }
            o.t("\n");
            i += 1;
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

        let arg_count = args.len() as i32;
        if arg_count < 1 {
            o.t("Usage: createfile <filename> [content]\n");
            return;
        }

        if self.file_count() >= MAX_FILES {
            o.t("Error: Maximum files reached\n");
            return;
        }

        // Check if file exists using strcmp
        let mut i: i32 = 0;
        while i < self.file_count() {
            if c_strcmp(&self.m.cstr(a_file(i) + F_NAME), &args[0]) == 0 {
                o.t("Error: File '");
                o.s(&args[0]);
                o.t("' already exists\n");
                return;
            }
            i += 1;
        }

        let fc = self.file_count();
        self.m.strcpy(a_file(fc) + F_NAME, &args[0]);

        let owner = self.cu_name();
        let fc = self.file_count();
        self.m.strcpy(a_file(fc) + F_OWNER, &owner);

        let fc = self.file_count();
        self.m.write_i32(a_file(fc) + F_PERM, 755);

        if arg_count >= 2 {
            let fc = self.file_count();
            self.m.strcpy(a_file(fc) + F_CONTENT, &args[1]);
        } else {
            // files[file_count].content[0] = '\0';
            let fc = self.file_count();
            self.m.write_u8(a_file(fc) + F_CONTENT, 0);
        }

        let fc = self.file_count();
        self.set_file_count(fc.wrapping_add(1));
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
        let mut i: i32 = 0;
        while i < self.file_count() {
            if c_strcmp(&self.m.cstr(a_file(i) + F_NAME), &args[0]) == 0 {
                o.t("=== ");
                let n = self.m.cstr(a_file(i) + F_NAME);
                o.s(&n);
                o.t(" ===\n");
                o.t("Owner: ");
                let n = self.m.cstr(a_file(i) + F_OWNER);
                o.s(&n);
                o.t("\n");
                o.t("Permissions: ");
                let p = self.m.read_i32(a_file(i) + F_PERM);
                o.d(p);
                o.t("\n");
                o.t("Content: ");
                let n = self.m.cstr(a_file(i) + F_CONTENT);
                o.s(&n);
                o.t("\n");
                return;
            }
            i += 1;
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
        let mut i: i32 = 0;
        while i < self.file_count() {
            if c_strcmp(&self.m.cstr(a_file(i) + F_NAME), &args[0]) == 0 {
                // Check if current user owns the file
                let owner = self.m.cstr(a_file(i) + F_OWNER);
                let cu_name = self.cu_name();
                if c_strcmp(&owner, &cu_name) == 0 || self.cu_perm() >= 5 {
                    self.m.strcpy(a_file(i) + F_CONTENT, &args[1]);
                    o.t("File '");
                    o.s(&args[0]);
                    o.t("' updated\n");
                    return;
                } else {
                    o.t("Error: Permission denied\n");
                    return;
                }
            }
            i += 1;
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
        let mut i: i32 = 0;
        while i < self.file_count() {
            if c_strcmp(&self.m.cstr(a_file(i) + F_NAME), &args[0]) == 0 {
                let owner = self.m.cstr(a_file(i) + F_OWNER);
                let cu_name = self.cu_name();
                if c_strcmp(&owner, &cu_name) == 0 || self.cu_perm() >= 9 {
                    // Shift remaining files
                    let mut j: i32 = i;
                    while j < self.file_count().wrapping_sub(1) {
                        self.m.copy_struct(a_file(j), a_file(j.wrapping_add(1)), F_SIZE);
                        j += 1;
                    }
                    let fc = self.file_count();
                    self.set_file_count(fc.wrapping_sub(1));
                    o.t("File '");
                    o.s(&args[0]);
                    o.t("' deleted\n");
                    return;
                } else {
                    o.t("Error: Permission denied\n");
                    return;
                }
            }
            i += 1;
        }

        o.t("Error: File '");
        o.s(&args[0]);
        o.t("' not found\n");
    }

    fn cmd_listfiles(&mut self, o: &mut Out) {
        if self.file_count() == 0 {
            o.t("No files\n");
            return;
        }

        o.t("Files:\n");
        let mut i: i32 = 0;
        while i < self.file_count() {
            o.t("  ");
            let n = self.m.cstr(a_file(i) + F_NAME);
            o.s(&n);
            o.t(" (owner: ");
            let n = self.m.cstr(a_file(i) + F_OWNER);
            o.s(&n);
            o.t(", perm: ");
            let p = self.m.read_i32(a_file(i) + F_PERM);
            o.d(p);
            o.t(")\n");
            i += 1;
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
        let mut i: i32 = 0;
        while i < self.variable_count() {
            if c_strcmp(&self.m.cstr(a_var(i) + V_NAME), &args[0]) == 0 {
                self.m.strcpy(a_var(i) + V_VALUE, &args[1]);
                o.t("Variable '");
                o.s(&args[0]);
                o.t("' updated\n");
                return;
            }
            i += 1;
        }

        // Create new variable
        if self.variable_count() >= MAX_VARIABLES {
            o.t("Error: Maximum variables reached\n");
            return;
        }

        let vc = self.variable_count();
        self.m.strcpy(a_var(vc) + V_NAME, &args[0]);
        let vc = self.variable_count();
        self.m.strcpy(a_var(vc) + V_VALUE, &args[1]);
        let vc = self.variable_count();
        self.set_variable_count(vc.wrapping_add(1));
        o.t("Variable '");
        o.s(&args[0]);
        o.t("' set\n");
    }

    fn cmd_get(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Usage: get <name>\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.variable_count() {
            if c_strcmp(&self.m.cstr(a_var(i) + V_NAME), &args[0]) == 0 {
                let n = self.m.cstr(a_var(i) + V_NAME);
                o.s(&n);
                o.t(" = ");
                let v = self.m.cstr(a_var(i) + V_VALUE);
                o.s(&v);
                o.t("\n");
                return;
            }
            i += 1;
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

        let mut i: i32 = 0;
        while i < self.variable_count() {
            if c_strcmp(&self.m.cstr(a_var(i) + V_NAME), &args[0]) == 0 {
                let mut j: i32 = i;
                while j < self.variable_count().wrapping_sub(1) {
                    self.m.copy_struct(a_var(j), a_var(j.wrapping_add(1)), V_SIZE);
                    j += 1;
                }
                let vc = self.variable_count();
                self.set_variable_count(vc.wrapping_sub(1));
                o.t("Variable '");
                o.s(&args[0]);
                o.t("' unset\n");
                return;
            }
            i += 1;
        }

        o.t("Error: Variable '");
        o.s(&args[0]);
        o.t("' not found\n");
    }

    fn cmd_listvars(&mut self, o: &mut Out) {
        if self.variable_count() == 0 {
            o.t("No variables set\n");
            return;
        }

        o.t("Variables:\n");
        let mut i: i32 = 0;
        while i < self.variable_count() {
            o.t("  ");
            let n = self.m.cstr(a_var(i) + V_NAME);
            o.s(&n);
            o.t(" = ");
            let v = self.m.cstr(a_var(i) + V_VALUE);
            o.s(&v);
            o.t("\n");
            i += 1;
        }
    }

    // ---------------------------------------------------------------
    // System commands
    // ---------------------------------------------------------------

    fn cmd_debug(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Debug mode: ");
            let on = self.debug_mode() != 0;
            o.t(if on { "ON" } else { "OFF" });
            o.t("\n");
            return;
        }

        if c_strcmp(&args[0], &b"on"[..]) == 0 {
            self.set_debug_mode(1);
            o.t("Debug mode enabled\n");
        } else if c_strcmp(&args[0], &b"off"[..]) == 0 {
            self.set_debug_mode(0);
            o.t("Debug mode disabled\n");
        } else {
            o.t("Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, o: &mut Out, args: &[Vec<u8>]) {
        if args.is_empty() {
            o.t("Verbose mode: ");
            let on = self.verbose_mode() != 0;
            o.t(if on { "ON" } else { "OFF" });
            o.t("\n");
            return;
        }

        if c_strcmp(&args[0], &b"on"[..]) == 0 {
            self.set_verbose_mode(1);
            o.t("Verbose mode enabled\n");
        } else if c_strcmp(&args[0], &b"off"[..]) == 0 {
            self.set_verbose_mode(0);
            o.t("Verbose mode disabled\n");
        } else {
            o.t("Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&mut self, o: &mut Out) {
        o.t("\n=== System Status ===\n");
        o.t("Users: ");
        let v = self.user_count();
        o.d(v);
        o.t("/");
        o.d(MAX_USERS);
        o.t("\n");
        o.t("Files: ");
        let v = self.file_count();
        o.d(v);
        o.t("/");
        o.d(MAX_FILES);
        o.t("\n");
        o.t("Variables: ");
        let v = self.variable_count();
        o.d(v);
        o.t("/");
        o.d(MAX_VARIABLES);
        o.t("\n");
        o.t("Current user: ");
        if self.logged_in() {
            let n = self.cu_name();
            o.s(&n);
        } else {
            o.t("none");
        }
        o.t("\n");
        o.t("Debug mode: ");
        let on = self.debug_mode() != 0;
        o.t(if on { "ON" } else { "OFF" });
        o.t("\n");
        o.t("Verbose mode: ");
        let on = self.verbose_mode() != 0;
        o.t(if on { "ON" } else { "OFF" });
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
        let bytes = unsafe { std::slice::from_raw_parts(p, len) }.to_vec();
        o.s(&bytes);
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

    if state.debug_mode() != 0 {
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
        // exit(0) runs the stdio cleanup, which flushes the buffer.
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
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
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
    let mut stdin = std::io::BufReader::new(std::io::stdin());

    o.t("|----------------------------------------|\n");
    o.t("|   COMMAND INTERPRETER                  |\n");
    o.t("|   strcmp/strncmp demonstration         |\n");
    o.t("|----------------------------------------|\n");
    o.t("Type 'help' for available commands\n\n");

    loop {
        o.t("> ");

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

        if state.verbose_mode() != 0 {
            o.t("[VERBOSE] Processing: '");
            o.s(&input);
            o.t("'\n");
        }

        process_command(&mut state, &mut o, &input);
    }

    // Returning from main runs the stdio cleanup, which flushes the buffer.
    o.flush();
}
