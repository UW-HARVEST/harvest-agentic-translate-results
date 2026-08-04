// Translation of C driver program. Preserves original behavior including bugs.

use std::cell::Cell;
use std::io::{self, Read, Write};

thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

// ======================== lib.c ========================
fn target_lib(code: i32) -> i32 {
    if code < 0 {
        return 7;
    }
    let m = code % 10;
    if m == 0 {
        return 0;
    }
    if m <= 3 {
        return 1;
    }
    if m <= 6 {
        return 2;
    }
    if m == 7 {
        return 3;
    }
    4
}

// ======================== a.c ========================
fn target_a(code: i32) -> i32 {
    if code < 0 {
        let s = STATE_A.with(|c| c.get());
        return if s & 1 != 0 { 6 } else { 5 };
    }
    let new_s = STATE_A.with(|c| {
        let s = c.get();
        let v = s ^ code.wrapping_shl(1);
        c.set(v);
        v
    });
    let k = ((code >> 2) ^ new_s) & 7;
    match k {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        5 | 6 => 5,
        _ => 7,
    }
}

fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp((x ^ 0x55).wrapping_add(7))
}

fn wrap_a(x: i32) -> i32 {
    target_a(x.wrapping_sub(5))
}

fn call_a_once(x: i32) -> i32 {
    let a = target_a(x);
    let b = wrap_a(a);
    let c = target_a(b ^ 3);
    let d = a_bias_call(target_a, b);
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

fn process_a_stream(xs: &[i32]) -> i32 {
    // acc is size_t (u64) in C; replicate that exactly so saturation logic matches.
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3i32 {
            let t = target_a(v.wrapping_add(j));
            if (t & 1) == 0 {
                // acc += t; t is int converted to size_t
                acc = acc.wrapping_add(t as i64 as u64);
                continue;
            }
            // acc ^= (t<<j); (t<<j) is int converted to size_t
            let shifted = (t.wrapping_shl(j as u32)) as i64 as u64;
            acc ^= shifted;
            if t == 5 {
                break;
            }
        }
    }
    // Reproduce original (buggy) saturation:
    //   if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    //   if (acc < -0x80000000LL) acc = -0x80000000LL;
    // With acc as u64 and signed long long literals, comparisons happen in u64 space.
    if acc > 0x7fffffff_u64 {
        acc = 0x7fffffff_u64;
    }
    if acc < 0xFFFFFFFF80000000_u64 {
        acc = 0xFFFFFFFF80000000_u64;
    }
    acc as i32
}

// ======================== b.c ========================
fn target_b(code: i32) -> i32 {
    let f = FLIPFLOP.with(|c| {
        let v = c.get() ^ 1;
        c.set(v);
        v
    });
    if code < 0 {
        return if f != 0 { 2 } else { 6 };
    }
    let mask: i32 = if f != 0 { 0x7f } else { 0x1f };
    let z = (code ^ mask) % 8;
    if z == 0 || z == 7 {
        return 4;
    }
    if z == 1 || z == 2 {
        return 3;
    }
    if z == 3 {
        return 1;
    }
    if z == 4 {
        return 0;
    }
    if z == 5 {
        return 5;
    }
    7
}

fn b_twist_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
}

fn w2_b(x: i32) -> i32 {
    target_b(x.wrapping_add(9))
}

fn call_b_once(x: i32) -> i32 {
    let a = target_b(x);
    let b = w2_b(a);
    let c = b_twist_call(target_b, a);
    let d = target_b(c ^ x);
    a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
}

fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter: i32 = 0;
        loop {
            iter += 1;
            if iter > 4 {
                break;
            }
            let t = target_b(v.wrapping_sub(iter));
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

// ======================== engine.c ========================

// classify with three implementations
// impl 0: inline_call(call_a_once, x)
// impl 1: MAC_CALL(call_b_once, x) -> call_b_once(x+1)
// impl 2: inline_call(target_lib, MAC_CALL(target_lib, x)) -> target_lib(target_lib(x+1))
fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return call_a_once(x);
    }
    if impl_id == 1 {
        return call_b_once(x.wrapping_add(1));
    }
    target_lib(target_lib(x.wrapping_add(1)))
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
        let t = target_lib(x);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

