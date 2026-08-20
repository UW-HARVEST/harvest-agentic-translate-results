// Rust translation of the C `driver` program (c_src/).
//
// Goal: byte-identical behaviour with the C implementation, both for the
// command line program (`main`) and for every individual function with external
// linkage, when called through the C ABI.
//
// The C code has a number of quirks / latent bugs.  They are reproduced
// deliberately, NOT fixed:
//
//   * `process_a_stream` clamps a `size_t` accumulator against
//     `-0x80000000LL`.  Because the comparison is performed in
//     `unsigned long long`, the second clamp is *always* taken and the function
//     therefore always returns `INT_MIN`.
//   * engine opcode 9 pops `m` elements twice; the second round of pops may
//     fail, in which case the values from the first round are left in place.
//   * engine opcode 6 casts a possibly negative `int` jump distance to
//     `size_t`, so negative distances are rejected as "too far".
//   * `a.c` and `b.c` each keep a file-scope `static int` that persists across
//     calls, so results depend on the whole call history.
//
// Layout of this file mirrors the C sources:
//   util.c  -> "util" section
//   lib.c   -> "lib" section
//   a.c     -> "a" section
//   b.c     -> "b" section
//   engine.c-> "engine" section
//   main.c  -> "main" section

#![allow(clippy::missing_safety_doc)]

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int, c_long, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// libc / stdio bindings.
//
// The C program's observable output is produced by <stdio.h> (`printf`,
// `fprintf`, `fputc`) and its input parsing by `strtol` / `fgets`.  Calling the
// very same libc functions is the only way to guarantee identical formatting,
// identical stream buffering (and therefore identical stdout/stderr
// interleaving) and identical numeric parsing.
// ---------------------------------------------------------------------------

/// Opaque `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut FILE) -> *mut c_char;
    fn fputc(c: c_int, stream: *mut FILE) -> c_int;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;

    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    static mut stderr: *mut FILE;
}

const SIZEOF_INT: usize = core::mem::size_of::<c_int>();

// ---------------------------------------------------------------------------
// util.h types (must stay ABI compatible with the C structs).
// ---------------------------------------------------------------------------

/// `typedef struct { int *data; size_t len, cap; } IntVec;`
#[repr(C)]
pub struct IntVec {
    pub data: *mut c_int,
    pub len: usize,
    pub cap: usize,
}

/// `typedef struct { const int *code; size_t n; size_t ip; } Program;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Program {
    pub code: *const c_int,
    pub n: usize,
    pub ip: usize,
}

/// `typedef struct { IntVec stack; IntVec trace; int steps; } VM;`
#[repr(C)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: c_int,
}

// ---------------------------------------------------------------------------
// util.c
// ---------------------------------------------------------------------------

/// `void iv_init(IntVec *v){ v->data=NULL; v->len=v->cap=0; }`
#[no_mangle]
pub unsafe extern "C" fn iv_init(v: *mut IntVec) {
    (*v).data = ptr::null_mut();
    (*v).cap = 0;
    (*v).len = 0;
}

/// `void iv_free(IntVec *v){ free(v->data); v->data=NULL; v->len=v->cap=0; }`
#[no_mangle]
pub unsafe extern "C" fn iv_free(v: *mut IntVec) {
    free((*v).data as *mut c_void);
    (*v).data = ptr::null_mut();
    (*v).cap = 0;
    (*v).len = 0;
}

/// ```c
/// bool iv_reserve(IntVec *v, size_t need){
///     if (need <= v->cap) return true;
///     size_t nc = v->cap? v->cap:8;
///     while (nc < need) { if (nc > (SIZE_MAX/2)) return false; nc *= 2; }
///     int *p = (int*)realloc(v->data, nc*sizeof(int));
///     if (!p) return false;
///     v->data = p; v->cap = nc; return true;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn iv_reserve(v: *mut IntVec, need: usize) -> bool {
    if need <= (*v).cap {
        return true;
    }
    let mut nc: usize = if (*v).cap != 0 { (*v).cap } else { 8 };
    while nc < need {
        if nc > usize::MAX / 2 {
            return false;
        }
        nc = nc.wrapping_mul(2);
    }
    // `nc*sizeof(int)` is computed in `size_t`, i.e. it wraps on overflow.
    let p = realloc((*v).data as *mut c_void, nc.wrapping_mul(SIZEOF_INT)) as *mut c_int;
    if p.is_null() {
        return false;
    }
    (*v).data = p;
    (*v).cap = nc;
    true
}

