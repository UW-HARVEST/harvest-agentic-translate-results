// Translation of c_src/src/util.c and c_src/include/util.h

use std::io::Write;

/// C: `typedef struct { int *data; size_t len, cap; } IntVec;`
///
/// The C code's `cap`/`iv_reserve` growth policy exists only to manage the
/// manual allocation; it is not observable in the output, so a `Vec` stands in.
/// `iv_push` can only fail on allocation failure, which every call site ignores.
pub type IntVec = Vec<i32>;

/// C: `int iv_peek(const IntVec *v, int def)`
pub fn iv_peek(v: &IntVec, def: i32) -> i32 {
    match v.last() {
        Some(&x) => x,
        None => def,
    }
}

/// C: `bool iv_pop(IntVec *v, int *out)` -- returns None when empty, in which
/// case the C version leaves `*out` untouched. Call sites must preserve that.
pub fn iv_pop(v: &mut IntVec) -> Option<i32> {
    v.pop()
}

/// C: `typedef struct { const int *code; size_t n; size_t ip; } Program;`
pub struct Program<'a> {
    pub code: &'a [i32],
    pub n: usize,
    pub ip: usize,
}

impl<'a> Program<'a> {
    /// C: `void prog_init(Program *p, const int *code, size_t n)`
    pub fn new(code: &'a [i32], n: usize) -> Self {
        Program { code, n, ip: 0 }
    }

    /// C: `bool prog_fetch(Program *p, int *out)`
    pub fn fetch(&mut self) -> Option<i32> {
        if self.ip >= self.n {
            return None;
        }
        let out = self.code[self.ip];
        self.ip += 1;
        Some(out)
    }
}

/// C: `typedef struct { IntVec stack; IntVec trace; int steps; } VM;`
pub struct Vm {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: i32,
}

impl Vm {
    /// C: `void vm_init(VM *vm)`
    pub fn new() -> Self {
        Vm {
            stack: IntVec::new(),
            trace: IntVec::new(),
            steps: 0,
        }
    }

    /// C: `void vm_trace(VM *vm, int t)`
    pub fn trace(&mut self, t: i32) {
        self.trace.push(t);
    }
}

const ALPHABET: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

/// C: `void vm_print(FILE *fp, const char *label, const VM *vm)`
pub fn vm_print<W: Write>(fp: &mut W, label: &str, vm: &Vm) {
    let _ = write!(
        fp,
        "{}STACK_TOP={} STEPS={} TRACE=",
        label,
        iv_peek(&vm.stack, -777),
        vm.steps
    );
    for &t in vm.trace.iter() {
        // Faithfully reproduces the original's `& 25` (not `% 26`) index bug.
        // 25 == 0b11001, so e.g. trace value 2 maps to 'a', not 'c'.
        let _ = fp.write_all(&[ALPHABET[(t & 25) as usize]]);
    }
    let _ = fp.write_all(b"\n");
}