#[derive(Default)]
struct Vm {
    stack: Vec<i32>,
    trace: Vec<i32>,
    steps: i32,
}

impl Vm {
    fn new() -> Self {
        Self::default()
    }
    fn peek(&self, def: i32) -> i32 {
        self.stack.last().copied().unwrap_or(def)
    }
    fn push_stack(&mut self, x: i32) {
        self.stack.push(x);
    }
    fn pop_stack(&mut self) -> Option<i32> {
        self.stack.pop()
    }
    fn trace_push(&mut self, t: i32) {
        self.trace.push(t);
    }
    fn print(&self, label: &str, out: &mut dyn Write) -> io::Result<()> {
        write!(
            out,
            "{}STACK_TOP={} STEPS={} TRACE=",
            label,
            self.peek(-777),
            self.steps
        )?;
        let alphabet = b"abcdefghijklmnopqrstuvwxyz";
        for &t in &self.trace {
            let idx = (t & 25) as usize;
            out.write_all(&[alphabet[idx]])?;
        }
        out.write_all(b"\n")?;
        Ok(())
    }
}

fn run_engine(impl_id: i32, code: &[i32], vm: &mut Vm) -> i32 {
    let n = code.len();
    let mut ip: usize = 0;
    while ip < n {
        let op = code[ip];
        ip += 1;
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            0 => {
                if ip >= n {
                    return 1;
                }
                let imm = code[ip];
                ip += 1;
                vm.push_stack(imm);
                vm.trace_push(0);
            }
            1 => {
                let b = match vm.pop_stack() {
                    Some(v) => v,
                    None => return 2,
                };
                let a = match vm.pop_stack() {
                    Some(v) => v,
                    None => return 2,
                };
                vm.push_stack(a.wrapping_add(b));
                vm.trace_push(1);
            }
            2 => {
                let b = match vm.pop_stack() {
                    Some(v) => v,
                    None => return 3,
                };
                let a = match vm.pop_stack() {
                    Some(v) => v,
                    None => return 3,
                };
                vm.push_stack(a.wrapping_mul(b));
                vm.trace_push(2);
            }
            3 => {
                let a = vm.peek(0);
                vm.push_stack(a);
                vm.trace_push(3);
            }
            4 => {
                if vm.pop_stack().is_none() {
                    return 4;
                }
                vm.trace_push(4);
            }
            5 => {
                let x = vm.peek(0);
                let bucket = classify(impl_id, x);
                vm.push_stack(bucket);
                match bucket {
                    0 => vm.trace_push(5),
                    1 => vm.trace_push(6),
                    2 => vm.trace_push(7),
                    3 | 4 => vm.trace_push(8),
                    _ => vm.trace_push(9),
                }
            }
            6 => {
                if ip >= n {
                    return 5;
                }
                let k = code[ip];
                ip += 1;
                let cond = match vm.pop_stack() {
                    Some(v) => v,
                    None => return 6,
                };
                if cond != 0 {
                    // (size_t)k > p.n - p.ip
                    if (k as usize) > n - ip {
                        return 7;
                    }
                    ip += k as usize;
                    vm.trace_push(10);
                } else {
                    vm.trace_push(11);
                }
            }
            7 => {
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
                    let inner_slice = &code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, inner_slice, &mut *vm);
                    if rc != 0 {
                        // Match C: assigns ip = saved_ip + 1 here, then unconditionally again below.
                        vm.trace_push(12);
                        break;
                    }
                    i += 1;
                }
                ip = saved_ip + 1;
            }
            8 => {
                let x = vm.peek(0);
                let y = classify(impl_id, x);
                vm.push_stack(y);
                vm.trace_push(13);
            }
            9 => {
                if ip >= n {
                    return 10;
                }
                let m = code[ip];
                ip += 1;
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m_usize = m as usize;
                let mut tmp = vec![0i32; m_usize];
                // Original C code pops the same items twice (bug); replicate exactly.
                for i in (0..m_usize).rev() {
                    if let Some(val) = vm.pop_stack() {
                        tmp[i] = val;
                    }
                }
                for i in (0..m_usize).rev() {
                    if let Some(val) = vm.pop_stack() {
                        tmp[i] = val;
                    }
                }
                let s = process_stream(impl_id, &tmp);
                vm.push_stack(s);
                vm.trace_push(14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}

// ======================== main.c ========================

fn usage(prog: &str) {
    eprintln!("Usage: {} [--stdin] [bytecodes...]", prog);
    eprintln!("Bytecodes are integers forming a small VM program.");
}

// Replicates strtol(s, &e, 10) followed by check that *e == '\0'.
// Returns Some(i32) if entire string parses; None otherwise.
fn try_parse_strtol_full(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    let mut i: usize = 0;
    // C isspace: ' ', \t, \n, \v, \f, \r
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
            _ => break,
        }
    }
    let neg = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
        false
    } else {
        false
    };
    let digits_start = i;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i64;
        val = val.wrapping_mul(10);
        val = if neg {
            val.wrapping_sub(d)
        } else {
            val.wrapping_add(d)
        };
        i += 1;
    }
    if i == digits_start {
        return None; // no digits parsed
    }
    if i != bytes.len() {
        return None; // trailing garbage
    }
    // Truncate i64 (long) to i32 (int) like (int)t in C
    Some(val as i32)
}