/// `bool iv_push(IntVec *v,int x){ if(v->len==v->cap && !iv_reserve(v, v->cap? v->cap*2:8)) return false; v->data[v->len++]=x; return true; }`
#[no_mangle]
pub unsafe extern "C" fn iv_push(v: *mut IntVec, x: c_int) -> bool {
    if (*v).len == (*v).cap {
        let want = if (*v).cap != 0 {
            (*v).cap.wrapping_mul(2)
        } else {
            8
        };
        if !iv_reserve(v, want) {
            return false;
        }
    }
    *(*v).data.add((*v).len) = x;
    (*v).len += 1;
    true
}

/// `bool iv_pop(IntVec *v,int *out){ if(!v->len) return false; if(out) *out=v->data[v->len-1]; v->len--; return true; }`
#[no_mangle]
pub unsafe extern "C" fn iv_pop(v: *mut IntVec, out: *mut c_int) -> bool {
    if (*v).len == 0 {
        return false;
    }
    if !out.is_null() {
        *out = *(*v).data.add((*v).len - 1);
    }
    (*v).len -= 1;
    true
}

/// `int iv_peek(const IntVec *v,int def){ return v->len? v->data[v->len-1]: def; }`
#[no_mangle]
pub unsafe extern "C" fn iv_peek(v: *const IntVec, def: c_int) -> c_int {
    if (*v).len != 0 {
        *(*v).data.add((*v).len - 1)
    } else {
        def
    }
}

/// `void prog_init(Program *p, const int *code, size_t n){ p->code=code; p->n=n; p->ip=0; }`
#[no_mangle]
pub unsafe extern "C" fn prog_init(p: *mut Program, code: *const c_int, n: usize) {
    (*p).code = code;
    (*p).n = n;
    (*p).ip = 0;
}

/// `bool prog_fetch(Program *p, int *out){ if(p->ip>=p->n) return false; *out=p->code[p->ip++]; return true; }`
#[no_mangle]
pub unsafe extern "C" fn prog_fetch(p: *mut Program, out: *mut c_int) -> bool {
    if (*p).ip >= (*p).n {
        return false;
    }
    *out = *(*p).code.add((*p).ip);
    (*p).ip += 1;
    true
}

/// `void vm_init(VM *vm){ iv_init(&vm->stack); iv_init(&vm->trace); vm->steps=0; }`
#[no_mangle]
pub unsafe extern "C" fn vm_init(vm: *mut VM) {
    iv_init(ptr::addr_of_mut!((*vm).stack));
    iv_init(ptr::addr_of_mut!((*vm).trace));
    (*vm).steps = 0;
}

/// `void vm_free(VM *vm){ iv_free(&vm->stack); iv_free(&vm->trace); vm->steps=0; }`
#[no_mangle]
pub unsafe extern "C" fn vm_free(vm: *mut VM) {
    iv_free(ptr::addr_of_mut!((*vm).stack));
    iv_free(ptr::addr_of_mut!((*vm).trace));
    (*vm).steps = 0;
}

/// `void vm_trace(VM *vm, int t){ iv_push(&vm->trace, t); }`
#[no_mangle]
pub unsafe extern "C" fn vm_trace(vm: *mut VM, t: c_int) {
    iv_push(ptr::addr_of_mut!((*vm).trace), t);
}

