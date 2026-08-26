//! Quick sanity probe: are the two `.so`s loadable and do the leaf scalar
//! helpers agree (including on NaN payloads / signed zeros)?

#![allow(non_snake_case)]

mod common;
use common::*;

#[test]
fn leaf_scalars_agree() {
    let l = libs();
    let mut rng = Rng::new(1);

    macro_rules! vv_v {
        ($name:literal) => {{
            let fc = l.sym::<unsafe extern "C" fn(c2v, c2v) -> c2v>(Side::C, $name);
            let fr = l.sym::<unsafe extern "C" fn(c2v, c2v) -> c2v>(Side::R, $name);
            for _ in 0..20000 {
                let a = rng.v_mixed(50.0);
                let b = rng.v_mixed(50.0);
                unsafe { assert_same($name, &(a, b), fc(a, b), fr(a, b)) };
            }
        }};
    }
    macro_rules! vv_f {
        ($name:literal) => {{
            let fc = l.sym::<unsafe extern "C" fn(c2v, c2v) -> f32>(Side::C, $name);
            let fr = l.sym::<unsafe extern "C" fn(c2v, c2v) -> f32>(Side::R, $name);
            for _ in 0..20000 {
                let a = rng.v_mixed(50.0);
                let b = rng.v_mixed(50.0);
                unsafe { assert_same($name, &(a, b), fc(a, b), fr(a, b)) };
            }
        }};
    }
    macro_rules! v_v {
        ($name:literal) => {{
            let fc = l.sym::<unsafe extern "C" fn(c2v) -> c2v>(Side::C, $name);
            let fr = l.sym::<unsafe extern "C" fn(c2v) -> c2v>(Side::R, $name);
            for _ in 0..20000 {
                let a = rng.v_mixed(50.0);
                unsafe { assert_same($name, &a, fc(a), fr(a)) };
            }
        }};
    }
    macro_rules! v_f {
        ($name:literal) => {{
            let fc = l.sym::<unsafe extern "C" fn(c2v) -> f32>(Side::C, $name);
            let fr = l.sym::<unsafe extern "C" fn(c2v) -> f32>(Side::R, $name);
            for _ in 0..20000 {
                let a = rng.v_mixed(50.0);
                unsafe { assert_same($name, &a, fc(a), fr(a)) };
            }
        }};
    }

    vv_v!("c2Maxv");
    vv_v!("c2Minv");
    vv_v!("c2Sub");
    vv_v!("c2Add");
    vv_f!("c2Dot");
    vv_f!("c2Det2");
    v_v!("c2Neg");
    v_v!("c2Skew");
    v_v!("c2CCW90");
    v_v!("c2Norm");
    v_f!("c2Len");
}
