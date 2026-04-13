use std::io::Write;

pub struct IntVec {
    pub data: Vec<i32>,
    pub len: usize,
    pub cap: usize,
}

impl IntVec {
    pub fn new() -> Self {
        IntVec {
            data: Vec::new(),
            len: 0,
            cap: 0,
        }
    }
}

pub fn iv_init(v: &mut IntVec) {
    v.data.clear();
    v.len = 0;
    v.cap = 0;
}

pub fn iv_free(v: &mut IntVec) {
    v.data.clear();
    v.len = 0;
    v.cap = 0;
}

pub fn iv_reserve(v: &mut IntVec, need: usize) -> bool {
    if need <= v.cap {
        return true;
    }
    let mut nc = if v.cap == 0 { 8 } else { v.cap };
    while nc < need {
        if nc > usize::MAX / 2 {
            return false;
        }
        nc *= 2;
    }
    v.data.reserve(nc - v.data.len());
    v.cap = nc;
    true
}

pub fn iv_push(v: &mut IntVec, x: i32) -> bool {
    if v.len == v.cap && !iv_reserve(v, if v.cap == 0 { 8 } else { v.cap * 2 }) {
        return false;
    }
    if v.len < v.data.len() {
        v.data[v.len] = x;
    } else {
        v.data.push(x);
    }
    v.len += 1;
    true
}

pub fn iv_pop(v: &mut IntVec, out: &mut i32) -> bool {
    if v.len == 0 {
        return false;
    }
    *out = v.data[v.len - 1];
    v.len -= 1;
    true
}

pub fn iv_peek(v: &IntVec, def: i32) -> i32 {
    if v.len > 0 {
        v.data[v.len - 1]
    } else {
        def
    }
}

pub struct Program<'a> {
    pub code: &'a [i32],
    pub n: usize,
    pub ip: usize,
}

impl<'a> Program<'a> {
    pub fn new(code: &'a [i32]) -> Self {
        Program {
            code,
            n: code.len(),
            ip: 0,
        }
    }
}

pub fn prog_init<'a>(p: &mut Program<'a>, code: &'a [i32]) {
    p.code = code;
    p.n = code.len();
    p.ip = 0;
}

pub fn prog_fetch(p: &mut Program, out: &mut i32) -> bool {
    if p.ip >= p.n {
        return false;
    }
    *out = p.code[p.ip];
    p.ip += 1;
    true
}

pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: i32,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: IntVec::new(),
            trace: IntVec::new(),
            steps: 0,
        }
    }
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
    iv_push(&mut vm.trace, t);
}

pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &VM) {
    let top = iv_peek(&vm.stack, -777);
    write!(fp, "{}STACK_TOP={} STEPS={} TRACE=", label, top, vm.steps).unwrap();
    let letters = "abcdefghijklmnopqrstuvwxyz";
    for i in 0..vm.trace.len {
        let idx = (vm.trace.data[i] & 25) as usize;
        if idx < letters.len() {
            fp.write_all(&[letters.as_bytes()[idx]]).unwrap();
        }
    }
    fp.write_all(b"\n").unwrap();
}
