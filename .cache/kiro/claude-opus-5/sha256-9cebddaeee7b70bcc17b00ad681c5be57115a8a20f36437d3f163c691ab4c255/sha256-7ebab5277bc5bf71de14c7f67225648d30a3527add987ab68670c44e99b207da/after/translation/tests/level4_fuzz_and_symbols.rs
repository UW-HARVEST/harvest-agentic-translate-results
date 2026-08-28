//! Level 4: whole-API randomized differential fuzzing, plus the exported
//! symbol-table comparison.

mod harness;
use harness::*;

use std::path::{Path, PathBuf};
use std::process::Command;

/// Generates a structurally valid stream with randomized contents: random
/// filler chunks, random field values, `desc` and `pakt` always present before
/// the terminating `data` chunk (the C dereferences both unconditionally).
fn random_valid_stream(rng: &mut Rng) -> Vec<u8> {
    let mut caf = Caf::new().valid_header();

    let n_pre = rng.below(4) as usize;
    for i in 0..n_pre {
        let len = rng.below(40) as usize;
        let ty = [
            0x20 + (rng.next_u64() % 90) as u8,
            0x20 + (rng.next_u64() % 90) as u8,
            b'0' + (i as u8 % 10),
            0x20 + (rng.next_u64() % 90) as u8,
        ];
        // Avoid accidentally emitting a recognised FourCC.
        if &ty == FOURCC_DESC || &ty == FOURCC_PAKT || &ty == FOURCC_DATA {
            continue;
        }
        caf = caf.chunk(&ty, &rng.bytes(len));
    }

    let desc = desc_body(
        rng.next_u64().to_le_bytes(),
        FOURCC_IMA4,
        rng.next_u32(),
        rng.next_u32(),
        rng.next_u32(),
        rng.next_u32(),
        rng.next_u32(),
    );
    let pakt = pakt_body(
        rng.next_u64() as i64,
        rng.next_u64() as i64,
        rng.next_u32() as i32,
        rng.next_u32() as i32,
    );

    // Randomize which of desc/pakt comes first.
    if rng.next_u64() & 1 == 0 {
        caf = caf.chunk(FOURCC_DESC, &desc).chunk(FOURCC_PAKT, &pakt);
    } else {
        caf = caf.chunk(FOURCC_PAKT, &pakt).chunk(FOURCC_DESC, &desc);
    }

    let n_mid = rng.below(3) as usize;
    for i in 0..n_mid {
        let len = rng.below(24) as usize;
        let ty = [b'm', b'd', b'0' + (i as u8 % 10), b'z'];
        caf = caf.chunk(&ty, &rng.bytes(len));
    }

    // The data chunk's declared size becomes `info->size` verbatim, so it need
    // not match the real payload length.
    let payload_len = rng.below(80) as usize + 4;
    let payload = rng.bytes(payload_len);
    let declared = if rng.next_u64() & 3 == 0 {
        rng.next_u64() as i64
    } else {
        payload.len() as i64
    };
    caf = caf.chunk_raw(FOURCC_DATA, declared, &data_body(rng.next_u32(), &payload));
    caf.build()
}

#[test]
fn fuzz_structured_streams() {
    let mut rng = Rng::new(0xF0_02_2E);
    for i in 0..20_000 {
        let bytes = random_valid_stream(&mut rng);
        let out = assert_same(&format!("fuzz structured #{i}"), &bytes);
        assert!(out.ret == 0 || out.ret == -3, "unexpected ret {}", out.ret);
    }
}

#[test]
fn fuzz_structured_streams_unaligned() {
    let mut rng = Rng::new(0xF0_02_2F);
    for i in 0..4000 {
        let bytes = random_valid_stream(&mut rng);
        let skew = (i % 8) as usize;
        assert_same_skew(&format!("fuzz unaligned #{i} skew={skew}"), &bytes, skew);
    }
}

/// Random `format_id` values: mostly the -3 path, occasionally the valid one.
#[test]
fn fuzz_format_id() {
    let mut rng = Rng::new(0xF1_D_1D);
    for i in 0..5000 {
        let mut fid = [0u8; 4];
        if rng.next_u64() % 4 == 0 {
            fid = *FOURCC_IMA4;
            let lane = rng.below(4) as usize;
            fid[lane] = rng.next_u64() as u8;
        } else {
            fid.copy_from_slice(&rng.next_u32().to_be_bytes());
        }
        let desc = desc_body(rng.next_u64().to_le_bytes(), &fid, 0, 0, 0, rng.next_u32(), 0);
        let bytes = Caf::new()
            .valid_header()
            .chunk(FOURCC_DESC, &desc)
            .chunk(FOURCC_PAKT, &pakt_body(0, rng.next_u64() as i64, 0, 0))
            .chunk(FOURCC_DATA, &data_body(0, &[0u8; 34]))
            .build();
        assert_same(&format!("fuzz format_id #{i}"), &bytes);
    }
}

