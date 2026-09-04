// Translation of c_src/src/util.c and c_src/include/util.h

use std::io::Write;

/// C: `typedef struct { int *data; size_t len, cap; } IntVec;`
///
/// The `cap`/`iv_reserve` growth policy IS observable, so it is reproduced
/// exactly rather than delegated to `Vec`'s amortized growth.
///
/// C's `iv_reserve` returns false when `realloc` returns NULL, which makes
/// `iv_push` return false and drop the value **while leaving the vector
/// intact**. Every call site in the C program ignores that return value, so on
/// memory exhaustion the C program silently stops appending and keeps running.
/// `Vec::push` instead aborts the process on allocation failure, so pushes here
/// go through fallible allocation to preserve the C behavior.
pub struct IntVec {
    data: Vec<i32>,
}

impl IntVec {
    /// C: `void iv_init(IntVec *v)` -- `data=NULL; len=cap=0;`
    pub fn new() -> Self {
        IntVec { data: Vec::new() }
    }

    /// C: `bool iv_reserve(IntVec *v, size_t need)`
    ///
    /// Grows to a power-of-two capacity (starting at 8) that is `>= need`,
    /// returning false if the allocation fails.
    fn reserve(&mut self, need: usize) -> bool {
        let cap = self.data.capacity();
        if need <= cap {
            return true;
        }
        let mut nc = if cap != 0 { cap } else { 8 };
        while nc < need {
            if nc > usize::MAX / 2 {
                return false;
            }
            nc *= 2;
        }
        // `try_reserve_exact` will not deliberately over-allocate, so this
        // lands on `nc` just as C's `realloc(nc * sizeof(int))` does.
        self.data.try_reserve_exact(nc - self.data.len()).is_ok()
    }

    /// C: `bool iv_push(IntVec *v, int x)`
    ///
    /// `if (v->len == v->cap && !iv_reserve(v, v->cap ? v->cap*2 : 8)) return false;`
    pub fn push(&mut self, x: i32) -> bool {
        let cap = self.data.capacity();
        if self.data.len() == cap {
            let need = if cap != 0 { cap * 2 } else { 8 };
            if !self.reserve(need) {
                return false;
            }
        }
        // Capacity is guaranteed above, so this cannot reallocate or abort.
        self.data.push(x);
        true
    }

    /// C: `size_t len`
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn as_slice(&self) -> &[i32] {
        &self.data
    }

    pub fn iter(&self) -> std::slice::Iter<'_, i32> {
        self.data.iter()
    }
}

/// C: `int iv_peek(const IntVec *v, int def)`
pub fn iv_peek(v: &IntVec, def: i32) -> i32 {
    match v.data.last() {
        Some(&x) => x,
        None => def,
    }
}

/// C: `bool iv_pop(IntVec *v, int *out)` -- returns None when empty, in which
/// case the C version leaves `*out` untouched. Call sites must preserve that.
pub fn iv_pop(v: &mut IntVec) -> Option<i32> {
    v.data.pop()
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

    /// C: `void vm_trace(VM *vm, int t)` -- `iv_push(&vm->trace, t)`, return
    /// value discarded (the trace entry is dropped if allocation fails).
    pub fn trace(&mut self, t: i32) {
        let _ = self.trace.push(t);
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