/// ```c
/// void vm_print(FILE *fp, const char *label, const VM *vm){
///     fprintf(fp, "%sSTACK_TOP=%d STEPS=%d TRACE=", label, iv_peek(&vm->stack, -777), vm->steps);
///     for (size_t i=0;i<vm->trace.len;i++) fputc("abcdefghijklmnopqrstuvwxyz"[(vm->trace.data[i])&25], fp);
///     fputc('\n', fp);
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vm_print(fp: *mut FILE, label: *const c_char, vm: *const VM) {
    const LETTERS: &[u8; 27] = b"abcdefghijklmnopqrstuvwxyz\0";
    fprintf(
        fp,
        c"%sSTACK_TOP=%d STEPS=%d TRACE=".as_ptr(),
        label,
        iv_peek(ptr::addr_of!((*vm).stack), -777),
        (*vm).steps,
    );
    let trace = ptr::addr_of!((*vm).trace);
    let mut i: usize = 0;
    while i < (*trace).len {
        let t = *(*trace).data.add(i);
        // `t & 25` is always in 0..=25 (25 is a positive mask), so the index is
        // in bounds even for negative trace values.
        fputc(LETTERS[(t & 25) as usize] as c_int, fp);
        i += 1;
    }
    fputc(b'\n' as c_int, fp);
}

// ---------------------------------------------------------------------------
// File scope `static int` state.
//
// `a.c` has `static int state_a;` and `b.c` has `static int flipflop;`.  Both
// are zero initialised and persist for the lifetime of the module, so results
// depend on the entire history of calls.  A process-global (not thread-local)
// cell is used so that the Rust shared object behaves exactly like the C one.
// ---------------------------------------------------------------------------
struct StaticInt(UnsafeCell<c_int>);
// Same (lack of) thread safety as a C `static int`.
unsafe impl Sync for StaticInt {}
impl StaticInt {
    const fn new(v: c_int) -> Self {
        StaticInt(UnsafeCell::new(v))
    }
    #[inline]
    unsafe fn get(&self) -> c_int {
        *self.0.get()
    }
    #[inline]
    unsafe fn set(&self, v: c_int) {
        *self.0.get() = v;
    }
}

static STATE_A: StaticInt = StaticInt::new(0);
static FLIPFLOP: StaticInt = StaticInt::new(0);

// ---------------------------------------------------------------------------
// lib.c  --  the global `target` declared by api.h.
// ---------------------------------------------------------------------------

/// ```c
/// int target(int code) {
///     if (code < 0) return 7;
///     int m = code % 10;
///     if (m == 0) return 0;
///     if (m <= 3) return 1;
///     if (m <= 6) return 2;
///     if (m == 7) return 3;
///     return 4;
/// }
/// ```
#[no_mangle]
pub extern "C" fn target(code: c_int) -> c_int {
    if code < 0 {
        return 7;
    }
    let m = code % 10;
    if m == 0 {
        return 0;
    }
    if m <= 3 {
        return 1;
    }
    if m <= 6 {
        return 2;
    }
    if m == 7 {
        return 3;
    }
    4
}

// ---------------------------------------------------------------------------
// a.c  --  file-local `target` over `state_a`, plus the two exported entries.
// ---------------------------------------------------------------------------

/// `static int target(int code)` from a.c.
unsafe fn a_target(code: c_int) -> c_int {
    if code < 0 {
        return if (STATE_A.get() & 1) != 0 { 6 } else { 5 };
    }
    // `code<<1` overflows for large `code`; C wraps here (gcc, no -O), and so
    // does Rust's `<<` (which only traps on out-of-range shift *amounts*).
    STATE_A.set(STATE_A.get() ^ (code << 1));
    let k = ((code >> 2) ^ STATE_A.get()) & 7;
    match k {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        // `case 5:;` falls through to `case 6:`
        5 | 6 => 5,
        _ => 7,
    }
}

/// `static inline int a_bias_call(int (*fp)(int), int x){ return fp((x ^ 0x55) + 7); }`
#[inline]
unsafe fn a_bias_call(x: c_int) -> c_int {
    a_target((x ^ 0x55).wrapping_add(7))
}

/// `static inline int wrap(int x){ return target(x-5); }`
#[inline]
unsafe fn a_wrap(x: c_int) -> c_int {
    a_target(x.wrapping_sub(5))
}

