//! Phase B — CONFIGS.md rows C66..C68: cross-library interoperability.
//!
//! A data structure created by ONE library is then handed to the OTHER library's
//! exported functions and back again.  This is the strongest available proof
//! that every shared structure (`stbds_array_header`, `stbds_hash_index`,
//! `stbds_hash_bucket`, `stbds_string_arena`, `stbds_string_block`) has an
//! identical layout and that both libraries agree on every internal invariant —
//! a single field offset or size mismatch corrupts the structure immediately.
//!
//! Because both `.so`s call the *same* libc `realloc`/`free`, memory allocated
//! by one can legitimately be grown/freed by the other.
//!
//! Note that only a *fresh* `stbds_make_hash_index` consumes the per-library
//! global seed; grow/shrink/rebuild copy `ot->seed`, so a mixed run stays
//! deterministic once the first table has been created.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

/// Execute one driver op and return a library-independent result string.
unsafe fn apply(d: &mut Drv, keys: &[Vec<u8>], op: Op) -> String {
    unsafe {
        match op {
            Op::Put(k, t) => format!("put={}", d.put(&keys[k], t)),
            Op::PutStruct(k, t) => format!("puts={:?}", d.puts_struct(&keys[k], t)),
            Op::Get(k) => format!("get={}", d.get(&keys[k])),
            Op::GetTs(k) => format!("gets={}", d.get_ts(&keys[k])),
            Op::Del(k, o) => format!("del={}", d.del(&keys[k], o)),
            Op::Default(t) => {
                d.put_default(t);
                "default".into()
            }
            Op::Len => format!("len={}", d.len()),
        }
    }
}

/// Run `ops` twice: once entirely inside `lib_a`, once alternating between the
/// two libraries according to `pick`.  Results and full state must agree at
/// every step.
fn mixed_vs_pure(
    ctx: &str,
    es: usize,
    ks: usize,
    mode: i32,
    sh: Option<i32>,
    seed: usize,
    keys: &[Vec<u8>],
    ops: &[Op],
    pick: &dyn Fn(usize) -> bool, // true => C, false => Rust
) {
    let (c, r) = both();
    unsafe {
        // reference run: everything in C
        sync_seed(seed);
        let mut refd = match sh {
            Some(s) => Drv::shmode(c, es, ks, mode, s),
            None => Drv::empty(c, es, ks, mode),
        };
        let mut refres = Vec::new();
        let mut refsnap = Vec::new();
        for &op in ops {
            refres.push(apply(&mut refd, keys, op));
            refsnap.push(refd.snap());
        }
        refd.free();

        // mixed run: the structure is created by C, then every op is executed by
        // whichever library `pick` selects.
        sync_seed(seed);
        let mut mixd = match sh {
            Some(s) => Drv::shmode(c, es, ks, mode, s),
            None => Drv::empty(c, es, ks, mode),
        };
        for (i, &op) in ops.iter().enumerate() {
            mixd.lib = if pick(i) { c } else { r };
            let got = apply(&mut mixd, keys, op);
            assert_eq!(refres[i], got, "{ctx} op#{i} {op:?}: result (lib={})", mixd.lib.name);
            eqs(&format!("{ctx} op#{i} {op:?} lib={}", mixd.lib.name), &refsnap[i], &mixd.snap());
        }
        // and let the OTHER library free it
        mixd.lib = r;
        mixd.free();
    }
}

fn skeys(n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| padded_key(format!("ikey{i:04}").as_bytes()))
        .collect()
}

// ---------------------------------------------------------------------------
// C66 — map built by C, then driven by Rust (and vice versa)
// ---------------------------------------------------------------------------
#[test]
fn c66_map_interop_binary() {
    let _g = lock();
    let bkeys: Vec<Vec<u8>> = (0..40).map(|i| bin_key(i as u64 * 2654435761 + 1, 8)).collect();
    let mut ops: Vec<Op> = Vec::new();
    for i in 0..40 {
        ops.push(Op::Put(i, i as u8));
        ops.push(Op::Get(i));
        ops.push(Op::Len);
    }
    for i in 0..40 {
        ops.push(Op::Del(i, 0));
        ops.push(Op::Get(i));
    }
    for i in 0..20 {
        ops.push(Op::Put(i, 0x77));
    }

    for &(es, ks) in &[(8usize, 4usize), (16, 8), (24, 16), (64, 8)] {
        for &seed in &[0usize, DEFAULT_SEED, 0x1357] {
            for (name, pick) in interleavings() {
                mixed_vs_pure(
                    &format!("bin-interop es={es} ks={ks} seed={seed:#x} {name}"),
                    es,
                    ks,
                    HM_BINARY,
                    None,
                    seed,
                    &bkeys,
                    &ops,
                    pick.as_ref(),
                );
            }
        }
    }
}

#[test]
fn c66_map_interop_string_all_modes() {
    let _g = lock();
    let keys = skeys(40);
    let mut ops: Vec<Op> = Vec::new();
    for i in 0..40 {
        ops.push(Op::Put(i, i as u8));
        ops.push(Op::Get(i));
        ops.push(Op::GetTs(i));
    }
    for i in 0..40 {
        ops.push(Op::Put(i, 0x90 + i as u8));
    }
    for i in 0..40 {
        ops.push(Op::Del(i, 0));
        ops.push(Op::Len);
    }

    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &seed in &[0usize, DEFAULT_SEED] {
            for (name, pick) in interleavings() {
                mixed_vs_pure(
                    &format!("str-interop sh={sh} seed={seed:#x} {name}"),
                    16,
                    8,
                    HM_STRING,
                    Some(sh),
                    seed,
                    &keys,
                    &ops,
                    pick.as_ref(),
                );
            }
        }
    }
    // implicit SH_DEFAULT (map starts as NULL, so even the *first* allocation is
    // done by whichever library happens to be selected)
    for &seed in &[0usize, DEFAULT_SEED] {
        for (name, pick) in interleavings() {
            mixed_vs_pure(
                &format!("str-interop-from-null seed={seed:#x} {name}"),
                16,
                8,
                HM_STRING,
                None,
                seed,
                &keys,
                &ops,
                pick.as_ref(),
            );
        }
    }
}

