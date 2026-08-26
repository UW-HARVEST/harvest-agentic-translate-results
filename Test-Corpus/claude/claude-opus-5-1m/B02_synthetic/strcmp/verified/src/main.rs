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
//! # Why this is written the way it is
//!
//! The C program stores all of its state in `static` (`.bss`) objects made of
//! fixed-size `char` arrays and copies user supplied tokens into them with
//! `strcpy`, which happily overruns the individual members *and* the arrays
//! themselves.  Those overruns are observable: they clobber the neighbouring
//! `.bss` objects (`user_count`, `current_user`, `file_count`, `variable_count`,
//! `debug_mode`, ...) and, when they reach far enough, kill the process with
//! `SIGSEGV`.
//!
//! To reproduce the reference program byte-for-byte, this translation emulates
//! the writable data mapping of the reference binary as one flat byte array
//! using the exact layout gcc produces:
//!
//! ```text
//!   0x406000            start of the writable page mapping
//!   0x406de8 .data / .got / .dynamic
//!   0x40707c __bss_start
//!   0x407080 stdin
//!   0x4070a0 users[10]        (10 * 72   = 720 bytes)
//!   0x407370 user_count       (int)
//!   0x407378 current_user     (user_t *)
//!   0x407380 files[20]        (20 * 612  = 12240 bytes)
//!   0x40a350 file_count       (int)
//!   0x40a360 variables[20]    (20 * 160  = 3200 bytes)
//!   0x40afe0 variable_count   (int)
//!   0x40afe4 debug_mode       (int)
//!   0x40afe8 verbose_mode     (int)
//!   0x40aff0 _end
//!   0x40b000            end of the writable page mapping
//! ```
//!
//! Every global is therefore read back out of that array (never cached), which
//! is what the unoptimised reference build does, and accesses outside the
//! mapping terminate the process with `SIGSEGV` exactly like the original.
//! `stdout` is emulated as glibc's fully buffered stream (4096 byte blocks) so
//! that the amount of output that survives a crash matches as well.

use std::io::{BufRead, BufReader, Read, Write};

// ---------------------------------------------------------------------------
// #define constants
// ---------------------------------------------------------------------------

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: i32 = 20;
const MAX_USERS: i32 = 10;
const MAX_VARIABLES: i32 = 20;

// ---------------------------------------------------------------------------
// Emulated writable data mapping
// ---------------------------------------------------------------------------

/// First byte of the writable page mapping of the reference binary.
const REGION_BASE: i64 = 0x0040_6000;
/// Size of that mapping (page aligned end of the RW `PT_LOAD` segment).
const REGION_SIZE: i64 = 0x5000;
const REGION_END: i64 = REGION_BASE + REGION_SIZE;

// Addresses of the individual `static` objects.
const A_USERS: i64 = 0x0040_70a0;
const A_USER_COUNT: i64 = 0x0040_7370;
const A_CURRENT_USER: i64 = 0x0040_7378;
const A_FILES: i64 = 0x0040_7380;
const A_FILE_COUNT: i64 = 0x0040_a350;
const A_VARIABLES: i64 = 0x0040_a360;
const A_VARIABLE_COUNT: i64 = 0x0040_afe0;
const A_DEBUG_MODE: i64 = 0x0040_afe4;
const A_VERBOSE_MODE: i64 = 0x0040_afe8;

// struct layouts (x86-64 SysV)
//
//   typedef struct { char name[32]; char password[32];
//                    int permission_level; int logged_in; } user_t;   // 72
//   typedef struct { char filename[64]; char content[512];
//                    char owner[32]; int permissions; } file_t;       // 612
//   typedef struct { char name[32]; char value[128]; } variable_t;    // 160
const USER_SIZE: i64 = 72;
const U_NAME: i64 = 0;
const U_PASSWORD: i64 = 32;
const U_PERM_LEVEL: i64 = 64;
const U_LOGGED_IN: i64 = 68;

const FILE_SIZE: i64 = 612;
const F_FILENAME: i64 = 0;
const F_CONTENT: i64 = 64;
const F_OWNER: i64 = 576;
const F_PERMISSIONS: i64 = 608;

const VAR_SIZE: i64 = 160;
const V_NAME: i64 = 0;
const V_VALUE: i64 = 32;

