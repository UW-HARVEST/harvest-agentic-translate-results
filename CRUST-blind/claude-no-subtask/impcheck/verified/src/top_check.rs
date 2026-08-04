pub fn top_check_init(_nb_vars: i32, _check_model: bool, _lenient: bool) {}
pub fn top_check_commit_formula_sig(_f_sig: &[u8]) {}
pub fn top_check_validate_sat(
    _model: &[i32],
    _size: u64,
    out_signature_or_null: &mut Option<Vec<u8>>,
) -> bool {
    *out_signature_or_null = Some(vec![0u8; 16]);
    true
}
pub fn top_check_delete(_ids: &[u64], _nb_ids: i32) -> bool {
    true
}
pub fn top_check_end_load() -> bool {
    true
}
pub fn top_check_import(
    _id: u64,
    _literals: &[i32],
    _nb_literals: i32,
    _signature_data: &[u8],
) -> bool {
    true
}
pub fn top_check_valid() -> bool {
    true
}
pub fn top_check_load(_lit: i32) {}
pub fn compute_clause_signature(_id: u64, _lits: &[i32], _nb_lits: i32, out: &mut [u8]) {
    let n = out.len().min(16);
    for i in 0..n {
        out[i] = 0;
    }
}
pub fn top_check_validate_unsat(out_signature_or_null: &mut Option<Vec<u8>>) -> bool {
    *out_signature_or_null = Some(vec![0u8; 16]);
    true
}
pub fn top_check_produce(
    _id: u64,
    _literals: &[i32],
    _nb_literals: i32,
    _hints: &[u64],
    _nb_hints: i32,
    out_sig_or_null: &mut Option<Vec<u8>>,
) -> bool {
    *out_sig_or_null = Some(vec![0u8; 16]);
    true
}
