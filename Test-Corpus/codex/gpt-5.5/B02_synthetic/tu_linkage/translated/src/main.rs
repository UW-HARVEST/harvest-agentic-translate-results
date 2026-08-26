use std::env;
use std::io::{self, Read};
use std::process;
use std::sync::atomic::{AtomicI32, Ordering};

#[derive(Default)]
struct IntVec {
    data: Vec<i32>,
}

impl IntVec {
    fn push(&mut self, x: i32) -> bool {
        self.data.push(x);
        true
    }

    fn pop(&mut self) -> Option<i32> {
        self.data.pop()
    }

    fn peek(&self, def: i32) -> i32 {
        self.data.last().copied().unwrap_or(def)
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

struct Program<'a> {
    code: &'a [i32],
    ip: usize,
}

impl<'a> Program<'a> {
    fn new(code: &'a [i32]) -> Self {
        Self { code, ip: 0 }
    }

    fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.code.len() {
            None
        } else {
            let out = self.code[self.ip];
            self.ip += 1;
            Some(out)
        }
    }
}

#[derive(Default)]
struct Vm {
    stack: IntVec,
    trace: IntVec,
    steps: i32,
}

impl Vm {
    fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }

    fn print(&self, label: &str) {
        print!(
            "{}STACK_TOP={} STEPS={} TRACE=",
            label,
            self.stack.peek(-777),
            self.steps
        );
        for &t in &self.trace.data {
            let idx = (t & 25) as usize;
            print!("{}", (b"abcdefghijklmnopqrstuvwxyz"[idx]) as char);
        }
        println!();
    }
}

static STATE_A: AtomicI32 = AtomicI32::new(0);
static FLIPFLOP: AtomicI32 = AtomicI32::new(0);

fn c_bool(x: i32) -> bool {
    x != 0
}

fn target_ext(code: i32) -> i32 {
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

fn target_a(code: i32) -> i32 {
    let state = STATE_A.load(Ordering::Relaxed);
    if code < 0 {
        return if (state & 1) != 0 { 6 } else { 5 };
    }
    let next = state ^ code.wrapping_shl(1);
    STATE_A.store(next, Ordering::Relaxed);
    let k = ((code >> 2) ^ next) & 7;
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
    let fp: fn(i32) -> i32 = target_a;
    let a = fp(x);
    let b = wrap_a(a);
    let c = target_a(b ^ 3);
    let d = a_bias_call(target_a, b);
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: usize = 0;
    for &v in xs {
        for j in 0..3 {
            let t = target_a(v.wrapping_add(j));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t as usize);
                continue;
            }
            acc ^= t.wrapping_shl(j as u32) as usize;
            if t == 5 {
                break;
            }
        }
    }
    if acc > 0x7fffffff {
        acc = 0x7fffffff;
    }
    if acc < (-0x80000000_i64 as usize) {
        acc = -0x80000000_i64 as usize;
    }
    acc as i32
}

fn target_b(code: i32) -> i32 {
    let next = FLIPFLOP.load(Ordering::Relaxed) ^ 1;
    FLIPFLOP.store(next, Ordering::Relaxed);
    if code < 0 {
        return if next != 0 { 2 } else { 6 };
    }
    let z = (code ^ if next != 0 { 0x7f } else { 0x1f }) % 8;
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
    fp((x.wrapping_add(9) ^ 0x2222).wrapping_sub(17))
}

fn w2(x: i32) -> i32 {
    target_b(x.wrapping_add(9))
}

fn call_b_once(x: i32) -> i32 {
    let fp: fn(i32) -> i32 = target_b;
    let a = target_b(x);
    let b = w2(a);
    let c = b_twist_call(target_b, a);
    let d = fp(c ^ x);
    a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
}

fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter = 0;
        while {
            iter += 1;
            iter <= 4
        } {
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

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return call_a_once(x);
    }
    if impl_id == 1 {
        return call_b_once(x.wrapping_add(1));
    }
    target_ext(target_ext(x.wrapping_add(1)))
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
        let t = target_ext(x);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

