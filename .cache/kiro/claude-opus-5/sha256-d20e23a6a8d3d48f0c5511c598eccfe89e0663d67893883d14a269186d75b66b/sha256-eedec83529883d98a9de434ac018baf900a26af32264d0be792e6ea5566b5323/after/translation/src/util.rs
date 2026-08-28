//! Port of `c_src/src/util.c` / `c_src/include/util.h`.
//!
//! `IntVec` is replaced by `Vec<i32>`; the growth policy of the C version is
//! unobservable, but the pop/peek semantics are not, so those are kept exactly.

use std::io::Write;

/// `iv_pop`: returns the popped value, or `None` when the vector is empty.
/// The C version leaves `*out` untouched on failure -- callers here must
/// reproduce that by simply not assigning.
pub fn iv_pop(v: &mut Vec<i32>) -> Option<i32> {
    v.pop()
}

/// `iv_peek`
pub fn iv_peek(v: &[i32], def: i32) -> i32 {
    match v.last() {
        Some(&x) => x,
        None => def,
    }
}

/// `Program` -- an instruction cursor over a borrowed code slice.
pub struct Program<'a> {
    pub code: &'a [i32],
    pub ip: usize,
}

impl<'a> Program<'a> {
    /// `prog_init`
    pub fn new(code: &'a [i32]) -> Self {
        Program { code, ip: 0 }
    }

    /// `p->n`
    pub fn n(&self) -> usize {
        self.code.len()
    }

    /// `prog_fetch`
    pub fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.code.len() {
            return None;
        }
        let v = self.code[self.ip];
        self.ip += 1;
        Some(v)
    }
}

/// `VM`
pub struct Vm {
    pub stack: Vec<i32>,
    pub trace: Vec<i32>,
    pub steps: i32,
}

impl Vm {
    /// `vm_init`
    pub fn new() -> Self {
        Vm {
            stack: Vec::new(),
            trace: Vec::new(),
            steps: 0,
        }
    }

    /// `vm_trace`
    pub fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }
}

const ALPHABET: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

/// `vm_print`
///
/// Note the mask in the original is `& 25`, not `& 31`; it is reproduced
/// verbatim, which is why distinct trace codes can map to the same letter.
pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &Vm) {
    let _ = write!(
        fp,
        "{}STACK_TOP={} STEPS={} TRACE=",
        label,
        iv_peek(&vm.stack, -777),
        vm.steps
    );
    for &t in &vm.trace {
        let idx = (t & 25) as usize;
        let _ = fp.write_all(&[ALPHABET[idx]]);
    }
    let _ = fp.write_all(b"\n");
}
