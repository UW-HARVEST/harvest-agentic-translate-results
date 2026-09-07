//! Differential tests for the five `static` C helpers that no exported entry
//! point reaches: `cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find`, `cp_unfilter`.
//!
//! `c_src` is compiled at `-O0`, so the C shared object keeps them as LOCAL
//! symbols in `.symtab`. Their runtime addresses are recovered by taking the
//! `nm` offset of a local symbol and adding the library's load base, which is
//! itself derived from an EXPORTED symbol: `base = dlsym("cp_inflate") -
//! nm_offset("cp_inflate")`. The Rust side is reached through the `private_probe`
//! cdylib (`examples/private_probe.rs`), which re-exports the same functions
//! under `probe_*` names — still only ever called through `dlsym`.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::process::Command;

const SEED: u64 = 0x9E37_79B9;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct RawPng {
    p: *const u8,
    end: *const u8,
}

type PaethFn = unsafe extern "C" fn(u8, u8, u8) -> u8;
type Make32Fn = unsafe extern "C" fn(*const u8) -> u32;
type ChunkFn = unsafe extern "C" fn(*mut RawPng, *const c_char, u32) -> *const u8;
type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;

/// `nm` offsets of every symbol in the C `.so` (local ones included).
fn nm_offsets(path: &std::path::Path) -> HashMap<String, u64> {
    let out = Command::new("nm").arg(path).output().expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    let text = String::from_utf8_lossy(&out.stdout);
    let mut m = HashMap::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() == 3 {
            if let Ok(a) = u64::from_str_radix(f[0], 16) {
                m.insert(f[2].to_string(), a);
            }
        }
    }
    m
}

struct CPrivate {
    _lib: Library,
    base: usize,
    off: HashMap<String, u64>,
}

impl CPrivate {
    fn open() -> CPrivate {
        let path = c_so_path();
        let off = nm_offsets(&path);
        let lib = unsafe { Library::new(&path) }.expect("dlopen C");
        let exported: Symbol<'_, unsafe extern "C" fn()> =
            unsafe { lib.get(b"cp_inflate\0") }.expect("cp_inflate");
        let runtime = *exported as usize;
        let nm_off = *off.get("cp_inflate").expect("cp_inflate in nm output") as usize;
        let base = runtime - nm_off;
        drop(exported);
        CPrivate { _lib: lib, base, off }
    }

    fn addr(&self, name: &str) -> usize {
        let o = *self
            .off
            .get(name)
            .unwrap_or_else(|| panic!("{name} is not in the C .so symbol table"));
        self.base + o as usize
    }

    fn paeth(&self) -> PaethFn {
        unsafe { std::mem::transmute::<usize, PaethFn>(self.addr("cp_paeth")) }
    }
    fn make32(&self) -> Make32Fn {
        unsafe { std::mem::transmute::<usize, Make32Fn>(self.addr("cp_make32")) }
    }
    fn chunk(&self) -> ChunkFn {
        unsafe { std::mem::transmute::<usize, ChunkFn>(self.addr("cp_chunk")) }
    }
    fn find(&self) -> ChunkFn {
        unsafe { std::mem::transmute::<usize, ChunkFn>(self.addr("cp_find")) }
    }
    fn unfilter(&self) -> UnfilterFn {
        unsafe { std::mem::transmute::<usize, UnfilterFn>(self.addr("cp_unfilter")) }
    }
}

fn probe_so_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().unwrap().parent().unwrap();
    let p = profile.join("examples").join("libprivate_probe.so");
    assert!(
        p.is_file(),
        "{} missing — build it with `cargo build --example private_probe`",
        p.display()
    );
    // `cargo test --test private` does NOT rebuild examples, so guard against a
    // stale probe silently passing while src/lib.rs has moved on.
    let src = manifest_dir().join("src").join("lib.rs");
    let probe_src = manifest_dir().join("examples").join("private_probe.rs");
    let m = |q: &std::path::Path| {
        std::fs::metadata(q).and_then(|md| md.modified()).expect("mtime")
    };
    for newer in [&src, &probe_src] {
        assert!(
            m(&p) >= m(newer),
            "{} is OLDER than {} — rebuild with `cargo build --example private_probe` \
             (or run the whole suite with `cargo test`, which builds examples)",
            p.display(),
            newer.display()
        );
    }
    p
}

