//! Level 1: the array growth primitive and the string arena.
//!   stbds_arrgrowf, stbds_arrfreef, stbds_stralloc, stbds_strreset

mod common;

use common::*;
use std::ffi::{c_char, c_void};

fn cbuf(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

/// Replays the same `arrgrowf` call sequence on one library and returns the
/// observable state after every step.
unsafe fn grow_seq(api: &Api, elemsize: usize, steps: &[(usize, usize)]) -> Vec<(bool, Option<HeaderSnap>, Vec<u8>)> {
    unsafe {
        let mut a: *mut c_void = std::ptr::null_mut();
        let mut out = Vec::new();
        for &(addlen, min_cap) in steps {
            a = (api.arrgrowf)(a, elemsize, addlen, min_cap);
            if a.is_null() {
                out.push((true, None, Vec::new()));
                continue;
            }
            let h = header_snap(a);
            // Write a deterministic pattern over the live prefix so the next
            // realloc has something to preserve, then record it.
            let live = h.length * elemsize;
            for i in 0..live {
                // only re-stamp bytes we have not stamped before is not needed:
                // the pattern is a pure function of the index.
                *(a as *mut u8).add(i) = (i as u8).wrapping_mul(31).wrapping_add(7);
            }
            let bytes = std::slice::from_raw_parts(a as *const u8, live).to_vec();
            out.push((false, Some(h), bytes));
        }
        if !a.is_null() {
            (api.arrfreef)(a);
        }
        out
    }
}

#[test]
fn arrgrowf_matches() {
    let (c, r) = both();

    let seqs: Vec<Vec<(usize, usize)>> = vec![
        vec![(0, 1)],
        vec![(1, 0)],
        vec![(0, 0)],                     // returns NULL in the C source
        vec![(0, 0), (1, 0)],
        vec![(1, 0), (1, 0), (1, 0), (1, 0), (1, 0), (1, 0), (1, 0), (1, 0), (1, 0)],
        vec![(4, 0), (4, 0), (4, 0)],
        vec![(0, 3)],
        vec![(0, 4)],
        vec![(0, 5)],
        vec![(0, 100), (0, 101), (0, 300), (0, 301)],
        vec![(7, 2), (0, 9), (5, 0), (0, 64), (1, 0)],
        vec![(0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8)],
        vec![(3, 3), (0, 6), (0, 7), (0, 13), (0, 14), (0, 27), (0, 28)],
    ];

    for elemsize in [1usize, 2, 4, 8, 12, 16, 24, 64] {
        for seq in &seqs {
            let a = unsafe { grow_seq(&c, elemsize, seq) };
            let b = unsafe { grow_seq(&r, elemsize, seq) };
            assert_eq!(a, b, "arrgrowf elemsize={elemsize} seq={seq:?}");
        }
    }
}

/// `arrgrowf` must preserve the payload across reallocation. Drive length
/// forward the way the stb macros do (the caller bumps `length`) and check the
/// bytes survive on both sides.
#[test]
fn arrgrowf_preserves_payload() {
    let (c, r) = both();
    for elemsize in [4usize, 8, 16] {
        let run = |api: &Api| unsafe {
            let mut a: *mut c_void = std::ptr::null_mut();
            let mut states = Vec::new();
            for n in 0..40usize {
                let h = if a.is_null() {
                    std::ptr::null_mut()
                } else {
                    (a as *mut ArrayHeader).sub(1)
                };
                let len = if h.is_null() { 0 } else { (*h).length };
                let cap = if h.is_null() { 0 } else { (*h).capacity };
                if h.is_null() || len + 1 > cap {
                    a = (api.arrgrowf)(a, elemsize, 1, 0);
                }
                let h = (a as *mut ArrayHeader).sub(1);
                for k in 0..elemsize {
                    *(a as *mut u8).add((*h).length * elemsize + k) =
                        (n as u8).wrapping_mul(11).wrapping_add(k as u8);
                }
                (*h).length += 1;
                states.push((
                    header_snap(a),
                    std::slice::from_raw_parts(a as *const u8, (*h).length * elemsize).to_vec(),
                ));
                let _ = n;
            }
            (api.arrfreef)(a);
            states
        };
        assert_eq!(run(&c), run(&r), "payload preservation elemsize={elemsize}");
    }
}

// ---------------------------------------------------------------------------
// string arena
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct ArenaStep {
    ret: Option<String>,
    remaining: usize,
    block: u8,
    mode: u8,
    has_storage: bool,
    /// Every string previously handed out, re-read afterwards: the arena must
    /// keep them alive and unmodified.
    live: Vec<Option<String>>,
}

unsafe fn arena_seq(api: &Api, inputs: &[String]) -> Vec<ArenaStep> {
    unsafe {
        let mut arena = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        let ap = &raw mut arena as *mut c_void;
        let mut handed: Vec<*mut c_char> = Vec::new();
        let mut out = Vec::new();
        for s in inputs {
            let mut buf = cbuf(s);
            let p = (api.stralloc)(ap, buf.as_mut_ptr());
            handed.push(p);
            out.push(ArenaStep {
                ret: c_string(p),
                remaining: arena.remaining,
                block: arena.block,
                mode: arena.mode,
                has_storage: !arena.storage.is_null(),
                live: if handed.len() <= 256 {
                    handed.iter().map(|&q| c_string(q)).collect()
                } else {
                    // O(n^2) otherwise; spot-check a sliding window instead
                    handed[handed.len() - 256..]
                        .iter()
                        .map(|&q| c_string(q))
                        .collect()
                },
            });
        }
        (api.strreset)(ap);
        // after reset the arena must be fully zeroed
        out.push(ArenaStep {
            ret: None,
            remaining: arena.remaining,
            block: arena.block,
            mode: arena.mode,
            has_storage: !arena.storage.is_null(),
            live: Vec::new(),
        });
        out
    }
}

#[test]
fn stralloc_matches() {
    let (c, r) = both();

    let mut seqs: Vec<Vec<String>> = vec![
        vec!["".into()],
        vec!["a".into()],
        vec!["hello".into(), "world".into()],
        // exactly fills / just overflows the first 512-byte block
        vec!["x".repeat(511)],
        vec!["x".repeat(512)],
        vec!["x".repeat(513)],
        vec!["x".repeat(511), "y".to_string()],
        vec!["x".repeat(510), "yy".to_string(), "z".to_string()],
        // an oversized string takes the dedicated-block path
        vec!["q".repeat(5000)],
        vec!["a".into(), "q".repeat(5000), "b".into()],
        vec!["q".repeat(5000), "q".repeat(5000)],
    ];
    // many small strings: forces repeated block growth (512, 512, 1024, ...)
    seqs.push((0..600).map(|i| format!("s{i:04}")).collect());
    // growth interleaved with oversized allocations
    seqs.push(
        (0..80)
            .map(|i| if i % 17 == 0 { "Z".repeat(3000) } else { format!("k{i}") })
            .collect(),
    );

    for seq in &seqs {
        let a = unsafe { arena_seq(&c, seq) };
        let b = unsafe { arena_seq(&r, seq) };
        assert_eq!(a.len(), b.len());
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(x, y, "stralloc step {i} of seq(len={})", seq.len());
        }
    }
}