fn run_engine(impl_id: i32, code: &[i32], vm: &mut Vm) -> i32 {
    let mut p = Program::new(code);
    while let Some(op) = p.fetch() {
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            0 => {
                let Some(imm) = p.fetch() else { return 1 };
                vm.stack.push(imm);
                vm.trace(0);
            }
            1 => {
                let Some(b) = vm.stack.pop() else { return 2 };
                let Some(a) = vm.stack.pop() else { return 2 };
                vm.stack.push(a.wrapping_add(b));
                vm.trace(1);
            }
            2 => {
                let Some(b) = vm.stack.pop() else { return 3 };
                let Some(a) = vm.stack.pop() else { return 3 };
                vm.stack.push(a.wrapping_mul(b));
                vm.trace(2);
            }
            3 => {
                let a = vm.stack.peek(0);
                vm.stack.push(a);
                vm.trace(3);
            }
            4 => {
                if vm.stack.pop().is_none() {
                    return 4;
                }
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
                let Some(k) = p.fetch() else { return 5 };
                let Some(cond) = vm.stack.pop() else { return 6 };
                if c_bool(cond) {
                    let rem = p.code.len() - p.ip;
                    if k < 0 || (k as usize) > rem {
                        return 7;
                    }
                    p.ip += k as usize;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            7 => {
                let Some(times) = p.fetch() else { return 8 };
                if p.ip >= p.code.len() {
                    return 9;
                }
                let saved_ip = p.ip;
                for _ in 0..times {
                    let rc = run_engine(impl_id, &p.code[saved_ip..saved_ip + 1], vm);
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
                let Some(m) = p.fetch() else { return 10 };
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m = m as usize;
                let mut tmp = vec![0_i32; m];
                for i in (0..m).rev() {
                    if let Some(v) = vm.stack.pop() {
                        tmp[i] = v;
                    }
                }
                for i in (0..m).rev() {
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

fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn parse_c_long_token(bytes: &[u8]) -> Option<i32> {
    if bytes.is_empty() {
        return Some(0);
    }
    let mut i = 0;
    while i < bytes.len() && c_isspace(bytes[i]) {
        i += 1;
    }
    if i == bytes.len() {
        return None;
    }
    let mut neg = false;
    if bytes[i] == b'+' || bytes[i] == b'-' {
        neg = bytes[i] == b'-';
        i += 1;
    }
    if i == bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    let mut val: i128 = 0;
    let limit = i64::MAX as i128 + 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = (val * 10 + i128::from(bytes[i] - b'0')).min(limit);
        i += 1;
    }
    if i != bytes.len() {
        return None;
    }
    let signed = if neg { -val } else { val };
    let clamped = signed.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
    Some(clamped as i32)
}

fn usage(p: &str) {
    eprint!(
        "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
        p
    );
}

fn read_stdin(v: &mut IntVec) -> usize {
    let mut stdin = io::stdin();
    let mut input = Vec::new();
    if stdin.read_to_end(&mut input).is_err() {
        return 0;
    }
    let mut count = 0;
    let mut pos = 0;
    while pos < input.len() {
        let limit = (pos + 4095).min(input.len());
        let mut end = limit;
        if let Some(rel) = input[pos..limit].iter().position(|&b| b == b'\n') {
            end = pos + rel + 1;
        }

        let chunk = &input[pos..end];
        let visible_end = chunk.iter().position(|&b| b == 0).unwrap_or(chunk.len());
        let mut start = 0;
        for i in 0..=visible_end {
            let at_end = i == visible_end;
            if at_end || matches!(chunk[i], b' ' | b'\t' | b'\n' | b'\r') {
                if start < i {
                    if let Some(t) = parse_c_long_token(&chunk[start..i]) {
                        v.push(t);
                        count += 1;
                    }
                }
                start = i + 1;
            }
        }
        pos = end;
    }
    count
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut use_stdin = false;
    let mut code = IntVec::default();

    for arg in args.iter().skip(1) {
        if arg == "--help" {
            usage(&args[0]);
            process::exit(0);
        } else if arg == "--stdin" {
            use_stdin = true;
        } else if let Some(t) = parse_c_long_token(arg.as_bytes()) {
            code.push(t);
        } else {
            eprintln!("skip '{}'", arg);
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }
    if code.len() == 0 {
        eprintln!("no program");
        process::exit(2);
    }

    let mut vm_a = Vm::default();
    let mut vm_b = Vm::default();
    let mut vm_e = Vm::default();

    let rc_a = run_engine(0, &code.data, &mut vm_a);
    let rc_b = run_engine(1, &code.data, &mut vm_b);
    let rc_e = run_engine(2, &code.data, &mut vm_e);

    println!("RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    vm_a.print("A:");
    vm_b.print("B:");
    vm_e.print("EXT:");
}
