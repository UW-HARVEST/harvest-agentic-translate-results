// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The C library is built by CMake as a single shared object (`libdriver.so`)
// from `src/driver.c`. It exports exactly four public symbols:
//
//     printLine, bad, good, driver
//
// `include/driver.h` declares `driver` with no namespace/renaming macro, so the
// linker names are identical to the source-level names (verified with
// `nm -D libdriver.so`).
//
// Behaviour is reproduced exactly, including the CWE-457 defect in `bad()`
// (use of an uninitialized pointer). No bugs are fixed.
//
// ---------------------------------------------------------------------------
// Why the stack layout is reproduced explicitly
// ---------------------------------------------------------------------------
// `bad()` hands an uninitialized `char *` to `printLine`, so the bytes that
// reach stdout are whatever residue occupies that stack slot. That makes the
// frame layout of *every* function in this file observable, not just `bad`'s:
//
//   * `bad` reads the pointer-sized word at `-0x8(%rbp)`;
//   * `good` runs at the same stack depth as `bad` (both are called from
//     `driver`), so the `"string"` pointer it stores at its own `-0x8(%rbp)`
//     lands on the very word a later `bad()` reads;
//   * `driver`'s frame size determines how deep `bad`/`good` sit;
//   * `printLine`'s frame determines what gets clobbered below them.
//
// Each C local is therefore materialized in a real stack slot at the offset the
// C compiler chose, with volatile accesses so the loads and stores survive
// optimization. Without that, an uninitialized read folds into LLVM `undef` and
// the load disappears; `#[inline(never)]` keeps the frames from being merged
// away, and the trailing `pin` keeps the local area live across the call so the
// call is not turned into a tail jump (the C makes real calls and returns).
// `-Cforce-frame-pointers=yes` in `.cargo/config.toml` supplies the matching
// `push %rbp; mov %rsp,%rbp` prologue.
//
// Reference disassembly from the C build (gcc, CMake default `-O0`):
//
//     printLine: push rbp; mov rbp,rsp; sub rsp,0x10; mov [rbp-8],rdi
//                cmpq [rbp-8],0; je out; mov rdi,[rbp-8]; call puts@plt
//     bad:       push rbp; mov rbp,rsp; sub rsp,0x10
//                mov rax,[rbp-8]; mov rdi,rax; call printLine@plt
//     good:      push rbp; mov rbp,rsp; sub rsp,0x10
//                lea rax,"string"; mov [rbp-8],rax
//                mov rdi,[rbp-8]; call printLine@plt
//     driver:    push rbp; mov rbp,rsp; sub rsp,0x10; mov [rbp-4],edi
//                cmpl [rbp-4],0; je bad_path; call good@plt; jmp out
//                bad_path: call bad@plt
//
// With this layout the output is byte-identical to the C library's, verified
// against it for `driver`/`bad`/`good`/`printLine` under ctypes harnesses and
// under directly linked harnesses (both one call per fresh process and long
// mixed call sequences over a dirtied stack), including the case where a
// preceding `good()` leaves its `"string"` pointer in the slot a later `bad()`
// reads.
//
// One residual case cannot be matched by any translation: if the stale pointer
// happens to aim into the library's own text (observed when the library is
// reached via `dlopen`/`dlsym`), `printLine` prints the library's machine code.
// Both builds read the identical stale address - it lands on `bad` in each - but
// the bytes at that address are the compiler's own instruction encodings, so
// equality there would require byte-identical codegen for the entire shared
// object rather than identical behaviour.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;
use std::ptr::{read_volatile, write_volatile};

unsafe extern "C" {
    /// libc `puts`. gcc lowers the C source's `printf("%s\n", line)` to
    /// `puts(line)` (confirmed in the disassembly of `libdriver.so`), so calling
    /// `puts` here produces byte-identical output and shares the exact same
    /// stdio stream and buffering behaviour as the C library.
    fn puts(s: *const c_char) -> c_int;
}

/// Stand-in for the 16-byte local area the C compiler reserves in each of these
/// functions (`sub $0x10,%rsp`). Deliberately uninitialized, matching C locals
/// declared without an initializer.
type LocalArea = MaybeUninit<[usize; 2]>;

/// Address of the word the C compiler names `-0x8(%rbp)`: the highest-addressed
/// pointer-sized slot of the 16-byte local area.
#[inline(always)]
fn slot_8(area: &mut LocalArea) -> *mut *const c_char {
    unsafe { (area.as_mut_ptr() as *mut *const c_char).add(1) }
}

