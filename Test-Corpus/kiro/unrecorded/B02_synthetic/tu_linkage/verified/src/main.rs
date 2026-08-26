use std::env;
use std::io::{self, BufRead};

// --------------- Static mutable state (mirrors C statics in a.c and b.c) ---------------
static mut STATE_A: i32 = 0;
static mut FLIPFLOP: i32 = 0;

// --------------- target functions (3 separate versions like a.c, b.c, lib.c) ---------------

fn target_a(code: i32) -> i32 {
    unsafe {
        if code < 0 {
            return if (STATE_A & 1) != 0 { 6 } else { 5 };
        }
        STATE_A ^= code << 1;
        let k = ((code >> 2) ^ STATE_A) & 7;
        match k {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 1,
            4 => 3,
            5 | 6 => 5,
            _ => 7,
        }
    }
}

fn target_b(code: i32) -> i32 {
    unsafe {
        FLIPFLOP ^= 1;
        if code < 0 {
            return if FLIPFLOP != 0 { 2 } else { 6 };
        }
        let z = (code ^ (if FLIPFLOP != 0 { 0x7f } else { 0x1f })) % 8;
        if z == 0 || z == 7 { return 4; }
        if z == 1 || z == 2 { return 3; }
        if z == 3 { return 1; }
        if z == 4 { return 0; }
        if z == 5 { return 5; }
        7
    }
}

fn target_lib(code: i32) -> i32 {
    if code < 0 { return 7; }
    let m = code % 10;
    if m == 0 { return 0; }
    if m <= 3 { return 1; }
    if m <= 6 { return 2; }
    if m == 7 { return 3; }
    4
}

// --------------- a.c functions ---------------

fn a_bias_call(x: i32) -> i32 {
    target_a((x ^ 0x55).wrapping_add(7))
}

fn call_a_once_impl(x: i32) -> i32 {
    let a = target_a(x);
    let b = target_a(a.wrapping_sub(5));
    let c = target_a(b ^ 3);
    let d = a_bias_call(b);
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

fn process_a_stream_impl(xs: &[i32]) -> i32 {
    let mut acc: u64 = 0;
    for &v in xs {
        for j in 0..3i32 {
            let t = target_a(v.wrapping_add(j));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t as u64);
                continue;
            }
            acc ^= (t << j) as u64;
            if t == 5 { break; }
        }
    }
    if acc > 0x7fffffff_u64 { acc = 0x7fffffff_u64; }
    if acc < (-0x80000000i64 as u64) { acc = (-0x80000000i64) as u64; }
    acc as i32
}

// --------------- b.c functions ---------------

fn b_twist_call(x: i32) -> i32 {
    target_b((x.wrapping_add(9) ^ 0x2222).wrapping_sub(17))
}

