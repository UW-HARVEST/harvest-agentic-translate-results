//! Supplemental Phase B/C coverage for combinations not owned by a single row:
//! the `_ts` lookup under string mode, and `keysize == 0`.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

/// `stbds_hmget_key_ts` driven in STRING mode over every `string.mode`
/// (the `_ts` variant writes the index through an out-param instead of the
/// array header, so it has its own code path in `hmget_key_ts`).
#[test]
fn sup_hmget_key_ts_string_mode() {
    for mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        run(0x3141_5926, |c, r| unsafe {
            let es = 16usize;
            let ks = 8usize;
            let keys: Vec<Vec<u8>> = (0..45)
                .map(|i| format!("ts_key_{}\0", i).into_bytes())
                .collect();
            let mut mc = (c.shmode_func)(es, mode);
            let mut mr = (r.shmode_func)(es, mode);
            for (i, k) in keys.iter().enumerate() {
                mc = sh_put(c, mc, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), HM_STRING);
                mr = sh_put(r, mr, es, k.as_ptr() as *mut c_char, &(i as u64).to_le_bytes(), HM_STRING);
            }
            // hits
            for (i, k) in keys.iter().enumerate() {
                let mut tc: isize = 0x7777;
                let mut tr: isize = 0x7777;
                let nc = (c.hmget_key_ts)(mc, es, k.as_ptr() as *mut c_void, ks, &mut tc, HM_STRING);
                let nr = (r.hmget_key_ts)(mr, es, k.as_ptr() as *mut c_void, ks, &mut tr, HM_STRING);
                mc = nc;
                mr = nr;
                assert_eq!(tc, tr, "mode={} _ts hit #{}", mode, i);
                assert!(tc >= 0, "_ts must find key #{}", i);
            }
            // misses
            for i in 0..45 {
                let probe = format!("ts_key_MISS_{}\0", i).into_bytes();
                let mut tc: isize = 0x7777;
                let mut tr: isize = 0x7777;
                let nc =
                    (c.hmget_key_ts)(mc, es, probe.as_ptr() as *mut c_void, ks, &mut tc, HM_STRING);
                let nr =
                    (r.hmget_key_ts)(mr, es, probe.as_ptr() as *mut c_void, ks, &mut tr, HM_STRING);
                mc = nc;
                mr = nr;
                assert_eq!(tc, tr, "mode={} _ts miss #{}", mode, i);
                assert_eq!(tc, -1, "_ts miss must be STBDS_INDEX_EMPTY");
            }
            assert_snap_eq(
                &snap_map(mc, es, ElemFmt::KeyPtr, true),
                &snap_map(mr, es, ElemFmt::KeyPtr, true),
                &format!("sup _ts mode={}", mode),
            );
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        });
    }
}

/// `keysize == 0` in binary mode.
///
/// `stbds_hash_bytes(key, 0, seed)` collapses every key to one hash and
/// `memcmp(key, elem, 0) == 0` makes every key compare equal, so *every*
/// subsequent insert is treated as an update of the first element. The C does
/// not special-case this, so the Rust must collapse identically.
#[test]
fn sup_keysize_zero_binary() {
    run(0x3141_5926, |c, r| unsafe {
        for es in [4usize, 8, 16] {
            let mut rng = Rng::new(0xC0DE_1000 + es as u64);
            let mut mc: *mut c_void = std::ptr::null_mut();
            let mut mr: *mut c_void = std::ptr::null_mut();
            for i in 0..30u32 {
                let k = rng.next_u32().to_le_bytes();
                let mut kk = k.to_vec();
                // write the WHOLE element: with keysize == 0 hmput_key memcpys
                // nothing, so any byte we leave alone stays uninitialised
                // realloc memory and would compare as heap noise.
                let pay: Vec<u8> = (0..es).map(|b| (i as u8).wrapping_add(b as u8)).collect();
                let nc = (c.hmput_key)(mc, es, kk.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let nr = (r.hmput_key)(mr, es, kk.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                mc = nc;
                mr = nr;
                let tc = (*header_of(mc, es)).temp;
                let tr = (*header_of(mr, es)).temp;
                assert_eq!(tc, tr, "keysize=0 es={} temp #{}", es, i);
                // write the payload the way the stbds_hmput macro would
                let n = es;
                std::ptr::copy_nonoverlapping(
                    pay.as_ptr(),
                    (mc as *mut u8).offset(tc * es as isize),
                    n,
                );
                std::ptr::copy_nonoverlapping(
                    pay.as_ptr(),
                    (mr as *mut u8).offset(tr * es as isize),
                    n,
                );
                assert_snap_eq(
                    &snap_map(mc, es, ElemFmt::Raw, false),
                    &snap_map(mr, es, ElemFmt::Raw, false),
                    &format!("keysize=0 es={} insert #{}", es, i),
                );
            }
            // everything collapsed onto one element
            assert_eq!(hm_len(mc, es), 1, "keysize=0 must collapse to one element");
            assert_eq!(hm_len(mr, es), hm_len(mc, es));

            // lookups also always "hit" element 0
            for _ in 0..10 {
                let mut k = rng.next_u32().to_le_bytes();
                let nc = (c.hmget_key)(mc, es, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                let nr = (r.hmget_key)(mr, es, k.as_mut_ptr() as *mut c_void, 0, HM_BINARY);
                mc = nc;
                mr = nr;
                assert_eq!((*header_of(mc, es)).temp, (*header_of(mr, es)).temp);
                assert_eq!((*header_of(mc, es)).temp, 0);
            }

            // and deleting once empties the map, twice rejects
            let mut k = [0u8; 4];
            let (m1, tc) = {
                let m = (c.hmdel_key)(mc, es, k.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
                (m, (*header_of(m, es)).temp)
            };
            let (m2, tr) = {
                let m = (r.hmdel_key)(mr, es, k.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
                (m, (*header_of(m, es)).temp)
            };
            mc = m1;
            mr = m2;
            assert_eq!(tc, tr);
            assert_eq!(tc, 1);
            let (m3, tc2) = {
                let m = (c.hmdel_key)(mc, es, k.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
                (m, (*header_of(m, es)).temp)
            };
            let (m4, tr2) = {
                let m = (r.hmdel_key)(mr, es, k.as_mut_ptr() as *mut c_void, 0, 0, HM_BINARY);
                (m, (*header_of(m, es)).temp)
            };
            mc = m3;
            mr = m4;
            assert_eq!(tc2, tr2);
            assert_eq!(tc2, 0, "second delete must reject");
            assert_snap_eq(
                &snap_map(mc, es, ElemFmt::Raw, false),
                &snap_map(mr, es, ElemFmt::Raw, false),
                &format!("keysize=0 es={} final", es),
            );
            hm_free(c, mc, es);
            hm_free(r, mr, es);
        }
    });
}

/// `stbds_hash_bytes` / `stbds_hash_string` called on the SAME buffer through
/// both libraries with unaligned pointers (the SipHash loader reads byte by
/// byte, so alignment must not matter).
#[test]
fn sup_hash_unaligned() {
    run(1, |c, r| unsafe {
        let mut rng = Rng::new(0xC0DE_2000);
        let mut buf = rng.bytes(256);
        for off in 0..17usize {
            for len in 0..64usize {
                let p = buf.as_mut_ptr().add(off) as *mut c_void;
                for s in [0usize, 0x3141_5926, usize::MAX] {
                    assert_eq!(
                        (c.hash_bytes)(p, len, s),
                        (r.hash_bytes)(p, len, s),
                        "unaligned off={} len={} seed={:#x}",
                        off,
                        len,
                        s
                    );
                }
            }
        }
    });
}
