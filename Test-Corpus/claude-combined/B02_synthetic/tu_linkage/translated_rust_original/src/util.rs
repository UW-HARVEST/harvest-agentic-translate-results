use std::io::Write;

#[derive(Default)]
pub struct IntVec {
    pub data: Vec<i32>,
}

impl IntVec {
    pub fn new() -> Self {
        IntVec { data: Vec::new() }
    }

    pub fn push(&mut self, x: i32) -> bool {
        self.data.push(x);
        true
    }

    /// Pop one item off the back; returns Some(value) on success, None on empty stack.
    pub fn pop(&mut self) -> Option<i32> {
        self.data.pop()
    }

    pub fn peek(&self, default: i32) -> i32 {
        if self.data.is_empty() {
            default
        } else {
            self.data[self.data.len() - 1]
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct Program<'a> {
    pub code: &'a [i32],
    pub n: usize,
    pub ip: usize,
}

impl<'a> Program<'a> {
    pub fn init(code: &'a [i32], n: usize) -> Self {
        Program { code, n, ip: 0 }
    }

    pub fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.n {
            return None;
        }
        let v = self.code[self.ip];
        self.ip += 1;
        Some(v)
    }
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

    pub fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }
}

/// Print VM state in the same format as C's vm_print.
pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &VM) {
    write!(
        fp,
        "{}STACK_TOP={} STEPS={} TRACE=",
        label,
        vm.stack.peek(-777),
        vm.steps
    )
    .unwrap();
    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    for &t in vm.trace.data.iter() {
        // C: "abcdefghijklmnopqrstuvwxyz"[(t)&25]
        // Operates on int with bitwise AND of 25.
        let idx = (t & 25) as usize;
        fp.write_all(&[alphabet[idx]]).unwrap();
    }
    fp.write_all(b"\n").unwrap();
}
