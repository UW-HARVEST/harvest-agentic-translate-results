//! Level 1: `stbds_arrgrowf` / `stbds_arrfreef` and
//! `stbds_stralloc` / `stbds_strreset`.

mod common;

use common::*;
use std::ffi::c_void;

/// Header snapshot without addresses.
#[derive(Debug, PartialEq, Eq)]
struct Hdr {
    length: usize,
    capacity: usize,
    table_null: bool,
    temp: isize,
}

unsafe fn hdr(a: *mut c_void) -> Hdr {
    unsafe {
        let h = header(a);
        Hdr {
            length: h.length,
            capacity: h.capacity,
            table_null: h.hash_table.is_null(),
            temp: h.temp,
        }
    }
}

#[test]
fn arrgrowf_fresh_allocation() {
    let _g = serial();
    let (c, r) = apis();

    for elemsize in [1usize, 2, 4, 8, 12, 16, 64] {
        for addlen in [0usize, 1, 2, 3, 4, 5, 7, 8, 100] {
            for min_cap in [0usize, 1, 2, 3, 4, 5, 9, 1000] {
                let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap) };
                assert_eq!(
                    ac.is_null(),
                    ar.is_null(),
                    "arrgrowf(NULL, {}, {}, {}) nullness mismatch",
                    elemsize,
                    addlen,
                    min_cap
                );
                if ac.is_null() {
                    // min_cap == 0 && addlen == 0 -> the early return hands the
                    // NULL right back.
                    continue;
                }
                let hc = unsafe { hdr(ac) };
                let hr = unsafe { hdr(ar) };
                assert_eq!(
                    hc, hr,
                    "arrgrowf(NULL, {}, {}, {}) header mismatch",
                    elemsize, addlen, min_cap
                );
                unsafe {
                    (c.arrfreef)(ac);
                    (r.arrfreef)(ar);
                }
            }
        }
    }
}

#[test]
fn arrgrowf_repeated_growth_sequences() {
    let _g = serial();
    let (c, r) = apis();

    for elemsize in [1usize, 4, 8, 16] {
        let mut ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        let mut ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
        assert_eq!(unsafe { hdr(ac) }, unsafe { hdr(ar) });

        // Mimic the arrput growth pattern: bump length then grow by one.
        for step in 0..200usize {
            let addlen = 1 + (step % 5);
            // set an identical length in both so `min_len` matches
            unsafe {
                let want = (*(ac as *mut common::ArrayHeader).sub(1)).capacity.min(step);
                (*(ac as *mut common::ArrayHeader).sub(1)).length = want;
                (*(ar as *mut common::ArrayHeader).sub(1)).length = want;
            }
            ac = unsafe { (c.arrgrowf)(ac, elemsize, addlen, 0) };
            ar = unsafe { (r.arrgrowf)(ar, elemsize, addlen, 0) };
            assert_eq!(
                unsafe { hdr(ac) },
                unsafe { hdr(ar) },
                "growth step {} (elemsize {}) mismatch",
                step,
                elemsize
            );
        }
        unsafe {
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    }
}

#[test]
fn arrgrowf_no_op_when_capacity_sufficient() {
    let _g = serial();
    let (c, r) = apis();

    let ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), 8, 0, 64) };
    let ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), 8, 0, 64) };
    let ac2 = unsafe { (c.arrgrowf)(ac, 8, 4, 0) };
    let ar2 = unsafe { (r.arrgrowf)(ar, 8, 4, 0) };
    assert_eq!(ac2, ac, "C must return the same pointer");
    assert_eq!(ar2, ar, "Rust must return the same pointer");
    assert_eq!(unsafe { hdr(ac2) }, unsafe { hdr(ar2) });
    unsafe {
        (c.arrfreef)(ac2);
        (r.arrfreef)(ar2);
    }
}

