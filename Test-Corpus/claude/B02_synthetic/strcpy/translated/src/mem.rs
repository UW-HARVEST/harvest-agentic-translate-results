//! Emulation of the C `main` stack frame that holds `input_buffer` and
//! `ref_buffer`.
//!
//! The C code deliberately performs `strcmp`/`strlen` on buffers that are not
//! guaranteed to be NUL terminated, so its behaviour depends on the bytes that
//! happen to live in the uninitialised parts of the stack frame (and even past
//! the end of the two arrays).  To reproduce that behaviour the two 1024 byte
//! arrays are modelled as one contiguous byte region laid out exactly like the
//! compiler lays them out (`ref_buffer` first, `input_buffer` 1024 bytes above
//! it, followed by the remaining locals of `main`), pre-filled with junk that
//! mimics the pointer-dominated garbage a freshly entered stack frame contains
//! (six non-zero bytes followed by two zero bytes per machine word).
//!
//! Reads that run past the modelled region correspond to the C program walking
//! off the top of its stack, which terminates the process with SIGSEGV.

/// Offset of `ref_buffer` inside the modelled frame.
pub const REF_OFF: usize = 0;
/// Offset of `input_buffer` inside the modelled frame.
pub const INPUT_OFF: usize = 1024;
/// A "pointer" value that can never be a valid buffer (stands in for NULL).
pub const NULL_OFF: usize = usize::MAX;

/// Size of each of the two `char[1024]` arrays.
const BUF_SIZE: usize = 1024;
/// Bytes of additional stack modelled above `input_buffer` before a read is
/// treated as a fatal out-of-bounds access.
const TAIL_SIZE: usize = 4096;

pub struct Mem {
    bytes: Vec<u8>,
}

impl Mem {
    /// Build the frame: both arrays uninitialised (junk), followed by the
    /// remaining locals of `main` at the offsets the compiler assigns them.
    pub fn new(operation: i32, flags: u32, input_len: u64, ref_len: u64) -> Mem {
        let total = 2 * BUF_SIZE + TAIL_SIZE;
        let mut bytes: Vec<u8> = (0..total).map(junk_byte).collect();

        // Above `input_buffer` (in address order) live `ref_len`, `input_len`,
        // `flags` and `operation`.
        let base = 2 * BUF_SIZE;
        bytes[base..base + 8].copy_from_slice(&ref_len.to_le_bytes());
        bytes[base + 8..base + 16].copy_from_slice(&input_len.to_le_bytes());
        bytes[base + 16..base + 20].copy_from_slice(&flags.to_le_bytes());
        bytes[base + 20..base + 24].copy_from_slice(&operation.to_le_bytes());

        Mem { bytes }
    }

    /// Read one byte; running off the modelled stack aborts like the C program.
    pub fn get(&self, idx: usize) -> u8 {
        match self.bytes.get(idx) {
            Some(&b) => b,
            None => segfault(),
        }
    }

    /// Store one byte (`input_buffer[i] = ...` / `ref_buffer[i] = ...`).
    pub fn set(&mut self, idx: usize, value: u8) {
        match self.bytes.get_mut(idx) {
            Some(slot) => *slot = value,
            None => segfault(),
        }
    }
}

/// Junk left over in a fresh stack frame: mostly saved pointers, i.e. six
/// non-zero low bytes followed by two zero high bytes per 8 byte word.
fn junk_byte(i: usize) -> u8 {
    if i % 8 >= 6 {
        0
    } else {
        let mixed = (i as u64)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .rotate_left(17)
            ^ 0x5851_F42D_4C95_7F2D;
        ((mixed >> 24) as u8) | 0x80
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
