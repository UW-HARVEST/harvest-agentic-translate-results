use std::cell::Cell;
use std::env;
use std::io::{self, BufRead};

// ---------------------------------------------------------------------------
// IntVec
// ---------------------------------------------------------------------------
struct IntVec {
    data: Vec<i32>,
}
impl IntVec {
    fn new() -> Self { IntVec { data: Vec::new() } }
    fn len(&self) -> usize { self.data.len() }
    fn push(&mut self, x: i32) { self.data.push(x); }
    fn pop(&mut self) -> Option<i32> { self.data.pop() }
    fn peek(&self, def: i32) -> i32 {
        self.data.last().copied().unwrap_or(def)
    }
}

// ---------------------------------------------------------------------------
// Program
// ---------------------------------------------------------------------------
struct Program<'a> {
    code: &'a [i32],
    ip: usize,
}
impl<'a> Program<'a> {
    fn new(code: &'a [i32]) -> Self { Program { code, ip: 0 } }
    fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.code.len() { return None; }
        let v = self.code[self.ip];
        self.ip += 1;
        Some(v)
    }
}

// ---------------------------------------------------------------------------
// VM
// ---------------------------------------------------------------------------
struct VM {
    stack: IntVec,
    trace: IntVec,
    steps: i32,
}
impl VM {
    fn new() -> Self { VM { stack: IntVec::new(), trace: IntVec::new(), steps: 0 } }
    fn trace(&mut self, t: i32) { self.trace.push(t); }
    fn print(&self, label: &str) {
        let top = self.stack.peek(-777);
        print!("{}STACK_TOP={} STEPS={} TRACE=", label, top, self.steps);
        let alpha = b"abcdefghijklmnopqrstuvwxyz";
        for &t in &self.trace.data {
            let idx = (t as usize) & 25;
            print!("{}", alpha[idx] as char);
        }
        println!();
    }
}

// ---------------------------------------------------------------------------
// TU-A: target_a  (file-static state_a)
// ---------------------------------------------------------------------------
thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
}

