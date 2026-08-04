// Translated from c_src/src/util.c

use std::io::Write;

pub struct Vm {
    pub stack: Vec<i32>,
    pub trace: Vec<i32>,
    pub steps: i32,
}

impl Vm {
    pub fn new() -> Self {
        Vm {
            stack: Vec::new(),
            trace: Vec::new(),
            steps: 0,
        }
    }

    pub fn trace_push(&mut self, t: i32) {
        self.trace.push(t);
    }
}

pub fn iv_peek(v: &[i32], def: i32) -> i32 {
    if v.is_empty() {
        def
    } else {
        v[v.len() - 1]
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

pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &Vm) {
    let top = iv_peek(&vm.stack, -777);
    write!(fp, "{}STACK_TOP={} STEPS={} TRACE=", label, top, vm.steps).unwrap();
    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    for &t in &vm.trace {
        // Reproduce C: ((vm->trace.data[i]) & 25)
        // In C this uses int bitwise AND with 25.
        let idx = (t & 25) as usize;
        // C indexes a 26-char string; idx in {0,1,8,9,16,17,24,25} stays in range.
        let ch = alphabet[idx];
        fp.write_all(&[ch]).unwrap();
    }
    fp.write_all(b"\n").unwrap();
}
