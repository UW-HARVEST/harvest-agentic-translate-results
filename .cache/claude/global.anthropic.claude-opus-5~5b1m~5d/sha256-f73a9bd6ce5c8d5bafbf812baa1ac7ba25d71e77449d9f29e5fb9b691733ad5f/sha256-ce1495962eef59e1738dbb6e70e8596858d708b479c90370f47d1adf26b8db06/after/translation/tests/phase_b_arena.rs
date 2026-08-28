//! Phase B — CONFIGS.md rows 41-46: `stbds_stralloc` / `stbds_strreset`
//! driven directly on a caller-owned `stbds_string_arena`.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

#[derive(Clone, Debug)]
enum AOp {
    /// `stbds_stralloc(&arena, str)`
    Alloc(Vec<u8>),
    /// `stbds_strreset(&arena)`
    Reset,
    /// hand-set `arena.block` (a caller may own the arena)
    SetBlock(u8),
}

/// Runs an arena script against one library.
///
/// Every returned pointer is remembered and re-verified after each step, so a
/// wrong `memmove`/offset shows up immediately.  Absolute addresses are never
/// compared; the *offset inside the block* is, because the C computes it
/// deterministically as `storage->storage + remaining - len`.
fn run_arena(api: &Api, ops: &[AOp]) -> Vec<String> {
    let mut t = Vec::new();
    let mut arena = StringArena::zeroed();
    let ap: *mut StringArena = &mut arena;
    let mut live: Vec<(*mut c_char, Vec<u8>)> = Vec::new();

    unsafe {
        for (n, op) in ops.iter().enumerate() {
            match op {
                AOp::SetBlock(b) => {
                    (*ap).block = *b;
                    t.push(format!("[{n}] set block={b}"));
                }
                AOp::Reset => {
                    (api.strreset)(ap as *mut c_void);
                    live.clear();
                    t.push(format!("[{n}] reset"));
                }
                AOp::Alloc(sbytes) => {
                    let len = sbytes.len(); // includes the NUL
                    let rem_before = (*ap).remaining;
                    let block_before = (*ap).block;
                    let had_storage = !(*ap).storage.is_null();
                    // predict which branch the C takes
                    let (oversized, base_rem) = if len > rem_before {
                        let blocksize = ARENA_BLOCKSIZE_MIN << (block_before >> 1);
                        if len > blocksize { (true, 0) } else { (false, blocksize) }
                    } else {
                        (false, rem_before)
                    };
                    let mut buf = sbytes.clone();
                    let p = (api.stralloc)(ap as *mut c_void, buf.as_mut_ptr() as *mut c_char);
                    t.push(format!(
                        "[{n}] alloc len={len} rem_before={rem_before} block_before={block_before} \
                         had_storage={had_storage} oversized={oversized}"
                    ));
                    t.extend(snap_arena(ap));
                    // returned content
                    let got = cstr_bytes(p).expect("stralloc returned NULL");
                    t.push(format!("   got={}", hex(&got)));
                    assert_eq!(&got[..], &sbytes[..len - 1], "{}: content", api.tag);
                    if !oversized {
                        // p == storage->storage + base_rem - len
                        let block_storage = ((*ap).storage as *const u8).add(8);
                        let off = (p as usize).wrapping_sub(block_storage as usize);
                        t.push(format!("   offset_in_block={off}"));
                        assert_eq!(off, base_rem - len, "{}: offset", api.tag);
                        assert_eq!((*ap).remaining, base_rem - len, "{}: remaining", api.tag);
                    }
                    live.push((p, sbytes.clone()));
                    // every previously handed-out string must still be intact
                    for (i, (q, want)) in live.iter().enumerate() {
                        let have = cstr_bytes(*q).unwrap();
                        assert_eq!(
                            &have[..],
                            &want[..want.len() - 1],
                            "{}: live string {i} corrupted",
                            api.tag
                        );
                    }
                    t.push(format!("   live={}", live.len()));
                }
            }
        }
        // always release
        (api.strreset)(ap as *mut c_void);
        t.extend(snap_arena(ap));
    }
    t
}

fn diff_arena(ctx: &str, ops: &[AOp]) {
    let p = seeded(DEFAULT_SEED);
    let tc = run_arena(p.c, ops);
    let tr = run_arena(p.r, ops);
    assert_traces_eq(ctx, &tc, &tr);
}