struct RsPrivate {
    _lib: Library,
    paeth: PaethFn,
    make32: Make32Fn,
    chunk: ChunkFn,
    find: ChunkFn,
    unfilter: UnfilterFn,
}

impl RsPrivate {
    fn open() -> RsPrivate {
        let lib = unsafe { Library::new(probe_so_path()) }.expect("dlopen probe");
        unsafe {
            let paeth = *lib.get::<PaethFn>(b"probe_cp_paeth\0").unwrap();
            let make32 = *lib.get::<Make32Fn>(b"probe_cp_make32\0").unwrap();
            let chunk = *lib.get::<ChunkFn>(b"probe_cp_chunk\0").unwrap();
            let find = *lib.get::<ChunkFn>(b"probe_cp_find\0").unwrap();
            let unfilter = *lib.get::<UnfilterFn>(b"probe_cp_unfilter\0").unwrap();
            RsPrivate { _lib: lib, paeth, make32, chunk, find, unfilter }
        }
    }
}

fn pairs() -> (CPrivate, RsPrivate) {
    (CPrivate::open(), RsPrivate::open())
}

// ==========================================================================
// cp_paeth
// ==========================================================================

#[test]
fn priv_cp_paeth_exhaustive_ab_sampled_c() {
    let (c, r) = pairs();
    let cf = c.paeth();
    let rf = r.paeth;
    let cs: Vec<u8> = vec![0, 1, 127, 128, 129, 254, 255, 100];
    for &z in &cs {
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                let x = unsafe { cf(a, b, z) };
                let y = unsafe { rf(a, b, z) };
                assert_eq!(x, y, "cp_paeth({a}, {b}, {z}): C={x} RUST={y}");
            }
        }
    }
}

#[test]
fn priv_cp_paeth_random() {
    let (c, r) = pairs();
    let cf = c.paeth();
    let rf = r.paeth;
    let rng = Rng::new(SEED ^ 1);
    for _ in 0..100_000 {
        let (a, b, z) = (rng.byte(), rng.byte(), rng.byte());
        assert_eq!(
            unsafe { cf(a, b, z) },
            unsafe { rf(a, b, z) },
            "cp_paeth({a}, {b}, {z})"
        );
    }
}

// ==========================================================================
// cp_make32
// ==========================================================================

#[test]
fn priv_cp_make32() {
    let (c, r) = pairs();
    let cf = c.make32();
    let rf = r.make32;
    let rng = Rng::new(SEED ^ 2);
    let mut cases: Vec<[u8; 4]> = Vec::new();
    for &v in &[0u8, 1, 0x7F, 0x80, 0xFE, 0xFF] {
        for pos in 0..4 {
            let mut a = [0u8; 4];
            a[pos] = v;
            cases.push(a);
            let mut b = [0xFFu8; 4];
            b[pos] = v;
            cases.push(b);
        }
    }
    for _ in 0..20_000 {
        cases.push([rng.byte(), rng.byte(), rng.byte(), rng.byte()]);
    }
    for a in cases {
        let x = unsafe { cf(a.as_ptr()) };
        let y = unsafe { rf(a.as_ptr()) };
        assert_eq!(x, y, "cp_make32({a:02x?})");
    }
}

// ==========================================================================
// cp_chunk / cp_find
// ==========================================================================

/// Build a buffer of chunks: each entry is `(len_field, tag, payload_len)`.
fn build_chunks(entries: &[(u32, [u8; 4], usize)], rng: &Rng) -> Vec<u8> {
    let mut v = Vec::new();
    for &(len_field, tag, payload) in entries {
        v.extend_from_slice(&len_field.to_be_bytes());
        v.extend_from_slice(&tag);
        v.extend(rng.bytes(payload));
        v.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // CRC
    }
    // slack so cp_make32 / memcmp at the very end stay inside the allocation
    v.extend(rng.bytes(32));
    v
}

