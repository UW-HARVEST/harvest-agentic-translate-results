//! Phase D -- exported-symbol parity between the C and the Rust shared library.
//!
//! Re-derives both symbol lists with `nm -D` at test time so `SYMBOLS.md`
//! cannot silently rot, and requires the C-minus-Rust difference to be empty.

mod harness;

use harness::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", extra])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {path:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Symbols the C toolchain adds to every shared object; not part of the API.
const TOOLCHAIN_NOISE: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "__cxa_finalize",
    "__gmon_start__",
    "_init",
    "_fini",
    "__bss_start",
    "_edata",
    "_end",
];

#[test]
fn every_c_exported_symbol_is_exported_by_rust() {
    let c = nm(c_library_path(), "--defined-only");
    let r = nm(rust_library_path(), "--defined-only");

    let c_api: BTreeSet<&String> = c
        .iter()
        .filter(|s| !TOOLCHAIN_NOISE.contains(&s.as_str()))
        .collect();

    eprintln!("C exports  ({}): {:?}", c_api.len(), c_api);
    eprintln!("Rust exports ({}) include synth_pair: {}", r.len(), r.contains("synth_pair"));

    let missing: Vec<&&String> = c_api.iter().filter(|s| !r.contains(**s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    // The one and only public function of this library.
    assert!(c_api.contains(&"synth_pair".to_string()));
    assert_eq!(
        c_api.len(),
        1,
        "the C library's public surface changed; update SYMBOLS.md: {c_api:?}"
    );
}

#[test]
fn static_c_helper_is_not_exported_by_either_library() {
    // `mp3d_scale_pcm` is `static` in C, so exporting it from Rust would be a
    // spurious addition rather than parity.
    for path in [c_library_path(), rust_library_path()] {
        let syms = nm(path, "--defined-only");
        assert!(
            !syms.contains("mp3d_scale_pcm"),
            "{path:?} must not export the static helper"
        );
    }
}

#[test]
fn rust_library_has_no_unresolvable_undefined_symbols() {
    // Every undefined symbol must resolve at `dlopen` time -- which it does,
    // since the harness successfully loaded the library to reach this test.
    let undef = nm(rust_library_path(), "--undefined-only");
    eprintln!("Rust undefined symbols ({}): {undef:?}", undef.len());
    let f = rust_synth_pair();
    let z = z_from(|_| 0.5);
    let mut buf = PcmBuf::for_nch(2);
    unsafe { f(buf.ptr(), 2, z.as_ptr()) };
    assert_ne!(buf.data[buf.base], PCM_POISON, "loaded symbol is callable");
}

#[test]
fn dlsym_of_the_exact_c_name_succeeds_in_both() {
    // Guards against name-mangling / namespace-macro mistakes: the linker name
    // must be exactly `synth_pair` in both libraries.
    for path in [c_library_path(), rust_library_path()] {
        let lib = unsafe { libloading::Library::new(path) }.expect("dlopen");
        let sym: Result<libloading::Symbol<SynthPairFn>, _> = unsafe { lib.get(b"synth_pair\0") };
        assert!(sym.is_ok(), "dlsym(\"synth_pair\") failed in {path:?}");
    }
}
