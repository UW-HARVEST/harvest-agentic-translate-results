//! CONFIGS.md A25 — the hash-seed axis, which needs a process that has *not*
//! already pinned the seed (`hashtable_seed` is one-shot), hence its own test
//! binary.
mod common;

use common::*;

fn load_fresh() -> (&'static Api, &'static Api) {
    let root = workspace_root();
    let c: &'static Api = Box::leak(Box::new(Api::load(
        &root.join("c_src/build/libjansson.so"),
        "C",
    )));
    let r: &'static Api = Box::leak(Box::new(Api::load(
        &root.join("translation/target/release/libjansson.so"),
        "Rust",
    )));
    (c, r)
}

/// Everything in one test: the seed can only be set once per process.
#[test]
fn a25_seed_autoseed_then_explicit_is_a_noop_and_output_is_seed_independent() {
    let (c, r) = load_fresh();
    unsafe {
        // Before any object exists the seed is 0 on both sides.
        assert_eq!(*c.hashtable_seed, 0, "C seed should start at 0");
        assert_eq!(*r.hashtable_seed, 0, "Rust seed should start at 0");

        // json_object() autoseeds via json_object_seed(0).
        let oc = (c.json_object)();
        let or = (r.json_object)();
        let sc = *c.hashtable_seed;
        let sr = *r.hashtable_seed;
        assert_ne!(sc, 0, "C autoseed produced 0");
        assert_ne!(sr, 0, "Rust autoseed produced 0");

        // A later json_object_seed(n) must be ignored by both (seed != 0).
        (c.json_object_seed)(0xABCD_1234);
        (r.json_object_seed)(0xABCD_1234);
        assert_eq!(*c.hashtable_seed, sc, "C seed changed after autoseed");
        assert_eq!(*r.hashtable_seed, sr, "Rust seed changed after autoseed");
        (c.json_object_seed)(0);
        (r.json_object_seed)(0);
        assert_eq!(*c.hashtable_seed, sc);
        assert_eq!(*r.hashtable_seed, sr);

        // The two libraries now hold DIFFERENT random seeds.  Nothing the public
        // API exposes may depend on the seed: iteration follows the insertion
        // (ordered) list, so every observable result must still match.
        let mut rng = Rng::new(0xA25);
        for _ in 0..300 {
            let n = 1 + rng.below(40);
            let keys: Vec<String> = (0..n).map(|_| rng.ascii_string(8)).collect();
            let vals: Vec<i64> = (0..n).map(|_| rng.i64()).collect();
            let mut outs = Vec::new();
            for api in [c, r] {
                let o = (api.json_object)();
                for (k, v) in keys.iter().zip(vals.iter()) {
                    (api.json_object_set_new)(o, cstr(k).as_ptr(), (api.json_integer)(*v));
                }
                // insertion order, sorted order, sizes, lookups
                let mut trace = String::new();
                trace.push_str(&format!("{:?}", dumps(api, o, 0)));
                trace.push_str(&format!("{:?}", dumps(api, o, JSON_SORT_KEYS)));
                trace.push_str(&format!("size={}", (api.json_object_size)(o)));
                let mut it = (api.json_object_iter)(o);
                while !it.is_null() {
                    let kl = (api.json_object_iter_key_len)(it);
                    let kp = (api.json_object_iter_key)(it);
                    trace.push_str(&String::from_utf8_lossy(std::slice::from_raw_parts(
                        kp as *const u8,
                        kl,
                    )));
                    trace.push(';');
                    it = (api.json_object_iter_next)(o, it);
                }
                for k in &keys {
                    let g = (api.json_object_get)(o, cstr(k).as_ptr());
                    trace.push_str(&format!("{},", (api.json_integer_value)(g)));
                }
                // deletion order too
                for k in keys.iter().step_by(3) {
                    (api.json_object_del)(o, cstr(k).as_ptr());
                }
                trace.push_str(&format!("{:?}", dumps(api, o, 0)));
                outs.push(trace);
                decref(api, o);
            }
            assert_eq!(
                outs[0], outs[1],
                "output depended on the hash seed (C seed={sc:#x}, Rust seed={sr:#x})"
            );
        }

        // Round-tripping documents through the parser must also be seed-independent.
        for t in corpus() {
            let z = cstr(&t);
            let jc = (c.json_loads)(z.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
            let jr = (r.json_loads)(z.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
            for f in [0usize, JSON_SORT_KEYS, json_indent(2)] {
                assert_eq!(
                    dumps(c, jc, f | JSON_ENCODE_ANY),
                    dumps(r, jr, f | JSON_ENCODE_ANY),
                    "seed-dependent parse/dump for {t:?}"
                );
            }
            decref(c, jc);
            decref(r, jr);
        }

        decref(c, oc);
        decref(r, or);
    }
}
