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
// Why the stack layout is part of the observable behaviour
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
// A translation written in ordinary Rust cannot pin any of this: the frame sizes
// it gets are whatever the code generator picks, and they change with the
// optimization level. Measured on this crate, a `MaybeUninit`-slot version
// reproduces the C exactly at `--release` but not in the `dev` profile, where
// `printLine`/`bad`/`good` grow from `sub $0x10,%rsp` to `0x90`/`0x70`/`0x80`
// and the slot `bad` reads moves by ten words — a different string comes out.
//
// So all four functions are transcribed as `#[naked]` functions,
// instruction-for-instruction from the C build's disassembly. The stack geometry
// is then a property of *this source file* rather than of the profile, the
// optimizer, or the rustc version.
//
// Reference disassembly from the C build (gcc, CMake default `-O0`), which each
// `naked_asm!` block below mirrors line by line:
//
//     printLine: push rbp; mov rbp,rsp; sub rsp,0x10; mov [rbp-8],rdi
//                cmpq [rbp-8],0; je out; mov rax,[rbp-8]; mov rdi,rax
//                call puts@plt; out: nop; leave; ret
//     bad:       push rbp; mov rbp,rsp; sub rsp,0x10
//                mov rax,[rbp-8]; mov rdi,rax; call printLine@plt
//                nop; leave; ret
//     good:      push rbp; mov rbp,rsp; sub rsp,0x10
//                lea rax,"string"; mov [rbp-8],rax
//                mov rax,[rbp-8]; mov rdi,rax; call printLine@plt
//                nop; leave; ret
//     driver:    push rbp; mov rbp,rsp; sub rsp,0x10; mov [rbp-4],edi
//                cmpl [rbp-4],0; je bad_path
//                mov eax,0; call good@plt; jmp out
//                bad_path: mov eax,0; call bad@plt
//                out: nop; leave; ret
//
// (The `mov eax,0` pairs come from `good`/`bad` being declared `()` in C, i.e.
// unprototyped, so gcc zeroes `al` at the call as if they were variadic. They
// are kept because they are part of the frame's instruction stream, which is
// itself observable in the residual case described below.)
//
// ---------------------------------------------------------------------------
// Lazy PLT binding
// ---------------------------------------------------------------------------
// The C `.so` reaches `good`/`bad`/`printLine`/`puts` through
// `R_X86_64_JUMP_SLOT` PLT entries, so the *first* such call in a process runs
// `_dl_runtime_resolve`, which scribbles several hundred bytes of stack starting
// just below the caller's frame — including the very word `bad()` is about to
// read when it is reached via `driver`. rustc marks extern calls `nonlazybind`
// and its default link flags are `-z relro -z now`; `call` via a `sym` operand
// emits a real `R_X86_64_PLT32` reference and `build.rs` passes `-z lazy`, so
// the binding happens at the same moment as in the C build.
//
// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------
// `tests/differential.rs` loads this `.so` *and* the C `.so` with `libloading`
// and compares captured stdout byte-for-byte across 26 configuration rows and
// 15 error rows (see `CONFIGS.md` / `ERRORS.md`), pinning the stack residue so
// the CWE-457 read is deterministic. It also checks *which* stack word each
// library reads, by planting 64 distinct pointers in the window below the call
// and comparing the label that comes out.
//
// One residual case cannot be matched by any translation: if the stale pointer
// happens to aim into the library's own text, `printLine` prints the library's
// machine code. Both builds read the identical stale address, but the bytes at
// that address are the compiler's own instruction encodings, so equality there
// would require byte-identical codegen for the entire shared object rather than
// identical behaviour.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// libc `puts`. gcc lowers the C source's `printf("%s\n", line)` to
    /// `puts(line)` (confirmed in the disassembly of `libdriver.so`), so calling
    /// `puts` here produces byte-identical output and shares the exact same
    /// stdio stream and buffering behaviour as the C library.
    fn puts(s: *const c_char) -> c_int;
}

/// The `"string"` literal `good()` assigns. A plain `static` lands in read-only
/// data, where gcc puts the C literal.
#[cfg(target_arch = "x86_64")]
#[used]
static STRING_LITERAL: [u8; 7] = *b"string\0";

// ---------------------------------------------------------------------------
// x86-64: transcribed from the C build
// ---------------------------------------------------------------------------

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
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov qword ptr [rbp - 8], rdi", // spill `line`
        "cmp qword ptr [rbp - 8], 0",   // if (line != NULL)
        "je 2f",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
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
/// CWE-457: `data` is never assigned, so this forwards whatever stale value is
/// sitting in the `-0x8(%rbp)` slot. Reproduced, not fixed — there is no store
/// to `[rbp-8]` before the load, exactly as in the C.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, qword ptr [rbp - 8]", // uninitialized read
        "mov rdi, rax",
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
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn good() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "lea rax, [rip + {string}]",
        "mov qword ptr [rbp - 8], rax", // data = "string";
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_line}",
        "nop",
        "leave",
        "ret",
        string = sym STRING_LITERAL,
        print_line = sym printLine,
    )
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
/// `useGood` is spilled to `-0x4(%rbp)` inside a 16-byte local area and tested
/// with a full 32-bit `cmp`, so *every* non-zero bit pattern selects `good()` —
/// including ones whose low byte is zero. That frame geometry is also what puts
/// `bad`/`good` at the stack depth where `bad` reads the word the lazy PLT
/// resolver clobbers on the first call.
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
    )
}

// ---------------------------------------------------------------------------
// Portable fallback for non-x86-64 targets
// ---------------------------------------------------------------------------
//
// Semantically identical. The exact stack residue observed by `bad()` is
// whatever the code generator produces, so it is *not* claimed to be
// byte-identical to a C build for the same architecture; each C local is still
// materialized in a real stack slot with volatile accesses so the uninitialized
// read survives (without that it folds into LLVM `undef` and disappears), and
// `-Cforce-frame-pointers=yes` in `.cargo/config.toml` keeps a conventional
// prologue.

#[cfg(not(target_arch = "x86_64"))]
mod portable {
    use super::puts;
    use std::ffi::{c_char, c_int};
    use std::mem::MaybeUninit;
    use std::ptr::{read_volatile, write_volatile};

    /// Stand-in for the 16-byte local area the C compiler reserves in each of
    /// these functions (`sub $0x10,%rsp`). Deliberately uninitialized, matching
    /// C locals declared without an initializer.
    type LocalArea = MaybeUninit<[usize; 2]>;

    /// Address of the word the C compiler names `-0x8(%rbp)`: the
    /// highest-addressed pointer-sized slot of the 16-byte local area.
    #[inline(always)]
    fn slot_8(area: &mut LocalArea) -> *mut *const c_char {
        unsafe { (area.as_mut_ptr() as *mut *const c_char).add(1) }
    }

    /// Keeps the local area live past a call so the call is emitted as a real
    /// call (with the frame still standing) rather than a tail jump, matching
    /// the C. Reads only; it must not disturb the residue.
    #[inline(always)]
    fn pin(area: &LocalArea) {
        unsafe {
            read_volatile(area.as_ptr());
        }
    }

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
}