/// ```c
/// int call_a_once(int x){
///     int (*fp)(int) = &target;
///     int a = fp(x);
///     int b = wrap(a);
///     int c = target(b ^ 3);
///     int d = A_MAC_CALL(&target, b);   // a_bias_call(&target, b)
///     return a ^ (b << 1) ^ (c << 2) ^ (d << 3);
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn call_a_once(x: c_int) -> c_int {
    let a = a_target(x);
    let b = a_wrap(a);
    let c = a_target(b ^ 3);
    let d = a_bias_call(b);
    a ^ (b << 1) ^ (c << 2) ^ (d << 3)
}

/// ```c
/// int process_a_stream(const int *xs, size_t n){
///     size_t acc=0;
///     for(size_t i=0;i<n;i++){
///         int v=xs[i];
///         for(int j=0;j<3;j++){
///             int t=target(v+j);
///             if ((t&1)==0) { acc += t; continue; }
///             acc ^= (t<<j);
///             if (t==5) break;
///         }
///     }
///     if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
///     if (acc < -0x80000000LL) acc = -0x80000000LL;
///     return (int)acc;
/// }
/// ```
///
/// `acc` is a `size_t`.  In `acc < -0x80000000LL` the usual arithmetic
/// conversions turn both operands into `unsigned long long`, so the right hand
/// side becomes 0xFFFF_FFFF_8000_0000 and the comparison is true for every
/// value `acc` can hold after the first clamp.  The function therefore always
/// returns `INT_MIN`.  Reproduced verbatim.
#[no_mangle]
pub unsafe extern "C" fn process_a_stream(xs: *const c_int, n: usize) -> c_int {
    let mut acc: u64 = 0; // size_t
    let mut i: usize = 0;
    while i < n {
        let v = *xs.add(i);
        let mut j: c_int = 0;
        while j < 3 {
            let t = a_target(v.wrapping_add(j));
            if (t & 1) == 0 {
                // `acc += t` : t is 0..=7, converted to size_t.
                acc = acc.wrapping_add(t as u64);
                j += 1;
                continue;
            }
            acc ^= (t << j) as u64;
            if t == 5 {
                break;
            }
            j += 1;
        }
        i += 1;
    }
    if acc > 0x7fff_ffff_u64 {
        acc = 0x7fff_ffff_u64;
    }
    if acc < 0xFFFF_FFFF_8000_0000_u64 {
        acc = 0xFFFF_FFFF_8000_0000_u64;
    }
    acc as u32 as c_int
}

// ---------------------------------------------------------------------------
// b.c  --  file-local `target` over `flipflop`, plus the two exported entries.
// ---------------------------------------------------------------------------

/// `static int target(int code)` from b.c.
unsafe fn b_target(code: c_int) -> c_int {
    FLIPFLOP.set(FLIPFLOP.get() ^ 1);
    if code < 0 {
        return if FLIPFLOP.get() != 0 { 2 } else { 6 };
    }
    let z = (code ^ if FLIPFLOP.get() != 0 { 0x7f } else { 0x1f }) % 8;
    if z == 0 || z == 7 {
        return 4;
    }
    if z == 1 || z == 2 {
        return 3;
    }
    if z == 3 {
        return 1;
    }
    if z == 4 {
        return 0;
    }
    if z == 5 {
        return 5;
    }
    7
}

/// `static inline int b_twist_call(int (*fp)(int), int x){ return fp(((x + 9) ^ 0x2222) - 17); }`
#[inline]
unsafe fn b_twist_call(x: c_int) -> c_int {
    b_target((x.wrapping_add(9) ^ 0x2222).wrapping_sub(17))
}

/// `static inline int w2(int x){ return target(x+9); }`
#[inline]
unsafe fn b_w2(x: c_int) -> c_int {
    b_target(x.wrapping_add(9))
}

