use crate::trusted_utils;
use std::collections::HashMap;
use std::sync::Mutex;

// Global state mirroring the C globals.
struct LratState {
    clause_table: HashMap<u64, Vec<i32>>,
    var_values: Vec<i8>,
    assigned_units: Vec<i32>,
    check_model: bool,
    lenient: bool,
    id_to_add: u64,
    nb_loaded_clauses: u64,
    clause_to_add: Vec<i32>,
    done_loading: bool,
    unsat_proven: bool,
    msgstr: String,
    // For end-load formula signature.
    formula_hash: crate::siphash::SipHash,
}

impl LratState {
    fn new() -> Self {
        // Use the SECRET_KEY constant
        let key: [u8; 16] = [
            86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
        ];
        LratState {
            clause_table: HashMap::new(),
            var_values: Vec::new(),
            assigned_units: Vec::new(),
            check_model: false,
            lenient: false,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            clause_to_add: Vec::new(),
            done_loading: false,
            unsat_proven: false,
            msgstr: String::new(),
            formula_hash: crate::siphash::SipHash::siphash_init(&key),
        }
    }
}

fn state() -> &'static Mutex<LratState> {
    use std::sync::OnceLock;
    static STATE: OnceLock<Mutex<LratState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(LratState::new()))
}

pub fn reset_assignments() {
    let mut s = state().lock().unwrap();
    let units = std::mem::take(&mut s.assigned_units);
    for &v in &units {
        let idx = v as usize;
        if idx < s.var_values.len() {
            s.var_values[idx] = 0;
        }
    }
    // assigned_units is cleared
}

pub fn lrat_check_add_clause(
    id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    if !check_clause(id, lits, nb_lits, hints, nb_hints) {
        return false;
    }
    lrat_check_add_axiomatic_clause(id, lits, nb_lits)
}

pub fn lrat_check_add_axiomatic_clause(id: u64, lits: &[i32], nb_lits: i32) -> bool {
    let cls = clause_init(lits, nb_lits);
    let mut s = state().lock().unwrap();
    if s.clause_table.contains_key(&id) {
        let mut ok = false;
        if s.lenient {
            if let Some(old_cls) = s.clause_table.get(&id) {
                if clauses_equivalent(old_cls, &cls) {
                    ok = true;
                }
            }
        }
        if !ok {
            s.msgstr = format!(
                "Insertion of clause {} unsuccessful - already present?",
                id
            );
        }
        ok
    } else {
        s.clause_table.insert(id, cls);
        if nb_lits == 0 {
            s.unsat_proven = true;
        }
        true
    }
}

