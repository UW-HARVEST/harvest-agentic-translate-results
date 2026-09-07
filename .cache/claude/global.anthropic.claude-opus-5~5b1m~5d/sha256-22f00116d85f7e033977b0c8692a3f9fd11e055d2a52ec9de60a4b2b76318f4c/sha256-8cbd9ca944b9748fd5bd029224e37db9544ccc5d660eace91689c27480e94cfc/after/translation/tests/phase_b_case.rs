//! Phase B — CONFIGS.md rows 31/32/33/46/48/52/55, randomized:
//! `case_insensitive_strcmp` is reached from object lookup and from
//! `cJSON_Compare`, and its bugs only show up for probes that are prefixes of a
//! key, that share a prefix with a key, or that contain upper case letters in
//! the middle.  Those shapes are generated here.

mod common;
use common::*;
use std::ffi::{c_char, c_int};

/// Random key made of letters in random case plus digits and separators.
fn rand_key(rng: &mut Rng, maxlen: usize) -> String {
    let n = 1 + rng.below(maxlen as u64) as usize;
    (0..n)
        .map(|_| match rng.below(10) {
            0..=3 => (b'a' + rng.below(26) as u8) as char,
            4..=7 => (b'A' + rng.below(26) as u8) as char,
            8 => (b'0' + rng.below(10) as u8) as char,
            _ => '_',
        })
        .collect()
}

fn flip_case(rng: &mut Rng, s: &str) -> String {
    s.chars()
        .map(|ch| {
            if rng.below(2) == 0 {
                ch.to_ascii_uppercase()
            } else {
                ch.to_ascii_lowercase()
            }
        })
        .collect()
}

/// Probes derived from a key: exact, case flipped, prefixes, extended, mutated.
fn probes_for(rng: &mut Rng, key: &str) -> Vec<String> {
    let mut v = vec![key.to_string(), flip_case(rng, key)];
    for cut in 0..key.len() {
        v.push(key[..cut].to_string());
        v.push(flip_case(rng, &key[..cut]));
    }
    v.push(format!("{}x", key));
    v.push(format!("{}X", key));
    v.push(flip_case(rng, &format!("{}q", key)));
    if !key.is_empty() {
        let mut m: Vec<char> = key.chars().collect();
        let idx = rng.below(m.len() as u64) as usize;
        m[idx] = (b'a' + rng.below(26) as u8) as char;
        v.push(m.iter().collect());
        let mut m2: Vec<char> = key.chars().collect();
        m2[idx] = m2[idx].to_ascii_uppercase();
        v.push(m2.iter().collect());
    }
    v
}