/// ```c
/// int call_b_once(int x){
///     int (*fp)(int) = &target;
///     int a = target(x);
///     int b = w2(a);
///     int c = B_MAC_CALL(&target, a);   // b_twist_call(&target, a)
///     int d = fp(c ^ x);
///     return (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4);
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn call_b_once(x: c_int) -> c_int {
    let a = b_target(x);
    let b = b_w2(a);
    let c = b_twist_call(a);
    let d = b_target(c ^ x);
    (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4)
}

/// ```c
/// int process_b_stream(const int *xs, size_t n){
///     int acc=1;
///     for(size_t i=0;i<n;i++){
///         int v=xs[i];
///         int iter=0;
///         while(++iter<=4){
///             int t = target(v-iter);
///             if (t==6) { acc -= t; break; }
///             if (t==3) { continue; }
///             acc = (acc * 3) ^ t;
///         }
///     }
///     return acc;
/// }
/// ```
#[no_mangle]
// `!(iter <= 4)` is kept verbatim so the loop reads like the C `while (++iter <= 4)`
// (pre-increment, then test).
#[allow(clippy::nonminimal_bool)]
pub unsafe extern "C" fn process_b_stream(xs: *const c_int, n: usize) -> c_int {
    let mut acc: c_int = 1;
    let mut i: usize = 0;
    while i < n {
        let v = *xs.add(i);
        let mut iter: c_int = 0;
        loop {
            iter = iter.wrapping_add(1);
            if !(iter <= 4) {
                break;
            }
            let t = b_target(v.wrapping_sub(iter));
            if t == 6 {
                acc = acc.wrapping_sub(t);
                break;
            }
            if t == 3 {
                continue;
            }
            acc = acc.wrapping_mul(3) ^ t;
        }
        i += 1;
    }
    acc
}

// ---------------------------------------------------------------------------
// engine.c
// ---------------------------------------------------------------------------

/// `static inline int inline_call(int (*f)(int), int x){ return f(x); }` combined
/// with `#define MAC_CALL(F,X) ((F)((X)+1))`:
///
/// ```c
/// static int classify(int impl, int x){
///     if (impl==0) return inline_call(call_a_once, x);
///     if (impl==1) return MAC_CALL(call_b_once, x);
///     return inline_call(&target, MAC_CALL(target, x));
/// }
/// ```
unsafe fn classify(imp: c_int, x: c_int) -> c_int {
    if imp == 0 {
        return call_a_once(x);
    }
    if imp == 1 {
        return call_b_once(x.wrapping_add(1));
    }
    target(target(x.wrapping_add(1)))
}

/// ```c
/// static int process_stream(int impl, const int *buf, size_t n){
///     if (impl==0) return process_a_stream(buf, n);
///     if (impl==1) return process_b_stream(buf, n);
///     int acc=0;
///     for(size_t i=0;i<n;i++){
///         int t = target(buf[i]);
///         if ((t&1)==0) acc += (t*2);
///         else          acc ^= (t+7);
///     }
///     return acc;
/// }
/// ```
unsafe fn process_stream(imp: c_int, buf: *const c_int, n: usize) -> c_int {
    if imp == 0 {
        return process_a_stream(buf, n);
    }
    if imp == 1 {
        return process_b_stream(buf, n);
    }
    let mut acc: c_int = 0;
    let mut i: usize = 0;
    while i < n {
        let t = target(*buf.add(i));
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
        i += 1;
    }
    acc
}