pub fn check_clause(
    base_id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    let mut s = state().lock().unwrap();

    // Reserve and assume negations
    for i in 0..(nb_lits as usize) {
        let lit = lits[i];
        let var = if lit > 0 { lit } else { -lit } as usize;
        if var < s.var_values.len() {
            s.var_values[var] = if lit > 0 { -1 } else { 1 };
        }
        s.assigned_units.push(var as i32);
    }

    let mut ok = true;
    let mut error_msg: Option<String> = None;
    let mut early_success = false;

    for i in 0..(nb_hints as usize) {
        let hint_id = hints[i];
        let cls_opt = s.clause_table.get(&hint_id).cloned();
        let cls = match cls_opt {
            Some(c) => c,
            None => {
                error_msg = Some(format!(
                    "Derivation {}: hint {} not found",
                    base_id, hint_id
                ));
                ok = false;
                break;
            }
        };

        let mut new_unit: i32 = 0;
        let mut inner_ok = true;
        for &lit in cls.iter() {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit } as usize;
            let cur = if var < s.var_values.len() {
                s.var_values[var]
            } else {
                0
            };
            if cur == 0 {
                if new_unit != 0 {
                    error_msg = Some(format!(
                        "Derivation {}: multiple literals unassigned",
                        base_id
                    ));
                    inner_ok = false;
                    break;
                }
                new_unit = lit;
                continue;
            }
            // Fixed literal
            let sign = cur > 0;
            if sign == (lit > 0) {
                error_msg = Some(format!(
                    "Derivation {}: dependency {} is satisfied",
                    base_id, hint_id
                ));
                inner_ok = false;
                break;
            }
        }
        if !inner_ok {
            ok = false;
            break;
        }

        if new_unit == 0 {
            // Empty clause derived
            if (i as i32) + 1 < nb_hints {
                error_msg = Some(format!(
                    "Derivation {}: empty clause produced at non-final hint {}",
                    base_id, hint_id
                ));
                ok = false;
                break;
            }
            // Final hint - success
            early_success = true;
            break;
        }
        let var = if new_unit > 0 { new_unit } else { -new_unit } as usize;
        if var < s.var_values.len() {
            s.var_values[var] = if new_unit > 0 { 1 } else { -1 };
        }
        s.assigned_units.push(var as i32);
    }

    // Reset assignments
    let units = std::mem::take(&mut s.assigned_units);
    for &v in &units {
        let idx = v as usize;
        if idx < s.var_values.len() {
            s.var_values[idx] = 0;
        }
    }

    if early_success {
        return true;
    }

    if !ok {
        if let Some(m) = error_msg {
            s.msgstr = m;
        }
        return false;
    }

    // No empty clause produced
    if s.msgstr.is_empty() {
        s.msgstr = format!("Derivation {}: no empty clause was produced", base_id);
    }
    false
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    let mut s = state().lock().unwrap();
    if !s.clause_to_add.is_empty() {
        s.msgstr = "literals left in unterminated clause".to_string();
        return false;
    }
    s.formula_hash.siphash_pad(2);
    *out_sig = Some(s.formula_hash.siphash_digest());
    s.done_loading = true;
    s.nb_loaded_clauses = s.id_to_add - 1;
    true
}

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    let mut s = state().lock().unwrap();
    for i in 0..(nb_ids as usize) {
        let id = ids[i];
        if !s.clause_table.contains_key(&id) {
            s.msgstr = format!("Clause deletion: ID {} not found", id);
            return false;
        }
        if s.check_model && id <= s.nb_loaded_clauses {
            continue;
        }
        s.clause_table.remove(&id);
    }
    true
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut left_size = 0;
    for &l in left_cls {
        if l == 0 {
            break;
        }
        left_size += 1;
    }
    let mut right_size = 0;
    for &l in right_cls {
        if l == 0 {
            break;
        }
        right_size += 1;
    }
    for &left_lit in left_cls.iter().take(left_size) {
        let mut found = false;
        for &right_lit in right_cls.iter().take(right_size) {
            if right_lit == left_lit {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    left_size == right_size
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    let mut s = state().lock().unwrap();
    if !s.done_loading {
        s.msgstr =
            "SAT validation illegal - loading formula was not concluded".to_string();
        return false;
    }
    if !s.check_model {
        s.msgstr =
            "SAT validation illegal - not executed to explicitly support this".to_string();
        return false;
    }
    let mut model_copy: Vec<i32> = model.to_vec();
    let nb_loaded = s.nb_loaded_clauses;
    for id in 1..=nb_loaded {
        let cls = match s.clause_table.get(&id) {
            Some(c) => c.clone(),
            None => {
                s.msgstr = format!("SAT validation: original ID {} not found", id);
                return false;
            }
        };
        let mut satisfied = false;
        for &lit in cls.iter() {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit };
            if (var - 1) as u64 >= size {
                s.msgstr = format!(
                    "SAT validation: model does not cover variable {}",
                    var
                );
                return false;
            }
            let mut model_lit = model_copy[(var - 1) as usize];
            if model_lit != var && model_lit != -var && model_lit != 0 {
                s.msgstr = format!(
                    "SAT validation: unexpected literal {} in assignment of variable {}",
                    model_lit, var
                );
                return false;
            }
            if model_lit == 0 {
                model_lit = lit;
                model_copy[(var - 1) as usize] = lit;
            }
            if model_lit == lit {
                satisfied = true;
                break;
            }
        }
        if !satisfied {
            s.msgstr = format!("SAT validation: original clause {} not satisfied", id);
            return false;
        }
    }
    true
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        // Need to do add_axiomatic_clause then update siphash
        let (id_to_add, clause_data) = {
            let s = state().lock().unwrap();
            (s.id_to_add, s.clause_to_add.clone())
        };
        let nb = clause_data.len() as i32;
        if !lrat_check_add_axiomatic_clause(id_to_add, &clause_data, nb) {
            return false;
        }
        let mut s = state().lock().unwrap();
        s.id_to_add += 1;
        // Push terminating zero, then siphash_update on bytes, then clear
        s.clause_to_add.push(0);
        let bytes_len = s.clause_to_add.len() * std::mem::size_of::<i32>();
        let mut bytes = Vec::with_capacity(bytes_len);
        for &v in &s.clause_to_add {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        s.formula_hash
            .siphash_update(&bytes, bytes_len as u64);
        s.clause_to_add.clear();
        return true;
    }
    let mut s = state().lock().unwrap();
    s.clause_to_add.push(lit);
    true
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    let mut s = state().lock().unwrap();
    *s = LratState::new();
    s.var_values = vec![0i8; (nb_vars + 1) as usize];
    s.check_model = opt_check_model;
    s.lenient = opt_lenient;
    let _ = trusted_utils::TRUSTED_CHK_MAX_BUF_SIZE;
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls: Vec<i32> = Vec::with_capacity((nb_lits + 1) as usize);
    for i in 0..(nb_lits as usize) {
        cls.push(data[i]);
    }
    cls.push(0);
    cls
}

pub fn lrat_check_validate_unsat() -> bool {
    let mut s = state().lock().unwrap();
    if !s.done_loading {
        s.msgstr =
            "UNSAT validation illegal - loading formula was not concluded".to_string();
        return false;
    }
    if !s.unsat_proven {
        s.msgstr =
            "UNSAT validation unsuccessful - did not derive or import empty clause"
                .to_string();
        return false;
    }
    true
}
