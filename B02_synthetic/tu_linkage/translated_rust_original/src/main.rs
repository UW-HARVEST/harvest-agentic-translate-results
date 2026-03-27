use std::io::{self, BufRead, Write};

// ── IntVec (util.c) ──────────────────────────────────────────────────
struct IntVec {
    data: Vec<i32>,
}
impl IntVec {
    fn new() -> Self { IntVec { data: Vec::new() } }
    fn push(&mut self, x: i32) { self.data.push(x); }
    fn pop(&mut self) -> Option<i32> { self.data.pop() }
    fn peek(&self, def: i32) -> i32 {
        self.data.last().copied().unwrap_or(def)
    }
    fn len(&self) -> usize { self.data.len() }
}

// ── Program (util.c) ────────────────────────────────────────────────
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

// ── VM (util.c) ─────────────────────────────────────────────────────
struct VM {
    stack: IntVec,
    trace: IntVec,
    steps: i32,
}
impl VM {
    fn new() -> Self { VM { stack: IntVec::new(), trace: IntVec::new(), steps: 0 } }
    fn trace(&mut self, t: i32) { self.trace.push(t); }
    // vm_print: uses &25 (not %26) — reproducing C exactly
    fn print(&self, fp: &mut dyn Write, label: &str) {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        write!(fp, "{}STACK_TOP={} STEPS={} TRACE=",
               label, self.stack.peek(-777), self.steps).unwrap();
        for &t in &self.trace.data {
            fp.write_all(&[chars[(t & 25) as usize]]).unwrap();
        }
        fp.write_all(b"\n").unwrap();
    }
}

// ── lib.c target (external / impl_id==2 fallback) ───────────────────
fn lib_target(code: i32) -> i32 {
    if code < 0 { return 7; }
    let m = code % 10;
    if m == 0 { return 0; }
    if m <= 3 { return 1; }
    if m <= 6 { return 2; }
    if m == 7 { return 3; }
    4
}

// ── a.c ──────────────────────────────────────────────────────────────
mod tu_a {
    use std::cell::Cell;
    thread_local! { static STATE_A: Cell<i32> = const { Cell::new(0) }; }

