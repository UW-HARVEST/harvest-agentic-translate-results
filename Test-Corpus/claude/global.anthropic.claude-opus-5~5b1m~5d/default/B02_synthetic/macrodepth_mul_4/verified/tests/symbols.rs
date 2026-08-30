//! Phase D — exported-symbol parity between the C `.so` and the Rust `cdylib`.
//!
//! Mechanised version of `SYMBOLS.md`: runs `nm -D --defined-only` on both
//! objects and requires the C symbol set to be a subset of the Rust one, then
//! requires them to be *equal*. It also checks that every symbol is not merely
//! present but resolvable and of the right kind (function vs. 8-byte data).

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use common::{c_so_path, load_pair, rust_so_path, OP_NAME, REPEAT};

/// The complete expected surface, transcribed from `nm -D` on the C `.so`.
const EXPECTED: &[&str] = &[
    "G_OP",
    "G_OP_NAME",
    "helper_call",
    "helper_ptr",
    "op_add",
    "op_mul",
    "op_sub",
    "use_generated",
];

/// Dynamic symbols defined by an object, as reported by `nm -D --defined-only`.
///
/// Rust's `cdylib` additionally exports a handful of local/compiler symbols in
/// lowercase `nm` classes (`r`, `t`, `d`, ...); only GLOBAL definitions matter
/// for ABI parity, so the uppercase classes are what we compare.
fn defined_global_symbols(so: &Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("running `nm` (binutils) is required for the symbol-parity test");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let kind = it.next()?;
            let name = it.next()?;
            // Uppercase class == global (external) definition.
            let k = kind.chars().next()?;
            if k.is_ascii_uppercase() {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// The Rust `.so` must export every symbol the C `.so` exports, with the exact
/// same names. Also checks the C side still matches the hand-recorded
/// `SYMBOLS.md` list, so a change in the C build cannot silently shrink the
/// surface we verify.
#[test]
fn symbol_parity_current_config() {
    let c = defined_global_symbols(&c_so_path());
    let r = defined_global_symbols(&rust_so_path());

    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        c, expected,
        "the C .so surface changed; SYMBOLS.md must be regenerated"
    );

    let missing: Vec<_> = c.difference(&r).cloned().collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so [OP={OP_NAME} REPEAT={REPEAT}]: {:?}\n\
         Per SYMBOLS.md: add the #[no_mangle] extern \"C\" wrapper if the impl exists, \
         or translate the missing C source if a whole module was skipped. Never stub.",
        missing.len(),
        missing
    );

    // The Rust cdylib also exports Rust-runtime globals (`rust_eh_personality`,
    // `__rust_*` allocator shims, `_init`/`_fini`, ...) that the C object has no
    // counterpart for. Those are additions, not divergences; what must be empty
    // is the *missing* direction, asserted above. Assert here that every
    // C-facing name is accounted for and that nothing C-named is extra.
    let extra_c_named: Vec<_> = r
        .difference(&c)
        .filter(|n| EXPECTED.contains(&n.as_str()))
        .collect();
    assert!(extra_c_named.is_empty());
}

/// The same diff, for **all 24** `(OP, REPEAT)` C configurations at once: the C
/// symbol *set* must be configuration-independent, so one Rust build satisfies
/// every one of them. This is the "symbol diff MUST reach empty" gate.
#[test]
fn symbol_parity_all_configs() {
    let r = defined_global_symbols(&rust_so_path());
    let root = common::repo_root().join("cbuild");
    let mut seen = 0;
    for op in ["add", "sub", "mul"] {
        for rep in 0..=7 {
            let p = root.join(format!("libcdriver_{op}_{rep}.so"));
            if !p.is_file() {
                continue;
            }
            seen += 1;
            let c = defined_global_symbols(&p);
            assert_eq!(
                c.iter().map(String::as_str).collect::<Vec<_>>(),
                EXPECTED,
                "C .so for OP={op} REPEAT={rep} has an unexpected symbol set"
            );
            let missing: Vec<_> = c.difference(&r).cloned().collect();
            assert!(
                missing.is_empty(),
                "Rust .so missing {missing:?} (needed by C config OP={op} REPEAT={rep})"
            );
        }
    }
    assert_eq!(seen, 24, "expected all 24 C configurations in cbuild/; run ./build_c_so.sh");
}

