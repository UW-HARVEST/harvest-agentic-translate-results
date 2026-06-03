//! Translation of the `stbds_string_arena` family of functions
//! (`stbds_stralloc`, `stbds_strreset`) from `c_src/src/lib.c`.
//!
//! The C implementation grows an arena of variable-sized blocks,
//! dropping zero-terminated strings into them and returning raw
//! `char *` pointers. The blocks themselves are kept on a singly
//! linked list so they can be freed in one shot by `strreset`.
//!
//! In Rust we model the arena as a `Vec<Box<[u8]>>` — each `Box`
//! corresponds to one of the original blocks. `alloc` writes the
//! string (including its terminating NUL byte to stay
//! byte-for-byte compatible) into the current block, returning
//! the offset where it was placed. `reset` simply truncates the
//! `Vec`, which drops every block.

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// Slot describing where the most recently allocated string lives
/// inside the arena. Returned by [`StringArena::alloc`] in case a
/// caller needs to read the bytes back.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ArenaSlot {
    pub block: usize,
    pub offset: usize,
    pub len: usize,
}

/// Rust equivalent of the C `stbds_string_arena` struct.
pub struct StringArena {
    blocks: Vec<Vec<u8>>,
    /// Bytes still available at the *end* of the most recent
    /// block. Mirrors the C `remaining` field, which counts down
    /// as strings are packed into the block from the high end.
    remaining: usize,
    /// Block-size doubling counter. Each call that needs a new
    /// regular-sized block bumps this until the next block size
    /// would exceed `STBDS_STRING_ARENA_BLOCKSIZE_MAX`.
    block: u8,
}

impl StringArena {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            remaining: 0,
            block: 0,
        }
    }

    /// Allocate space for `str` (plus a trailing NUL byte) inside
    /// the arena and return a slot describing where it landed.
    ///
    /// Mirrors `stbds_stralloc` from `c_src/src/lib.c`.
    pub fn alloc(&mut self, s: &str) -> ArenaSlot {
        let len = s.len() + 1; // +1 for the NUL terminator

        if len > self.remaining {
            // Compute the next regular block size, doubling each
            // time we hit the path that allocates a fresh block.
            let mut blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (self.block as usize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                self.block = self.block.saturating_add(1);
            }

            if len > blocksize {
                // Oversized strings get their own dedicated block
                // — same fast-path as the C implementation.
                blocksize = len;

                let mut block = vec![0u8; blocksize];
                block[..s.len()].copy_from_slice(s.as_bytes());
                // The trailing zero is already present because we
                // initialised the block with zeros.

                let slot = ArenaSlot {
                    block: self.blocks.len(),
                    offset: 0,
                    len,
                };

                // The C code splices the oversized block into
                // position 1 of the list (right after the head)
                // when a head exists, otherwise it becomes the
                // head. We don't expose the linkage, so simply
                // pushing the block is sufficient for our needs.
                self.blocks.push(block);
                return slot;
            }

            // Regular-sized fresh block.
            let block = vec![0u8; blocksize];
            self.blocks.push(block);
            self.remaining = blocksize;
        }

        // We are guaranteed enough room in the current block now.
        debug_assert!(len <= self.remaining);

        let block_idx = self.blocks.len() - 1;
        let block = &mut self.blocks[block_idx];
        let offset = self.remaining - len;
        block[offset..offset + s.len()].copy_from_slice(s.as_bytes());
        block[offset + s.len()] = 0;
        self.remaining -= len;

        ArenaSlot {
            block: block_idx,
            offset,
            len,
        }
    }

    /// Release every block held by the arena. Equivalent to
    /// `stbds_strreset`.
    pub fn reset(&mut self) {
        self.blocks.clear();
        self.remaining = 0;
        self.block = 0;
    }
}

impl Default for StringArena {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for StringArena {
    fn drop(&mut self) {
        // `Vec`'s own drop impl handles the per-block frees, so
        // there's nothing extra to do here. Defining `Drop`
        // explicitly documents that the C version releases the
        // backing storage at this point.
    }
}