type Pick = Box<dyn Fn(usize) -> bool>;

fn interleavings() -> Vec<(&'static str, Pick)> {
    vec![
        ("alternate", Box::new(|i: usize| i % 2 == 0) as Pick),
        ("rust-only", Box::new(|_: usize| false) as Pick),
        ("c-first-half", Box::new(|i: usize| i < 40) as Pick),
        ("rust-first-half", Box::new(|i: usize| i >= 40) as Pick),
        ("every-third", Box::new(|i: usize| i % 3 != 0) as Pick),
    ]
}

// ---------------------------------------------------------------------------
// C67 — arena built by C, continued/reset by Rust and vice versa
// ---------------------------------------------------------------------------
#[test]
fn c67_arena_interop() {
    let _g = lock();
    let (c, r) = both();
    let strings: Vec<Vec<u8>> = (0..400)
        .map(|i| {
            let n = match i % 7 {
                0 => 0,
                1 => 3,
                2 => 40,
                3 => 200,
                4 => 700,
                5 => 3000,
                _ => 17,
            };
            let mut v: Vec<u8> = (0..n).map(|k| b'a' + ((i + k) % 26) as u8).collect();
            v.push(0);
            v
        })
        .collect();

    for (name, pick) in interleavings() {
        unsafe {
            // reference: all C
            let mut refa = StringArena::zeroed();
            let mut refstate = Vec::new();
            for s in &strings {
                let p = (c.stralloc)(&mut refa, s.as_ptr() as *mut c_char);
                refstate.push(format!(
                    "{} content={}",
                    snap_arena(&refa),
                    cstr_opt(p as *const c_char)
                ));
            }
            (c.strreset)(&mut refa);
            refstate.push(snap_arena(&refa));

            // mixed
            let mut mixa = StringArena::zeroed();
            for (i, s) in strings.iter().enumerate() {
                let lib = if pick(i) { c } else { r };
                let p = (lib.stralloc)(&mut mixa, s.as_ptr() as *mut c_char);
                let got = format!(
                    "{} content={}",
                    snap_arena(&mixa),
                    cstr_opt(p as *const c_char)
                );
                eqs(&format!("arena-interop {name} #{i} lib={}", lib.name), &refstate[i], &got);
            }
            // reset with the other library
            (r.strreset)(&mut mixa);
            eqs(
                &format!("arena-interop {name} reset"),
                &refstate[strings.len()],
                &snap_arena(&mixa),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C68 — array grown by C, grown further and freed by Rust and vice versa
// ---------------------------------------------------------------------------
#[test]
fn c68_array_interop() {
    let _g = lock();
    let (c, r) = both();
    for &es in &[1usize, 4, 8, 16, 24, 32, 64] {
        for (name, pick) in interleavings() {
            unsafe {
                // reference: all C
                let mut refp: *mut c_void = std::ptr::null_mut();
                let mut refstate = Vec::new();
                for n in 0..80usize {
                    if refp.is_null() || (*header(refp)).length + 1 > (*header(refp)).capacity {
                        refp = (c.arrgrowf)(refp, es, 1, 0);
                    }
                    for k in 0..es {
                        *(refp as *mut u8).add(es * n + k) = (n as u8).wrapping_add(k as u8);
                    }
                    (*header(refp)).length += 1;
                    refstate.push(format!(
                        "{} data={}",
                        snap_hdr(refp),
                        hex(refp as *const u8, es * (n + 1))
                    ));
                }
                (c.arrfreef)(refp);

                // mixed
                let mut p: *mut c_void = std::ptr::null_mut();
                for n in 0..80usize {
                    let lib = if pick(n) { c } else { r };
                    if p.is_null() || (*header(p)).length + 1 > (*header(p)).capacity {
                        p = (lib.arrgrowf)(p, es, 1, 0);
                    }
                    for k in 0..es {
                        *(p as *mut u8).add(es * n + k) = (n as u8).wrapping_add(k as u8);
                    }
                    (*header(p)).length += 1;
                    eqs(
                        &format!("array-interop es={es} {name} n={n}"),
                        &refstate[n],
                        &format!("{} data={}", snap_hdr(p), hex(p as *const u8, es * (n + 1))),
                    );
                }
                // freed by the other library
                (r.arrfreef)(p);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-library hash agreement: the same key must hash identically, otherwise
// none of the above could work.
// ---------------------------------------------------------------------------
#[test]
fn c66_c68_hash_agreement_under_shared_tables() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x1010_1010);
    unsafe {
        for _ in 0..5000 {
            let n = rng.below(40) as usize;
            let buf = rng.cstring(n);
            let seed = rng.next_u64() as usize;
            assert_eq!(
                (c.hash_string)(buf.as_ptr() as *mut c_char, seed),
                (r.hash_string)(buf.as_ptr() as *mut c_char, seed)
            );
            assert_eq!(
                (c.hash_bytes)(buf.as_ptr() as *mut c_void, buf.len(), seed),
                (r.hash_bytes)(buf.as_ptr() as *mut c_void, buf.len(), seed)
            );
        }
    }
}