#[test]
fn strreset_on_empty_arena() {
    let (c, r) = both();
    for api in [&c, &r] {
        let mut arena = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 7,
        };
        unsafe { (api.strreset)(&raw mut arena as *mut c_void) };
        assert_eq!(arena.remaining, 0);
        assert_eq!(arena.block, 0);
        assert_eq!(arena.mode, 0, "{} did not zero the arena", api.name);
        assert!(arena.storage.is_null());
    }
}

/// The arena's block size is `512 << (block >> 1)` and `block` stops advancing
/// once the size reaches 1 MiB. Allocating past that point is the only way to
/// reach the saturated state, so drive ~8 MiB of strings through it.
#[test]
fn stralloc_saturates_block_counter() {
    let (c, r) = both();
    let inputs: Vec<String> = (0..9000).map(|i| format!("{:1023}", i)).collect();
    let a = unsafe { arena_seq(&c, &inputs) };
    let b = unsafe { arena_seq(&r, &inputs) };
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            (x.remaining, x.block, x.mode, x.has_storage, &x.ret),
            (y.remaining, y.block, y.mode, y.has_storage, &y.ret),
            "saturating arena step {i}"
        );
    }
    // block must have stopped growing (512 << 11 == 1 MiB)
    let last = &a[a.len() - 2];
    assert!(last.block >= 22, "block only reached {}", last.block);
    assert_eq!(last.block, b[b.len() - 2].block);
}
