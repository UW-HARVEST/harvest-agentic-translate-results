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
    pub fn pop(&mut self) -> Option<i32> {
        self.data.pop()
    }
    pub fn peek(&self, def: i32) -> i32 {
        if self.data.is_empty() {
            def
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
    pub fn new(code: &'a [i32], n: usize) -> Self {
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
}

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
    for i in 0..vm.trace.len() {
        let t = vm.trace.data[i];
        // C: alphabet[(trace_data[i]) & 25]
        // C does signed & 25 — for non-negative t this is just t & 25
        // For negative values: bitwise AND with 25 in C uses two's complement representation
        let idx = (t & 25) as u32 as usize;
        // alphabet has 26 entries (0..25). Index up to 25 is safe.
        let ch = alphabet[idx % 26];
        fp.write_all(&[ch]).unwrap();
    }
    fp.write_all(b"\n").unwrap();
}