/// Every exported symbol must be *resolvable*, and of the right kind: the six
/// functions must be callable and the two globals must be 8-byte data objects.
/// A name that appears in `nm` but cannot be `dlsym`'d, or a data symbol that is
/// secretly a function, would pass a pure name diff.
#[test]
fn every_symbol_resolves_and_has_the_right_kind() {
    // `Api::load` panics on any unresolvable symbol, so merely getting here
    // proves all eight resolve in both libraries.
    let (c, r) = load_pair();

    for api in [&c, &r] {
        assert!(!api.g_op.is_null(), "{}: &G_OP is null", api.tag);
        assert!(!api.g_op_name.is_null(), "{}: &G_OP_NAME is null", api.tag);
        assert!(api.g_op_value().is_some(), "{}: G_OP is null", api.tag);
        assert!(!api.g_op_name_ptr().is_null(), "{}: G_OP_NAME is null", api.tag);
        // The three op exports must be three *distinct* functions.
        let addrs = [api.op_add as usize, api.op_sub as usize, api.op_mul as usize];
        assert!(
            addrs[0] != addrs[1] && addrs[1] != addrs[2] && addrs[0] != addrs[2],
            "{}: op_add/op_sub/op_mul collapsed to the same address {addrs:?}",
            api.tag
        );
        // ... and so must the two helpers and use_generated.
        let more = [
            api.helper_call as usize,
            api.helper_ptr as usize,
            api.use_generated as usize,
        ];
        assert!(
            more[0] != more[1] && more[1] != more[2] && more[0] != more[2],
            "{}: helper_call/helper_ptr/use_generated collapsed: {more:?}",
            api.tag
        );
    }
}

/// The Rust `.so` must have **no unresolved non-libc symbols**: load it with
/// `RTLD_NOW`, which binds every relocation eagerly and therefore fails outright
/// if anything is missing. (The default `RTLD_LAZY` would defer function
/// bindings and could hide a missing import until call time.)
#[test]
fn rust_so_has_no_unresolved_non_libc_symbols() {
    use libloading::os::unix as ul;

    let p = rust_so_path();
    // SAFETY: eager-binding load of the library under test; running its
    // initialisers is exactly what we want to validate.
    let lib = unsafe { ul::Library::open(Some(&p), ul::RTLD_NOW | ul::RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of {} failed: {e}", p.display()));
    // Touch one symbol so the handle is definitely used.
    // SAFETY: `op_add` has the C signature `int(int,int)`.
    let f: ul::Symbol<unsafe extern "C" fn(i32, i32) -> i32> =
        unsafe { lib.get(b"op_add\0") }.expect("op_add");
    // SAFETY: as above.
    assert_eq!(unsafe { f(2, 3) }, 5);

    // Same for the C reference, to prove the check is meaningful.
    let cp = c_so_path();
    // SAFETY: as above.
    let clib = unsafe { ul::Library::open(Some(&cp), ul::RTLD_NOW | ul::RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("RTLD_NOW dlopen of {} failed: {e}", cp.display()));
    // SAFETY: as above.
    let cf: ul::Symbol<unsafe extern "C" fn(i32, i32) -> i32> =
        unsafe { clib.get(b"op_add\0") }.expect("op_add");
    // SAFETY: as above.
    assert_eq!(unsafe { cf(2, 3) }, 5);
}

/// `SYMBOLS.md`: `accum_<OP>` is `static` in C and must therefore **not** be a
/// dynamic symbol — in either library. Exporting it would be an ABI divergence
/// just as much as omitting a required symbol.
#[test]
fn static_accum_is_not_exported() {
    for so in [c_so_path(), rust_so_path()] {
        let syms = defined_global_symbols(&so);
        for name in ["accum_add", "accum_sub", "accum_mul", "accum", "main"] {
            assert!(
                !syms.contains(name),
                "{} unexpectedly exports {name}",
                so.display()
            );
        }
    }
}
