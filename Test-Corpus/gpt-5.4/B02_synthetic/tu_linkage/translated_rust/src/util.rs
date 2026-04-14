use std::io::Write;

#[derive(Default, Clone)]
pub struct IntVec {
    pub data: Vec<i32>,
    pub len: usize,
    pub cap: usize,
}

pub fn iv_init(v: &mut IntVec) {
    v.data.clear();
    v.len = 0;
    v.cap = v.data.capacity();
}

pub fn iv_free(v: &mut IntVec) {
    v.data = Vec::new();
    v.len = 0;
    v.cap = 0;
}

pub fn iv_reserve(v: &mut IntVec, need: usize) -> bool {
    if need <= v.cap {
        return true;
    }
    let mut nc = if v.cap != 0 { v.cap } else { 8 };
    while nc < need {
        if nc > (usize::MAX / 2) {
            return false;
        }
        nc *= 2;
    }
    let additional = nc.saturating_sub(v.data.capacity());
    v.data.reserve_exact(additional);
    v.cap = v.data.capacity().max(nc);
    true
}

pub fn iv_push(v: &mut IntVec, x: i32) -> bool {
    if v.len == v.cap && !iv_reserve(v, if v.cap != 0 { v.cap * 2 } else { 8 }) {
        return false;
    }
    if v.len == v.data.len() {
        v.data.push(x);
    } else {
        v.data[v.len] = x;
    }
    v.len += 1;
    v.cap = v.data.capacity();
    true
}

pub fn iv_pop(v: &mut IntVec, out: Option<&mut i32>) -> bool {
    if v.len == 0 {
        return false;
    }
    let value = v.data[v.len - 1];
    if let Some(dst) = out {
        *dst = value;
    }
    v.len -= 1;
    true
}

pub fn iv_peek(v: &IntVec, def: i32) -> i32 {
    if v.len != 0 {
        v.data[v.len - 1]
    } else {
        def
    }
}

#[derive(Clone, Copy)]
pub struct Program<'a> {
    pub code: &'a [i32],
    pub n: usize,
    pub ip: usize,
}

pub fn prog_init<'a>(p: &mut Program<'a>, code: &'a [i32]) {
    p.code = code;
    p.n = code.len();
    p.ip = 0;
}

pub fn prog_fetch(p: &mut Program<'_>, out: &mut i32) -> bool {
    if p.ip >= p.n {
        return false;
    }
    *out = p.code[p.ip];
    p.ip += 1;
    true
}

#[derive(Default, Clone)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: i32,
}

pub fn vm_init(vm: &mut VM) {
    iv_init(&mut vm.stack);
    iv_init(&mut vm.trace);
    vm.steps = 0;
}

pub fn vm_free(vm: &mut VM) {
    iv_free(&mut vm.stack);
    iv_free(&mut vm.trace);
    vm.steps = 0;
}

pub fn vm_trace(vm: &mut VM, t: i32) {
    let _ = iv_push(&mut vm.trace, t);
}

pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &VM) {
    let _ = write!(fp, "{}STACK_TOP={} STEPS={} TRACE=", label, iv_peek(&vm.stack, -777), vm.steps);
    for i in 0..vm.trace.len {
        let idx = (vm.trace.data[i] & 25) as usize;
        let ch = b"abcdefghijklmnopqrstuvwxyz"[idx] as char;
        let _ = write!(fp, "{}", ch);
    }
    let _ = writeln!(fp);
}
