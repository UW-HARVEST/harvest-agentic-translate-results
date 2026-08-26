// Rust translation of the C `driver` executable.
//
// Goal: produce byte-identical output to the original C program for the same
// inputs. The original C exhibits some quirky / buggy behavior (e.g. the
// `process_a_stream` clamp logic which always yields INT_MIN, and the
// double-pop in engine opcode 9). These behaviors are reproduced exactly rather
// than "fixed".

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::exit;

// ---------------------------------------------------------------------------
// Persistent per-implementation state.
//
// In the C code, `a.c` and `b.c` each declare a file-scope `static int`
// (`state_a` / `flipflop`) that persists for the whole program run. We model
// them with thread-local Cells (the program is single-threaded).
// ---------------------------------------------------------------------------
thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

// ---------------------------------------------------------------------------
// lib.c : the global (extern) `target`.
// ---------------------------------------------------------------------------
fn target_global(code: i32) -> i32 {
    if code < 0 {
        return 7;
    }
    let m = code % 10;
    if m == 0 {
        0
    } else if m <= 3 {
        1
    } else if m <= 6 {
        2
    } else if m == 7 {
        3
    } else {
        4
    }
}

// ---------------------------------------------------------------------------
// a.c : static `target` (uses `state_a`), plus call_a_once / process_a_stream.
// ---------------------------------------------------------------------------
fn a_target(code: i32) -> i32 {
    STATE_A.with(|s| {
        if code < 0 {
            return if s.get() & 1 != 0 { 6 } else { 5 };
        }
        let ns = s.get() ^ (code << 1);
        s.set(ns);
        let k = ((code >> 2) ^ ns) & 7;
        match k {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 1,
            4 => 3,
            5 | 6 => 5,
            _ => 7,
        }
    })
}

// static inline int a_bias_call(int (*fp)(int), int x){ return fp((x ^ 0x55) + 7); }
fn a_bias_call(x: i32) -> i32 {
    a_target((x ^ 0x55).wrapping_add(7))
}

// static inline int wrap(int x){ return target(x-5); }
fn a_wrap(x: i32) -> i32 {
    a_target(x.wrapping_sub(5))
}

fn call_a_once(x: i32) -> i32 {
    let a = a_target(x);
    let b = a_wrap(a);
    let c = a_target(b ^ 3);
    let d = a_bias_call(b); // A_MAC_CALL(&target, b)
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: u64 = 0; // size_t
    for &v in xs {
        for j in 0..3i32 {
            let t = a_target(v.wrapping_add(j));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t as u64);
                continue;
            }
            acc ^= (t << j) as u64;
            if t == 5 {
                break;
            }
        }
    }
    // The C comparisons promote both operands to unsigned long long.
    //   if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    //   if (acc < -0x80000000LL) acc = -0x80000000LL;
    // -0x80000000LL, when converted to unsigned 64-bit, is 0xFFFFFFFF_80000000,
    // so for any "small" acc the second comparison is always true and acc ends
    // up being that huge value, which truncates to INT_MIN on the (int) cast.
    if acc > 0x7fff_ffff_u64 {
        acc = 0x7fff_ffff_u64;
    }
    if acc < 0xFFFF_FFFF_8000_0000_u64 {
        acc = 0xFFFF_FFFF_8000_0000_u64;
    }
    acc as i32
}

// ---------------------------------------------------------------------------
// b.c : static `target` (uses `flipflop`), plus call_b_once / process_b_stream.
// ---------------------------------------------------------------------------
fn b_target(code: i32) -> i32 {
    FLIPFLOP.with(|f| {
        f.set(f.get() ^ 1);
        let ff = f.get();
        if code < 0 {
            return if ff != 0 { 2 } else { 6 };
        }
        let mask = if ff != 0 { 0x7f } else { 0x1f };
        let z = (code ^ mask) % 8;
        if z == 0 || z == 7 {
            4
        } else if z == 1 || z == 2 {
            3
        } else if z == 3 {
            1
        } else if z == 4 {
            0
        } else if z == 5 {
            5
        } else {
            7
        }
    })
}

// static inline int b_twist_call(int (*fp)(int), int x){ return fp(((x + 9) ^ 0x2222) - 17); }
fn b_twist_call(x: i32) -> i32 {
    b_target((x.wrapping_add(9) ^ 0x2222).wrapping_sub(17))
}

// static inline int w2(int x){ return target(x+9); }
fn b_w2(x: i32) -> i32 {
    b_target(x.wrapping_add(9))
}