/// Rows 31, 32, 33, 46, 48, 52 — randomized mixed-case object lookups.
#[test]
fn case_insensitive_object_lookup() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(0xCA5E);
        for _ in 0..120 {
            let nkeys = 1 + rng.below(6) as usize;
            let keys: Vec<String> = (0..nkeys).map(|_| rand_key(&mut rng, 6)).collect();
            let node = Node::Object(
                keys.iter()
                    .enumerate()
                    .map(|(i, k)| (k.clone(), Node::Number(i as f64)))
                    .collect(),
            );

            let mut all_probes: Vec<String> = Vec::new();
            for k in &keys {
                all_probes.extend(probes_for(&mut rng, k));
            }
            for _ in 0..5 {
                all_probes.push(rand_key(&mut rng, 8));
            }
            all_probes.push(String::new());

            let ci = build(&c, &node);
            let ri = build(&r, &node);
            for p in &all_probes {
                let kb = cbytes(p.as_bytes());
                let kp = kb.as_ptr() as *const c_char;

                let a = (c.cJSON_GetObjectItem)(ci, kp);
                let b = (r.cJSON_GetObjectItem)(ri, kp);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "GetObjectItem keys={:?} probe={:?}",
                    keys,
                    p
                );
                if !a.is_null() {
                    assert_eq!(
                        read_cstr((*a).string),
                        read_cstr((*b).string),
                        "GetObjectItem matched key keys={:?} probe={:?}",
                        keys,
                        p
                    );
                    assert_eq!(
                        (*a).valuedouble.to_bits(),
                        (*b).valuedouble.to_bits(),
                        "GetObjectItem matched value keys={:?} probe={:?}",
                        keys,
                        p
                    );
                }

                let a = (c.cJSON_GetObjectItemCaseSensitive)(ci, kp);
                let b = (r.cJSON_GetObjectItemCaseSensitive)(ri, kp);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "GetObjectItemCaseSensitive keys={:?} probe={:?}",
                    keys,
                    p
                );
                if !a.is_null() {
                    assert_eq!(
                        read_cstr((*a).string),
                        read_cstr((*b).string),
                        "GetObjectItemCaseSensitive matched key"
                    );
                }

                assert_eq!(
                    (c.cJSON_HasObjectItem)(ci, kp),
                    (r.cJSON_HasObjectItem)(ri, kp),
                    "HasObjectItem keys={:?} probe={:?}",
                    keys,
                    p
                );
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);

            // Detach / Delete / Replace go through the same comparison
            for p in &all_probes {
                let kb = cbytes(p.as_bytes());
                let kp = kb.as_ptr() as *const c_char;
                for cs_flag in [false, true] {
                    let ci = build(&c, &node);
                    let ri = build(&r, &node);
                    let (dc, dr) = if cs_flag {
                        (
                            (c.cJSON_DetachItemFromObjectCaseSensitive)(ci, kp),
                            (r.cJSON_DetachItemFromObjectCaseSensitive)(ri, kp),
                        )
                    } else {
                        (
                            (c.cJSON_DetachItemFromObject)(ci, kp),
                            (r.cJSON_DetachItemFromObject)(ri, kp),
                        )
                    };
                    assert_eq!(
                        dc.is_null(),
                        dr.is_null(),
                        "Detach keys={:?} probe={:?} cs={}",
                        keys,
                        p,
                        cs_flag
                    );
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, ci)),
                        show(&print_unformatted_and_free(&r, ri)),
                        "Detach remainder keys={:?} probe={:?} cs={}",
                        keys,
                        p,
                        cs_flag
                    );
                    (c.cJSON_Delete)(dc);
                    (r.cJSON_Delete)(dr);
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);

                    let ci = build(&c, &node);
                    let ri = build(&r, &node);
                    let ct = (c.cJSON_CreateString)(cbytes(b"NEW").as_ptr() as *const c_char);
                    let rt = (r.cJSON_CreateString)(cbytes(b"NEW").as_ptr() as *const c_char);
                    let (rc, rr) = if cs_flag {
                        (
                            (c.cJSON_ReplaceItemInObjectCaseSensitive)(ci, kp, ct),
                            (r.cJSON_ReplaceItemInObjectCaseSensitive)(ri, kp, rt),
                        )
                    } else {
                        (
                            (c.cJSON_ReplaceItemInObject)(ci, kp, ct),
                            (r.cJSON_ReplaceItemInObject)(ri, kp, rt),
                        )
                    };
                    assert_eq!(
                        rc, rr,
                        "Replace keys={:?} probe={:?} cs={}",
                        keys, p, cs_flag
                    );
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, ci)),
                        show(&print_unformatted_and_free(&r, ri)),
                        "Replace result keys={:?} probe={:?} cs={}",
                        keys,
                        p,
                        cs_flag
                    );
                    if rc == 0 {
                        (c.cJSON_Delete)(ct);
                        (r.cJSON_Delete)(rt);
                    }
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

