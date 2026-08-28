//! `nm -D` parity: every symbol the C shared object exports must also be
//! exported, under the exact same name, by the Rust shared object.

mod support;

#[test]
fn rust_so_exports_every_c_symbol() {
    let c = support::exported_symbols(&support::c_shared_lib());
    let r = support::exported_symbols(&support::rust_shared_lib());

    assert!(
        c.iter().any(|s| s == "call_predict"),
        "sanity: C .so should export call_predict, got {c:?}"
    );

    let missing: Vec<&String> = c
        .iter()
        // Toolchain-injected bookkeeping symbols are not part of the API.
        .filter(|s| {
            !matches!(
                s.as_str(),
                "_ITM_deregisterTMCloneTable"
                    | "_ITM_registerTMCloneTable"
                    | "__cxa_finalize"
                    | "__gmon_start__"
                    | "_init"
                    | "_fini"
                    | "__bss_start"
                    | "_edata"
                    | "_end"
            ) && !s.starts_with("__cxa_finalize@")
        })
        .filter(|s| !r.contains(s))
        .collect();

    assert!(
        missing.is_empty(),
        "Rust .so is missing exports present in the C .so: {missing:?}\nC: {c:?}\nRust: {r:?}"
    );
}

#[test]
fn c_statics_stay_internal_in_both() {
    // The twelve `_PfnN` helpers, `BTAC1C2_PredictSample` and
    // `BTAC1C2_GetPredictFunc` are `static` in C, so neither library may
    // publish them.
    let c = support::exported_symbols(&support::c_shared_lib());
    let r = support::exported_symbols(&support::rust_shared_lib());

    let mut internal: Vec<String> = vec![
        "BTAC1C2_PredictSample".to_string(),
        "BTAC1C2_GetPredictFunc".to_string(),
    ];
    for n in 0..12 {
        internal.push(format!("BTAC1C2_PredictSample_Pfn{n}"));
    }

    for name in internal {
        assert!(
            !c.contains(&name),
            "sanity: C .so unexpectedly exports {name}"
        );
        assert!(
            !r.contains(&name),
            "Rust .so exports {name}, but the C original keeps it static"
        );
    }
}

#[test]
fn c_so_declares_no_symbol_the_header_promises_but_never_defines() {
    // `include/lib.h` declares `get_predict_func`, which `src/lib.c` never
    // defines. Neither library may export it.
    let c = support::exported_symbols(&support::c_shared_lib());
    let r = support::exported_symbols(&support::rust_shared_lib());
    assert!(!c.iter().any(|s| s == "get_predict_func"));
    assert!(
        !r.iter().any(|s| s == "get_predict_func"),
        "Rust .so exports get_predict_func but the C .so does not"
    );
}