fn call_b_once(x: i32) -> i32 {
    let a = b_target(x);
    let b = b_w2(a);
    let c = b_twist_call(a); // B_MAC_CALL(&target, a)
    let d = b_target(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter: i32 = 0;
        loop {
            iter += 1;
            if !(iter <= 4) {
                break;
            }
            let t = b_target(v.wrapping_sub(iter));
            if t == 6 {
                acc = acc.wrapping_sub(t);
                break;
            }
            if t == 3 {
                continue;
            }
            acc = acc.wrapping_mul(3) ^ t;
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// engine.c
// ---------------------------------------------------------------------------
fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return call_a_once(x); // inline_call(call_a_once, x)
    }
    if impl_id == 1 {
        return call_b_once(x.wrapping_add(1)); // MAC_CALL(call_b_once, x)
    }
    // inline_call(&target, MAC_CALL(target, x)) == target(target(x + 1))
    target_global(target_global(x.wrapping_add(1)))
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }
    let mut acc: i32 = 0;
    for &x in buf {
        let t = target_global(x);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

struct Vm {
    stack: Vec<i32>,
    trace: Vec<i32>,
    steps: i32,
}

impl Vm {
    fn new() -> Vm {
        Vm {
            stack: Vec::new(),
            trace: Vec::new(),
            steps: 0,
        }
    }
}

fn iv_peek(v: &[i32], def: i32) -> i32 {
    match v.last() {
        Some(&x) => x,
        None => def,
    }
}

fn vm_trace(vm: &mut Vm, t: i32) {
    vm.trace.push(t);
}

fn run_engine(impl_id: i32, code: &[i32], vm: &mut Vm) -> i32 {
    let n = code.len();
    let mut ip: usize = 0;
    // prog_fetch(&p, &out): read code[ip], advance; fail if ip >= n.
    while ip < n {
        let op = code[ip];
        ip += 1;
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            0 => {
                // push immediate
                if ip >= n {
                    return 1;
                }
                let imm = code[ip];
                ip += 1;
                vm.stack.push(imm);
                vm_trace(vm, 0);
            }
            1 => {
                // add
                let b = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 2,
                };
                let a = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 2,
                };
                vm.stack.push(a.wrapping_add(b));
                vm_trace(vm, 1);
            }
            2 => {
                // mul
                let b = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 3,
                };
                let a = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 3,
                };
                vm.stack.push(a.wrapping_mul(b));
                vm_trace(vm, 2);
            }
            3 => {
                // dup
                let a = iv_peek(&vm.stack, 0);
                vm.stack.push(a);
                vm_trace(vm, 3);
            }
            4 => {
                // drop
                if vm.stack.pop().is_none() {
                    return 4;
                }
                vm_trace(vm, 4);
            }
            5 => {
                let x = iv_peek(&vm.stack, 0);
                let bucket = classify(impl_id, x);
                vm.stack.push(bucket);
                match bucket {
                    0 => vm_trace(vm, 5),
                    1 => vm_trace(vm, 6),
                    2 => vm_trace(vm, 7),
                    3 | 4 => vm_trace(vm, 8),
                    _ => vm_trace(vm, 9),
                }
            }
            6 => {
                // conditional jump
                if ip >= n {
                    return 5;
                }
                let k = code[ip];
                ip += 1;
                let cond = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 6,
                };
                if cond != 0 {
                    // (size_t)k > p.n - p.ip
                    let k_sz = k as usize; // sign-extends negative k to a huge value
                    if k_sz > n - ip {
                        return 7;
                    }
                    ip += k_sz;
                    vm_trace(vm, 10);
                } else {
                    vm_trace(vm, 11);
                }
            }
            7 => {
                // bounded loop over a single inner instruction
                if ip >= n {
                    return 8;
                }
                let times = code[ip];
                ip += 1;
                if ip >= n {
                    return 9;
                }
                let saved_ip = ip;
                let mut i: i32 = 0;
                while i < times {
                    let sub = &code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, sub, vm);
                    if rc != 0 {
                        vm_trace(vm, 12);
                        break;
                    }
                    i += 1;
                }
                ip = saved_ip + 1;
            }
            8 => {
                let x = iv_peek(&vm.stack, 0);
                let y = classify(impl_id, x);
                vm.stack.push(y);
                vm_trace(vm, 13);
            }
            9 => {
                // stream reduce (note the C double-pop bug is reproduced here)
                if ip >= n {
                    return 10;
                }
                let m = code[ip];
                ip += 1;
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m_us = m as usize;
                let mut tmp = vec![0i32; m_us];
                // First pop loop: always fully succeeds (stack.len >= m).
                for i in (0..m_us).rev() {
                    if let Some(v) = vm.stack.pop() {
                        tmp[i] = v;
                    }
                }
                // Second pop loop: pops again; failing pops leave tmp[i] intact.
                for i in (0..m_us).rev() {
                    if let Some(v) = vm.stack.pop() {
                        tmp[i] = v;
                    }
                }
                let s = process_stream(impl_id, &tmp);
                vm.stack.push(s);
                vm_trace(vm, 14);
            }
            10 => {
                return 0;
            }
            _ => {
                return 99;
            }
        }
    }
    0
}

fn vm_print(out: &mut Vec<u8>, label: &str, vm: &Vm) {
    let top = iv_peek(&vm.stack, -777);
    out.extend_from_slice(
        format!("{}STACK_TOP={} STEPS={} TRACE=", label, top, vm.steps).as_bytes(),
    );
    const LETTERS: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";
    for &t in &vm.trace {
        let idx = (t & 25) as usize;
        out.push(LETTERS[idx]);
    }
    out.push(b'\n');
}

