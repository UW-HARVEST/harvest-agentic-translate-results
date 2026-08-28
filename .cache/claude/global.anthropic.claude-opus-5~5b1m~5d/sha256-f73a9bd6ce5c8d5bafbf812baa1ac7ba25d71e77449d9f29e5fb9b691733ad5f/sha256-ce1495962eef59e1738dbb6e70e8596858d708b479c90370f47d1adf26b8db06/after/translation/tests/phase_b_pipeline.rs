//! Phase B — CONFIGS.md row 51: the composed pipeline
//! `rand_seed -> shmode_func -> put* -> get/get_ts -> del -> put -> hmfree`
//! over the full cross-product of arena mode × hash mode × element size.

mod common;

use common::*;

fn key(i: u64) -> Vec<u8> {
    // always >= 8 bytes: binary mode hashes/compares exactly `keysize` (8) bytes
    let mut v = format!("pipe-key-{i:06}").into_bytes();
    v.push(0);
    v
}

fn pay(v: u64) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

#[test]
fn full_matrix() {
    let sh_modes = [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA];
    let hm_modes = [HM_BINARY, HM_STRING];
    let sizes = [8usize, 16, 24];

    for &sh in &sh_modes {
        for &mode in &hm_modes {
            for &es in &sizes {
                // SH_NONE + HM_STRING stores raw string bytes where a `char *`
                // is expected, so any key comparison would dereference them
                // (the C behaves the same).  Restricted script: distinct keys,
                // absent lookups only — see CONFIGS.md row 35.
                let dangerous = sh == SH_NONE && mode >= HM_STRING;
                let kind = if mode >= HM_STRING && sh != SH_NONE {
                    KeyKind::StrPtr { keyoffset: 0 }
                } else if mode >= HM_STRING {
                    KeyKind::Binary
                } else if sh == SH_NONE {
                    KeyKind::Binary
                } else {
                    // binary mode, but the switch still stores a `char *`
                    KeyKind::StrPtr { keyoffset: 0 }
                };
                let cfg = MapCfg {
                    elemsize: es,
                    keysize: 8,
                    mode,
                    payload_off: 8,
                    kind,
                    digest: false,
                    temp_key_always: false,
                };

                for trial in 0..20u64 {
                    let mut rng = Rng::new(
                        0x91BE_0000u64.wrapping_add(trial)
                            ^ (sh as u64) << 40
                            ^ (mode as u64) << 32
                            ^ (es as u64) << 24,
                    );
                    let mut ops = vec![Op::ShMode { sh_mode: sh }];
                    let mut next_key = 0u64;
                    for i in 0..120u64 {
                        if dangerous {
                            // only fresh keys and absent lookups
                            match rng.below(4) {
                                0..=2 => {
                                    ops.push(put(&key(next_key), &pay(i)));
                                    next_key += 1;
                                }
                                _ => ops.push(get(&key(1_000_000 + i))),
                            }
                            continue;
                        }
                        let k = rng.below(20);
                        match rng.below(12) {
                            0..=4 => ops.push(put(&key(k), &pay(i))),
                            5..=6 => ops.push(get(&key(k))),
                            7 => ops.push(get_ts(&key(k))),
                            8..=9 => ops.push(del(&key(k))),
                            10 => ops.push(Op::PutDefault { payload: pay(i) }),
                            _ => ops.push(get(&key(500 + k))),
                        }
                    }
                    ops.push(Op::Free);
                    let seed = [DEFAULT_SEED, 0, 1, usize::MAX][(trial as usize) % 4];
                    diff_script(
                        &format!("pipeline sh={sh} mode={mode} es={es} trial={trial}"),
                        seed,
                        cfg,
                        &ops,
                    );
                }
            }
        }
    }
}