fn read_stdin(v: &mut Vec<i32>) -> usize {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut count: usize = 0;
    loop {
        // fgets-like: read up to 4095 bytes or until newline
        let buf = match fgets_like(&mut handle, 4096) {
            Some(b) => b,
            None => break,
        };
        // Tokenize on space/tab/newline/CR
        let mut p: usize = 0;
        while p < buf.len() {
            let mut q = p;
            while q < buf.len() {
                let c = buf[q];
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                    break;
                }
                q += 1;
            }
            if q > p {
                if let Ok(token) = std::str::from_utf8(&buf[p..q]) {
                    if let Some(t) = try_parse_strtol_full(token) {
                        v.push(t);
                        count += 1;
                    }
                }
            }
            // C: p = (*q? q+1 : q)
            if q < buf.len() {
                p = q + 1;
            } else {
                p = q;
            }
        }
    }
    count
}

fn fgets_like<R: Read>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    if size <= 1 {
        return None;
    }
    let cap = size - 1;
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < cap {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

fn run() -> i32 {
    let argv: Vec<String> = std::env::args().collect();
    let prog = if argv.is_empty() {
        String::from("driver")
    } else {
        argv[0].clone()
    };

    let mut use_stdin = false;
    let mut code: Vec<i32> = Vec::new();

    let mut i = 1;
    while i < argv.len() {
        let arg = &argv[i];
        if arg == "--help" {
            usage(&prog);
            return 0;
        } else if arg == "--stdin" {
            use_stdin = true;
        } else {
            match try_parse_strtol_full(arg) {
                Some(t) => code.push(t),
                None => {
                    eprintln!("skip '{}'", arg);
                }
            }
        }
        i += 1;
    }

    if use_stdin {
        read_stdin(&mut code);
    }

    if code.is_empty() {
        eprintln!("no program");
        return 2;
    }

    let mut vm_a = Vm::new();
    let mut vm_b = Vm::new();
    let mut vm_e = Vm::new();

    let rc_a = run_engine(0, &code, &mut vm_a);
    let rc_b = run_engine(1, &code, &mut vm_b);
    let rc_e = run_engine(2, &code, &mut vm_e);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    let _ = vm_a.print("A:", &mut out);
    let _ = vm_b.print("B:", &mut out);
    let _ = vm_e.print("EXT:", &mut out);
    let _ = out.flush();

    0
}

fn main() {
    let code = run();
    std::process::exit(code);
}