// ---------------------------------------------------------------------------
// strtol emulation (base 10), matching C semantics used by the program.
//
// Returns (value_as_long, index_of_first_unconverted_byte). When no conversion
// is performed the end index is 0 (i.e. the original start), matching C which
// stores the original nptr into endptr.
// ---------------------------------------------------------------------------
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn c_strtol(bytes: &[u8]) -> (i64, usize) {
    let n = bytes.len();
    let mut i = 0usize;
    while i < n && is_c_space(bytes[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut any = false;
    let mut acc: u128 = 0;
    let mut saturated = false;
    while i < n && bytes[i].is_ascii_digit() {
        any = true;
        let d = (bytes[i] - b'0') as u128;
        if !saturated {
            acc = acc * 10 + d;
            if acc > (1u128 << 64) {
                saturated = true;
            }
        }
        i += 1;
    }
    if !any {
        // No conversion performed: endptr = original nptr (index 0).
        return (0, 0);
    }
    let result: i64 = if neg {
        if saturated || acc >= (1u128 << 63) {
            i64::MIN
        } else {
            -(acc as i64)
        }
    } else if saturated || acc > i64::MAX as u128 {
        i64::MAX
    } else {
        acc as i64
    };
    (result, i)
}

// Parse a full token/argument. Valid iff the entire slice is consumed
// (equivalent to C's `e && *e == '\0'`). Returns the (int)-cast value.
fn c_strtol_full(bytes: &[u8]) -> Option<i32> {
    let (val, end) = c_strtol(bytes);
    if end == bytes.len() {
        Some(val as i32)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// stdin reading: replicate fgets(buf, 4096, stdin) + manual tokenizer.
// ---------------------------------------------------------------------------
fn tokenize_chunk(buf: &[u8], v: &mut Vec<i32>) {
    // The C tokenizer walks a NUL-terminated string, so stop at the first NUL.
    let slen = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let s = &buf[..slen];
    let mut i = 0usize;
    while i < s.len() {
        let start = i;
        while i < s.len()
            && s[i] != b' '
            && s[i] != b'\t'
            && s[i] != b'\n'
            && s[i] != b'\r'
        {
            i += 1;
        }
        let token = &s[start..i];
        if !token.is_empty() {
            if let Some(val) = c_strtol_full(token) {
                v.push(val);
            }
        }
        // Skip the single delimiter (q + 1); if at end of string, loop stops.
        if i < s.len() {
            i += 1;
        }
    }
}

fn read_stdin(v: &mut Vec<i32>) {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() {
        return;
    }
    let mut pos = 0usize;
    while pos < input.len() {
        // Emulate one fgets() call: read up to 4095 bytes, stopping after a
        // newline (which is kept in the buffer).
        let mut end = pos;
        let mut taken = 0usize;
        while end < input.len() && taken < 4095 {
            let c = input[end];
            end += 1;
            taken += 1;
            if c == b'\n' {
                break;
            }
        }
        let chunk = &input[pos..end];
        pos = end;
        tokenize_chunk(chunk, v);
    }
}

fn usage(prog: &[u8]) {
    let mut msg = Vec::new();
    msg.extend_from_slice(b"Usage: ");
    msg.extend_from_slice(prog);
    msg.extend_from_slice(b" [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n");
    let stderr = io::stderr();
    let mut h = stderr.lock();
    let _ = h.write_all(&msg);
    let _ = h.flush();
}

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();
    let prog_name: &[u8] = args.first().map(|v| v.as_slice()).unwrap_or(b"");

    let mut use_stdin = false;
    let mut code: Vec<i32> = Vec::new();

    for arg in args.iter().skip(1) {
        if arg.as_slice() == b"--help" {
            usage(prog_name);
            exit(0);
        } else if arg.as_slice() == b"--stdin" {
            use_stdin = true;
        } else if let Some(val) = c_strtol_full(arg) {
            code.push(val);
        } else {
            let mut msg = Vec::new();
            msg.extend_from_slice(b"skip '");
            msg.extend_from_slice(arg);
            msg.extend_from_slice(b"'\n");
            let stderr = io::stderr();
            let mut h = stderr.lock();
            let _ = h.write_all(&msg);
            let _ = h.flush();
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.is_empty() {
        let stderr = io::stderr();
        let mut h = stderr.lock();
        let _ = h.write_all(b"no program\n");
        let _ = h.flush();
        exit(2);
    }

    let mut vm_a = Vm::new();
    let mut vm_b = Vm::new();
    let mut vm_e = Vm::new();

    let rc_a = run_engine(0, &code, &mut vm_a);
    let rc_b = run_engine(1, &code, &mut vm_b);
    let rc_e = run_engine(2, &code, &mut vm_e);

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(format!("RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e).as_bytes());
    vm_print(&mut out, "A:", &vm_a);
    vm_print(&mut out, "B:", &vm_b);
    vm_print(&mut out, "EXT:", &vm_e);

    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(&out);
    let _ = h.flush();
}
