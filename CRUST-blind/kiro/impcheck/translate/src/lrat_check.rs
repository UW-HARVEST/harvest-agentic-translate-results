use crate::trusted_utils;
use crate::hash::SimpleHashTable;
use crate::siphash::SipHash;
use crate::secret::SECRET_KEY;
use crate::trusted_utils::SIG_SIZE_BYTES;
use std::cell::RefCell;

struct LratState {
    clause_table: SimpleHashTable,
    var_values: Vec<i8>,
    assigned_units: Vec<i32>,
    check_model: bool,
    lenient: bool,
    id_to_add: u64,
    nb_loaded_clauses: u64,
    clause_to_add: Vec<i32>,
    done_loading: bool,
    unsat_proven: bool,
    siphash: SipHash,
    msgstr: String,
}

thread_local! {
    static STATE: RefCell<Option<LratState>> = RefCell::new(None);
}

fn with_state<F, R>(f: F) -> R where F: FnOnce(&mut LratState) -> R {
    STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        f(borrow.as_mut().expect("lrat_check not initialized"))
    })
}

pub fn reset_assignments() {
    with_state(|st| {
        for i in 0..st.assigned_units.len() {
            let var = st.assigned_units[i] as usize;
            st.var_values[var] = 0;
        }
        st.assigned_units.clear();
    });
}

pub fn lrat_check_add_clause(id: u64, lits: &[i32], nb_lits: i32, hints: &[u64], nb_hints: i32) -> bool {
    if !check_clause(id, lits, nb_lits, hints, nb_hints) {
        return false;
    }
    lrat_check_add_axiomatic_clause(id, lits, nb_lits)
}

pub fn lrat_check_add_axiomatic_clause(id: u64, lits: &[i32], nb_lits: i32) -> bool {
    with_state(|st| {
        let cls = clause_init_internal(lits, nb_lits as usize);
        let ok = st.clause_table.hash_table_insert(id, Box::new(cls));
        if !ok {
            if st.lenient {
                let old = st.clause_table.hash_table_find(id);
                if let Some(old_box) = old {
                    if let Some(old_cls) = old_box.downcast_ref::<Vec<i32>>() {
                        let new_cls = clause_init_internal(lits, nb_lits as usize);
                        if clauses_equivalent_internal(old_cls, &new_cls) {
                            return true;
                        }
                    }
                }
            }
            st.msgstr = format!("Insertion of clause {} unsuccessful - already present?", id);
            return false;
        }
        if nb_lits == 0 {
            st.unsat_proven = true;
        }
        true
    })
}

fn clause_init_internal(data: &[i32], nb_lits: usize) -> Vec<i32> {
    let mut cls = vec![0i32; nb_lits + 1];
    for i in 0..nb_lits {
        cls[i] = data[i];
    }
    cls[nb_lits] = 0;
    cls
}