/// Row 55 — randomized mixed-case object comparison.
#[test]
fn case_insensitive_compare() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(0xC0FFEE);
        for _ in 0..400 {
            let nkeys = 1 + rng.below(4) as usize;
            let keys: Vec<String> = (0..nkeys).map(|_| rand_key(&mut rng, 6)).collect();
            let a = Node::Object(
                keys.iter()
                    .enumerate()
                    .map(|(i, k)| (k.clone(), Node::Number(i as f64)))
                    .collect(),
            );
            // b: same values, keys perturbed in case / length / order
            let mut bkeys: Vec<String> = keys
                .iter()
                .map(|k| match rng.below(6) {
                    0 => k.clone(),
                    1 => flip_case(&mut rng, k),
                    2 => k.to_ascii_uppercase(),
                    3 => k.to_ascii_lowercase(),
                    4 => format!("{}x", k),
                    _ => {
                        if k.len() > 1 {
                            k[..k.len() - 1].to_string()
                        } else {
                            k.clone()
                        }
                    }
                })
                .collect();
            if rng.below(2) == 0 {
                bkeys.reverse();
            }
            let b = Node::Object(
                bkeys
                    .iter()
                    .enumerate()
                    .map(|(i, k)| (k.clone(), Node::Number(i as f64)))
                    .collect(),
            );
            let ca = build(&c, &a);
            let cb = build(&c, &b);
            let ra = build(&r, &a);
            let rb = build(&r, &b);
            for cs_flag in [0i32, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ca, cb, cs_flag),
                    (r.cJSON_Compare)(ra, rb, cs_flag),
                    "Compare keys={:?} vs {:?} cs={}",
                    keys,
                    bkeys,
                    cs_flag
                );
                assert_eq!(
                    (c.cJSON_Compare)(cb, ca, cs_flag),
                    (r.cJSON_Compare)(rb, ra, cs_flag),
                    "Compare reversed keys={:?} vs {:?} cs={}",
                    keys,
                    bkeys,
                    cs_flag
                );
            }
            (c.cJSON_Delete)(ca);
            (c.cJSON_Delete)(cb);
            (r.cJSON_Delete)(ra);
            (r.cJSON_Delete)(rb);
        }
        let _: c_int = 0;
    }
}

/// Row 55/56 — `cJSON_Compare` over string and raw values that share prefixes
/// (the `strcmp` in the `cJSON_String`/`cJSON_Raw` arm).
#[test]
fn compare_strings_with_shared_prefixes() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(0x5712);
        let mut pool: Vec<String> = vec![
            String::new(),
            "a".into(),
            "ab".into(),
            "abc".into(),
            "abd".into(),
            "abcd".into(),
            "abce".into(),
            "A".into(),
            "Ab".into(),
            "aB".into(),
            "abC".into(),
            "aaa".into(),
            "aab".into(),
            "zzz".into(),
            "\n".into(),
            "\t".into(),
            "a\nb".into(),
            "a\tb".into(),
        ];
        for _ in 0..60 {
            let base = rand_key(&mut rng, 6);
            pool.push(base.clone());
            pool.push(format!("{}x", base));
            pool.push(format!("{}y", base));
            if base.len() > 1 {
                pool.push(base[..base.len() - 1].to_string());
            }
            pool.push(flip_case(&mut rng, &base));
        }

        for i in 0..pool.len() {
            for j in 0..pool.len() {
                // only a strided subset of the full cross product, but every
                // element still participates many times
                if (i * 7 + j * 13) % 5 != 0 {
                    continue;
                }
                for kind in 0..4 {
                    let mk = |s: &String| -> Node {
                        match kind {
                            0 => Node::Str(s.clone()),
                            1 => Node::Raw(s.clone()),
                            2 => Node::Array(vec![Node::Str(s.clone())]),
                            _ => Node::Object(vec![("k".into(), Node::Str(s.clone()))]),
                        }
                    };
                    // cJSON_CreateRaw("") is valid; skip nothing
                    let na = mk(&pool[i]);
                    let nb = mk(&pool[j]);
                    let ca = build(&c, &na);
                    let cb = build(&c, &nb);
                    let ra = build(&r, &na);
                    let rb = build(&r, &nb);
                    for cs_flag in [0i32, 1] {
                        assert_eq!(
                            (c.cJSON_Compare)(ca, cb, cs_flag),
                            (r.cJSON_Compare)(ra, rb, cs_flag),
                            "Compare kind={} {:?} vs {:?} cs={}",
                            kind,
                            pool[i],
                            pool[j],
                            cs_flag
                        );
                    }
                    (c.cJSON_Delete)(ca);
                    (c.cJSON_Delete)(cb);
                    (r.cJSON_Delete)(ra);
                    (r.cJSON_Delete)(rb);
                }
            }
        }
    }
}
