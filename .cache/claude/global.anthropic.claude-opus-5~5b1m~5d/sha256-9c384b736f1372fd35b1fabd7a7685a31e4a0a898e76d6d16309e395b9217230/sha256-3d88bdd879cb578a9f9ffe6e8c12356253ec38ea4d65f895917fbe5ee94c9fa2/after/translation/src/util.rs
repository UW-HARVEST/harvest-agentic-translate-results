// Translation of c_src/src/util.c + c_src/include/util.h
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::io::Write;

/// `IntVec` from util.h.  The C version is a manually grown `int *` buffer;
/// a `Vec<i32>` reproduces every observable behaviour (allocation failure
/// paths in the C code are unreachable for these input sizes).
#[derive(Default)]
pub struct IntVec {
    pub data: Vec<i32>,
}

impl IntVec {
    /// `iv_init`
    pub fn new() -> IntVec {
        IntVec { data: Vec::new() }
    }

    /// `iv_free`
    pub fn free(&mut self) {
        self.data.clear();
        self.data.shrink_to_fit();
    }

    /// `iv_push` (the C function returns false only on allocation failure)
    pub fn push(&mut self, x: i32) -> bool {
        self.data.push(x);
        true
    }

    /// `iv_pop`: on an empty vector it returns false and leaves `*out`
    /// untouched, which the callers in engine.c actually depend on.
    pub fn pop_into(&mut self, out: &mut i32) -> bool {
        match self.data.pop() {
            Some(v) => {
                *out = v;
                true
            }
            None => false,
        }
    }

    /// `iv_pop` with a NULL `out`
    pub fn pop(&mut self) -> Option<i32> {
        self.data.pop()
    }

    /// `iv_peek`
    pub fn peek(&self, def: i32) -> i32 {
        match self.data.last() {
            Some(&v) => v,
            None => def,
        }
    }

    /// `v->len`
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

/// `Program` from util.h.  `code`/`n` are represented by a slice; `ip`
/// indexes into it exactly like the C member does.
pub struct Program<'a> {
    pub code: &'a [i32],
    pub n: usize,
    pub ip: usize,
}

impl<'a> Program<'a> {
    /// `prog_init`
    pub fn new(code: &'a [i32], n: usize) -> Program<'a> {
        Program { code, n, ip: 0 }
    }

    /// `prog_fetch`
    pub fn fetch(&mut self, out: &mut i32) -> bool {
        if self.ip >= self.n {
            return false;
        }
        *out = self.code[self.ip];
        self.ip += 1;
        true
    }
}

/// `VM` from util.h
pub struct Vm {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: i32,
}

impl Vm {
    /// `vm_init`
    pub fn new() -> Vm {
        Vm {
            stack: IntVec::new(),
            trace: IntVec::new(),
            steps: 0,
        }
    }

    /// `vm_free`
    pub fn free(&mut self) {
        self.stack.free();
        self.trace.free();
        self.steps = 0;
    }

    /// `vm_trace`
    pub fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }
}

const ALPHABET: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

/// `vm_print`
pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &Vm) {
    let _ = write!(
        fp,
        "{}STACK_TOP={} STEPS={} TRACE=",
        label,
        vm.stack.peek(-777),
        vm.steps
    );
    for i in 0..vm.trace.len() {
        // NOTE: the C code masks with 25 (bitwise AND), not modulo 26.
        let idx = (vm.trace.data[i] & 25) as usize;
        let _ = fp.write_all(&[ALPHABET[idx]]);
    }
    let _ = fp.write_all(b"\n");
}
