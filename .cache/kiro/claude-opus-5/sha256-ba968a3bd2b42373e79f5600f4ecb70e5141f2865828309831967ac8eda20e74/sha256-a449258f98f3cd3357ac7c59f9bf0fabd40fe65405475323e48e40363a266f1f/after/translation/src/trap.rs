//! Reproduction of the fatal signals the C driver can die from.
//!
//! `stb_perlin_noise3_wrap_nonpow2` performs unchecked table indexing and an
//! unchecked `%`, so for some inputs the C program never reaches its `printf`
//! and is killed by the kernel instead. The observable result is an empty
//! stdout, an empty stderr and a wait status of "terminated by signal N"
//! (`128 + N` as a shell reports it), which a Rust process can only match by
//! actually taking the same signal — an `exit(139)` is a *different* wait
//! status from death by `SIGSEGV`.
//!
//! Both helpers therefore provoke the real hardware fault rather than
//! synthesising an exit code.

/// Die exactly the way the C program does when it dereferences an unmapped
/// address: `SIGSEGV`, default disposition, core dumped.
#[cold]
#[inline(never)]
pub fn sigsegv() -> ! {
    // `black_box` hides the constant from the optimiser, so this stays a real
    // load from an unmapped page instead of being folded into an `ud2` on the
    // grounds that dereferencing a known-invalid pointer is UB.
    let addr = std::hint::black_box(8usize) as *const u8;
    unsafe {
        std::ptr::read_volatile(addr);
    }
    // Not reached. If some future target somehow tolerates the load, fall back
    // to a fatal signal rather than continuing with a bogus value.
    std::process::abort()
}

/// Die the way the C program does on `INT_MIN % -1`: the `idiv` instruction
/// raises `#DE`, which Linux delivers as `SIGFPE`.
#[cold]
#[inline(never)]
pub fn sigfpe() -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let divisor = std::hint::black_box(-1i32);
        std::arch::asm!(
            "cdq",
            "idiv {d:e}",
            d = in(reg) divisor,
            inout("eax") std::hint::black_box(i32::MIN) => _,
            out("edx") _,
            options(nostack),
        );
    }
    std::process::abort()
}