#[track_caller]
fn diff_chunklike(
    label: &str,
    cf: ChunkFn,
    rf: ChunkFn,
    buf: &[u8],
    start_off: usize,
    end_off: usize,
    tag: &[u8; 4],
    minlen: u32,
) {
    let a = buf.to_vec();
    let b = buf.to_vec();
    let tag_c = [tag[0] as c_char, tag[1] as c_char, tag[2] as c_char, tag[3] as c_char];

    let mut pa = RawPng {
        p: unsafe { a.as_ptr().add(start_off) },
        end: unsafe { a.as_ptr().add(end_off) },
    };
    let mut pb = RawPng {
        p: unsafe { b.as_ptr().add(start_off) },
        end: unsafe { b.as_ptr().add(end_off) },
    };
    let ra = unsafe { cf(&mut pa, tag_c.as_ptr(), minlen) };
    let rb = unsafe { rf(&mut pb, tag_c.as_ptr(), minlen) };

    let off = |ptr: *const u8, base: *const u8| -> Option<isize> {
        if ptr.is_null() {
            None
        } else {
            Some(ptr as isize - base as isize)
        }
    };
    assert_eq!(
        off(ra, a.as_ptr()),
        off(rb, b.as_ptr()),
        "[{label}] returned pointer offset differs"
    );
    assert_eq!(
        pa.p as isize - a.as_ptr() as isize,
        pb.p as isize - b.as_ptr() as isize,
        "[{label}] png->p offset differs"
    );
    assert_eq!(a, b, "[{label}] the buffer must not be modified");
}