// Sub-ranges of the writable mapping that are *not* plain data: clobbering
// them does not fault at the time of the write, but makes a later libc call
// fault.  (Taken from `readelf -S c_src/build/driver`.)
/// `.init_array` + `.fini_array` + `.dynamic`: used by `exit()`.
const A_DYNAMIC_LO: i64 = 0x0040_6de8;
const A_DYNAMIC_HI: i64 = 0x0040_6fc8;
/// `.got` + `.got.plt`: every libc call goes through it.
const A_GOT_LO: i64 = 0x0040_6fc8;
const A_GOT_HI: i64 = 0x0040_7078;
/// the `stdin` copy relocation: dereferenced by the next `fgets`.
const A_STDIN_LO: i64 = 0x0040_7080;
const A_STDIN_HI: i64 = 0x0040_7088;

/// glibc's stdout block size for pipes / regular files (`st_blksize`).
const STDIO_BUFSIZ: usize = 4096;

extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: i32) -> i32;
    #[link_name = "raise"]
    fn libc_raise(sig: i32) -> i32;
    #[link_name = "signal"]
    fn libc_signal(sig: i32, handler: usize) -> usize;
    #[link_name = "time"]
    fn libc_time(t: *mut i64) -> i64;
    #[link_name = "ctime"]
    fn libc_ctime(t: *const i64) -> *const std::os::raw::c_char;
}

const SIGSEGV: i32 = 11;
const SIG_DFL: usize = 0;

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
// C library helpers that work on plain slices (the `char args[][64]` locals of
// the reference program, which are always NUL terminated inside their bounds).
// ---------------------------------------------------------------------------

/// glibc `strcmp`: difference of the first differing bytes, taken as
/// `unsigned char`.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
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
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// glibc `atoi` == `(int) strtol(s, NULL, 10)`, including the LONG_MAX /
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
// The emulated machine: writable data mapping + buffered stdout
// ---------------------------------------------------------------------------

struct Mach {
    /// The whole writable page mapping, `REGION_BASE .. REGION_END`.
    mem: Vec<u8>,
    /// Bytes sitting in glibc's stdout buffer (always `< STDIO_BUFSIZ`).
    out: Vec<u8>,
    /// The `stdin` pointer was clobbered -> the next `fgets` dereferences
    /// garbage.
    stdin_dead: bool,
    /// `.fini_array`/`.dynamic` was clobbered -> `exit()` calls garbage.
    exit_dead: bool,
    /// glibc line-buffers stdout when it is a terminal (`_IO_LINE_BUF`).
    line_buffered: bool,
    /// glibc flushes the *line-buffered* streams before reading from an
    /// interactive input stream; a fully buffered stdout keeps its contents.
    flush_before_read: bool,
}

impl Mach {
    fn new() -> Mach {
        Mach {
            mem: vec![0u8; REGION_SIZE as usize],
            out: Vec::with_capacity(STDIO_BUFSIZ * 2),
            stdin_dead: false,
            exit_dead: false,
            line_buffered: unsafe { libc_isatty(1) } == 1,
            flush_before_read: unsafe { libc_isatty(0) == 1 && libc_isatty(1) == 1 },
        }
    }

    // --- stdout ----------------------------------------------------------

    /// `printf` -- appends to the stdio buffer, writing it out in whole
    /// `STDIO_BUFSIZ` blocks exactly like glibc does.
    fn emit(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
        while self.out.len() >= STDIO_BUFSIZ {
            let chunk: Vec<u8> = self.out.drain(..STDIO_BUFSIZ).collect();
            Mach::raw_write(&chunk);
        }
        if self.line_buffered {
            if let Some(p) = self.out.iter().rposition(|&c| c == b'\n') {
                let chunk: Vec<u8> = self.out.drain(..=p).collect();
                Mach::raw_write(&chunk);
            }
        }
    }

    fn raw_write(bytes: &[u8]) {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(bytes);
        let _ = lock.flush();
    }

    /// `exit(0)` / returning from `main`: the `.fini_array` destructors run
    /// before stdio is flushed, so a clobbered `.fini_array` loses the buffer.
    fn exit_flush(&mut self) {
        if self.exit_dead {
            self.segv();
        }
        self.flush();
    }

    /// `fflush(stdout)` as performed by `exit()` / returning from `main`.
    fn flush(&mut self) {
        if !self.out.is_empty() {
            let pending = std::mem::take(&mut self.out);
            Mach::raw_write(&pending);
        }
    }

