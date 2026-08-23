//! Phase B — CONFIGS.md row 131b: autoseeding.  `json_object()` on a freshly
//! loaded library calls `json_object_seed(0)`, which derives a non-zero seed
//! from /dev/urandom (or the timestamp+pid fallback).
//!
//! The value itself is random, so only its *properties* can be compared.

mod common;
use common::*;

#[test]
fn cfg131b_autoseed() {
    let c = Api::load(&c_so_path(), "C");
    let rust = Api::load(&rust_so_path(), "RUST");
    let mut rc = Rec::new();
    let mut rr = Rec::new();
    for (api, rec) in [(&c, &mut rc), (&rust, &mut rr)] {
        unsafe {
            assert_eq!(*api.hashtable_seed, 0, "[{}] fresh seed must be 0", api.tag);
            // creating an object triggers the autoseed
            let o = (api.json_object)();
            assert!(!o.is_null());
            let seeded = *api.hashtable_seed;
            assert_ne!(seeded, 0, "[{}] autoseed produced 0", api.tag);
            // a later explicit seed must be ignored
            (api.json_object_seed)(0x4242_4242);
            assert_eq!(*api.hashtable_seed, seeded, "[{}] reseeded", api.tag);
            (api.json_object_seed)(0);
            assert_eq!(*api.hashtable_seed, seeded, "[{}] reseeded", api.tag);

            // the observable behaviour must not depend on the seed value
            for i in 0..40i64 {
                let k = cs(&format!("key{i}"));
                (api.json_object_set_new)(o, k.as_ptr(), (api.json_integer)(i));
            }
            rec.tag_u("size", (api.json_object_size)(o));
            match dumps(api, o, JSON_SORT_KEYS) {
                None => rec.line("dump=NULL"),
                Some(d) => rec.tag_bytes("dump", &d),
            }
            // insertion order is seed independent
            match dumps(api, o, 0) {
                None => rec.line("dump2=NULL"),
                Some(d) => rec.tag_bytes("dump2", &d),
            }
            for i in (0..40i64).step_by(3) {
                let k = cs(&format!("key{i}"));
                rec.tag_i("del", (api.json_object_del)(o, k.as_ptr()) as i64);
            }
            match dumps(api, o, 0) {
                None => rec.line("dump3=NULL"),
                Some(d) => rec.tag_bytes("dump3", &d),
            }
            decref(api, o);
        }
    }
    assert_eq!(rc.out, rr.out, "autoseeded object behaviour differs");
}
