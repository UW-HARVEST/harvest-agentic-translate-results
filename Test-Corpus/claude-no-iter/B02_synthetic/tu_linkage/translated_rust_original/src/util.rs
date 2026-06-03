// Translated from c_src/src/util.c

use std::io::Write;

pub struct VM {
    pub stack: Vec<i32>,
    pub trace: Vec<i32>,
    pub steps: i32,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            trace: Vec::new(),
            steps: 0,
        }
    }

    pub fn trace_push(&mut self, t: i32) {
        self.trace.push(t);
    }
}

pub fn iv_peek(v: &Vec<i32>, def: i32) -> i32 {
    if let Some(&x) = v.last() {
        x
    } else {
        def
    }
}

pub fn iv_pop(v: &mut Vec<i32>) -> Option<i32> {
    v.pop()
}

pub fn iv_push(v: &mut Vec<i32>, x: i32) {
    v.push(x);
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

    pub fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.n {
            None
        } else {
            let v = self.code[self.ip];
            self.ip += 1;
            Some(v)
        }
    }
}

pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &VM) {
    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    write!(
        fp,
        "{}STACK_TOP={} STEPS={} TRACE=",
        label,
        iv_peek(&vm.stack, -777),
        vm.steps
    )
    .unwrap();
    for &t in &vm.trace {
        // In C: "abcdefghijklmnopqrstuvwxyz"[(trace[i]) & 25]
        // & 25 = & 0b11001 — produces values in {0,1,8,9,16,17,24,25}
        let idx = (t & 25) as usize;
        fp.write_all(&[alphabet[idx]]).unwrap();
    }
    writeln!(fp).unwrap();
}