    /// Death by `SIGSEGV`: the buffered stdout contents are *lost*, exactly
    /// like in the reference program.
    fn segv(&mut self) -> ! {
        unsafe {
            libc_signal(SIGSEGV, SIG_DFL);
            libc_raise(SIGSEGV);
        }
        // Not reached: the process is killed by the signal above.
        std::process::exit(139);
    }

    // --- raw memory accesses ---------------------------------------------

    fn readable(addr: i64, len: i64) -> bool {
        addr >= REGION_BASE && len >= 0 && addr <= REGION_END - len
    }

    /// Prepares a write of `len` bytes at `addr`: faults when it leaves the
    /// mapping, and records the delayed failure modes of the ranges that hold
    /// linker/libc state rather than program data.  (`.data`, `completed.0`
    /// and the inter-object padding are plain data and survive.)
    fn begin_write(&mut self, addr: i64, len: i64) -> usize {
        if !Mach::readable(addr, len) {
            self.segv();
        }
        if addr < A_GOT_HI && addr + len > A_GOT_LO {
            // the next PLT call jumps to the clobbered slot
            self.segv();
        }
        if addr < A_STDIN_HI && addr + len > A_STDIN_LO {
            self.stdin_dead = true;
        }
        if addr < A_DYNAMIC_HI && addr + len > A_DYNAMIC_LO {
            self.exit_dead = true;
        }
        (addr - REGION_BASE) as usize
    }

    fn rb(&mut self, addr: i64) -> u8 {
        if !Mach::readable(addr, 1) {
            self.segv();
        }
        self.mem[(addr - REGION_BASE) as usize]
    }

    fn wb(&mut self, addr: i64, v: u8) {
        let o = self.begin_write(addr, 1);
        self.mem[o] = v;
    }

    fn ri(&mut self, addr: i64) -> i32 {
        if !Mach::readable(addr, 4) {
            self.segv();
        }
        let o = (addr - REGION_BASE) as usize;
        i32::from_le_bytes([self.mem[o], self.mem[o + 1], self.mem[o + 2], self.mem[o + 3]])
    }

