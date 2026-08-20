//! Memory access abstraction plus an emulation of the C `main` stack frame that
//! holds `input_buffer` and `ref_buffer`.
//!
//! The C code deliberately performs `strcmp`/`strlen` on buffers that are not
//! guaranteed to be NUL terminated, so its behaviour depends on the bytes that
//! happen to live in the uninitialised parts of the stack frame (and even past
//! the end of the two arrays).  The frame of `c_src/src/main.c` compiled by GCC
//! looks like this (addresses increasing downwards):
//!
//! ```text
//!   rbp-0x838  unsigned int byte      (reference loop scratch)
//!   rbp-0x834  unsigned int byte      (input loop scratch)
//!   rbp-0x830  char ref_buffer[1024]      <-- modelled offset    0
//!   rbp-0x430  char input_buffer[1024]    <-- modelled offset 1024
//!   rbp-0x30   size_t ref_len             <-- modelled offset 2048
//!   rbp-0x28   size_t input_len           <-- modelled offset 2056
//!   rbp-0x20   (4 bytes padding)          <-- modelled offset 2064
//!   rbp-0x1c   uint32_t flags             <-- modelled offset 2068
//!   rbp-0x18   int operation              <-- modelled offset 2072
//!   rbp-0x14   int result (uninitialised) <-- modelled offset 2076
//!   rbp-0x10   size_t i (reference loop)  <-- modelled offset 2080
//!   rbp-0x8    size_t i (input loop)      <-- modelled offset 2088
//!   rbp        saved rbp                  <-- modelled offset 2096
//!   rbp+0x8    return address             <-- modelled offset 2104
//! ```
//!
//! The uninitialised bytes are taken from [`crate::frame_junk::FRAME_JUNK`],
//! a snapshot of the real frame captured from the compiled C program, so that
//! reads which run past the data read from stdin observe the same bytes the C
//! program observes.
//!
//! Reads that run past the modelled region correspond to the C program walking
//! off the top of its stack, which terminates the process with SIGSEGV.

use crate::frame_junk::FRAME_JUNK;

/// Offset of `ref_buffer` inside the modelled frame.
pub const REF_OFF: usize = 0;
/// Offset of `input_buffer` inside the modelled frame.
pub const INPUT_OFF: usize = 1024;
/// A "pointer" value that can never be a valid buffer (stands in for NULL).
pub const NULL_OFF: usize = usize::MAX;

/// Size of each of the two `char[1024]` arrays.
const BUF_SIZE: usize = 1024;
/// Offset of the locals that follow the two arrays (`rbp-0x30`).
const LOCALS_OFF: usize = 2 * BUF_SIZE;

/// Byte addressable memory as seen by the translated library code.
///
/// The translated code manipulates `char *` values as plain integers, so a
/// single trait can serve both the emulated `main` frame used by the driver
/// binary and real pointers handed to the exported C ABI entry point.
pub trait MemAccess {
    /// Read one byte at `addr`.
    fn get(&self, addr: usize) -> u8;
    /// Is `addr` the NULL pointer?
    fn is_null(&self, addr: usize) -> bool;
}

/// The emulated `main` stack frame of `c_src/src/main.c`.
pub struct Mem {
    bytes: Vec<u8>,
}

impl Mem {
    /// Build the frame: both arrays uninitialised (junk), followed by the
    /// remaining locals of `main` at the offsets the compiler assigns them.
    pub fn new(operation: i32, flags: u32, input_len: u64, ref_len: u64) -> Mem {
        let mut bytes: Vec<u8> = FRAME_JUNK.to_vec();

        // Above `input_buffer` (in address order) live the scalar locals of
        // `main`; `result` keeps its uninitialised value because it is only
        // written after `process_strings` returns, and both loop counters hold
        // the length they finished at.
        let base = LOCALS_OFF;
        bytes[base..base + 8].copy_from_slice(&ref_len.to_le_bytes());
        bytes[base + 8..base + 16].copy_from_slice(&input_len.to_le_bytes());
        bytes[base + 20..base + 24].copy_from_slice(&flags.to_le_bytes());
        bytes[base + 24..base + 28].copy_from_slice(&operation.to_le_bytes());
        bytes[base + 32..base + 40].copy_from_slice(&ref_len.to_le_bytes());
        bytes[base + 40..base + 48].copy_from_slice(&input_len.to_le_bytes());

        Mem { bytes }
    }

    /// Store one byte (`input_buffer[i] = ...` / `ref_buffer[i] = ...`).
    pub fn set(&mut self, idx: usize, value: u8) {
        match self.bytes.get_mut(idx) {
            Some(slot) => *slot = value,
            None => segfault(),
        }
    }
}

impl MemAccess for Mem {
    /// Read one byte; running off the modelled stack aborts like the C program.
    fn get(&self, idx: usize) -> u8 {
        match self.bytes.get(idx) {
            Some(&b) => b,
            None => segfault(),
        }
    }

    fn is_null(&self, idx: usize) -> bool {
        idx == NULL_OFF
    }
}

/// Real process memory: `char *` values are the actual pointers passed across
/// the C ABI boundary.  Used by the exported [`crate::ffi`] entry point so that
/// an external caller observes exactly what the C implementation does.
pub struct RawMem;

impl MemAccess for RawMem {
    fn get(&self, addr: usize) -> u8 {
        // SAFETY: reproduces the C code's unchecked read; the caller is
        // responsible for the same preconditions the C function requires.
        unsafe { std::ptr::read_volatile(addr as *const u8) }
    }

    fn is_null(&self, addr: usize) -> bool {
        addr == 0
    }
}

/// The C program dies with SIGSEGV when one of its unbounded string functions
/// walks off the end of the stack.  Reproduce the same fatal signal so that the
/// observable behaviour (no output on stdout, killed by SIGSEGV) is identical.
pub fn segfault() -> ! {
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 0u8);
    }
    std::process::abort();
}
