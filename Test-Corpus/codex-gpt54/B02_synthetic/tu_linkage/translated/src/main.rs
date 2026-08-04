use std::cell::Cell;
use std::env;
use std::io::{self, Read, Write};

#[derive(Default)]
struct IntVec {
    data: Vec<i32>,
}

impl IntVec {
    fn init() -> Self {
        Self { data: Vec::new() }
    }

    fn free(&mut self) {
        self.data.clear();
        self.data.shrink_to(0);
    }

    fn push(&mut self, x: i32) -> bool {
        self.data.push(x);
        true
    }

    fn pop(&mut self, out: Option<&mut i32>) -> bool {
        match self.data.pop() {
            Some(value) => {
                if let Some(slot) = out {
                    *slot = value;
                }
                true
            }
            None => false,
        }
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
    n: usize,
    ip: usize,
}

impl<'a> Program<'a> {
    fn init(code: &'a [i32]) -> Self {
        Self {
            code,
            n: code.len(),
            ip: 0,
        }
    }

    fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.n {
            None
        } else {
            let out = self.code[self.ip];
            self.ip += 1;
            Some(out)
        }
    }
}

struct Vm {
    stack: IntVec,
    trace: IntVec,
    steps: i32,
}

impl Vm {
    fn init() -> Self {
        Self {
            stack: IntVec::init(),
            trace: IntVec::init(),
            steps: 0,
        }
    }

    fn free(&mut self) {
        self.stack.free();
        self.trace.free();
        self.steps = 0;
    }

    fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }

    fn print<W: Write>(&self, mut fp: W, label: &str) -> io::Result<()> {
        write!(
            fp,
            "{}STACK_TOP={} STEPS={} TRACE=",
            label,
            self.stack.peek(-777),
            self.steps
        )?;
        const LETTERS: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";
        for &value in &self.trace.data {
            fp.write_all(&[LETTERS[(value & 25) as usize]])?;
        }
        fp.write_all(b"\n")
    }
}

fn usage(program: &str) {
    let _ = write!(
        io::stderr(),
        "Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n",
        program
    );
}

fn parse_c_int_token(token: &str) -> Option<i32> {
    if token.is_empty() {
        return Some(0);
    }

    let bytes = token.as_bytes();
    let mut index = 0usize;
    let mut negative = false;

    match bytes[0] {
        b'+' => index = 1,
        b'-' => {
            negative = true;
            index = 1;
        }
        _ => {}
    }

    if index >= bytes.len() || !bytes[index].is_ascii_digit() {
        return None;
    }

    let limit: u128 = if negative {
        (i64::MAX as u128) + 1
    } else {
        i64::MAX as u128
    };
    let mut value: u128 = 0;
    let mut overflowed = false;
    while index < bytes.len() {
        let b = bytes[index];
        if !b.is_ascii_digit() {
            return None;
        }
        let digit = u128::from(b - b'0');
        if value > (limit.saturating_sub(digit)) / 10 {
            overflowed = true;
            value = limit;
        } else {
            value = value * 10 + digit;
        }
        index += 1;
    }

    let clamped = if overflowed {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        if value == limit {
            i64::MIN
        } else {
            -(value as i64)
        }
    } else {
        value as i64
    };
    Some(clamped as i32)
}

fn tokenize_and_push(buf: &[u8], v: &mut IntVec) -> usize {
    let mut count = 0usize;
    let mut p = 0usize;
    while p < buf.len() {
        let mut q = p;
        while q < buf.len() && !matches!(buf[q], b' ' | b'\t' | b'\n' | b'\r') {
            q += 1;
        }
        if q > p {
            if let Ok(token) = std::str::from_utf8(&buf[p..q]) {
                if let Some(value) = parse_c_int_token(token) {
                    v.push(value);
                    count += 1;
                }
            }
        }
        p = if q < buf.len() { q + 1 } else { q };
    }
    count
}