#[test]
fn arrgrowf_preserves_payload() {
    let _g = serial();
    let (c, r) = apis();

    let elemsize = 4usize;
    let mut ac = unsafe { (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
    let mut ar = unsafe { (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1) };
    for i in 0..500u32 {
        unsafe {
            let hc = (ac as *mut common::ArrayHeader).sub(1);
            let hr = (ar as *mut common::ArrayHeader).sub(1);
            if (*hc).length + 1 > (*hc).capacity {
                ac = (c.arrgrowf)(ac, elemsize, 1, 0);
                ar = (r.arrgrowf)(ar, elemsize, 1, 0);
            }
            let hc = (ac as *mut common::ArrayHeader).sub(1);
            let hr2 = (ar as *mut common::ArrayHeader).sub(1);
            let n = (*hc).length;
            *(ac as *mut u32).add(n) = i;
            *(ar as *mut u32).add(n) = i;
            (*hc).length = n + 1;
            (*hr2).length = n + 1;
            let _ = hr;
        }
    }
    assert_eq!(unsafe { hdr(ac) }, unsafe { hdr(ar) });
    let n = unsafe { header(ac).length };
    let sc = unsafe { std::slice::from_raw_parts(ac as *const u32, n) }.to_vec();
    let sr = unsafe { std::slice::from_raw_parts(ar as *const u32, n) }.to_vec();
    assert_eq!(sc, sr);
    unsafe {
        (c.arrfreef)(ac);
        (r.arrfreef)(ar);
    }
}

// ---------------------------------------------------------------------------
// string arena
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct ArenaState {
    remaining: usize,
    block: u8,
    mode: u8,
    storage_null: bool,
    blocks: usize,
}

unsafe fn arena_state(a: *const StringArena) -> ArenaState {
    unsafe {
        let mut blocks = 0usize;
        let mut p = (*a).storage as *const *const c_void;
        while !p.is_null() {
            blocks += 1;
            p = *p as *const *const c_void;
            if blocks > 10_000 {
                break;
            }
        }
        ArenaState {
            remaining: (*a).remaining,
            block: (*a).block,
            mode: (*a).mode,
            storage_null: (*a).storage.is_null(),
            blocks,
        }
    }
}

#[test]
fn stralloc_matches() {
    let _g = serial();
    let (c, r) = apis();

    // Mirror the sh_geti usage plus strings that overflow a block. The cases
    // that alternate over-sized allocations with small ones matter: the
    // over-sized branch splices its block in *behind* the current head so that
    // the partially filled head keeps serving small requests.
    let cases: Vec<Vec<Vec<u8>>> = vec![
        (0..40).map(|i| format!("test_{}", i).into_bytes()).collect(),
        (0..400).map(|i| format!("test_{}", i).into_bytes()).collect(),
        vec![b"".to_vec(), b"a".to_vec(), b"".to_vec()],
        vec![vec![b'x'; 600], vec![b'y'; 10], vec![b'z'; 4096]],
        (0..80).map(|i| vec![b'q'; i * 13 + 1]).collect(),
        vec![vec![b'A'; 511], vec![b'B'; 1], vec![b'C'; 1], vec![b'D'; 2000]],
        // small first (creates a partially used block), then over-sized, then
        // more small ones which must keep coming out of the *first* block
        vec![
            vec![b'a'; 2],
            vec![b'b'; 1000],
            vec![b'c'; 3],
            vec![b'd'; 4],
            vec![b'e'; 5000],
            vec![b'f'; 6],
            vec![b'g'; 7],
        ],
        vec![
            vec![b'h'; 100],
            vec![b'i'; 900],
            vec![b'j'; 50],
            vec![b'k'; 20000],
            vec![b'l'; 25],
            vec![b'm'; 30],
            vec![b'n'; 8000],
            vec![b'o'; 40],
        ],
        (0..30)
            .map(|i| {
                if i % 3 == 0 {
                    vec![b'X'; 3000 + i * 100]
                } else {
                    vec![b'y'; i + 1]
                }
            })
            .collect(),
    ];

    for (ci, case) in cases.iter().enumerate() {
        let mut sac = StringArena::new();
        let mut sar = StringArena::new();
        let mut ptrs_c: Vec<*mut std::ffi::c_char> = Vec::new();
        let mut ptrs_r: Vec<*mut std::ffi::c_char> = Vec::new();

        for (si, s) in case.iter().enumerate() {
            let mut kc = CStr8::from_bytes(s);
            let mut kr = CStr8::from_bytes(s);
            let pc = unsafe { (c.stralloc)(&mut sac, kc.as_ptr()) };
            let pr = unsafe { (r.stralloc)(&mut sar, kr.as_ptr()) };
            assert!(!pc.is_null() && !pr.is_null());
            ptrs_c.push(pc);
            ptrs_r.push(pr);
            assert_eq!(
                unsafe { arena_state(&sac) },
                unsafe { arena_state(&sar) },
                "case {} step {} arena state mismatch",
                ci,
                si
            );

            // Re-read *every* string handed out so far: a later allocation must
            // never scribble over an earlier one.
            let all_c: Vec<Vec<u8>> = ptrs_c.iter().map(|p| unsafe { cstr(*p) }).collect();
            let all_r: Vec<Vec<u8>> = ptrs_r.iter().map(|p| unsafe { cstr(*p) }).collect();
            assert_eq!(
                all_c, all_r,
                "case {} step {}: retained arena contents diverge",
                ci, si
            );
            assert_eq!(
                all_c,
                case[..=si].to_vec(),
                "case {} step {}: arena corrupted an earlier string",
                ci,
                si
            );
        }

        unsafe {
            (c.strreset)(&mut sac);
            (r.strreset)(&mut sar);
        }
        assert_eq!(unsafe { arena_state(&sac) }, unsafe { arena_state(&sar) });
        assert_eq!(
            unsafe { arena_state(&sac) },
            ArenaState {
                remaining: 0,
                block: 0,
                mode: 0,
                storage_null: true,
                blocks: 0
            }
        );
    }
}

#[test]
fn strreset_on_empty_arena() {
    let _g = serial();
    let (c, r) = apis();
    let mut sac = StringArena::new();
    let mut sar = StringArena::new();
    unsafe {
        (c.strreset)(&mut sac);
        (r.strreset)(&mut sar);
    }
    assert_eq!(unsafe { arena_state(&sac) }, unsafe { arena_state(&sar) });
}

#[test]
fn stralloc_block_growth_progression() {
    let _g = serial();
    let (c, r) = apis();

    // Repeatedly fill blocks so `a->block` climbs and the doubling in
    // `STBDS_STRING_ARENA_BLOCKSIZE_MIN << (block>>1)` is exercised.
    let mut sac = StringArena::new();
    let mut sar = StringArena::new();
    for i in 0..3000usize {
        let s = vec![b'k'; (i % 97) + 1];
        let mut kc = CStr8::from_bytes(&s);
        let mut kr = CStr8::from_bytes(&s);
        let pc = unsafe { (c.stralloc)(&mut sac, kc.as_ptr()) };
        let pr = unsafe { (r.stralloc)(&mut sar, kr.as_ptr()) };
        assert_eq!(unsafe { cstr(pc) }, unsafe { cstr(pr) });
        assert_eq!(
            unsafe { arena_state(&sac) },
            unsafe { arena_state(&sar) },
            "iteration {}",
            i
        );
    }
    unsafe {
        (c.strreset)(&mut sac);
        (r.strreset)(&mut sar);
    }
}
