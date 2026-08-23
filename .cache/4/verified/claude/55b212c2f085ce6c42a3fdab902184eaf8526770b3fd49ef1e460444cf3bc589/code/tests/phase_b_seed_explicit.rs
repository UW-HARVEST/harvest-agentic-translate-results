//! Phase B — CONFIGS.md row 131a: `json_object_seed(n)` as the *very first*
//! call installs exactly `n`, and a later call is a no-op.
//!
//! This needs its own test binary because `hashtable_seed` and the
//! `seed_initialized` flag are process-global one-shot state.

mod common;
use common::*;

#[test]
fn cfg131a_explicit_first_seed() {
    let c = Api::load(&c_so_path(), "C");
    let rust = Api::load(&rust_so_path(), "RUST");
    for api in [&c, &rust] {
        unsafe {
            assert_eq!(*api.hashtable_seed, 0, "[{}] fresh seed must be 0", api.tag);
            (api.json_object_seed)(0x0BAD_C0DE);
            assert_eq!(
                *api.hashtable_seed, 0x0BAD_C0DE,
                "[{}] explicit seed not installed",
                api.tag
            );
            // a second call must not change anything
            (api.json_object_seed)(0x1234_5678);
            assert_eq!(*api.hashtable_seed, 0x0BAD_C0DE, "[{}] reseeded", api.tag);
            (api.json_object_seed)(0);
            assert_eq!(*api.hashtable_seed, 0x0BAD_C0DE, "[{}] reseeded", api.tag);
        }
    }
    // truncation of a seed wider than uint32_t is identical on both sides
    assert_eq!(
        unsafe { *c.hashtable_seed },
        unsafe { *rust.hashtable_seed },
        "seed mismatch"
    );
    // ... and objects built afterwards behave identically
    let mut rc = Rec::new();
    let mut rr = Rec::new();
    for (api, rec) in [(&c, &mut rc), (&rust, &mut rr)] {
        unsafe {
            let o = (api.json_object)();
            for i in 0..40i64 {
                let k = cs(&format!("k{i}"));
                (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i));
            }
            match dumps(api, o, JSON_SORT_KEYS) {
                None => rec.line("dump=NULL"),
                Some(d) => rec.tag_bytes("dump", &d),
            }
            match dumps(api, o, 0) {
                None => rec.line("dump2=NULL"),
                Some(d) => rec.tag_bytes("dump2", &d),
            }
            decref(api, o);
        }
    }
    assert_eq!(rc.out, rr.out, "object built with an explicit seed differs");
}