/// `int run_engine(int impl_id, const int *code, size_t n, VM *vm)` -- see
/// c_src/src/engine.c.  Every `return <n>` error code and every trace value is
/// preserved exactly.
#[no_mangle]
pub unsafe extern "C" fn run_engine(
    impl_id: c_int,
    code: *const c_int,
    n: usize,
    vm: *mut VM,
) -> c_int {
    let mut p = Program {
        code: ptr::null(),
        n: 0,
        ip: 0,
    };
    prog_init(&mut p, code, n);
    let stack = ptr::addr_of_mut!((*vm).stack);
    let mut op: c_int = 0;
    while prog_fetch(&mut p, &mut op) {
        (*vm).steps = (*vm).steps.wrapping_add(1);
        match op {
            0 => {
                // push immediate
                let mut imm: c_int = 0;
                if !prog_fetch(&mut p, &mut imm) {
                    return 1;
                }
                iv_push(stack, imm);
                vm_trace(vm, 0);
            }
            1 => {
                // add
                let mut a: c_int = 0;
                let mut b: c_int = 0;
                if !iv_pop(stack, &mut b) || !iv_pop(stack, &mut a) {
                    return 2;
                }
                iv_push(stack, a.wrapping_add(b));
                vm_trace(vm, 1);
            }
            2 => {
                // multiply
                let mut a: c_int = 0;
                let mut b: c_int = 0;
                if !iv_pop(stack, &mut b) || !iv_pop(stack, &mut a) {
                    return 3;
                }
                iv_push(stack, a.wrapping_mul(b));
                vm_trace(vm, 2);
            }
            3 => {
                // dup (peek default 0)
                let a = iv_peek(stack, 0);
                iv_push(stack, a);
                vm_trace(vm, 3);
            }
            4 => {
                // drop
                let mut tmp: c_int = 0;
                if !iv_pop(stack, &mut tmp) {
                    return 4;
                }
                vm_trace(vm, 4);
            }
            5 => {
                let x = iv_peek(stack, 0);
                let bucket = classify(impl_id, x);
                iv_push(stack, bucket);
                match bucket {
                    0 => vm_trace(vm, 5),
                    1 => vm_trace(vm, 6),
                    2 => vm_trace(vm, 7),
                    // `case 3:;` falls through to `case 4:`
                    3 | 4 => vm_trace(vm, 8),
                    _ => vm_trace(vm, 9),
                }
            }
            6 => {
                // conditional relative jump
                let mut k: c_int = 0;
                if !prog_fetch(&mut p, &mut k) {
                    return 5;
                }
                let mut cond: c_int = 0;
                if !iv_pop(stack, &mut cond) {
                    return 6;
                }
                if cond != 0 {
                    // `(size_t)k` sign-extends: negative k becomes huge.
                    if (k as usize) > p.n - p.ip {
                        return 7;
                    }
                    p.ip += k as usize;
                    vm_trace(vm, 10);
                } else {
                    vm_trace(vm, 11);
                }
            }
            7 => {
                // repeat the single following instruction `times` times
                let mut times: c_int = 0;
                if !prog_fetch(&mut p, &mut times) {
                    return 8;
                }
                if p.ip >= p.n {
                    return 9;
                }
                let saved_ip = p.ip;
                let mut i: c_int = 0;
                while i < times {
                    let inner = Program {
                        code: p.code,
                        n: p.n,
                        ip: saved_ip,
                    };
                    let rc = run_engine(impl_id, inner.code.add(inner.ip), 1, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm_trace(vm, 12);
                        break;
                    }
                    i += 1;
                }
                p.ip = saved_ip + 1;
            }
            8 => {
                let x = iv_peek(stack, 0);
                let y = classify(impl_id, x);
                iv_push(stack, y);
                vm_trace(vm, 13);
            }
            9 => {
                // `int tmp[m]` VLA, then TWO rounds of pops (the second round's
                // failures silently leave the first round's values in place).
                let mut m: c_int = 0;
                if !prog_fetch(&mut p, &mut m) {
                    return 10;
                }
                if m < 0 || (m as usize) > (*stack).len {
                    return 11;
                }
                let mu = m as usize;
                // The C VLA is uninitialised, but the first pop loop always
                // succeeds for all `m` slots (m <= stack.len), so every slot is
                // written before it is read.
                let mut tmp: Vec<c_int> = vec![0; mu];
                let mut i = m - 1;
                while i >= 0 {
                    iv_pop(stack, tmp.as_mut_ptr().add(i as usize));
                    i -= 1;
                }
                let mut i = m - 1;
                while i >= 0 {
                    iv_pop(stack, tmp.as_mut_ptr().add(i as usize));
                    i -= 1;
                }
                let s = process_stream(impl_id, tmp.as_ptr(), mu);
                iv_push(stack, s);
                vm_trace(vm, 14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}

// ---------------------------------------------------------------------------
// main.c
// ---------------------------------------------------------------------------

/// ```c
/// static void usage(const char *p){
///     fprintf(stderr, "Usage: %s [--stdin] [bytecodes...]\n"
///                     "Bytecodes are integers forming a small VM program.\n", p);
/// }
/// ```
unsafe fn usage(p: *const c_char) {
    fprintf(
        stderr,
        c"Usage: %s [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n"
            .as_ptr(),
        p,
    );
}

/// ```c
/// static size_t read_stdin(IntVec *v){
///     char buf[4096];
///     size_t count=0;
///     while (fgets(buf, sizeof buf, stdin)) { ... }
///     return count;
/// }
/// ```
unsafe fn read_stdin(v: *mut IntVec) -> usize {
    let mut buf = [0 as c_char; 4096];
    let mut count: usize = 0;
    while !fgets(buf.as_mut_ptr(), buf.len() as c_int, stdin).is_null() {
        let mut p: *mut c_char = buf.as_mut_ptr();
        while *p != 0 {
            let mut q: *mut c_char = p;
            while *q != 0
                && *q != b' ' as c_char
                && *q != b'\t' as c_char
                && *q != b'\n' as c_char
                && *q != b'\r' as c_char
            {
                q = q.add(1);
            }
            let save = *q;
            *q = 0;
            if *p != 0 {
                let mut e: *mut c_char = ptr::null_mut();
                let t = strtol(p, &mut e, 10);
                if !e.is_null() && *e == 0 {
                    iv_push(v, t as c_int);
                    count += 1;
                }
            }
            *q = save;
            p = if *q != 0 { q.add(1) } else { q };
        }
    }
    count
}

/// `int main(int argc, char **argv)` from main.c.
///
/// This is the real process entry point of the `driver` binary (see
/// src/main.rs, which is `#![no_main]`), and it is also exported by the shared
/// object exactly like the C build.
#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let mut use_stdin = false;
    let mut code = IntVec {
        data: ptr::null_mut(),
        len: 0,
        cap: 0,
    };
    iv_init(&mut code);

    let mut i: c_int = 1;
    while i < argc {
        let arg = *argv.offset(i as isize);
        if strcmp(arg, c"--help".as_ptr()) == 0 {
            usage(*argv.offset(0));
            iv_free(&mut code);
            return 0;
        } else if strcmp(arg, c"--stdin".as_ptr()) == 0 {
            use_stdin = true;
        } else {
            let mut e: *mut c_char = ptr::null_mut();
            let t = strtol(arg, &mut e, 10);
            if !e.is_null() && *e == 0 {
                iv_push(&mut code, t as c_int);
            } else {
                fprintf(stderr, c"skip '%s'\n".as_ptr(), arg);
            }
        }
        i += 1;
    }
    if use_stdin {
        read_stdin(&mut code);
    }
    if code.len == 0 {
        fprintf(stderr, c"no program\n".as_ptr());
        iv_free(&mut code);
        return 2;
    }

    let mut vm_a = VM {
        stack: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    let mut vm_b = VM {
        stack: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    let mut vm_e = VM {
        stack: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    vm_init(&mut vm_a);
    vm_init(&mut vm_b);
    vm_init(&mut vm_e);

    let rc_a = run_engine(0, code.data, code.len, &mut vm_a);
    let rc_b = run_engine(1, code.data, code.len, &mut vm_b);
    let rc_e = run_engine(2, code.data, code.len, &mut vm_e);

    printf(c"RC:A=%d B=%d EXT=%d\n".as_ptr(), rc_a, rc_b, rc_e);
    vm_print(stdout, c"A:".as_ptr(), &vm_a);
    vm_print(stdout, c"B:".as_ptr(), &vm_b);
    vm_print(stdout, c"EXT:".as_ptr(), &vm_e);

    vm_free(&mut vm_a);
    vm_free(&mut vm_b);
    vm_free(&mut vm_e);
    iv_free(&mut code);
    0
}