fn st(text: &str) -> Vec<u8> {
    let mut v = text.as_bytes().to_vec();
    v.push(0);
    v
}

/// row 41 — fresh zeroed arena: the first allocation takes a `512 << 0` block
#[test]
fn stralloc_fresh() {
    for text in ["", "a", "hello", &"x".repeat(100), &"y".repeat(511)] {
        diff_arena(&format!("fresh len={}", text.len()), &[AOp::Alloc(st(text))]);
    }
}

/// row 42 — fill a block then overflow it: `block` increments and the block
/// size doubles every other step
#[test]
fn stralloc_block_growth() {
    let mut ops = Vec::new();
    for i in 0..120u32 {
        ops.push(AOp::Alloc(st(&format!("{:0>60}", i))));
    }
    diff_arena("block growth 60b", &ops);

    let mut ops = Vec::new();
    let mut rng = Rng::new(0x4242);
    for _ in 0..80 {
        let l = 400 + rng.below(120) as usize;
        ops.push(AOp::Alloc(rng.nul_free(l)));
    }
    diff_arena("block growth ~500b", &ops);

    // exactly filling `remaining` (len == remaining) then one more byte
    let mut ops = Vec::new();
    ops.push(AOp::Alloc(st(&"a".repeat(511)))); // len 512 == blocksize
    ops.push(AOp::Alloc(st("b"))); // remaining == 0 -> new block
    ops.push(AOp::Alloc(st(&"c".repeat(509)))); // len 510 <= 511 remaining
    ops.push(AOp::Alloc(st("d")));
    diff_arena("block exact fill", &ops);
}

/// row 43 — `len > blocksize`: the dedicated oversized block, with and without
/// an existing head block
#[test]
fn stralloc_oversized() {
    // no existing storage: sb->next = 0, storage = sb, remaining = 0
    diff_arena("oversized fresh", &[AOp::Alloc(st(&"z".repeat(2000)))]);
    // existing storage: sb is spliced after the head and `remaining` is kept
    diff_arena(
        "oversized after small",
        &[
            AOp::Alloc(st("small")),
            AOp::Alloc(st(&"z".repeat(2000))),
            AOp::Alloc(st("after")),
            AOp::Alloc(st(&"w".repeat(5000))),
            AOp::Alloc(st("tail")),
        ],
    );
    // several oversized allocations in a row
    let mut ops = Vec::new();
    for i in 0..8 {
        ops.push(AOp::Alloc(st(&"q".repeat(1024 * (i + 1)))));
    }
    diff_arena("oversized chain", &ops);
}

/// row 44 — the caller-visible `block` field: blocksize is
/// `512 << (block>>1)`, clamped (`block` stops advancing) once it reaches 1<<20
#[test]
fn stralloc_block_field() {
    // 0..=24 keeps the largest block at 2 MiB; larger values would ask malloc
    // for gigabytes (the C would fail the same way, see ERRORS.md)
    for b in 0u8..=24 {
        diff_arena(
            &format!("block field b={b}"),
            &[AOp::SetBlock(b), AOp::Alloc(st("hi")), AOp::Alloc(st("there"))],
        );
    }
}

/// row 45 — randomised lengths
#[test]
fn stralloc_random() {
    for trial in 0..8u64 {
        let mut rng = Rng::new(0x9000 + trial);
        let mut ops = Vec::new();
        for _ in 0..200 {
            let l = rng.below(2000) as usize;
            ops.push(AOp::Alloc(rng.nul_free(l)));
            if rng.below(40) == 0 {
                ops.push(AOp::Reset);
            }
        }
        diff_arena(&format!("random trial={trial}"), &ops);
    }
}

/// row 46 — `stbds_strreset` on empty / one-block / many-block / oversized
/// arenas, and reuse after reset
#[test]
fn strreset_paths() {
    diff_arena("reset empty", &[AOp::Reset, AOp::Reset]);
    diff_arena("reset one block", &[AOp::Alloc(st("one")), AOp::Reset]);
    let mut ops = Vec::new();
    for i in 0..30 {
        ops.push(AOp::Alloc(st(&format!("{:0>200}", i))));
    }
    ops.push(AOp::Reset);
    ops.push(AOp::Alloc(st("after reset")));
    ops.push(AOp::Reset);
    ops.push(AOp::Alloc(st(&"m".repeat(3000))));
    ops.push(AOp::Reset);
    ops.push(AOp::Alloc(st("last")));
    diff_arena("reset many blocks", &ops);
}
