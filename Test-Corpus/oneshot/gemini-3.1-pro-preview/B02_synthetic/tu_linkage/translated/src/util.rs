pub struct Program<'a> {
    pub code: &'a [i32],
    pub ip: usize,
}

impl<'a> Program<'a> {
    pub fn new(code: &'a [i32]) -> Self {
        Self { code, ip: 0 }
    }

    pub fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.code.len() {
            None
        } else {
            let val = self.code[self.ip];
            self.ip += 1;
            Some(val)
        }
    }
}

pub struct VM {
    pub stack: Vec<i32>,
    pub trace: Vec<i32>,
    pub steps: i32,
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            trace: Vec::new(),
            steps: 0,
        }
    }

    pub fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }

    pub fn print(&self, label: &str) {
        let top = self.stack.last().copied().unwrap_or(-777);
        print!("{}STACK_TOP={} STEPS={} TRACE=", label, top, self.steps);
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        for &t in &self.trace {
            let idx = (t & 25) as usize;
            print!("{}", chars[idx] as char);
        }
        println!();
    }
}