pub fn check_clause(base_id: u64, lits: &[i32], nb_lits: i32, hints: &[u64], nb_hints: i32) -> bool {
    with_state(|st| {
        let nl = nb_lits as usize;
        let nh = nb_hints as usize;

        // Assume negation of each literal
        for i in 0..nl {
            let lit = lits[i];
            let var = lit.unsigned_abs() as usize;
            st.var_values[var] = if lit > 0 { -1 } else { 1 };
            st.assigned_units.push(var as i32);
        }

        let mut ok = true;
        for i in 0..nh {
            let hint_id = hints[i];
            let cls_opt = st.clause_table.hash_table_find(hint_id);
            let cls: Vec<i32> = match cls_opt {
                Some(b) => {
                    if let Some(v) = b.downcast_ref::<Vec<i32>>() {
                        v.clone()
                    } else {
                        st.msgstr = format!("Derivation {}: hint {} not found", base_id, hint_id);
                        ok = false;
                        break;
                    }
                }
                None => {
                    st.msgstr = format!("Derivation {}: hint {} not found", base_id, hint_id);
                    ok = false;
                    break;
                }
            };

            let mut new_unit = 0i32;
            let mut inner_ok = true;
            for lit_idx in 0.. {
                let lit = cls[lit_idx];
                if lit == 0 { break; }
                let var = lit.unsigned_abs() as usize;
                if st.var_values[var] == 0 {
                    if new_unit != 0 {
                        st.msgstr = format!("Derivation {}: multiple literals unassigned", base_id);
                        inner_ok = false;
                        break;
                    }
                    new_unit = lit;
                    continue;
                }
                let sign = st.var_values[var] > 0;
                if sign == (lit > 0) {
                    st.msgstr = format!("Derivation {}: dependency {} is satisfied", base_id, hint_id);
                    inner_ok = false;
                    break;
                }
            }
            if !inner_ok { ok = false; break; }

            if new_unit == 0 {
                if i + 1 < nh {
                    st.msgstr = format!("Derivation {}: empty clause produced at non-final hint {}", base_id, hint_id);
                    break;
                }
                // Final hint produced empty clause - success
                for j in 0..st.assigned_units.len() {
                    let v = st.assigned_units[j] as usize;
                    st.var_values[v] = 0;
                }
                st.assigned_units.clear();
                return true;
            }
            let var = new_unit.unsigned_abs() as usize;
            st.var_values[var] = if new_unit > 0 { 1 } else { -1 };
            st.assigned_units.push(var as i32);
        }

        if ok && st.msgstr.is_empty() {
            st.msgstr = format!("Derivation {}: no empty clause was produced", base_id);
        }
        // Reset assignments
        for j in 0..st.assigned_units.len() {
            let v = st.assigned_units[j] as usize;
            st.var_values[v] = 0;
        }
        st.assigned_units.clear();
        false
    })
}

