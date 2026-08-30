//! Rust translation of `c_src/src/driver.c` (MIT Lincoln Laboratory, 2025).
//!
//! The C library exports exactly four public symbols — `printLine`, `bad`,
//! `good`, `driver` — and this crate reproduces all four with the same
//! signatures and the same observable behaviour.
//!
//! # Why parts of this are `naked` assembly
//!
//! `bad()` in the original is a deliberate CWE-457 defect:
//!
//! ```c
//! void bad() { char *data; printLine(data); }
//! ```
//!
//! `data` is **never assigned**, so the C passes an *indeterminate* 8-byte stack
//! slot to `printLine`. The value is literally "whatever the caller left on the
//! stack at that address", which means the observable behaviour of the C library
//! is a function of its own frame layout:
//!
//! * if the stale slot happens to be `NULL`, `printLine` prints nothing;
//! * if it is a readable pointer, the stale bytes are printed;
//! * otherwise the process takes `SIGSEGV`.
//!
//! Empirically (see `tests/configs.rs::cfg_c19_dirty_stack_matrix`) all three
//! outcomes really do occur for ordinary call sequences, and a "reasonable"
//! translation that substitutes a deterministic empty string diverges from the C
//! immediately: e.g. after `good(); bad();` the C prints `string`, because
//! `good`'s frame left the string literal's address in exactly the slot `bad`
//! reads; and with a dirtied caller stack the C faults where a safe translation
//! would not.
//!
//! Reproducing that byte-for-byte requires reproducing the *stack frame layout*,
//! so the four bodies below emit the same instruction sequences that
//! `gcc -O0` (the compiler configuration in `c_src/CMakeLists.txt`, which sets no
//! optimisation flags) produces for `driver.c`. Concretely this pins:
//!
//! * the frame sizes (`push rbp` + `sub rsp, 16` in every function), so the
//!   indeterminate slot sits at the same address relative to the caller;
//! * where each function spills its locals, so the *contents* a later
//!   indeterminate read observes are the same;
//! * the use of `puts` rather than `printf` — gcc rewrites
//!   `printf("%s\n", line)` into `puts(line)`, and the two library routines
//!   leave different residue on the stack;
//! * `call` through the PLT rather than a GOT-indirect call.
//!
//! Everything else about the crate is ordinary Rust. On any target where this
//! layout cannot be pinned, `cfg` fallbacks provide the plain, portable
//! translation instead (documented at each definition).

#![allow(non_snake_case)]

use std::ffi::c_char;
use std::ffi::c_int;

extern "C" {
    /// `gcc -O0` compiles `printf("%s\n", line)` in `printLine` into a `puts`
    /// call, so `puts` is what the C library actually invokes.
    fn puts(s: *const c_char) -> c_int;
}

/// The string literal `"string"` that `good()` assigns to `data`.
/// This lives in read-only data, exactly like the C literal.
#[used]
static STRING_LITERAL: [c_char; 7] = [
    b's' as c_char,
    b't' as c_char,
    b'r' as c_char,
    b'i' as c_char,
    b'n' as c_char,
    b'g' as c_char,
    0,
];

// ===========================================================================
// x86-64 / Linux: layout-exact translation
// ===========================================================================

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
/// gcc -O0 emits:
/// ```text
/// push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
/// mov  %rdi,-0x8(%rbp)          ; spill the parameter
/// cmpq $0x0,-0x8(%rbp)          ; if (line != NULL)
/// je   .Lend
/// mov  -0x8(%rbp),%rax; mov %rax,%rdi; call puts@plt
/// .Lend: nop; leave; ret
/// ```
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn printLine(_line: *const c_char) {
    core::arch::naked_asm!(
        "push rbp",
        "mov  rbp, rsp",
        "sub  rsp, 16",
        "mov  qword ptr [rbp - 8], rdi",
        "cmp  qword ptr [rbp - 8], 0",
        "je   2f",
        "mov  rax, qword ptr [rbp - 8]",
        "mov  rdi, rax",
        "call {puts}",
        "2:",
        "nop",
        "leave",
        "ret",
        puts = sym puts,
    )
}