    fn target(code: i32) -> i32 {
        STATE_A.with(|sa| {
            if code < 0 {
                return if (sa.get() & 1) != 0 { 6 } else { 5 };
            }
            sa.set(sa.get() ^ (code << 1));
            let k = ((code >> 2) ^ sa.get()) & 7;
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

    fn wrap(x: i32) -> i32 { target(x.wrapping_sub(5)) }

    // a_bias_call: fp((x ^ 0x55) + 7)
    fn a_bias_call(fp: fn(i32) -> i32, x: i32) -> i32 {
        fp((x ^ 0x55).wrapping_add(7))
    }

    // A_MAC_CALL(F, X) = a_bias_call(F, X)
    pub fn call_a_once(x: i32) -> i32 {
        let a = target(x);
        let b = wrap(a);
        let c = target(b ^ 3);
        let d = a_bias_call(target, b);
        a ^ (b << 1) ^ (c << 2) ^ (d << 3)
    }

    pub fn process_a_stream(xs: &[i32]) -> i32 {
        // C uses size_t acc (unsigned 64-bit), wrapping arithmetic
        let mut acc: u64 = 0;
        for &v in xs {
            for j in 0..3i32 {
                let t = target(v.wrapping_add(j));
                if (t & 1) == 0 {
                    // C: acc += t; (int t promoted to size_t via sign extension)
                    acc = acc.wrapping_add(t as i64 as u64);
                    continue;
                }
                // C: acc ^= (t<<j); (int result sign-extended to size_t)
                acc ^= (t << j) as i64 as u64;
                if t == 5 { break; }
            }
        }
        // C: if (acc > 0x7fffffffLL) — compares size_t with long long;
        //    the LL gets converted to unsigned, so this is acc > 0x7fffffff_u64
        if acc > 0x7fff_ffff_u64 { acc = 0x7fff_ffff_u64; }
        // C: if (acc < -0x80000000LL) — -0x80000000LL as size_t = 0xFFFFFFFF80000000
        //    so this checks acc < 0xFFFFFFFF80000000 (which can be true!)
        //    and sets acc = (size_t)(-0x80000000LL) = 0xFFFFFFFF80000000
        if acc < (-0x80000000_i64 as u64) { acc = (-0x80000000_i64) as u64; }
        acc as i32
    }
}

// ── b.c ──────────────────────────────────────────────────────────────
mod tu_b {
    use std::cell::Cell;
    thread_local! { static FLIPFLOP: Cell<i32> = const { Cell::new(0) }; }

    fn target(code: i32) -> i32 {
        FLIPFLOP.with(|ff| {
            ff.set(ff.get() ^ 1);
            if code < 0 {
                return if ff.get() != 0 { 2 } else { 6 };
            }
            let z = (code ^ (if ff.get() != 0 { 0x7f } else { 0x1f })) % 8;
            if z == 0 || z == 7 { return 4; }
            if z == 1 || z == 2 { return 3; }
            if z == 3 { return 1; }
            if z == 4 { return 0; }
            if z == 5 { return 5; }
            7
        })
    }

    fn w2(x: i32) -> i32 { target(x.wrapping_add(9)) }

    // b_twist_call: fp(((x + 9) ^ 0x2222) - 17)
    fn b_twist_call(fp: fn(i32) -> i32, x: i32) -> i32 {
        fp(((x.wrapping_add(9)) ^ 0x2222).wrapping_sub(17))
    }

    // B_MAC_CALL(F, X) = b_twist_call(F, X)
    pub fn call_b_once(x: i32) -> i32 {
        let a = target(x);
        let b = w2(a);
        let c = b_twist_call(target, a);
        let d = target(c ^ x);
        (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
    }

    pub fn process_b_stream(xs: &[i32]) -> i32 {
        let mut acc: i32 = 1;
        for &v in xs {
            let mut iter = 0i32;
            loop {
                iter += 1;
                if iter > 4 { break; }
                let t = target(v.wrapping_sub(iter));
                if t == 6 { acc = acc.wrapping_sub(t); break; }
                if t == 3 { continue; }
                acc = (acc.wrapping_mul(3)) ^ t;
            }
        }
        acc
    }
}

// ── engine.c ─────────────────────────────────────────────────────────

// classify: dispatches to the right TU's target
fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        // inline_call(call_a_once, x) — call_a_once IS the function passed
        return tu_a::call_a_once(x);
    }
    if impl_id == 1 {
        // MAC_CALL(call_b_once, x) => call_b_once(x+1)
        return tu_b::call_b_once(x.wrapping_add(1));
    }
    // impl==2: inline_call(&target, MAC_CALL(target, x))
    //   MAC_CALL(target, x) = target(x+1)
    //   then inline_call(&target, result) = target(result)
    let inner = lib_target(x.wrapping_add(1));
    lib_target(inner)
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 { return tu_a::process_a_stream(buf); }
    if impl_id == 1 { return tu_b::process_b_stream(buf); }
    let mut acc: i32 = 0;
    for &v in buf {
        let t = lib_target(v);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

fn run_engine(impl_id: i32, code: &[i32], vm: &mut VM) -> i32 {
    let mut p = Program::new(code);
    loop {
        let op = match p.fetch() {
            Some(v) => v,
            None => return 0,
        };
        vm.steps += 1;
        match op {
            0 => { // PUSH immediate
                let imm = match p.fetch() { Some(v) => v, None => return 1 };
                vm.stack.push(imm);
                vm.trace(0);
            }
            1 => { // ADD
                let b = match vm.stack.pop() { Some(v) => v, None => return 2 };
                let a = match vm.stack.pop() { Some(v) => v, None => return 2 };
                vm.stack.push(a.wrapping_add(b));
                vm.trace(1);
            }
            2 => { // MUL
                let b = match vm.stack.pop() { Some(v) => v, None => return 3 };
                let a = match vm.stack.pop() { Some(v) => v, None => return 3 };
                vm.stack.push(a.wrapping_mul(b));
                vm.trace(2);
            }
            3 => { // DUP
                let a = vm.stack.peek(0);
                vm.stack.push(a);
                vm.trace(3);
            }
            4 => { // DROP
                if vm.stack.pop().is_none() { return 4; }
                vm.trace(4);
            }
            5 => { // CLASSIFY
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
            6 => { // SKIP
                let k = match p.fetch() { Some(v) => v, None => return 5 };
                let cond = match vm.stack.pop() { Some(v) => v, None => return 6 };
                if cond != 0 {
                    if (k as usize) > p.code.len() - p.ip { return 7; }
                    p.ip += k as usize;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            7 => { // REPEAT
                let times = match p.fetch() { Some(v) => v, None => return 8 };
                if p.ip >= p.code.len() { return 9; }
                let saved_ip = p.ip;
                for _i in 0..times {
                    let inner_code = &p.code[saved_ip..];
                    let rc = run_engine(impl_id, &inner_code[..1], vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm.trace(12);
                        break;
                    }
                }
                p.ip = saved_ip + 1;
            }
            8 => { // CLASSIFY2
                let x = vm.stack.peek(0);
                let y = classify(impl_id, x);
                vm.stack.push(y);
                vm.trace(13);
            }
            9 => { // PROCESS_STREAM — double-pop bug reproduced
                let m = match p.fetch() { Some(v) => v, None => return 10 };
                if m < 0 || (m as usize) > vm.stack.len() { return 11; }
                let m = m as usize;
                let mut tmp = vec![0i32; m];
                // First pop pass
                for i in (0..m).rev() {
                    if let Some(v) = vm.stack.pop() { tmp[i] = v; }
                }
                // Second pop pass (overwrites tmp — this is the C bug)
                for i in (0..m).rev() {
                    if let Some(v) = vm.stack.pop() { tmp[i] = v; }
                }
                let s = process_stream(impl_id, &tmp);
                vm.stack.push(s);
                vm.trace(14);
            }
            10 => return 0, // HALT
            _ => return 99,
        }
    }
}

// ── main.c ───────────────────────────────────────────────────────────

fn usage(prog: &str) {
    eprint!("Usage: {} [--stdin] [bytecodes...]\n\
             Bytecodes are integers forming a small VM program.\n", prog);
}

fn read_stdin(v: &mut IntVec) -> usize {
    let stdin = io::stdin();
    let mut count = 0usize;
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        for token in line.split(|c: char| c == ' ' || c == '\t' || c == '\r') {
            if token.is_empty() { continue; }
            if let Ok(val) = token.parse::<i64>() {
                v.push(val as i32);
                count += 1;
            }
        }
    }
    count
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut use_stdin = false;
    let mut code = IntVec::new();

    for i in 1..args.len() {
        if args[i] == "--help" {
            usage(&args[0]);
            return;
        } else if args[i] == "--stdin" {
            use_stdin = true;
        } else {
            match args[i].parse::<i64>() {
                Ok(t) => code.push(t as i32),
                Err(_) => eprint!("skip '{}'\n", args[i]),
            }
        }
    }
    if use_stdin { read_stdin(&mut code); }
    if code.len() == 0 {
        eprint!("no program\n");
        std::process::exit(2);
    }

    let mut vm_a = VM::new();
    let mut vm_b = VM::new();
    let mut vm_e = VM::new();

    let rc_a = run_engine(0, &code.data, &mut vm_a);
    let rc_b = run_engine(1, &code.data, &mut vm_b);
    let rc_e = run_engine(2, &code.data, &mut vm_e);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "RC:A={} B={} EXT={}\n", rc_a, rc_b, rc_e).unwrap();
    vm_a.print(&mut out, "A:");
    vm_b.print(&mut out, "B:");
    vm_e.print(&mut out, "EXT:");
}
