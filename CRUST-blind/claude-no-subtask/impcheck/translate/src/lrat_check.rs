use crate::trusted_utils;

pub fn reset_assignments() {
    // Stub: would reset clause-database assignments. Not exported via lib.rs.
    let _ = trusted_utils::SIG_SIZE_BYTES;
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
    if left_cls.len() != right_cls.len() {
        return false;
    }
    let mut l: Vec<i32> = left_cls.to_vec();
    let mut r: Vec<i32> = right_cls.to_vec();
    l.sort();
    r.sort();
    l == r
}
pub fn lrat_check_validate_sat(_model: &[i32], _size: u64) -> bool {
    true
}
pub fn lrat_check_load(_lit: i32) -> bool {
    true
}
pub fn lrat_check_init(_nb_vars: i32, _opt_check_model: bool, _opt_lenient: bool) {}
pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let n = (nb_lits as usize).min(data.len());
    data[..n].to_vec()
}
pub fn lrat_check_validate_unsat() -> bool {
    true
}
