mod harness;

use harness::*;

/// Independent re-implementation of the C accumulation, transcribed straight
/// from `c_src/src/lib.c`.  Used only to validate the harness's model of the C
/// (tap indices, coefficients, evaluation order) -- never as the oracle.
fn model(nch: i32, z: &[f32]) -> (i16, isize, i16) {
    fn scale(sample: f32) -> i16 {
        if sample as f64 >= 32766.5 {
            return 32767;
        }
        if sample as f64 <= -32767.5 {
            return -32768;
        }
        let s = ((sample + 0.5f32) as i32) as i16;
        s.wrapping_sub((s < 0) as i16)
    }
    let mut a = (z[14 * 64] - z[0]) * 29.0;
    a += (z[1 * 64] + z[13 * 64]) * 213.0;
    a += (z[12 * 64] - z[2 * 64]) * 459.0;
    a += (z[3 * 64] + z[11 * 64]) * 2037.0;
    a += (z[10 * 64] - z[4 * 64]) * 5153.0;
    a += (z[5 * 64] + z[9 * 64]) * 6574.0;
    a += (z[8 * 64] - z[6 * 64]) * 37489.0;
    a += z[7 * 64] * 75038.0;
    let out0 = scale(a);

    let z = &z[2..];
    let mut a = z[14 * 64] * 104.0;
    a += z[12 * 64] * 1567.0;
    a += z[10 * 64] * 9727.0;
    a += z[8 * 64] * 64019.0;
    a += z[6 * 64] * -9975.0;
    a += z[4 * 64] * -45.0;
    a += z[2 * 64] * 146.0;
    a += z[0 * 64] * -5.0;
    let out1 = scale(a);
    (out0, 16i32.wrapping_mul(nch) as isize, out1)
}

#[test]
fn smoke_both_libraries_load_and_agree() {
    eprintln!("C   : {}", c_library_path().display());
    eprintln!("RUST: {}", rust_library_path().display());
    let mut rng = Rng::new(1);
    for _ in 0..1000 {
        let z = z_from(|_| rng.sym(1.0));
        assert_same("smoke", 2, &z);
    }
}

/// Confirms the harness's tap/coefficient/order model equals the real C, so the
/// accumulator-targeting helpers can be trusted.
#[test]
fn harness_model_matches_c_library() {
    let mut rng = Rng::new(7);
    let c = c_synth_pair();
    for it in 0..4000 {
        let z = match it % 4 {
            0 => z_from(|_| rng.sym(1.0)),
            1 => z_from(|_| rng.log_uniform(-2.0, 3.0)),
            2 => z_from(|_| rng.any_bits_f32()),
            _ => z_from(|_| rng.sym(1e5)),
        };
        let nch = *rng.pick(&[0i32, 1, 2, 3, -1, -4]);
        let mut buf = PcmBuf::for_nch(nch);
        let base = buf.base;
        unsafe { c(buf.ptr(), nch, z.as_ptr()) };
        let (m0, off, m1) = model(nch, &z);
        let got0 = buf.data[base];
        let got1 = buf.data[(base as isize + off) as usize];
        // When nch == 0 the second store overwrites the first.
        if off == 0 {
            assert_eq!(got1, m1, "iter {it}: aliased store");
        } else {
            assert_eq!(got0, m0, "iter {it}: pcm[0]");
            assert_eq!(got1, m1, "iter {it}: pcm[16*nch]");
        }
    }
}

/// Feasibility probe for the exact-accumulator targeting used by Phase C.
#[test]
fn probe_exact_targets_reachable() {
    let targets: Vec<f32> = vec![
        32766.5,
        -32767.5,
        nudge(32766.5, -1),
        nudge(-32767.5, 1),
        0.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        32766.0,
        -32767.0,
        -0.25,
        0.25,
    ];
    for t in targets {
        for chain in [Chain::Lo, Chain::Hi] {
            match find_single_tap_exact(chain, t) {
                Some((idx, v)) => {
                    let a = single_tap_accumulator(chain, idx, v);
                    assert_eq!(a.to_bits(), t.to_bits(), "targeting {t:e} on {chain:?}");
                    eprintln!("{chain:?} target {t:e} <- z[{idx}] = {v:e}");
                }
                None => eprintln!("{chain:?} target {t:e}: UNREACHABLE"),
            }
        }
    }
}