fn call_b_once_impl(x: i32) -> i32 {
    let a = target_b(x);
    let b = target_b(a.wrapping_add(9));
    let c = b_twist_call(a);
    let d = target_b(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

fn process_b_stream_impl(xs: &[i32]) -> i32 {
    let mut acc: i32 = 1;
    for &v in xs {
        let mut iter = 0i32;
        loop {
            iter += 1;
            if iter > 4 { break; }
            let t = target_b(v.wrapping_sub(iter));
            if t == 6 { acc = acc.wrapping_sub(t); break; }
            if t == 3 { continue; }
            acc = acc.wrapping_mul(3) ^ t;
        }
    }
    acc
}

// --------------- C-compatible structs ---------------

#[repr(C)]
pub struct IntVec {
    data: *mut i32,
    len: usize,
    cap: usize,
}

#[repr(C)]
pub struct Program {
    code: *const i32,
    n: usize,
    ip: usize,
}

#[repr(C)]
pub struct VM {
    stack: IntVec,
    trace: IntVec,
    steps: i32,
}

// --------------- IntVec operations ---------------

impl IntVec {
    fn new_empty() -> Self { IntVec { data: std::ptr::null_mut(), len: 0, cap: 0 } }

    fn push_val(&mut self, x: i32) -> bool {
        if self.len == self.cap {
            let nc = if self.cap == 0 { 8 } else { self.cap * 2 };
            if !self.reserve(nc) { return false; }
        }
        unsafe { *self.data.add(self.len) = x; }
        self.len += 1;
        true
    }

    fn pop_val(&mut self) -> Option<i32> {
        if self.len == 0 { return None; }
        self.len -= 1;
        Some(unsafe { *self.data.add(self.len) })
    }

    fn peek_val(&self, def: i32) -> i32 {
        if self.len == 0 { def } else { unsafe { *self.data.add(self.len - 1) } }
    }

    fn reserve(&mut self, need: usize) -> bool {
        if need <= self.cap { return true; }
        let mut nc = if self.cap == 0 { 8 } else { self.cap };
        while nc < need {
            if nc > usize::MAX / 2 { return false; }
            nc *= 2;
        }
        let layout_size = nc * std::mem::size_of::<i32>();
        let p = if self.data.is_null() {
            unsafe { libc_malloc(layout_size) as *mut i32 }
        } else {
            unsafe { libc_realloc(self.data as *mut u8, layout_size) as *mut i32 }
        };
        if p.is_null() { return false; }
        self.data = p;
        self.cap = nc;
        true
    }

    fn free_data(&mut self) {
        if !self.data.is_null() {
            unsafe { libc_free(self.data as *mut u8); }
        }
        self.data = std::ptr::null_mut();
        self.len = 0;
        self.cap = 0;
    }
}

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(size: usize) -> *mut u8;
    #[link_name = "realloc"]
    fn libc_realloc(ptr: *mut u8, size: usize) -> *mut u8;
    #[link_name = "free"]
    fn libc_free(ptr: *mut u8);
}

// --------------- Program operations ---------------

impl Program {
    fn fetch_val(&mut self) -> Option<i32> {
        if self.ip >= self.n { return None; }
        let v = unsafe { *self.code.add(self.ip) };
        self.ip += 1;
        Some(v)
    }
}

// --------------- VM operations ---------------

impl VM {
    fn new_empty() -> Self {
        VM { stack: IntVec::new_empty(), trace: IntVec::new_empty(), steps: 0 }
    }
    fn free_vm(&mut self) {
        self.stack.free_data();
        self.trace.free_data();
        self.steps = 0;
    }
    fn add_trace(&mut self, t: i32) { self.trace.push_val(t); }
}

// --------------- engine ---------------

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return call_a_once_impl(x);
    }
    if impl_id == 1 {
        return call_b_once_impl(x.wrapping_add(1));
    }
    let inner = target_lib(x.wrapping_add(1));
    target_lib(inner)
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 { return process_a_stream_impl(buf); }
    if impl_id == 1 { return process_b_stream_impl(buf); }
    let mut acc: i32 = 0;
    for &v in buf {
        let t = target_lib(v);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

fn run_engine_impl(impl_id: i32, code: *const i32, n: usize, vm: &mut VM) -> i32 {
    let mut p = Program { code, n, ip: 0 };
    while let Some(op) = p.fetch_val() {
        vm.steps += 1;
        match op {
            0 => {
                let imm = match p.fetch_val() { Some(v) => v, None => return 1 };
                vm.stack.push_val(imm);
                vm.add_trace(0);
            }
            1 => {
                let b = match vm.stack.pop_val() { Some(v) => v, None => return 2 };
                let a = match vm.stack.pop_val() { Some(v) => v, None => return 2 };
                vm.stack.push_val(a.wrapping_add(b));
                vm.add_trace(1);
            }
            2 => {
                let b = match vm.stack.pop_val() { Some(v) => v, None => return 3 };
                let a = match vm.stack.pop_val() { Some(v) => v, None => return 3 };
                vm.stack.push_val(a.wrapping_mul(b));
                vm.add_trace(2);
            }
            3 => {
                let a = vm.stack.peek_val(0);
                vm.stack.push_val(a);
                vm.add_trace(3);
            }
            4 => {
                if vm.stack.pop_val().is_none() { return 4; }
                vm.add_trace(4);
            }
            5 => {
                let x = vm.stack.peek_val(0);
                let bucket = classify(impl_id, x);
                vm.stack.push_val(bucket);
                match bucket {
                    0 => vm.add_trace(5),
                    1 => vm.add_trace(6),
                    2 => vm.add_trace(7),
                    3 | 4 => vm.add_trace(8),
                    _ => vm.add_trace(9),
                }
            }
            6 => {
                let k = match p.fetch_val() { Some(v) => v, None => return 5 };
                let cond = match vm.stack.pop_val() { Some(v) => v, None => return 6 };
                if cond != 0 {
                    let remaining = p.n - p.ip;
                    if (k as usize) > remaining { return 7; }
                    p.ip += k as usize;
                    vm.add_trace(10);
                } else {
                    vm.add_trace(11);
                }
            }
            7 => {
                let times = match p.fetch_val() { Some(v) => v, None => return 8 };
                if p.ip >= p.n { return 9; }
                let saved_ip = p.ip;
                for _ in 0..times {
                    let inner_code = unsafe { p.code.add(saved_ip) };
                    let rc = run_engine_impl(impl_id, inner_code, 1, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm.add_trace(12);
                        break;
                    }
                }
                p.ip = saved_ip + 1;
            }
            8 => {
                let x = vm.stack.peek_val(0);
                let y = classify(impl_id, x);
                vm.stack.push_val(y);
                vm.add_trace(13);
            }
            9 => {
                let m = match p.fetch_val() { Some(v) => v, None => return 10 };
                if m < 0 || (m as usize) > vm.stack.len { return 11; }
                let m = m as usize;
                let mut tmp = vec![0i32; m];
                for i in (0..m).rev() {
                    if let Some(v) = vm.stack.pop_val() { tmp[i] = v; }
                }
                for i in (0..m).rev() {
                    if let Some(v) = vm.stack.pop_val() { tmp[i] = v; }
                }
                let s = process_stream(impl_id, &tmp);
                vm.stack.push_val(s);
                vm.add_trace(14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}

// --------------- #[no_mangle] exports ---------------

#[no_mangle]
pub extern "C" fn target(code: i32) -> i32 {
    target_lib(code)
}

#[no_mangle]
pub extern "C" fn call_a_once(x: i32) -> i32 {
    call_a_once_impl(x)
}

#[no_mangle]
pub extern "C" fn process_a_stream(xs: *const i32, n: usize) -> i32 {
    let slice = unsafe { std::slice::from_raw_parts(xs, n) };
    process_a_stream_impl(slice)
}

#[no_mangle]
pub extern "C" fn call_b_once(x: i32) -> i32 {
    call_b_once_impl(x)
}

#[no_mangle]
pub extern "C" fn process_b_stream(xs: *const i32, n: usize) -> i32 {
    let slice = unsafe { std::slice::from_raw_parts(xs, n) };
    process_b_stream_impl(slice)
}

#[no_mangle]
pub extern "C" fn iv_init(v: *mut IntVec) {
    unsafe { *v = IntVec::new_empty(); }
}

#[no_mangle]
pub extern "C" fn iv_free(v: *mut IntVec) {
    unsafe { (*v).free_data(); }
}

#[no_mangle]
pub extern "C" fn iv_reserve(v: *mut IntVec, need: usize) -> bool {
    unsafe { (*v).reserve(need) }
}

#[no_mangle]
pub extern "C" fn iv_push(v: *mut IntVec, x: i32) -> bool {
    unsafe { (*v).push_val(x) }
}

#[no_mangle]
pub extern "C" fn iv_pop(v: *mut IntVec, out: *mut i32) -> bool {
    unsafe {
        if (*v).len == 0 { return false; }
        (*v).len -= 1;
        if !out.is_null() { *out = *(*v).data.add((*v).len); }
        true
    }
}

#[no_mangle]
pub extern "C" fn iv_peek(v: *const IntVec, def: i32) -> i32 {
    unsafe { (*v).peek_val(def) }
}

#[no_mangle]
pub extern "C" fn prog_init(p: *mut Program, code: *const i32, n: usize) {
    unsafe { *p = Program { code, n, ip: 0 }; }
}

#[no_mangle]
pub extern "C" fn prog_fetch(p: *mut Program, out: *mut i32) -> bool {
    unsafe {
        if (*p).ip >= (*p).n { return false; }
        *out = *(*p).code.add((*p).ip);
        (*p).ip += 1;
        true
    }
}

#[no_mangle]
pub extern "C" fn vm_init(vm: *mut VM) {
    unsafe { *vm = VM::new_empty(); }
}

#[no_mangle]
pub extern "C" fn vm_free(vm: *mut VM) {
    unsafe { (*vm).free_vm(); }
}

#[no_mangle]
pub extern "C" fn vm_trace(vm: *mut VM, t: i32) {
    unsafe { (*vm).trace.push_val(t); }
}

#[no_mangle]
pub extern "C" fn vm_print(fp: *mut libc_FILE, label: *const i8, vm: *const VM) {
    // We replicate the C fprintf behavior
    unsafe {
        let vm = &*vm;
        let label_str = std::ffi::CStr::from_ptr(label).to_str().unwrap_or("");
        let alphabet = b"abcdefghijklmnopqrstuvwxyz";
        // Use libc fprintf to write to the FILE*
        let fmt1 = b"%sSTACK_TOP=%d STEPS=%d TRACE=\0";
        let label_c = std::ffi::CString::new(label_str).unwrap();
        libc_fprintf(fp, fmt1.as_ptr() as *const i8, label_c.as_ptr(), vm.stack.peek_val(-777), vm.steps);
        for i in 0..vm.trace.len {
            let t = *vm.trace.data.add(i);
            libc_fputc(alphabet[(t & 25) as usize] as i32, fp);
        }
        libc_fputc(b'\n' as i32, fp);
    }
}

#[repr(C)]
pub struct libc_FILE { _private: [u8; 0] }

extern "C" {
    fn fprintf(fp: *mut libc_FILE, fmt: *const i8, ...) -> i32;
    fn fputc(c: i32, fp: *mut libc_FILE) -> i32;
}

// Use different names to avoid conflict with our exports
unsafe fn libc_fprintf(fp: *mut libc_FILE, fmt: *const i8, label: *const i8, stack_top: i32, steps: i32) -> i32 {
    fprintf(fp, fmt, label, stack_top, steps)
}

unsafe fn libc_fputc(c: i32, fp: *mut libc_FILE) -> i32 {
    fputc(c, fp)
}

#[no_mangle]
pub extern "C" fn run_engine(impl_id: i32, code: *const i32, n: usize, vm: *mut VM) -> i32 {
    unsafe {
        // Reset static state for each engine run to match C behavior
        // Actually no - C doesn't reset statics between calls. They persist.
        run_engine_impl(impl_id, code, n, &mut *vm)
    }
}

// --------------- stdin reading ---------------

fn read_stdin_internal(code: &mut Vec<i32>) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        for token in line.split(|c: char| c == ' ' || c == '\t' || c == '\r') {
            if !token.is_empty() {
                if let Ok(v) = token.parse::<i64>() {
                    code.push(v as i32);
                }
            }
        }
    }
}

// --------------- main ---------------

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut use_stdin = false;
    let mut code: Vec<i32> = Vec::new();

    for i in 1..args.len() {
        if args[i] == "--help" {
            eprintln!("Usage: {} [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.", args[0]);
            return;
        } else if args[i] == "--stdin" {
            use_stdin = true;
        } else {
            match args[i].parse::<i64>() {
                Ok(t) => code.push(t as i32),
                Err(_) => eprintln!("skip '{}'", args[i]),
            }
        }
    }

    if use_stdin { read_stdin_internal(&mut code); }

    if code.is_empty() {
        eprintln!("no program");
        std::process::exit(2);
    }

    let mut vm_a = VM::new_empty();
    let mut vm_b = VM::new_empty();
    let mut vm_e = VM::new_empty();

    let rc_a = run_engine_impl(0, code.as_ptr(), code.len(), &mut vm_a);
    let rc_b = run_engine_impl(1, code.as_ptr(), code.len(), &mut vm_b);
    let rc_e = run_engine_impl(2, code.as_ptr(), code.len(), &mut vm_e);

    println!("RC:A={} B={} EXT={}", rc_a, rc_b, rc_e);

    // Print using the same format as C
    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    for (label, vm) in [("A:", &vm_a), ("B:", &vm_b), ("EXT:", &vm_e)] {
        print!("{}STACK_TOP={} STEPS={} TRACE=", label, vm.stack.peek_val(-777), vm.steps);
        for i in 0..vm.trace.len {
            let t = unsafe { *vm.trace.data.add(i) };
            print!("{}", alphabet[(t & 25) as usize] as char);
        }
        println!();
    }

    vm_a.free_vm();
    vm_b.free_vm();
    vm_e.free_vm();
}