    fn wi(&mut self, addr: i64, v: i32) {
        let o = self.begin_write(addr, 4);
        self.mem[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// Reads a `user_t *`.
    fn rp(&mut self, addr: i64) -> i64 {
        if !Mach::readable(addr, 8) {
            self.segv();
        }
        let o = (addr - REGION_BASE) as usize;
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.mem[o..o + 8]);
        i64::from_le_bytes(b)
    }

    fn wp(&mut self, addr: i64, v: i64) {
        let o = self.begin_write(addr, 8);
        self.mem[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }

    // --- C string operations on emulated memory --------------------------

    /// Reads the NUL terminated string at `addr` (faults if it runs off the
    /// mapping, like the reference program does).
    fn cstr(&mut self, addr: i64) -> Vec<u8> {
        let mut out = Vec::new();
        let mut a = addr;
        loop {
            let c = self.rb(a);
            if c == 0 {
                return out;
            }
            out.push(c);
            a += 1;
        }
    }

    /// `strcpy(dst, src)` with `src` being a plain (already NUL delimited)
    /// slice.
    fn strcpy_from(&mut self, dst: i64, src: &[u8]) {
        for (k, &c) in src.iter().enumerate() {
            self.wb(dst + k as i64, c);
        }
        self.wb(dst + src.len() as i64, 0);
    }

    /// `strcpy(dst, src)` where `src` also lives in emulated memory.
    fn strcpy_mem(&mut self, dst: i64, src: i64) {
        let mut k: i64 = 0;
        loop {
            let c = self.rb(src + k);
            self.wb(dst + k, c);
            if c == 0 {
                return;
            }
            k += 1;
        }
    }

    /// `strcmp(mem_at_a, b)` -- compares lazily, so it only faults when the
    /// reference program would actually read past the mapping.
    fn strcmp_mem(&mut self, a: i64, b: &[u8]) -> i32 {
        let mut k: i64 = 0;
        loop {
            let ca = self.rb(a + k);
            let cb = if (k as usize) < b.len() { b[k as usize] } else { 0 };
            if ca != cb {
                return ca as i32 - cb as i32;
            }
            if ca == 0 {
                return 0;
            }
            k += 1;
        }
    }

    /// `strcmp` between two strings living in emulated memory.
    fn strcmp_mm(&mut self, a: i64, b: i64) -> i32 {
        let mut k: i64 = 0;
        loop {
            let ca = self.rb(a + k);
            let cb = self.rb(b + k);
            if ca != cb {
                return ca as i32 - cb as i32;
            }
            if ca == 0 {
                return 0;
            }
            k += 1;
        }
    }

    /// Struct assignment (`files[j] = files[j + 1]`).
    fn struct_copy(&mut self, dst: i64, src: i64, len: i64) {
        for k in 0..len {
            let c = self.rb(src + k);
            self.wb(dst + k, c);
        }
    }

    // --- typed access to the globals -------------------------------------

    fn user_count(&mut self) -> i32 {
        self.ri(A_USER_COUNT)
    }
    fn set_user_count(&mut self, v: i32) {
        self.wi(A_USER_COUNT, v)
    }
    fn current_user(&mut self) -> i64 {
        self.rp(A_CURRENT_USER)
    }
    fn set_current_user(&mut self, v: i64) {
        self.wp(A_CURRENT_USER, v)
    }
    fn file_count(&mut self) -> i32 {
        self.ri(A_FILE_COUNT)
    }
    fn set_file_count(&mut self, v: i32) {
        self.wi(A_FILE_COUNT, v)
    }
    fn variable_count(&mut self) -> i32 {
        self.ri(A_VARIABLE_COUNT)
    }
    fn set_variable_count(&mut self, v: i32) {
        self.wi(A_VARIABLE_COUNT, v)
    }
    fn debug_mode(&mut self) -> i32 {
        self.ri(A_DEBUG_MODE)
    }
    fn set_debug_mode(&mut self, v: i32) {
        self.wi(A_DEBUG_MODE, v)
    }
    fn verbose_mode(&mut self) -> i32 {
        self.ri(A_VERBOSE_MODE)
    }
    fn set_verbose_mode(&mut self, v: i32) {
        self.wi(A_VERBOSE_MODE, v)
    }

    /// `&users[i]`
    fn user_at(i: i32) -> i64 {
        A_USERS + i as i64 * USER_SIZE
    }
    /// `&files[i]`
    fn file_at(i: i32) -> i64 {
        A_FILES + i as i64 * FILE_SIZE
    }
    /// `&variables[i]`
    fn var_at(i: i32) -> i64 {
        A_VARIABLES + i as i64 * VAR_SIZE
    }

    /// `current_user && current_user->logged_in`
    fn have_login(&mut self) -> bool {
        let cu = self.current_user();
        cu != 0 && self.ri(cu + U_LOGGED_IN) != 0
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

impl Mach {
    // --- user management -------------------------------------------------

    fn cmd_adduser(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: adduser <username> <password> [permission_level]\n");
            return;
        }

        if self.user_count() >= MAX_USERS {
            pf!(self, "Error: Maximum users reached\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.user_count() {
            if self.strcmp_mem(Mach::user_at(i) + U_NAME, &args[0]) == 0 {
                pf!(self, "Error: User '%s' already exists\n", A::S(&args[0]));
                return;
            }
            i += 1;
        }

        let uc = self.user_count();
        self.strcpy_from(Mach::user_at(uc) + U_NAME, &args[0]);
        let uc = self.user_count();
        self.strcpy_from(Mach::user_at(uc) + U_PASSWORD, &args[1]);
        let level = if arg_count >= 3 { c_atoi(&args[2]) } else { 1 };
        let uc = self.user_count();
        self.wi(Mach::user_at(uc) + U_PERM_LEVEL, level);
        let uc = self.user_count();
        self.wi(Mach::user_at(uc) + U_LOGGED_IN, 0);
        let uc = self.user_count();
        self.set_user_count(uc + 1);

        let uc = self.user_count();
        let shown = self.ri(Mach::user_at(uc - 1) + U_PERM_LEVEL);
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

        if self.have_login() {
            let cu = self.current_user();
            let name = self.cstr(cu + U_NAME);
            pf!(
                self,
                "Error: User '%s' already logged in. Use 'logout' first.\n",
                A::S(&name)
            );
            return;
        }

        let mut i: i32 = 0;
        while i < self.user_count() {
            if self.strcmp_mem(Mach::user_at(i) + U_NAME, &args[0]) == 0 {
                if self.strcmp_mem(Mach::user_at(i) + U_PASSWORD, &args[1]) == 0 {
                    self.wi(Mach::user_at(i) + U_LOGGED_IN, 1);
                    self.set_current_user(Mach::user_at(i));
                    let cu = self.current_user();
                    let name = self.cstr(cu + U_NAME);
                    pf!(self, "Login successful. Welcome, %s!\n", A::S(&name));
                    return;
                } else {
                    pf!(self, "Error: Incorrect password\n");
                    return;
                }
            }
            i += 1;
        }

        pf!(self, "Error: User not found\n");
    }

    fn cmd_logout(&mut self) {
        if !self.have_login() {
            pf!(self, "Error: No user logged in\n");
            return;
        }

        let cu = self.current_user();
        let name = self.cstr(cu + U_NAME);
        pf!(self, "Goodbye, %s!\n", A::S(&name));
        let cu = self.current_user();
        self.wi(cu + U_LOGGED_IN, 0);
        self.set_current_user(0);
    }

    fn cmd_whoami(&mut self) {
        if !self.have_login() {
            pf!(self, "Not logged in\n");
            return;
        }

        let cu = self.current_user();
        let name = self.cstr(cu + U_NAME);
        pf!(self, "Current user: %s\n", A::S(&name));
        let cu = self.current_user();
        let perm = self.ri(cu + U_PERM_LEVEL);
        pf!(self, "Permission level: %d\n", A::I(perm));
    }

    fn cmd_listusers(&mut self) {
        if self.user_count() == 0 {
            pf!(self, "No users registered\n");
            return;
        }

        pf!(self, "Registered users:\n");
        let mut i: i32 = 0;
        while i < self.user_count() {
            let name = self.cstr(Mach::user_at(i) + U_NAME);
            let perm = self.ri(Mach::user_at(i) + U_PERM_LEVEL);
            let logged = self.ri(Mach::user_at(i) + U_LOGGED_IN);
            let flag: &[u8] = if logged != 0 { b"[logged in]" } else { b"" };
            pf!(
                self,
                "  %s (level %d) %s\n",
                A::S(&name),
                A::I(perm),
                A::S(flag)
            );
            i += 1;
        }
    }

    // --- file management -------------------------------------------------

    fn cmd_createfile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if !self.have_login() {
            pf!(self, "Error: Must be logged in\n");
            return;
        }

        if arg_count < 1 {
            pf!(self, "Usage: createfile <filename> [content]\n");
            return;
        }

        if self.file_count() >= MAX_FILES {
            pf!(self, "Error: Maximum files reached\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.file_count() {
            if self.strcmp_mem(Mach::file_at(i) + F_FILENAME, &args[0]) == 0 {
                pf!(self, "Error: File '%s' already exists\n", A::S(&args[0]));
                return;
            }
            i += 1;
        }

        let fc = self.file_count();
        self.strcpy_from(Mach::file_at(fc) + F_FILENAME, &args[0]);
        let fc = self.file_count();
        let cu = self.current_user();
        self.strcpy_mem(Mach::file_at(fc) + F_OWNER, cu + U_NAME);
        let fc = self.file_count();
        self.wi(Mach::file_at(fc) + F_PERMISSIONS, 755);

        if arg_count >= 2 {
            let fc = self.file_count();
            self.strcpy_from(Mach::file_at(fc) + F_CONTENT, &args[1]);
        } else {
            let fc = self.file_count();
            self.wb(Mach::file_at(fc) + F_CONTENT, 0);
        }

        let fc = self.file_count();
        self.set_file_count(fc + 1);
        pf!(self, "File '%s' created\n", A::S(&args[0]));
    }

    fn cmd_readfile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            pf!(self, "Usage: readfile <filename>\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.file_count() {
            if self.strcmp_mem(Mach::file_at(i) + F_FILENAME, &args[0]) == 0 {
                let name = self.cstr(Mach::file_at(i) + F_FILENAME);
                pf!(self, "=== %s ===\n", A::S(&name));
                let owner = self.cstr(Mach::file_at(i) + F_OWNER);
                pf!(self, "Owner: %s\n", A::S(&owner));
                let perm = self.ri(Mach::file_at(i) + F_PERMISSIONS);
                pf!(self, "Permissions: %d\n", A::I(perm));
                let content = self.cstr(Mach::file_at(i) + F_CONTENT);
                pf!(self, "Content: %s\n", A::S(&content));
                return;
            }
            i += 1;
        }

        pf!(self, "Error: File '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_writefile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if !self.have_login() {
            pf!(self, "Error: Must be logged in\n");
            return;
        }

        if arg_count < 2 {
            pf!(self, "Usage: writefile <filename> <content>\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.file_count() {
            if self.strcmp_mem(Mach::file_at(i) + F_FILENAME, &args[0]) == 0 {
                let cu = self.current_user();
                let owner_eq = self.strcmp_mm(Mach::file_at(i) + F_OWNER, cu + U_NAME) == 0;
                let allowed = if owner_eq {
                    true
                } else {
                    let cu = self.current_user();
                    self.ri(cu + U_PERM_LEVEL) >= 5
                };
                if allowed {
                    self.strcpy_from(Mach::file_at(i) + F_CONTENT, &args[1]);
                    pf!(self, "File '%s' updated\n", A::S(&args[0]));
                    return;
                } else {
                    pf!(self, "Error: Permission denied\n");
                    return;
                }
            }
            i += 1;
        }

        pf!(self, "Error: File '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_deletefile(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if !self.have_login() {
            pf!(self, "Error: Must be logged in\n");
            return;
        }

        if arg_count < 1 {
            pf!(self, "Usage: deletefile <filename>\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.file_count() {
            if self.strcmp_mem(Mach::file_at(i) + F_FILENAME, &args[0]) == 0 {
                let cu = self.current_user();
                let owner_eq = self.strcmp_mm(Mach::file_at(i) + F_OWNER, cu + U_NAME) == 0;
                let allowed = if owner_eq {
                    true
                } else {
                    let cu = self.current_user();
                    self.ri(cu + U_PERM_LEVEL) >= 9
                };
                if allowed {
                    let mut j: i32 = i;
                    while j < self.file_count() - 1 {
                        self.struct_copy(Mach::file_at(j), Mach::file_at(j + 1), FILE_SIZE);
                        j += 1;
                    }
                    let fc = self.file_count();
                    self.set_file_count(fc - 1);
                    pf!(self, "File '%s' deleted\n", A::S(&args[0]));
                    return;
                } else {
                    pf!(self, "Error: Permission denied\n");
                    return;
                }
            }
            i += 1;
        }

        pf!(self, "Error: File '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_listfiles(&mut self) {
        if self.file_count() == 0 {
            pf!(self, "No files\n");
            return;
        }

        pf!(self, "Files:\n");
        let mut i: i32 = 0;
        while i < self.file_count() {
            let name = self.cstr(Mach::file_at(i) + F_FILENAME);
            let owner = self.cstr(Mach::file_at(i) + F_OWNER);
            let perm = self.ri(Mach::file_at(i) + F_PERMISSIONS);
            pf!(
                self,
                "  %s (owner: %s, perm: %d)\n",
                A::S(&name),
                A::S(&owner),
                A::I(perm)
            );
            i += 1;
        }
    }

    // --- variables -------------------------------------------------------

    fn cmd_set(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 2 {
            pf!(self, "Usage: set <name> <value>\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.variable_count() {
            if self.strcmp_mem(Mach::var_at(i) + V_NAME, &args[0]) == 0 {
                self.strcpy_from(Mach::var_at(i) + V_VALUE, &args[1]);
                pf!(self, "Variable '%s' updated\n", A::S(&args[0]));
                return;
            }
            i += 1;
        }

        if self.variable_count() >= MAX_VARIABLES {
            pf!(self, "Error: Maximum variables reached\n");
            return;
        }

        let vc = self.variable_count();
        self.strcpy_from(Mach::var_at(vc) + V_NAME, &args[0]);
        let vc = self.variable_count();
        self.strcpy_from(Mach::var_at(vc) + V_VALUE, &args[1]);
        let vc = self.variable_count();
        self.set_variable_count(vc + 1);
        pf!(self, "Variable '%s' set\n", A::S(&args[0]));
    }

    fn cmd_get(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            pf!(self, "Usage: get <name>\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.variable_count() {
            if self.strcmp_mem(Mach::var_at(i) + V_NAME, &args[0]) == 0 {
                let name = self.cstr(Mach::var_at(i) + V_NAME);
                let value = self.cstr(Mach::var_at(i) + V_VALUE);
                pf!(self, "%s = %s\n", A::S(&name), A::S(&value));
                return;
            }
            i += 1;
        }

        pf!(self, "Error: Variable '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_unset(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            pf!(self, "Usage: unset <name>\n");
            return;
        }

        let mut i: i32 = 0;
        while i < self.variable_count() {
            if self.strcmp_mem(Mach::var_at(i) + V_NAME, &args[0]) == 0 {
                let mut j: i32 = i;
                while j < self.variable_count() - 1 {
                    self.struct_copy(Mach::var_at(j), Mach::var_at(j + 1), VAR_SIZE);
                    j += 1;
                }
                let vc = self.variable_count();
                self.set_variable_count(vc - 1);
                pf!(self, "Variable '%s' unset\n", A::S(&args[0]));
                return;
            }
            i += 1;
        }

        pf!(self, "Error: Variable '%s' not found\n", A::S(&args[0]));
    }

    fn cmd_listvars(&mut self) {
        if self.variable_count() == 0 {
            pf!(self, "No variables set\n");
            return;
        }

        pf!(self, "Variables:\n");
        let mut i: i32 = 0;
        while i < self.variable_count() {
            let name = self.cstr(Mach::var_at(i) + V_NAME);
            let value = self.cstr(Mach::var_at(i) + V_VALUE);
            pf!(self, "  %s = %s\n", A::S(&name), A::S(&value));
            i += 1;
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
        let result = c_strncmp(&args[0], &args[1], n as i64 as usize);

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
            let on = self.debug_mode() != 0;
            let s: &[u8] = if on { b"ON" } else { b"OFF" };
            pf!(self, "Debug mode: %s\n", A::S(s));
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.set_debug_mode(1);
            pf!(self, "Debug mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.set_debug_mode(0);
            pf!(self, "Debug mode disabled\n");
        } else {
            pf!(self, "Usage: debug [on|off]\n");
        }
    }

    fn cmd_verbose(&mut self, args: &[Vec<u8>], arg_count: usize) {
        if arg_count < 1 {
            let on = self.verbose_mode() != 0;
            let s: &[u8] = if on { b"ON" } else { b"OFF" };
            pf!(self, "Verbose mode: %s\n", A::S(s));
            return;
        }

        if c_strcmp(&args[0], b"on") == 0 {
            self.set_verbose_mode(1);
            pf!(self, "Verbose mode enabled\n");
        } else if c_strcmp(&args[0], b"off") == 0 {
            self.set_verbose_mode(0);
            pf!(self, "Verbose mode disabled\n");
        } else {
            pf!(self, "Usage: verbose [on|off]\n");
        }
    }

    fn cmd_status(&mut self) {
        pf!(self, "\n=== System Status ===\n");
        let uc = self.user_count();
        pf!(self, "Users: %d/%d\n", A::I(uc), A::I(MAX_USERS));
        let fc = self.file_count();
        pf!(self, "Files: %d/%d\n", A::I(fc), A::I(MAX_FILES));
        let vc = self.variable_count();
        pf!(self, "Variables: %d/%d\n", A::I(vc), A::I(MAX_VARIABLES));
        let cur = if self.have_login() {
            let cu = self.current_user();
            self.cstr(cu + U_NAME)
        } else {
            b"none".to_vec()
        };
        pf!(self, "Current user: %s\n", A::S(&cur));
        let d: &[u8] = if self.debug_mode() != 0 { b"ON" } else { b"OFF" };
        pf!(self, "Debug mode: %s\n", A::S(d));
        let v: &[u8] = if self.verbose_mode() != 0 { b"ON" } else { b"OFF" };
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

        if self.debug_mode() != 0 {
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
            self.exit_flush();
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
    let mut st = Mach::new();

    pf!(st, "|----------------------------------------|\n");
    pf!(st, "|   COMMAND INTERPRETER                  |\n");
    pf!(st, "|   strcmp/strncmp demonstration         |\n");
    pf!(st, "|----------------------------------------|\n");
    pf!(st, "Type 'help' for available commands\n\n");

    let stdin = std::io::stdin();
    let mut reader = BufReader::new(StdinRead(stdin));

    loop {
        pf!(st, "> ");

        if st.flush_before_read {
            st.flush();
        }

        if st.stdin_dead {
            // fgets(input, MAX_INPUT, stdin) dereferences the clobbered FILE *
            st.segv();
        }

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

        if st.verbose_mode() != 0 {
            pf!(st, "[VERBOSE] Processing: '%s'\n", A::S(input));
        }

        let owned = input.to_vec();
        st.process_command(&owned);
    }

    st.exit_flush();
}

/// Thin `Read` adapter so `BufReader` owns the stdin handle.
struct StdinRead(std::io::Stdin);

impl Read for StdinRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.lock().read(buf)
    }
}
