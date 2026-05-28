// Translation of c_src/src/util.c — uses repr(C) layouts and libc
// malloc/realloc/free so the structs are byte-compatible with the C version
// when shared across the FFI boundary.

use core::ffi::c_int;
use std::io::Write;

#[repr(C)]
pub struct IntVec {
    pub data: *mut c_int,
    pub len: usize,
    pub cap: usize,
}

impl Default for IntVec {
    fn default() -> Self {
        IntVec {
            data: core::ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }
}

impl IntVec {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn push(&mut self, x: c_int) -> bool {
        unsafe { iv_push_impl(self as *mut _, x) }
    }
    pub fn pop(&mut self) -> Option<c_int> {
        let mut out: c_int = 0;
        let ok = unsafe { iv_pop_impl(self as *mut _, &mut out as *mut _) };
        if ok {
            Some(out)
        } else {
            None
        }
    }
    pub fn peek(&self, def: c_int) -> c_int {
        unsafe { iv_peek_impl(self as *const _, def) }
    }
    pub fn as_slice(&self) -> &[c_int] {
        if self.data.is_null() || self.len == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(self.data, self.len) }
        }
    }
}

impl Drop for IntVec {
    fn drop(&mut self) {
        if !self.data.is_null() {
            unsafe {
                libc::free(self.data as *mut libc::c_void);
            }
            self.data = core::ptr::null_mut();
            self.len = 0;
            self.cap = 0;
        }
    }
}

// ---- Internal implementations matching C semantics exactly ----

pub(crate) unsafe fn iv_init_impl(v: *mut IntVec) {
    (*v).data = core::ptr::null_mut();
    (*v).len = 0;
    (*v).cap = 0;
}

pub(crate) unsafe fn iv_free_impl(v: *mut IntVec) {
    if !(*v).data.is_null() {
        libc::free((*v).data as *mut libc::c_void);
    }
    (*v).data = core::ptr::null_mut();
    (*v).len = 0;
    (*v).cap = 0;
}

pub(crate) unsafe fn iv_reserve_impl(v: *mut IntVec, need: usize) -> bool {
    if need <= (*v).cap {
        return true;
    }
    let mut nc: usize = if (*v).cap != 0 { (*v).cap } else { 8 };
    while nc < need {
        if nc > (usize::MAX / 2) {
            return false;
        }
        nc *= 2;
    }
    let new_size = nc.checked_mul(core::mem::size_of::<c_int>());
    let new_size = match new_size {
        Some(s) => s,
        None => return false,
    };
    let p = libc::realloc((*v).data as *mut libc::c_void, new_size) as *mut c_int;
    if p.is_null() {
        return false;
    }
    (*v).data = p;
    (*v).cap = nc;
    true
}

pub(crate) unsafe fn iv_push_impl(v: *mut IntVec, x: c_int) -> bool {
    if (*v).len == (*v).cap {
        let new_cap = if (*v).cap != 0 { (*v).cap * 2 } else { 8 };
        if !iv_reserve_impl(v, new_cap) {
            return false;
        }
    }
    let len = (*v).len;
    *(*v).data.add(len) = x;
    (*v).len = len + 1;
    true
}

pub(crate) unsafe fn iv_pop_impl(v: *mut IntVec, out: *mut c_int) -> bool {
    if (*v).len == 0 {
        return false;
    }
    if !out.is_null() {
        *out = *(*v).data.add((*v).len - 1);
    }
    (*v).len -= 1;
    true
}

pub(crate) unsafe fn iv_peek_impl(v: *const IntVec, def: c_int) -> c_int {
    if (*v).len == 0 {
        def
    } else {
        *(*v).data.add((*v).len - 1)
    }
}

// ---------------- Program ----------------
#[repr(C)]
pub struct Program {
    pub code: *const c_int,
    pub n: usize,
    pub ip: usize,
}

impl Program {
    pub fn new(code: &[c_int]) -> Self {
        Program {
            code: code.as_ptr(),
            n: code.len(),
            ip: 0,
        }
    }
    pub fn fetch(&mut self) -> Option<c_int> {
        if self.ip >= self.n {
            return None;
        }
        let v = unsafe { *self.code.add(self.ip) };
        self.ip += 1;
        Some(v)
    }
}

pub(crate) unsafe fn prog_init_impl(p: *mut Program, code: *const c_int, n: usize) {
    (*p).code = code;
    (*p).n = n;
    (*p).ip = 0;
}

pub(crate) unsafe fn prog_fetch_impl(p: *mut Program, out: *mut c_int) -> bool {
    if (*p).ip >= (*p).n {
        return false;
    }
    *out = *(*p).code.add((*p).ip);
    (*p).ip += 1;
    true
}

// ---------------- VM ----------------
#[repr(C)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: c_int,
}

impl Default for VM {
    fn default() -> Self {
        VM {
            stack: IntVec::default(),
            trace: IntVec::default(),
            steps: 0,
        }
    }
}

impl VM {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn trace(&mut self, t: c_int) {
        self.trace.push(t);
    }
}

pub(crate) unsafe fn vm_init_impl(vm: *mut VM) {
    iv_init_impl(&mut (*vm).stack as *mut _);
    iv_init_impl(&mut (*vm).trace as *mut _);
    (*vm).steps = 0;
}

pub(crate) unsafe fn vm_free_impl(vm: *mut VM) {
    iv_free_impl(&mut (*vm).stack as *mut _);
    iv_free_impl(&mut (*vm).trace as *mut _);
    (*vm).steps = 0;
}

pub(crate) unsafe fn vm_trace_impl(vm: *mut VM, t: c_int) {
    iv_push_impl(&mut (*vm).trace as *mut _, t);
}

/// Rust-side helper that writes the same string a C `vm_print` would print to a
/// `FILE*`. Used by the binary's stdout path. The C-compatible exported
/// `vm_print` lives in `lib.rs` and uses `fprintf` directly so callers can
/// pass a `FILE*`.
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
        let t = unsafe { *vm.trace.data.add(i) };
        // C: alphabet[(trace_data[i]) & 25]
        let idx = (t & 25) as u32 as usize;
        let ch = alphabet[idx % 26];
        fp.write_all(&[ch]).unwrap();
    }
    fp.write_all(b"\n").unwrap();
}
