//! Level 0: the exported global data objects must be byte-identical.

mod common;

use common::libs;

unsafe fn bytes(p: *const u8, n: usize) -> Vec<u8> {
    unsafe { std::slice::from_raw_parts(p, n).to_vec() }
}

#[test]
fn global_tables_match() {
    let l = libs();

    let cases: [(&str, *mut u8, *mut u8, usize); 7] = [
        ("cp_fixed_table", l.c.cp_fixed_table, l.rust.cp_fixed_table, 288 + 32),
        (
            "cp_permutation_order",
            l.c.cp_permutation_order,
            l.rust.cp_permutation_order,
            19,
        ),
        (
            "cp_len_extra_bits",
            l.c.cp_len_extra_bits,
            l.rust.cp_len_extra_bits,
            29 + 2,
        ),
        (
            "cp_len_base",
            l.c.cp_len_base as *mut u8,
            l.rust.cp_len_base as *mut u8,
            (29 + 2) * 4,
        ),
        (
            "cp_dist_extra_bits",
            l.c.cp_dist_extra_bits,
            l.rust.cp_dist_extra_bits,
            30 + 2,
        ),
        (
            "cp_dist_base",
            l.c.cp_dist_base as *mut u8,
            l.rust.cp_dist_base as *mut u8,
            (30 + 2) * 4,
        ),
        (
            // NULL-initialised pointer slot
            "cp_error_reason",
            l.c.cp_error_reason as *mut u8,
            l.rust.cp_error_reason as *mut u8,
            std::mem::size_of::<usize>(),
        ),
    ];

    for (name, cp, rp, n) in cases {
        let (a, b) = unsafe { (bytes(cp, n), bytes(rp, n)) };
        if name == "cp_error_reason" {
            // Both must start out NULL; the pointer values themselves differ
            // once set (they point into each library's own rodata).
            assert_eq!(a, vec![0u8; n], "C cp_error_reason not initially NULL");
            assert_eq!(b, vec![0u8; n], "Rust cp_error_reason not initially NULL");
            continue;
        }
        assert_eq!(a, b, "table `{name}` differs:\n{}", common::hexdiff(&a, &b));
    }
}
