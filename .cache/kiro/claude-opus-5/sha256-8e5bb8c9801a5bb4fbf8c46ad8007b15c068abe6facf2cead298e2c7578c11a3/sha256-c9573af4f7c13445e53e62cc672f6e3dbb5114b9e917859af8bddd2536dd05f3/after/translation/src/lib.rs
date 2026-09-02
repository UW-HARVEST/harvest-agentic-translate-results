// Rust translation of c_src/src/driver.c
//
// Original C library: Copyright 2025 MIT Lincoln Laboratory (MIT-style license,
// see c_src/src/driver.c for the full notice).
//
// The C library exports exactly four public symbols:
//     printIntPtrLine, bad, good, driver
// All four are reproduced here with the same linker names and signatures.
// driver.h contains no namespace-renaming macros, so the source-level names are
// also the final linker names.
//
// ---------------------------------------------------------------------------
// Why this translation is written as naked assembly on x86_64
// ---------------------------------------------------------------------------
// This library is a deliberate defect test case (CWE-457/CWE-824): `bad()`
// dereferences an *uninitialized* automatic pointer. The value it prints is
// therefore whatever stale bytes happen to occupy one particular stack slot.
// The observable output is not a function of the inputs at all -- it is a
// function of the exact frame layout of every function on the call path and of
// the stack residue left by whatever ran earlier in the process.
//
// Because the task requires replicating the C behaviour rather than fixing it,
// a source-level translation is not sufficient: any Rust that LLVM is free to
// optimise will build different frames (and, for `driver`, will tail-call
// instead of `call`), which makes `bad()` read a *different* stack slot and
// print a different number. See the divergence recorded in CONFIGS.md.
//
// The four functions below are therefore emitted as naked functions carrying
// instruction-for-instruction the same code GCC emits at `-O0` for
// c_src/src/driver.c, including the frame sizes, the redundant
// store/reload pairs, the `mov eax, 0` before each variadic/void call, the
// trailing `nop`, and `leave; ret`. That makes the frame geometry -- and hence
// the slot `bad()` reads and the residue every call leaves behind -- identical
// to the C library's.
//
// Note on the residual, unavoidable difference: the C `.so` is lazily bound
// while a Rust `cdylib` is linked `-z now` by default. The first call through a
// PLT slot in the C library runs `_dl_runtime_resolve`, which scribbles over the
// stack below the current frame and is what makes the C `driver(0)` print a
// leaked stack address instead of small residue. `.cargo/config.toml` passes
// `-z lazy` so that this library is lazily bound too, but the resolver's own
// stack footprint depends on the relocation set of the object being resolved and
// cannot be made bit-identical from the Rust side. `driver(0)` / `bad()` output
// is unspecified in the C library as well (it changes on every run under ASLR),
// so it is compared structurally rather than byte-wise -- see
// tests/differential.rs.

#![allow(non_snake_case)]

use core::ffi::c_int;
#[cfg(not(target_arch = "x86_64"))]
use core::mem::MaybeUninit;

#[cfg(not(target_arch = "x86_64"))]
use core::ffi::c_char;

#[cfg(not(target_arch = "x86_64"))]
unsafe extern "C" {
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// The C source's only string literal: `"%d\n"`, NUL terminated.
///
/// GCC places it in `.rodata`; `#[used]` plus `#[unsafe(link_section)]` keeps it
/// alive and in the same section so the RIP-relative `lea` in
/// `printIntPtrLine` resolves to it.
#[cfg(target_arch = "x86_64")]
#[used]
#[unsafe(link_section = ".rodata.fmt_d_nl")]
static FMT_D_NL: [u8; 4] = *b"%d\n\0";

#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    /// Referenced only from naked assembly, so the symbol has to be declared
    /// here for the linker to emit the PLT entry / relocation.
    #[link_name = "printf"]
    unsafe fn c_printf_sym();
}

// ---------------------------------------------------------------------------
// printIntPtrLine
// ---------------------------------------------------------------------------
/// C:
///
/// ```c
/// void printIntPtrLine(const int *intNumber)
/// {
///     printf("%d\n", *intNumber);
/// }
/// ```
///
/// GCC `-O0` codegen (spills the argument to `[rbp-8]` and reloads it):
///
/// ```text
/// push rbp; mov rbp,rsp; sub rsp,0x10
/// mov [rbp-8],rdi; mov rax,[rbp-8]; mov eax,[rax]; mov esi,eax
/// lea rax,[rip+fmt]; mov rdi,rax; mov eax,0; call printf@plt
/// nop; leave; ret
/// ```
///
/// A null or otherwise invalid `intNumber` faults on the `mov eax,[rax]`
/// exactly as the C does; there is no validation in the original.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov qword ptr [rbp - 8], rdi",
        "mov rax, qword ptr [rbp - 8]",
        "mov eax, dword ptr [rax]",
        "mov esi, eax",
        "lea rax, [rip + {fmt}]",
        "mov rdi, rax",
        "mov eax, 0",
        "call {printf}",
        "nop",
        "leave",
        "ret",
        fmt = sym FMT_D_NL,
        printf = sym c_printf_sym,
    );
}

