// Translated from c_src/src/util.c and c_src/include/util.h

use std::io::Write;

#[derive(Default)]
pub struct IntVec {
    pub data: Vec<i32>,
}

impl IntVec {
    pub fn new() -> Self {
        IntVec { data: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn push(&mut self, x: i32) -> bool {
        self.data.push(x);
        true
    }

    pub fn pop(&mut self, out: Option<&mut i32>) -> bool {
        match self.data.pop() {
            Some(v) => {
                if let Some(o) = out {
                    *o = v;
                }
                true
            }
            None => false,
        }
    }

    pub fn peek(&self, def: i32) -> i32 {
        match self.data.last() {
            Some(&v) => v,
            None => def,
        }
    }
}

pub struct Program<'a> {
    pub code: &'a [i32],
    pub ip: usize,
}

impl<'a> Program<'a> {
    pub fn new(code: &'a [i32]) -> Self {
        Program { code, ip: 0 }
    }

    pub fn fetch(&mut self, out: &mut i32) -> bool {
        if self.ip >= self.code.len() {
            return false;
        }
        *out = self.code[self.ip];
        self.ip += 1;
        true
    }
}

#[derive(Default)]
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

    pub fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }

    pub fn print<W: Write>(&self, fp: &mut W, label: &str) {
        write!(
            fp,
            "{}STACK_TOP={} STEPS={} TRACE=",
            label,
            self.stack.peek(-777),
            self.steps
        )
        .ok();
        let alphabet = b"abcdefghijklmnopqrstuvwxyz";
        for &t in self.trace.data.iter() {
            // Match C semantics: ((t) & 25) - bitwise AND with 25
            let idx = (t & 25) as usize;
            // The C code indexes into a 26-char string with idx that can be up to 25 (from t & 25, max is 25 when t has bits 0,3,4 set => 1+8+16=25)
            // (t & 25) max value is 25, which is in range 0..26
            let c = alphabet[idx % alphabet.len()];
            fp.write_all(&[c]).ok();
        }
        fp.write_all(b"\n").ok();
    }
}