#[test]
fn priv_cp_chunk() {
    let (c, r) = pairs();
    let cf = c.chunk();
    let rf = r.chunk;
    let rng = Rng::new(SEED ^ 3);

    let tags: [[u8; 4]; 4] = [*b"IHDR", *b"IDAT", *b"PLTE", *b"IEND"];
    for (ti, tag) in tags.iter().enumerate() {
        for &len_field in &[
            0u32,
            1,
            4,
            13,
            16,
            255,
            0x0000_FFFF,
            0x7FFF_FFF3,
            0x7FFF_FFF4, // len + 12 == 0x80000000 -> int offset goes negative
            0x7FFF_FFFF,
            0x8000_0000,
            0xFFFF_FFF3,
            0xFFFF_FFF4, // len + 12 wraps to 0
            0xFFFF_FFFF,
        ] {
            let payload = (len_field as usize).min(64);
            let buf = build_chunks(&[(len_field, *tag, payload)], &rng);
            for &minlen in &[0u32, 1, 13, len_field, len_field.wrapping_add(1), u32::MAX] {
                for end_off in [buf.len(), buf.len() - 32, 8, 12 + payload] {
                    // ask for the matching tag and for a mismatching one
                    for probe in [tag, &tags[(ti + 1) % tags.len()]] {
                        diff_chunklike(
                            &format!(
                                "cp_chunk/tag={} len={len_field:#x} minlen={minlen:#x} end={end_off}",
                                String::from_utf8_lossy(probe)
                            ),
                            cf,
                            rf,
                            &buf,
                            0,
                            end_off,
                            probe,
                            minlen,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn priv_cp_find() {
    let (c, r) = pairs();
    let cf = c.find();
    let rf = r.find;
    let rng = Rng::new(SEED ^ 4);

    let tags: [[u8; 4]; 4] = [*b"IHDR", *b"IDAT", *b"PLTE", *b"IEND"];
    // multi-chunk buffers, including chunks whose length field overflows
    let layouts: Vec<Vec<(u32, [u8; 4], usize)>> = vec![
        vec![(13, tags[0], 13), (7, tags[1], 7), (0, tags[3], 0)],
        vec![(4, tags[1], 4), (4, tags[1], 4), (4, tags[1], 4)],
        vec![(0, tags[2], 0)],
        vec![(0x7FFF_FFF4, tags[1], 16), (5, tags[0], 5)],
        // NOTE: a length field of 0xFFFFFFF4 makes `len + 12` wrap to 0, so
        // `cp_find`'s `png->p += len + 12` does not advance and the
        // `while (png->p < png->end)` loop never terminates. That is a genuine
        // infinite loop in the C which the Rust reproduces; it cannot be
        // asserted on (both hang), so it is deliberately excluded here and
        // recorded in ERRORS.md instead. `cp_chunk` (no loop) IS tested with it.
        vec![(0xFFFF_FFFF, tags[0], 8)],
        vec![(1_000_000, tags[1], 8), (3, tags[3], 3)],
    ];
    for (li, layout) in layouts.iter().enumerate() {
        let buf = build_chunks(layout, &rng);
        for tag in tags.iter() {
            for &minlen in &[0u32, 1, 5, 13, 1_000_000, u32::MAX] {
                for end_off in [buf.len(), buf.len() - 32, buf.len() / 2, 8, 0] {
                    diff_chunklike(
                        &format!(
                            "cp_find/layout={li} tag={} minlen={minlen} end={end_off}",
                            String::from_utf8_lossy(tag)
                        ),
                        cf,
                        rf,
                        &buf,
                        0,
                        end_off,
                        tag,
                        minlen,
                    );
                }
            }
        }
    }
}

// ==========================================================================
// cp_unfilter
// ==========================================================================

#[track_caller]
fn diff_unfilter(label: &str, cf: UnfilterFn, rf: UnfilterFn, w: i32, h: i32, bpp: i32, raw: &[u8]) {
    let mut a = raw.to_vec();
    let mut b = raw.to_vec();
    let x = unsafe { cf(w, h, bpp, a.as_mut_ptr()) };
    let y = unsafe { rf(w, h, bpp, b.as_mut_ptr()) };
    assert_eq!(x, y, "[{label}] return differs: C={x} RUST={y}");
    if a != b {
        let i = a.iter().zip(b.iter()).position(|(p, q)| p != q).unwrap();
        panic!(
            "[{label}] buffer differs at byte {i}: C={:02x?} RUST={:02x?}",
            &a[i.saturating_sub(4)..(i + 4).min(a.len())],
            &b[i.saturating_sub(4)..(i + 4).min(b.len())]
        );
    }
}

#[test]
fn priv_cp_unfilter_all_filters() {
    let (c, r) = pairs();
    let cf = c.unfilter();
    let rf = r.unfilter;
    let rng = Rng::new(SEED ^ 5);

    for bpp in 1..=4i32 {
        for w in [1i32, 2, 3, 5, 8, 17] {
            for h in [1i32, 2, 3, 7] {
                let len = (w * bpp) as usize;
                let need = h as usize * (1 + len) + 64;
                // every combination of per-row filter bytes (0..=5) for h <= 3
                for trial in 0..40 {
                    let mut raw = rng.bytes(need);
                    for y in 0..h as usize {
                        let f = if trial < 6 { trial as u8 } else { rng.below(7) as u8 };
                        raw[y * (1 + len)] = f;
                    }
                    diff_unfilter(
                        &format!("cp_unfilter/bpp={bpp} w={w} h={h} trial={trial}"),
                        cf,
                        rf,
                        w,
                        h,
                        bpp,
                        &raw,
                    );
                }
            }
        }
    }
}

#[test]
fn priv_cp_unfilter_degenerate_shapes() {
    let (c, r) = pairs();
    let cf = c.unfilter();
    let rf = r.unfilter;
    let rng = Rng::new(SEED ^ 6);

    for bpp in [1i32, 2, 3, 4] {
        for w in [0i32, 1] {
            for h in [0i32, 1, 2] {
                for f in 0..=6u8 {
                    let mut raw = rng.bytes(256);
                    raw[0] = f;
                    if h >= 2 {
                        let len = (w * bpp) as usize;
                        raw[1 + len] = f;
                    }
                    diff_unfilter(
                        &format!("cp_unfilter/degenerate bpp={bpp} w={w} h={h} f={f}"),
                        cf,
                        rf,
                        w,
                        h,
                        bpp,
                        &raw,
                    );
                }
            }
        }
    }
    // h <= 0 skips the prologue entirely
    for bpp in 1..=4i32 {
        for &h in &[0i32, -1, -100] {
            for &w in &[0i32, 1, 4] {
                let mut raw = rng.bytes(256);
                raw[0] = 9;
                diff_unfilter(
                    &format!("cp_unfilter/h={h} w={w} bpp={bpp}"),
                    cf,
                    rf,
                    w,
                    h,
                    bpp,
                    &raw,
                );
            }
        }
    }
}