fn target_a(code: i32) -> i32 {
    STATE_A.with(|sa| {
        let state_a = sa.get();
        if code < 0 {
            return if (state_a & 1) != 0 { 6 } else { 5 };
        }
        let new_state = state_a ^ (code << 1);
        sa.set(new_state);
        let k = ((code >> 2) ^ new_state) & 7;
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

fn a_bias_call(x: i32) -> i32 {
    target_a((x ^ 0x55) + 7)
}

fn wrap_a(x: i32) -> i32 {
    target_a(x - 5)
}

fn call_a_once(x: i32) -> i32 {
    let a = target_a(x);
    let b = wrap_a(a);
    let c = target_a(b ^ 3);
    let d = a_bias_call(b);
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: usize = 0;
    for &v in xs {
        for j in 0..3i32 {
            let t = target_a(v.wrapping_add(j));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t as usize);
                continue;
            }
            acc ^= (t << j) as usize;
            if t == 5 { break; }
        }
    }
    if acc > 0x7fffffff_usize { acc = 0x7fffffff_usize; }
    // C bug: "acc < -0x80000000LL" where acc is size_t.
    // -0x80000000LL converted to unsigned 64-bit = 0xFFFFFFFF80000000.
    // So this always triggers for small acc values, overwriting with that huge value.
    let neg_clamp = (-0x80000000i64) as u64 as usize; // 0xFFFFFFFF80000000
    if acc < neg_clamp { acc = neg_clamp; }
    acc as i32
}

// ---------------------------------------------------------------------------
// TU-B: target_b  (file-static flipflop)
// ---------------------------------------------------------------------------
thread_local! {
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

fn target_b(code: i32) -> i32 {
    FLIPFLOP.with(|ff| {
        let new_ff = ff.get() ^ 1;
        ff.set(new_ff);
        if code < 0 {
            return if new_ff != 0 { 2 } else { 6 };
        }
        let z = (code ^ (if new_ff != 0 { 0x7f } else { 0x1f })) % 8;
        if z == 0 || z == 7 { return 4; }
        if z == 1 || z == 2 { return 3; }
        if z == 3 { return 1; }
        if z == 4 { return 0; }
        if z == 5 { return 5; }
        7
    })
}

fn b_twist_call(x: i32) -> i32 {
    target_b(((x + 9) ^ 0x2222) - 17)
}

fn w2(x: i32) -> i32 {
    target_b(x + 9)
}

fn call_b_once(x: i32) -> i32 {
    let a = target_b(x);
    let b = w2(a);
    let c = b_twist_call(a);
    let d = target_b(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter = 0i32;
        loop {
            iter += 1;
            if iter > 4 { break; }
            let t = target_b(v - iter);
            if t == 6 { acc -= t; break; }
            if t == 3 { continue; }
            acc = (acc.wrapping_mul(3)) ^ t;
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// TU-LIB: target_ext  (no static state)
// ---------------------------------------------------------------------------
fn target_ext(code: i32) -> i32 {
    if code < 0 { return 7; }
    let m = code % 10;
    if m == 0 { return 0; }
    if m <= 3 { return 1; }
    if m <= 6 { return 2; }
    if m == 7 { return 3; }
    4
}

// ---------------------------------------------------------------------------
// Engine helpers: classify, process_stream
// ---------------------------------------------------------------------------
fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        // inline_call(call_a_once, x)
        return call_a_once(x);
    }
    if impl_id == 1 {
        // MAC_CALL(call_b_once, x) => call_b_once(x+1)
        return call_b_once(x + 1);
    }
    // impl==2: inline_call(&target, MAC_CALL(target, x))
    //        = target_ext(target_ext(x+1))
    let inner = target_ext(x + 1);
    target_ext(inner)
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 { return process_a_stream(buf); }
    if impl_id == 1 { return process_b_stream(buf); }
    let mut acc: i32 = 0;
    for &v in buf {
        let t = target_ext(v);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// run_engine
// ---------------------------------------------------------------------------
fn run_engine(impl_id: i32, code: &[i32], vm: &mut VM) -> i32 {
    let mut p = Program::new(code);
    while let Some(op) = p.fetch() {
        vm.steps += 1;
        match op {
            0 => {
                let imm = match p.fetch() { Some(v) => v, None => return 1 };
                vm.stack.push(imm);
                vm.trace(0);
            }
            1 => {
                let b = match vm.stack.pop() { Some(v) => v, None => return 2 };
                let a = match vm.stack.pop() { Some(v) => v, None => return 2 };
                vm.stack.push(a.wrapping_add(b));
                vm.trace(1);
            }
            2 => {
                let b = match vm.stack.pop() { Some(v) => v, None => return 3 };
                let a = match vm.stack.pop() { Some(v) => v, None => return 3 };
                vm.stack.push(a.wrapping_mul(b));
                vm.trace(2);
            }
            3 => {
                let a = vm.stack.peek(0);
                vm.stack.push(a);
                vm.trace(3);
            }
            4 => {
                if vm.stack.pop().is_none() { return 4; }
                vm.trace(4);
            }
            5 => {
                let x = vm.stack.peek(0);
                let bucket = classify(impl_id, x);
                vm.stack.push(bucket);
                match bucket {
                    0 => vm.trace(5),
                    1 => vm.trace(6),
                    2 => vm.trace(7),
                    3 | 4 => vm.trace(8),
                    _ => vm.trace(9),
                }
            }
            6 => {
                let k = match p.fetch() { Some(v) => v, None => return 5 };
                let cond = match vm.stack.pop() { Some(v) => v, None => return 6 };
                if cond != 0 {
                    let k_usize = k as usize;
                    let remaining = p.code.len() - p.ip;
                    if k_usize > remaining { return 7; }
                    p.ip += k_usize;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            7 => {
                let times = match p.fetch() { Some(v) => v, None => return 8 };
                if p.ip >= p.code.len() { return 9; }
                let saved_ip = p.ip;
                for _i in 0..times {
                    let inner_code = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, inner_code, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm.trace(12);
                        break;
                    }
                }
                p.ip = saved_ip + 1;
            }
            8 => {
                let x = vm.stack.peek(0);
                let y = classify(impl_id, x);
                vm.stack.push(y);
                vm.trace(13);
            }
            9 => {
                let m = match p.fetch() { Some(v) => v, None => return 10 };
                if m < 0 || (m as usize) > vm.stack.len() { return 11; }
                let m_usize = m as usize;
                // First pop: fill tmp[m-1..0]
                let mut tmp = vec![0i32; m_usize];
                for i in (0..m_usize).rev() {
                    tmp[i] = vm.stack.pop().unwrap();
                }
                // Second pop: overwrite tmp[m-1..0] — this is the C bug.
                // iv_pop only writes *out on success; on failure tmp[i] keeps first-pop value.
                for i in (0..m_usize).rev() {
                    if let Some(v) = vm.stack.pop() {
                        tmp[i] = v;
                    }
                }
                let s = process_stream(impl_id, &tmp);
                vm.stack.push(s);
                vm.trace(14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}

// ---------------------------------------------------------------------------
// read_stdin
// ---------------------------------------------------------------------------
fn read_stdin(code: &mut IntVec) -> usize {
    let stdin = io::stdin();
    let mut count = 0usize;
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        for token in line.split(|c: char| c == ' ' || c == '\t' || c == '\r') {
            if !token.is_empty() {
                if let Ok(v) = token.parse::<i64>() {
                    code.push(v as i32);
                    count += 1;
                }
            }
        }
    }
    count
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------
fn main() {
    let args: Vec<String> = env::args().collect();
    let mut use_stdin = false;
    let mut code = IntVec::new();

    for i in 1..args.len() {
        if args[i] == "--help" {
            eprintln!("Usage: {} [--stdin] [bytecodes...]", args[0]);
            eprintln!("Bytecodes are integers forming a small VM program.");
            return;
        } else if args[i] == "--stdin" {
            use_stdin = true;
        } else {
            match args[i].parse::<i64>() {
                Ok(t) => code.push(t as i32),
                Err(_) => eprintln!("skip '{}'", args[i]),
            }
        }
    }
    if use_stdin { read_stdin(&mut code); }
    if code.len() == 0 {
        eprintln!("no program");
        std::process::exit(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();

    let rc_a = run_engine(0, &code.data, &mut vm_a);
    let rc_b = run_engine(1, &code.data, &mut vm_b);
    let rc_e = run_engine(2, &code.data, &mut vm_e);

    println!("RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    vm_a.print("A:");
    vm_b.print("B:");
    vm_e.print("EXT:");
}