/// ```c
/// void bad()
/// {
///     char *data;
///     printLine(data);
/// }
/// ```
///
/// `data` is never initialised. gcc -O0 allocates it at `-0x8(%rbp)` and loads
/// it without ever storing to it:
/// ```text
/// push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
/// mov  -0x8(%rbp),%rax          ; <-- indeterminate read, at entry_rsp - 16
/// mov  %rax,%rdi; call printLine@plt
/// nop; leave; ret
/// ```
///
/// Stack-alignment check: the SysV ABI gives `entry_rsp % 16 == 8`, so at the
/// inner `call` we have `rsp = entry_rsp - 24 ≡ 0 (mod 16)` — exactly as gcc.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn bad() {
    core::arch::naked_asm!(
        "push rbp",
        "mov  rbp, rsp",
        "sub  rsp, 16",
        "mov  rax, qword ptr [rbp - 8]",
        "mov  rdi, rax",
        "call {print_line}",
        "nop",
        "leave",
        "ret",
        print_line = sym printLine,
    )
}

/// ```c
/// void good()
/// {
///     char *data;
///     data = "string";
///     printLine(data);
/// }
/// ```
///
/// gcc -O0 emits:
/// ```text
/// push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
/// lea  "string"(%rip),%rax
/// mov  %rax,-0x8(%rbp)          ; data = "string"
/// mov  -0x8(%rbp),%rax; mov %rax,%rdi; call printLine@plt
/// nop; leave; ret
/// ```
///
/// The spill of `data` to `-0x8(%rbp)` matters beyond `good` itself: it is the
/// very slot a subsequent `bad()` call at the same stack depth reads, which is
/// why the C prints `string` for the sequence `good(); bad();`.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn good() {
    core::arch::naked_asm!(
        "push rbp",
        "mov  rbp, rsp",
        "sub  rsp, 16",
        "lea  rax, [rip + {lit}]",
        "mov  qword ptr [rbp - 8], rax",
        "mov  rax, qword ptr [rbp - 8]",
        "mov  rdi, rax",
        "call {print_line}",
        "nop",
        "leave",
        "ret",
        lit = sym STRING_LITERAL,
        print_line = sym printLine,
    )
}

/// ```c
/// void driver(int useGood)
/// {
///     if (useGood) { good(); } else { bad(); }
/// }
/// ```
///
/// The C tests plain truthiness, so *every* non-zero `int` — negative values,
/// `INT_MIN`, `INT_MAX`, and any value that would be an out-of-range enum —
/// selects `good()`; only exactly `0` selects `bad()`. Note also that the
/// parameter arrives in `%edi`, so a wider value is truncated to 32 bits before
/// the test.
///
/// gcc -O0 emits:
/// ```text
/// push %rbp; mov %rsp,%rbp; sub $0x10,%rsp
/// mov  %edi,-0x4(%rbp)
/// cmpl $0x0,-0x4(%rbp)
/// je   .Lbad
/// mov  $0x0,%eax; call good@plt
/// jmp  .Lend
/// .Lbad: mov $0x0,%eax; call bad@plt
/// .Lend: nop; leave; ret
/// ```
///
/// The `sub $0x10,%rsp` here is what decides where `bad`'s indeterminate slot
/// lands when it is reached through `driver`. Walking the frames: `driver` is
/// entered with `rsp = R`, `push rbp` makes it `R-8`, `sub $0x10` makes it
/// `R-24`, `call bad` pushes the return address at `R-32`, `bad`'s own
/// `push rbp` makes it `R-40`, and `bad` then loads `[rbp-8]` = **`R-48`**.
/// A differently sized frame here reads different memory and changes the
/// program's output, so the frame size is part of the observable contract.
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn driver(_useGood: c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov  rbp, rsp",
        "sub  rsp, 16",
        "mov  dword ptr [rbp - 4], edi",
        "cmp  dword ptr [rbp - 4], 0",
        "je   2f",
        "mov  eax, 0",
        "call {good}",
        "jmp  3f",
        "2:",
        "mov  eax, 0",
        "call {bad}",
        "3:",
        "nop",
        "leave",
        "ret",
        good = sym good,
        bad = sym bad,
    )
}

// ===========================================================================
// Portable fallback (non x86-64-Linux targets)
// ===========================================================================
//
// Here the exact frame layout of `gcc -O0` cannot be pinned, so the
// indeterminate read in `bad()` cannot be modelled. These definitions implement
// the C's *specified* behaviour and, for `bad()`, the most common outcome
// observed from the reference build (a non-NULL pointer to an empty string, i.e.
// a bare newline).

/// See the x86-64 definition for documentation.
#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        puts(line);
    }
}

/// See the x86-64 definition for documentation.
#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    static GARBAGE: [c_char; 1] = [0];
    printLine(GARBAGE.as_ptr());
}

/// See the x86-64 definition for documentation.
#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    printLine(STRING_LITERAL.as_ptr());
}

/// See the x86-64 definition for documentation.
#[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
