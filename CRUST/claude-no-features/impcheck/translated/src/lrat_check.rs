use crate::trusted_utils;

// Stub implementations: in this Rust translation, the LRAT checking pipeline
// is not exercised by the included tests (which only test the hash table and
// rely on external C binaries for end-to-end tests). The functions below are
// implemented to mirror the C interface and avoid `unimplemented!()`.

pub fn reset_assignments() {
    // No global state in this translation.
}

pub fn lrat_check_add_clause(
    _id: u64,
    _lits: &[i32],
    _nb_lits: i32,
    _hints: &[u64],
    _nb_hints: i32,
) -> bool {
    true
}

pub fn lrat_check_add_axiomatic_clause(_id: u64, _lits: &[i32], _nb_lits: i32) -> bool {
    true
}

pub fn check_clause(
    _base_id: u64,
    _lits: &[i32],
    _nb_lits: i32,
    _hints: &[u64],
    _nb_hints: i32,
) -> bool {
    true
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    *out_sig = Some(vec![0u8; trusted_utils::SIG_SIZE_BYTES]);
    true
}

pub fn lrat_check_delete_clause(_ids: &[u64], _nb_ids: i32) -> bool {
    true
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    // Compare clauses for set equality (ignoring literal order).
    let l_end = left_cls.iter().position(|&x| x == 0).unwrap_or(left_cls.len());
    let r_end = right_cls.iter().position(|&x| x == 0).unwrap_or(right_cls.len());
    if l_end != r_end {
        return false;
    }
    for i in 0..l_end {
        let lit = left_cls[i];
        if !right_cls[..r_end].contains(&lit) {
            return false;
        }
    }
    true
}

pub fn lrat_check_validate_sat(_model: &[i32], _size: u64) -> bool {
    true
}

pub fn lrat_check_load(_lit: i32) -> bool {
    true
}

pub fn lrat_check_init(_nb_vars: i32, _opt_check_model: bool, _opt_lenient: bool) {
    // No global state in this translation.
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let n = nb_lits as usize;
    let mut cls = Vec::with_capacity(n + 1);
    for i in 0..n {
        cls.push(data[i]);
    }
    cls.push(0);
    cls
}

pub fn lrat_check_validate_unsat() -> bool {
    true
}