/// Fully random 8-byte headers: exercises the -1 / -2 early returns. Anything
/// that would get past both checks is skipped, since the chunk walk would then
/// run off the end of the buffer in both implementations alike.
#[test]
fn fuzz_random_headers() {
    let mut rng = Rng::new(0x0EAD_1234_u64);
    let tail_desc = desc_body_rate(44100.0, 2);
    let tail_pakt = pakt_body(0, 99, 0, 0);
    let tail_data = data_body(0, &[0u8; 34]);
    for i in 0..20_000 {
        let mut head = rng.bytes(8);
        // Bias toward near-valid headers.
        match rng.below(4) {
            0 => head[0..4].copy_from_slice(FOURCC_CAFF),
            1 => {
                head[0..4].copy_from_slice(FOURCC_CAFF);
                head[4..6].copy_from_slice(&[0, 1]);
            }
            _ => {}
        }
        let bytes = Caf::new()
            .raw(&head)
            .chunk(FOURCC_DESC, &tail_desc)
            .chunk(FOURCC_PAKT, &tail_pakt)
            .chunk(FOURCC_DATA, &tail_data)
            .build();
        assert_same(&format!("fuzz header #{i}"), &bytes);
    }
}

/// Chunk sizes that make the walk jump around inside the buffer, including
/// jumps that land on a later chunk and jumps that revisit ground already
/// covered. Every generated case is pre-validated by a reference walk so the
/// pointer stays inside the buffer and the loop terminates.
#[test]
fn fuzz_chunk_walk_jumps() {
    let mut rng = Rng::new(0x3_0_A_9_5);
    let desc = desc_body_rate(44100.0, 3);
    let pakt = pakt_body(0, 4242, 0, 0);
    let data = data_body(0, &[0u8; 40]);

    let mut checked = 0usize;
    for _ in 0..40_000 {
        // Lay out: header, N filler chunks with randomized declared sizes,
        // desc, pakt, data.
        let n = 1 + rng.below(4) as usize;
        let mut layout: Vec<(usize, i64, usize)> = Vec::new(); // (offset, declared, body_len)
        let mut caf = Caf::new().valid_header();
        let mut off = 8usize;
        for i in 0..n {
            let body_len = (rng.below(6) * 8) as usize;
            // Declared size is usually the body length, sometimes a multiple of
            // 8 that skips ahead or backtracks slightly.
            let declared = match rng.below(4) {
                0 => body_len as i64 + 8 * (rng.below(3) as i64),
                1 => body_len as i64 - 8 * (rng.below(2) as i64),
                _ => body_len as i64,
            };
            layout.push((off, declared, body_len));
            caf = caf.chunk_raw(&[b'j', b'p', b'0' + i as u8, b'!'], declared, &vec![0x33; body_len]);
            off += 16 + body_len;
        }
        let desc_off = off;
        caf = caf.chunk(FOURCC_DESC, &desc);
        off += 16 + desc.len();
        let pakt_off = off;
        caf = caf.chunk(FOURCC_PAKT, &pakt);
        off += 16 + pakt.len();
        let data_off = off;
        caf = caf.chunk(FOURCC_DATA, &data);
        let total = off + 16 + data.len();
        let bytes = caf.build();
        debug_assert_eq!(bytes.len(), total);

        // Reference walk: must reach `data_off` after at most 64 hops, with
        // every visited chunk header fully inside the buffer, and must visit
        // desc and pakt at least once.
        let mut cur = 8usize;
        let mut hops = 0;
        let mut saw_desc = false;
        let mut saw_pakt = false;
        let mut ok = false;
        loop {
            if hops > 64 || cur + 16 > total {
                break;
            }
            if cur == data_off {
                ok = saw_desc && saw_pakt;
                break;
            }
            if cur == desc_off {
                saw_desc = true;
                cur += 16 + desc.len();
            } else if cur == pakt_off {
                saw_pakt = true;
                cur += 16 + pakt.len();
            } else if let Some(&(_, declared, _)) = layout.iter().find(|(o, _, _)| *o == cur) {
                let next = cur as i64 + 16 + declared;
                if next < 8 {
                    break;
                }
                cur = next as usize;
            } else {
                // Landed mid-chunk: the type bytes there are arbitrary, so bail.
                break;
            }
            hops += 1;
        }
        if !ok {
            continue;
        }

        assert_same("fuzz walk", &bytes);
        checked += 1;
    }
    assert!(checked > 200, "only {checked} walk cases were exercised");
}

// ---------------------------------------------------------------------------
// exported symbols
// ---------------------------------------------------------------------------

fn c_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.parent().unwrap().join("c_src").join("build");
    std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .expect("C .so")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap().to_path_buf();
    dir.join("libima_parse_lib.so")
}

/// Global text/data symbols exported through the dynamic symbol table.
fn dynamic_defined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
            // Ignore linker/runtime-provided bookkeeping symbols.
            if name.starts_with("_ITM_")
                || name.starts_with("__gmon")
                || name.starts_with("_init")
                || name.starts_with("_fini")
                || name.starts_with("__cxa")
                || name.starts_with("_edata")
                || name.starts_with("_end")
                || name == "__bss_start"
                || name.starts_with("rust_")
                || name.starts_with("_ZN")
                || name.starts_with("__rust")
                || name.starts_with("_Unwind")
            {
                return None;
            }
            // Weak/undefined-ish kinds are not exports we care to compare.
            match kind {
                "T" | "t" | "D" | "B" | "R" | "W" => Some(name.to_string()),
                _ => None,
            }
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

#[test]
fn rust_so_exports_every_c_symbol() {
    // Make sure the Rust cdylib exists (the harness builds it on demand).
    let _ = rust_ima_parse();

    let c_syms = dynamic_defined_symbols(&c_so());
    let r_syms = dynamic_defined_symbols(&rust_so());

    assert!(
        c_syms.contains(&"ima_parse".to_string()),
        "C .so should export ima_parse; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n\
         C   : {c_syms:?}\nRust: {r_syms:?}"
    );
}
