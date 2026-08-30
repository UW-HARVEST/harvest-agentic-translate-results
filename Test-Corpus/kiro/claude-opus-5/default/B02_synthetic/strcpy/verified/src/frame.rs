//! Model of `main`'s stack frame in `c_src/src/main.c`.
//!
//! Why this exists
//! ---------------
//! `main` reads `input_len` bytes into `input_buffer[1024]` and `ref_len` bytes
//! into `ref_buffer[1024]`, then hands both to `process_strings`.  Neither
//! buffer is ever NUL-terminated by the driver, and `lib.c` compares them with
//! `strlen` / `strcmp` / `strncmp`.  Whatever follows the bytes the driver wrote
//! therefore decides the result, so it has to be modelled.
//!
//! Layout
//! ------
//! Taken from the reference binary (`objdump -d c_src/build/driver`, `main`
//! builds a frame with `sub $0x840,%rsp` and addresses its locals as):
//!
//! ```text
//!   rbp-0x838   unsigned int byte      (reference read loop temp)
//!   rbp-0x834   unsigned int byte      (input read loop temp)
//!   rbp-0x830   char ref_buffer[1024]
//!   rbp-0x430   char input_buffer[1024]
//!   rbp-0x030   size_t ref_len
//!   rbp-0x028   size_t input_len
//!   rbp-0x020   (4 bytes of padding)
//!   rbp-0x01c   uint32_t flags
//!   rbp-0x018   int operation
//!   rbp-0x014   int result             (still uninitialised during the call)
//!   rbp-0x010   size_t i               (reference read loop)
//!   rbp-0x008   size_t i               (input read loop)
//!   rbp+0x000   saved rbp
//!   rbp+0x008   return address
//! ```
//!
//! Two consequences drive most of the observable behaviour:
//!
//! * `ref_buffer` sits *directly below* `input_buffer`, so reading past the end
//!   of `ref_buffer` continues into `input_buffer`.
//! * Reading past the end of `input_buffer` lands on `ref_len`, then
//!   `input_len`.  Both are `size_t` values that `main` has already validated to
//!   be at most 1024, so at least their top six bytes are zero: an unterminated
//!   `input_buffer` always finds its NUL within two bytes of the array end.
//!
//! Bytes that are still uninitialised when `process_strings` is called come from
//! [`crate::uninit`].

use crate::uninit::{INPUT_RESIDUE, REF_RESIDUE};

/// `MAX_BUFFER_SIZE`.
pub const BUF_SIZE: usize = 1024;

/// Offset of `ref_buffer` within the model.
pub const REF_OFF: usize = 0;
/// Offset of `input_buffer` within the model.
pub const INPUT_OFF: usize = BUF_SIZE;
/// Offset of the scalar locals that follow `input_buffer`.
pub const TAIL_OFF: usize = 2 * BUF_SIZE;

/// Total size of the modelled region: both buffers plus the 64 bytes of scalar
/// locals, saved frame pointer and return address that follow them.
pub const FRAME_LEN: usize = TAIL_OFF + 64;

/// Stand-in for the two 4-byte holes in the frame (`rbp-0x20` padding and the
/// `result` slot) that are still uninitialised when `process_strings` runs.
/// Unreachable in practice - see the `input_len` note above - but non-zero so
/// that it behaves like residue rather than a terminator.
const HOLE: u8 = 0xf2;

/// Stand-in for the saved frame pointer and return address.  Both are addresses
/// randomised by ASLR on every run, and neither is reachable by any read the C
/// code performs, because the NUL bytes inside `ref_len` stop every `strlen`
/// long before here.
const SAVED_RBP: u64 = 0x0000_7fff_ffff_e2b0;
const RETURN_ADDR: u64 = 0x0000_7f00_0002_4d90;

pub struct Frame {
    pub mem: [u8; FRAME_LEN],
}

impl Frame {
    /// The frame as it looks on entry to `main`: both buffers hold loader
    /// residue and nothing else has been written yet.
    pub fn new() -> Self {
        let mut mem = [0u8; FRAME_LEN];
        mem[REF_OFF..REF_OFF + BUF_SIZE].copy_from_slice(&REF_RESIDUE);
        mem[INPUT_OFF..INPUT_OFF + BUF_SIZE].copy_from_slice(&INPUT_RESIDUE);
        Frame { mem }
    }

    /// `input_buffer[i] = (char)byte`
    pub fn store_input(&mut self, i: usize, byte: u8) {
        self.mem[INPUT_OFF + i] = byte;
    }

    /// `ref_buffer[i] = (char)byte`
    pub fn store_ref(&mut self, i: usize, byte: u8) {
        self.mem[REF_OFF + i] = byte;
    }

    /// Publishes the scalar locals that live immediately after `input_buffer`,
    /// in the order the compiler placed them.
    pub fn commit_locals(&mut self, input_len: usize, ref_len: usize, operation: i32, flags: u32) {
        let t = TAIL_OFF;
        self.mem[t..t + 8].copy_from_slice(&(ref_len as u64).to_le_bytes()); // rbp-0x30
        self.mem[t + 8..t + 16].copy_from_slice(&(input_len as u64).to_le_bytes()); // rbp-0x28
        self.mem[t + 16..t + 20].copy_from_slice(&[HOLE; 4]); // rbp-0x20 padding
        self.mem[t + 20..t + 24].copy_from_slice(&flags.to_le_bytes()); // rbp-0x1c
        self.mem[t + 24..t + 28].copy_from_slice(&operation.to_le_bytes()); // rbp-0x18
        self.mem[t + 28..t + 32].copy_from_slice(&[HOLE; 4]); // rbp-0x14 result
        // Both read loops exit with `i == len`.
        self.mem[t + 32..t + 40].copy_from_slice(&(ref_len as u64).to_le_bytes()); // rbp-0x10
        self.mem[t + 40..t + 48].copy_from_slice(&(input_len as u64).to_le_bytes()); // rbp-0x08
        self.mem[t + 48..t + 56].copy_from_slice(&SAVED_RBP.to_le_bytes());
        self.mem[t + 56..t + 64].copy_from_slice(&RETURN_ADDR.to_le_bytes());
    }
}