fn clauses_equivalent_internal(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut left_size = 0;
    for i in 0..left_cls.len() {
        if left_cls[i] == 0 { break; }
        let left_lit = left_cls[i];
        let mut found = false;
        for j in 0..right_cls.len() {
            if right_cls[j] == 0 { break; }
            if right_cls[j] == left_lit { found = true; break; }
        }
        if !found { return false; }
        left_size += 1;
    }
    let mut right_size = 0;
    for i in 0..right_cls.len() {
        if right_cls[i] == 0 { break; }
        right_size += 1;
    }
    left_size == right_size
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    clauses_equivalent_internal(left_cls, right_cls)
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    with_state(|st| {
        if !st.done_loading {
            st.msgstr = "SAT validation illegal - loading formula was not concluded".to_string();
            return false;
        }
        if !st.check_model {
            st.msgstr = "SAT validation illegal - not executed to explicitly support this".to_string();
            return false;
        }
        // We need a mutable copy of model for the "assign fitting value" logic
        // But the signature takes &[i32]. We'll work around by using a local copy.
        let mut model_copy: Vec<i32> = model.to_vec();
        for id in 1..=st.nb_loaded_clauses {
            let cls: Vec<i32> = match st.clause_table.hash_table_find(id) {
                Some(b) => {
                    if let Some(v) = b.downcast_ref::<Vec<i32>>() { v.clone() }
                    else {
                        st.msgstr = format!("SAT validation: original ID {} not found", id);
                        return false;
                    }
                }
                None => {
                    st.msgstr = format!("SAT validation: original ID {} not found", id);
                    return false;
                }
            };
            let mut satisfied = false;
            for lit_idx in 0.. {
                let lit = cls[lit_idx];
                if lit == 0 { break; }
                let var = if lit > 0 { lit } else { -lit };
                if (var as u64 - 1) >= size {
                    st.msgstr = format!("SAT validation: model does not cover variable {}", var);
                    return false;
                }
                let mut model_lit = model_copy[(var - 1) as usize];
                if model_lit != var && model_lit != -var && model_lit != 0 {
                    st.msgstr = format!("SAT validation: unexpected literal {} in assignment of variable {}", model_lit, var);
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
                st.msgstr = format!("SAT validation: original clause {} not satisfied", id);
                return false;
            }
        }
        true
    })
}

pub fn lrat_check_load(lit: i32) -> bool {
    with_state(|st| {
        if lit == 0 {
            let lits = st.clause_to_add.clone();
            let id = st.id_to_add;
            // Need to call add_axiomatic_clause without borrowing st
            let cls = clause_init_internal(&lits, lits.len());
            let ok = st.clause_table.hash_table_insert(id, Box::new(cls));
            if !ok {
                if st.lenient {
                    let old = st.clause_table.hash_table_find(id);
                    if let Some(old_box) = old {
                        if let Some(old_cls) = old_box.downcast_ref::<Vec<i32>>() {
                            let new_cls = clause_init_internal(&lits, lits.len());
                            if clauses_equivalent_internal(old_cls, &new_cls) {
                                // ok, continue
                            } else {
                                st.msgstr = format!("Insertion of clause {} unsuccessful - already present?", id);
                                return false;
                            }
                        } else {
                            st.msgstr = format!("Insertion of clause {} unsuccessful - already present?", id);
                            return false;
                        }
                    } else {
                        st.msgstr = format!("Insertion of clause {} unsuccessful - already present?", id);
                        return false;
                    }
                } else {
                    st.msgstr = format!("Insertion of clause {} unsuccessful - already present?", id);
                    return false;
                }
            } else if lits.is_empty() {
                st.unsat_proven = true;
            }
            st.id_to_add += 1;
            // Push 0 terminator and update siphash
            st.clause_to_add.push(0);
            let data_bytes: Vec<u8> = st.clause_to_add.iter()
                .flat_map(|x| x.to_ne_bytes())
                .collect();
            st.siphash.siphash_update(&data_bytes, data_bytes.len() as u64);
            st.clause_to_add.clear();
            return true;
        }
        st.clause_to_add.push(lit);
        true
    })
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    STATE.with(|s| {
        *s.borrow_mut() = Some(LratState {
            clause_table: SimpleHashTable::new(16),
            var_values: vec![0i8; (nb_vars + 1) as usize],
            assigned_units: Vec::with_capacity(512),
            check_model: opt_check_model,
            lenient: opt_lenient,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            clause_to_add: Vec::with_capacity(512),
            done_loading: false,
            unsat_proven: false,
            siphash: SipHash::siphash_init(&SECRET_KEY),
            msgstr: String::new(),
        });
    });
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    clause_init_internal(data, nb_lits as usize)
}

pub fn lrat_check_validate_unsat() -> bool {
    with_state(|st| {
        if !st.done_loading {
            st.msgstr = "UNSAT validation illegal - loading formula was not concluded".to_string();
            return false;
        }
        if !st.unsat_proven {
            st.msgstr = "UNSAT validation unsuccessful - did not derive or import empty clause".to_string();
            return false;
        }
        true
    })
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    with_state(|st| {
        if !st.clause_to_add.is_empty() {
            st.msgstr = "literals left in unterminated clause".to_string();
            return false;
        }
        st.siphash.siphash_pad(2);
        let sig = st.siphash.siphash_digest();
        *out_sig = Some(sig);
        st.done_loading = true;
        st.nb_loaded_clauses = st.id_to_add - 1;
        true
    })
}

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    with_state(|st| {
        for i in 0..nb_ids as usize {
            let id = ids[i];
            let found = st.clause_table.hash_table_find(id).is_some();
            if !found {
                st.msgstr = format!("Clause deletion: ID {} not found", id);
                return false;
            }
            if st.check_model && id <= st.nb_loaded_clauses {
                continue;
            }
            if !st.clause_table.hash_table_delete_last_found() {
                st.msgstr = format!("Clause deletion: Hash table error for ID {}", id);
                return false;
            }
        }
        true
    })
}

// Get the current error message
pub fn lrat_check_get_msgstr() -> String {
    with_state(|st| st.msgstr.clone())
}

pub fn lrat_check_clear_msgstr() {
    with_state(|st| st.msgstr.clear());
}