/// Portable fallback for non-x86_64 targets.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    const FMT: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];
    unsafe {
        c_printf(FMT.as_ptr(), *intNumber);
    }
}

// ---------------------------------------------------------------------------
// bad
// ---------------------------------------------------------------------------
/// C:
///
/// ```c
/// void bad()
/// {
///     int *data;
///     printIntPtrLine(data);
/// }
/// ```
///
/// The intentional defect: `data` is never initialised, and `printIntPtrLine`
/// dereferences it. GCC `-O0` allocates `data` at `[rbp-8]` and loads it without
/// ever storing to it, so the pointer passed on is whatever stale value that
/// slot holds.
///
/// ```text
/// push rbp; mov rbp,rsp; sub rsp,0x10
/// mov rax,[rbp-8]; mov rdi,rax; call printIntPtrLine@plt
/// nop; leave; ret
/// ```
///
/// The printed value is unspecified in both implementations; whether the call
/// prints garbage or faults depends on the stale value.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn bad() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_int_ptr_line}",
        "nop",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
}

/// Portable fallback for non-x86_64 targets: read the uninitialised stack slot
/// through a volatile load so the optimiser cannot exploit the `undef` value and
/// delete the call.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data_slot: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data: *const c_int = unsafe { core::ptr::read_volatile(data_slot.as_ptr()) };
    unsafe {
        printIntPtrLine(data);
    }
}

// ---------------------------------------------------------------------------
// good
// ---------------------------------------------------------------------------
/// C:
///
/// ```c
/// void good()
/// {
///     int data;
///     data = 5;
///     int *data_addr;
///     data_addr = &data;
///     printIntPtrLine(data_addr);
/// }
/// ```
///
/// GCC `-O0` puts `data` at `[rbp-12]` and `data_addr` at `[rbp-8]`:
///
/// ```text
/// push rbp; mov rbp,rsp; sub rsp,0x10
/// mov dword [rbp-12],5; lea rax,[rbp-12]; mov [rbp-8],rax
/// mov rax,[rbp-8]; mov rdi,rax; call printIntPtrLine@plt
/// nop; leave; ret
/// ```
///
/// Always prints `5\n`. The stores matter beyond that: they are the residue a
/// following `bad()` may read, so they are reproduced verbatim.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn good() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 12], 5",
        "lea rax, [rbp - 12]",
        "mov qword ptr [rbp - 8], rax",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_int_ptr_line}",
        "nop",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
}

/// Portable fallback for non-x86_64 targets.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let data: c_int = 5;
    let data_addr: *const c_int = &raw const data;
    unsafe {
        printIntPtrLine(data_addr);
    }
}

// ---------------------------------------------------------------------------
// driver
// ---------------------------------------------------------------------------
/// C:
///
/// ```c
/// void driver(int useGood)
/// {
///     if (useGood) { good(); } else { bad(); }
/// }
/// ```
///
/// GCC `-O0` spills `useGood` to `[rbp-4]`, compares it against 0, and issues a
/// real `call` (not a tail jump) to `good`/`bad`:
///
/// ```text
/// push rbp; mov rbp,rsp; sub rsp,0x10
/// mov [rbp-4],edi; cmp dword [rbp-4],0; je .Lelse
/// mov eax,0; call good@plt; jmp .Lend
/// .Lelse: mov eax,0; call bad@plt
/// .Lend: nop; leave; ret
/// ```
///
/// The frame and the `call` are load-bearing: they push `bad`'s frame 32 bytes
/// further down the stack than a direct `bad()` call would, which changes which
/// stale slot `bad` reads. An optimised Rust `if` compiles to a tail jump and
/// reads the wrong slot, so the C codegen is reproduced literally.
///
/// Any `int` is accepted; every non-zero value (including negatives and
/// `INT_MIN`) takes the `good` branch, exactly as C's truthiness rule requires.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 4], edi",
        "cmp dword ptr [rbp - 4], 0",
        "je 2f",
        "mov eax, 0",
        "call {good}",
        "jmp 3f",
        "2:",
        "mov eax, 0",
        "call {bad}",
        "3:",
        "nop",
        "leave",
        "ret",
        good = sym good,
        bad = sym bad,
    );
}

/// Portable fallback for non-x86_64 targets.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    unsafe {
        if useGood != 0 {
            good();
        } else {
            bad();
        }
    }
}
