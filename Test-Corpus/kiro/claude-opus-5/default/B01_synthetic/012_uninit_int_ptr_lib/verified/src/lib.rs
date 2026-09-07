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
// Why the x86-64 implementations are naked assembly
// ---------------------------------------------------------------------------
// This library's `bad()` is an intentional defect: it dereferences an
// uninitialized automatic pointer. The value it prints is therefore a function
// of the *exact stack layout* of the whole call chain, which makes the frame
// geometry of every function here observable behaviour rather than an
// implementation detail:
//
//   * `bad()` reads the 8 bytes at `frame_base - 8`, i.e. `entry_rsp - 16`.
//     Which byte range that is depends on the caller's frame size.
//   * `good()` and `printIntPtrLine()` write into their frames, so they decide
//     what stale bytes a *later* `bad()` will find.
//
// An idiomatic, optimized Rust translation diverges observably. Measured against
// the C `.so` through `dlopen`:
//
//   * optimized `driver` tail-jumps (`jmp bad`) instead of building a frame and
//     issuing `call bad`. `bad`'s uninitialized read then lands 24 bytes higher
//     than in C. Observed: C printed stale data and exited 0, Rust took SIGSEGV.
//   * optimized `good`/`printIntPtrLine` are inlined into a bare `printf` tail
//     call and never establish a frame, so they leave different residue behind.
//
// So on x86-64 each function is emitted as a naked function reproducing the
// instruction sequence GCC generates at `-O0` for `c_src/src/driver.c`
// (`objdump -d c_src/build/libdriver.so`): same prologue, same frame size, same
// spill offsets, same real (non-tail) calls, same epilogue. That makes the stack
// geometry identical, which is what makes the output identical.
//
// Non-x86-64 targets fall back to portable Rust; the frame layout of the C
// compiler cannot be replicated architecture-independently, and on those targets
// the uninitialized read is simply undefined in both languages.

// ---------------------------------------------------------------------------
// Why the crate is `no_std`
// ---------------------------------------------------------------------------
// The C library links only `libc.so.6` and exports a 43-entry `.dynsym`. Linking
// the Rust standard library instead produced three `NEEDED` entries
// (`libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`) and a 1003-entry
// `.dynsym`. That difference is observable here: under the loader's *default*
// lazy PLT binding, the first `call bad@plt` inside `driver` detours through
// `_dl_runtime_resolve`, whose stack consumption scales with the symbol-lookup
// work it has to do — and the residue it leaves below the caller's frame is
// exactly what `bad()` then reads. Measured over 30 lazily-bound runs of
// `driver(0)` against the std-linked build: the C library never faulted, while
// the std-linked Rust library faulted every single time. (That fault is sensitive
// to the whole link configuration, not to `std` alone — adding an explicit
// `--no-as-needed -lc` to a std build happened to make it stop faulting again.
// The point is that the resolver's footprint is only reliably matched by matching
// the C library's dependency and symbol-table profile, which is what `no_std`
// plus the flags in `build.rs` achieves: 1 `NEEDED` entry and 44 `.dynsym`
// entries against the C library's 1 and 43.)
//
// Nothing here needs an allocator, unwinding, or any other std facility: the four
// functions are naked assembly plus a single `printf` import.
#![no_std]
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};
#[cfg(not(target_arch = "x86_64"))]
use core::mem::MaybeUninit;

/// Required by `no_std`. Unreachable: on x86-64 all four exported functions are
/// naked assembly, and the portable fallbacks contain no panicking operation.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

unsafe extern "C" {
    /// Use the C library's own `printf` so that output formatting, stream
    /// buffering and interleaving with any other libc output are byte-identical
    /// to the original.
    unsafe fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string literal `"%d\n"` including its NUL terminator, matching the C
/// source exactly. `#[used]` keeps it in `.rodata` so the `lea` in the naked
/// bodies below has something to point at.
#[used]
static FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// C (`driver.c:28`):
///
///     void printIntPtrLine(const int *intNumber)
///     {
///         printf("%d\n", *intNumber);
///     }
///
/// GCC `-O0` codegen, reproduced instruction for instruction:
///
///     push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
///     mov %rdi,-0x8(%rbp); mov -0x8(%rbp),%rax; mov (%rax),%eax
///     mov %eax,%esi; lea FMT(%rip),%rax; mov %rax,%rdi; mov $0x0,%eax
///     call printf@plt; nop; leave; ret
///
/// The `mov (%rax),%eax` is a plain 32-bit load, so a misaligned `intNumber` is
/// read without faulting on x86-64, and a null or unmapped `intNumber` faults —
/// there is no check, exactly as in the C.
///
/// # Safety
///
/// `intNumber` must point to a readable `int`. The C imposes no requirement
/// beyond readability — it is not required to be aligned, and it is not checked
/// for null — so neither does this. Passing null or an unmapped address faults,
/// exactly as the C does.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov [rbp - 8], rdi",
        "mov rax, [rbp - 8]",
        "mov eax, [rax]",
        "mov esi, eax",
        "lea rax, [rip + {fmt}]",
        "mov rdi, rax",
        "mov eax, 0",
        "call {printf}",
        "nop",
        "leave",
        "ret",
        fmt = sym FMT_D_NL,
        printf = sym printf,
    );
}