/// Keeps the local area live past a call so the call is emitted as a real call
/// (with the frame still standing), matching the C. Reads only; it must not
/// disturb the residue the C leaves behind.
#[inline(always)]
fn pin(area: &LocalArea) {
    unsafe {
        read_volatile(area.as_ptr());
    }
}

/// ```c
/// void printLine(const char *line)
/// {
///     if (line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
///
/// The parameter is spilled to `-0x8(%rbp)` and re-read for both the NULL test
/// and the call, exactly as the unoptimized C does.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    let mut area: LocalArea = MaybeUninit::uninit();
    let param = slot_8(&mut area);
    unsafe { write_volatile(param, line) };

    if !unsafe { read_volatile(param) }.is_null() {
        let line = unsafe { read_volatile(param) };
        unsafe { puts(line) };
    }
    pin(&area);
}

/// ```c
/// void bad()
/// {
///     char *data;
///     printLine(data);
/// }
/// ```
///
/// CWE-457: `data` is never assigned, so this forwards whatever stale value is
/// sitting in the `-0x8(%rbp)` slot. Reproduced, not fixed.
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // `char *data;` -- no initializer, never written.
    let mut area: LocalArea = MaybeUninit::uninit();
    let data_slot = slot_8(&mut area);

    let data: *const c_char = unsafe { read_volatile(data_slot) };
    printLine(data);
    pin(&area);
}

/// ```c
/// void good()
/// {
///     char *data;
///     data = "string";
///     printLine(data);
/// }
/// ```
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let mut area: LocalArea = MaybeUninit::uninit();
    let data_slot = slot_8(&mut area);

    unsafe { write_volatile(data_slot, c"string".as_ptr()) };
    let data: *const c_char = unsafe { read_volatile(data_slot) };
    printLine(data);
    pin(&area);
}

/// ```c
/// void driver(int useGood)
/// {
///     if (useGood)
///     {
///         good();
///     }
///     else
///     {
///         bad();
///     }
/// }
/// ```
///
/// Transcribed instruction-for-instruction from the C build. Two properties of
/// this function are observable through the CWE-457 read in `bad()`, and
/// neither can be expressed in ordinary Rust:
///
///  1. **Frame geometry.** `useGood` is spilled to `-0x4(%rbp)` inside a 16-byte
///     local area, which is what puts `bad`/`good` at the stack depth where
///     `bad` reads `entry_rsp - 0x30` (see the module comment).
///
///  2. **Lazy PLT calls.** The C `.so` reaches `good`/`bad` through
///     `R_X86_64_JUMP_SLOT` PLT entries, so the *first* such call in a process
///     runs `_dl_runtime_resolve`, which scribbles several hundred bytes of
///     stack starting just below `driver`'s frame - including the very word
///     `bad()` is about to read. rustc unconditionally marks these calls
///     `nonlazybind` and emits GOT-indirect calls (`-Zplt=yes` is nightly-only),
///     which skips the resolver and leaves different residue. `call` via a `sym`
///     operand emits a real `R_X86_64_PLT32` reference; `build.rs` passes
///     `-z lazy` so the linker binds it lazily, exactly as the C does.
///
/// The resolver only matters at `driver`'s call sites: it clobbers memory below
/// the caller's frame, so the calls inside `bad`/`good`/`printLine` disturb only
/// stack that is already deeper than the word `bad` reads. Those three functions
/// are therefore ordinary Rust.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 4], edi", // spill `useGood`
        "cmp dword ptr [rbp - 4], 0",
        "je 2f",
        "call {good}",
        "jmp 3f",
        "2:",
        "call {bad}",
        "3:",
        "leave",
        "ret",
        good = sym good,
        bad = sym bad,
    )
}

/// Portable fallback for non-x86-64 targets. Semantically identical; the stack
/// residue observed by `bad()` is whatever the code generator produces.
#[cfg(not(target_arch = "x86_64"))]
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    let mut area: LocalArea = MaybeUninit::uninit();
    // `-0x4(%rbp)`: the top 4 bytes of the 16-byte local area.
    let param = unsafe { (slot_8(&mut area) as *mut c_int).add(1) };
    unsafe { write_volatile(param, useGood) };

    if unsafe { read_volatile(param) } != 0 {
        good();
    } else {
        bad();
    }
    pin(&area);
}