fn read_stdin(v: &mut IntVec) -> usize {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut count = 0usize;

    loop {
        let mut buf = Vec::with_capacity(4095);
        loop {
            let mut byte = [0u8; 1];
            match handle.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    buf.push(byte[0]);
                    if byte[0] == b'\n' || buf.len() == 4095 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if buf.is_empty() {
            break;
        }

        count += tokenize_and_push(&buf, v);
    }

    count
}

fn lib_target(code: i32) -> i32 {
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

thread_local! {
    static STATE_A: Cell<i32> = const { Cell::new(0) };
    static FLIPFLOP: Cell<i32> = const { Cell::new(0) };
}

fn a_target(code: i32) -> i32 {
    STATE_A.with(|state| {
        let current = state.get();
        if code < 0 {
            return if (current & 1) != 0 { 6 } else { 5 };
        }
        let updated = current ^ code.wrapping_shl(1);
        state.set(updated);
        let k = ((code >> 2) ^ updated) & 7;
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

fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp((x ^ 0x55).wrapping_add(7))
}

fn call_a_once(x: i32) -> i32 {
    let a = a_target(x);
    let b = a_target(a.wrapping_sub(5));
    let c = a_target(b ^ 3);
    let d = a_bias_call(a_target, b);
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

fn process_a_stream(xs: &[i32]) -> i32 {
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3u32 {
            let t = a_target(v.wrapping_add(j as i32));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t as u64);
                continue;
            }
            acc ^= (t.wrapping_shl(j)) as u64;
            if t == 5 {
                break;
            }
        }
    }
    if acc > 0x7fff_ffff {
        acc = 0x7fff_ffff;
    }
    if acc < 0xffff_ffff_8000_0000 {
        acc = 0xffff_ffff_8000_0000;
    }
    acc as i32
}

fn b_target(code: i32) -> i32 {
    FLIPFLOP.with(|flip| {
        let next = flip.get() ^ 1;
        flip.set(next);
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
    })
}

fn b_twist_call(fp: fn(i32) -> i32, x: i32) -> i32 {
    fp(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
}

fn call_b_once(x: i32) -> i32 {
    let a = b_target(x);
    let b = b_target(a.wrapping_add(9));
    let c = b_twist_call(b_target, a);
    let d = b_target(c ^ x);
    a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
}

fn process_b_stream(xs: &[i32]) -> i32 {
    let mut acc = 1i32;
    for &v in xs {
        let mut iter = 0i32;
        while {
            iter = iter.wrapping_add(1);
            iter <= 4
        } {
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

fn inline_call(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

fn mac_call(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x.wrapping_add(1))
}

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return inline_call(call_a_once, x);
    }
    if impl_id == 1 {
        return mac_call(call_b_once, x);
    }
    inline_call(lib_target, mac_call(lib_target, x))
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }

    let mut acc = 0i32;
    for &item in buf {
        let t = lib_target(item);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

fn run_engine(impl_id: i32, code: &[i32], vm: &mut Vm) -> i32 {
    let mut p = Program::init(code);
    while let Some(op) = p.fetch() {
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            0 => {
                let Some(imm) = p.fetch() else {
                    return 1;
                };
                vm.stack.push(imm);
                vm.trace(0);
            }
            1 => {
                let mut a = 0;
                let mut b = 0;
                if !vm.stack.pop(Some(&mut b)) || !vm.stack.pop(Some(&mut a)) {
                    return 2;
                }
                vm.stack.push(a.wrapping_add(b));
                vm.trace(1);
            }
            2 => {
                let mut a = 0;
                let mut b = 0;
                if !vm.stack.pop(Some(&mut b)) || !vm.stack.pop(Some(&mut a)) {
                    return 3;
                }
                vm.stack.push(a.wrapping_mul(b));
                vm.trace(2);
            }
            3 => {
                let a = vm.stack.peek(0);
                vm.stack.push(a);
                vm.trace(3);
            }
            4 => {
                let mut tmp = 0;
                if !vm.stack.pop(Some(&mut tmp)) {
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
                let Some(k) = p.fetch() else {
                    return 5;
                };
                let mut cond = 0;
                if !vm.stack.pop(Some(&mut cond)) {
                    return 6;
                }
                if cond != 0 {
                    let ku = k as u32 as usize;
                    if ku > p.n - p.ip {
                        return 7;
                    }
                    p.ip += ku;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            7 => {
                let Some(times) = p.fetch() else {
                    return 8;
                };
                if p.ip >= p.n {
                    return 9;
                }
                let saved_ip = p.ip;
                for _ in 0..times {
                    let end = saved_ip.saturating_add(1).min(code.len());
                    let rc = run_engine(impl_id, &code[saved_ip..end], vm);
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
                let Some(m) = p.fetch() else {
                    return 10;
                };
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let mu = m as usize;
                let mut tmp = vec![0i32; mu];
                for i in (0..mu).rev() {
                    vm.stack.pop(Some(&mut tmp[i]));
                }
                for i in (0..mu).rev() {
                    vm.stack.pop(Some(&mut tmp[i]));
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

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut use_stdin = false;
    let mut code = IntVec::init();

    for arg in args.iter().skip(1) {
        if arg == "--help" {
            usage(&args[0]);
            code.free();
            std::process::exit(0);
        } else if arg == "--stdin" {
            use_stdin = true;
        } else if let Some(value) = parse_c_int_token(arg) {
            code.push(value);
        } else {
            let _ = writeln!(io::stderr(), "skip '{}'", arg);
        }
    }

    if use_stdin {
        read_stdin(&mut code);
    }
    if code.len() == 0 {
        let _ = writeln!(io::stderr(), "no program");
        code.free();
        std::process::exit(2);
    }

    let mut vm_a = Vm::init();
    let mut vm_b = Vm::init();
    let mut vm_e = Vm::init();

    let rc_a = run_engine(0, &code.data, &mut vm_a);
    let rc_b = run_engine(1, &code.data, &mut vm_b);
    let rc_e = run_engine(2, &code.data, &mut vm_e);

    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);
    let _ = vm_a.print(&mut stdout, "A:");
    let _ = vm_b.print(&mut stdout, "B:");
    let _ = vm_e.print(&mut stdout, "EXT:");

    vm_a.free();
    vm_b.free();
    vm_e.free();
    code.free();
}