/// C (`driver.c:33`):
///
///     void bad()
///     {
///         int *data;
///         printIntPtrLine(data);
///     }
///
/// This is the intentional defect of the original test case: `data` is an
/// uninitialized automatic pointer that is then dereferenced. The bug is
/// preserved rather than fixed, as required.
///
/// GCC `-O0` codegen:
///
///     push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
///     mov -0x8(%rbp),%rax; mov %rax,%rdi; call printIntPtrLine@plt
///     nop; leave; ret
///
/// The read of `-0x8(%rbp)` is the uninitialized load. Reproducing the defect
/// faithfully means reproducing *where* it reads: `entry_rsp - 16`. Idiomatic
/// Rust (`MaybeUninit` + `read_volatile`) reads the red-zone slot at
/// `entry_rsp - 8` instead, which holds different bytes.
///
/// The value printed is garbage in both implementations and, for an uncontrolled
/// stack, differs from run to run even between two runs of the original C
/// library.
///
/// # Safety
///
/// Always unsound by construction: this function dereferences an uninitialized
/// pointer. It is exported only because the C library exports it. There is no
/// way to call it safely; the caller inherits whatever the stack happens to
/// contain, which may be a fatal fault.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn bad() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, [rbp - 8]",
        "mov rdi, rax",
        "call {print_int_ptr_line}",
        "nop",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
}

/// C (`driver.c:39`):
///
///     void good()
///     {
///         int data;
///         data = 5;
///         int *data_addr;
///         data_addr = &data;
///         printIntPtrLine(data_addr);
///     }
///
/// GCC `-O0` codegen:
///
///     push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
///     movl $0x5,-0xc(%rbp); lea -0xc(%rbp),%rax; mov %rax,-0x8(%rbp)
///     mov -0x8(%rbp),%rax; mov %rax,%rdi; call printIntPtrLine@plt
///     nop; leave; ret
///
/// `data` lives at `-0xc(%rbp)` and the pointer spill at `-0x8(%rbp)`; both
/// writes are part of the stack residue a later `bad()` can observe, so they are
/// reproduced at the same offsets rather than optimized away.
///
/// # Safety
///
/// Safe to call from anywhere. It is `unsafe` only to match the C ABI surface and
/// because it calls `printIntPtrLine`, whose contract it satisfies internally.
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
        "mov [rbp - 8], rax",
        "mov rax, [rbp - 8]",
        "mov rdi, rax",
        "call {print_int_ptr_line}",
        "nop",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
}

/// C (`driver.c:48`):
///
///     void driver(int useGood)
///     {
///         if (useGood) { good(); } else { bad(); }
///     }
///
/// GCC `-O0` codegen:
///
///     push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
///     mov %edi,-0x4(%rbp); cmpl $0x0,-0x4(%rbp); je .Lbad
///     mov $0x0,%eax; call good@plt; jmp .Ldone
///     .Lbad: mov $0x0,%eax; call bad@plt
///     .Ldone: nop; leave; ret
///
/// Two details that are observable and therefore reproduced exactly:
///
///   * the argument is spilled and tested as a **32-bit** value (`mov %edi,...`
///     then `cmpl $0x0,...`), so a 64-bit caller value such as `0x1_0000_0000`
///     has its high half discarded and takes the `bad()` arm;
///   * `good`/`bad` are reached by a real `call` from inside a 16-byte frame,
///     not by a tail jump. This is what places `bad()`'s uninitialized read 32
///     bytes lower than a tail-called `bad()` would.
///
/// # Safety
///
/// `useGood` may be any `int`. A non-zero value routes to `good()` and is safe;
/// zero routes to `bad()` and therefore inherits `bad()`'s unsoundness.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov [rbp - 4], edi",
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

// ---------------------------------------------------------------------------
// Portable fallbacks for non-x86-64 targets.
// ---------------------------------------------------------------------------

///
/// # Safety
///
/// `intNumber` must point to a readable `int`. The C imposes no requirement
/// beyond readability — it is not required to be aligned, and it is not checked
/// for null — so neither does this. Passing null or an unmapped address faults,
/// exactly as the C does.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    unsafe {
        printf(FMT_D_NL.as_ptr(), core::ptr::read_unaligned(intNumber));
    }
}

/// Read an uninitialized stack slot through a volatile load so the optimizer
/// cannot exploit the `undef` value and delete the call.
///
/// # Safety
///
/// Always unsound by construction: this function dereferences an uninitialized
/// pointer. It is exported only because the C library exports it. There is no
/// way to call it safely; the caller inherits whatever the stack happens to
/// contain, which may be a fatal fault.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data_slot: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data: *const c_int = unsafe { core::ptr::read_volatile(data_slot.as_ptr()) };
    unsafe {
        printIntPtrLine(data);
    }
}

///
/// # Safety
///
/// Safe to call from anywhere. It is `unsafe` only to match the C ABI surface and
/// because it calls `printIntPtrLine`, whose contract it satisfies internally.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // C declares `int data;` then assigns 5 on the next statement; the observable
    // result is identical to initializing directly.
    let data: c_int = 5;
    let data_addr: *const c_int = &raw const data;
    unsafe {
        printIntPtrLine(data_addr);
    }
}

///
/// # Safety
///
/// `useGood` may be any `int`. A non-zero value routes to `good()` and is safe;
/// zero routes to `bad()` and therefore inherits `bad()`'s unsoundness.
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
