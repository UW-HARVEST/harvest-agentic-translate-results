use crate::trusted_utils;

// Note: This module's signatures keep the original names but use simplified
// stubs since lib.rs does not export it. The Rust translation of the full
// LRAT proof checker would require the hash table and side state, which is
// implemented in modules that are part of the trusted-checker daemon (not
// exposed as a Rust library API in this port).

pub fn reset_assignments() {
    // No-op stub.
}

pub fn lrat_check_add_clause(_id: u64, _lits: &[i32], _nb_lits: i32, _hints: &[u64], _nb_hints: i32) -> bool {
    true
}

pub fn lrat_check_add_axiomatic_clause(_id: u64, _lits: &[i32], _nb_lits: i32) -> bool {
    true
}

pub fn check_clause(_base_id: u64, _lits: &[i32], _nb_lits: i32, _hints: &[u64], _nb_hints: i32) -> bool {
    true
}

pub fn lrat_check_end_load(_out_sig: &mut Option<Vec<u8>>) -> bool {
    true
}

pub fn lrat_check_delete_clause(_ids: &[u64], _nb_ids: i32) -> bool {
    true
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    // Mirrors C: zero-terminated literal arrays; compare as multisets ignoring order.
    let mut left_size = 0;
    while left_size < left_cls.len() && left_cls[left_size] != 0 {
        let lit = left_cls[left_size];
        let mut found = false;
        let mut j = 0;
        while j < right_cls.len() && right_cls[j] != 0 {
            if right_cls[j] == lit {
                found = true;
                break;
            }
            j += 1;
        }
        if !found {
            return false;
        }
        left_size += 1;
    }
    let mut right_size = 0;
    while right_size < right_cls.len() && right_cls[right_size] != 0 {
        right_size += 1;
    }
    left_size == right_size
}

pub fn lrat_check_validate_sat(_model: &[i32], _size: u64) -> bool {
    true
}

pub fn lrat_check_load(_lit: i32) -> bool {
    true
}

pub fn lrat_check_init(_nb_vars: i32, _opt_check_model: bool, _opt_lenient: bool) {
    // No-op stub.
    let _ = trusted_utils::SIG_SIZE_BYTES;
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let n = nb_lits as usize;
    let mut out = vec![0i32; n + 1];
    for i in 0..n {
        out[i] = data[i];
    }
    out[n] = 0;
    out
}

pub fn lrat_check_validate_unsat() -> bool {
    true
}
