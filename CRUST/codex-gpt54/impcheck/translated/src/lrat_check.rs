use std::cell::RefCell;
use std::collections::HashMap;

use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils::{trusted_utils_get_msg, trusted_utils_set_msg};

thread_local! {
    static STATE: RefCell<Option<LratState>> = const { RefCell::new(None) };
}

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
    formula_hasher: SipHash,
}

pub fn reset_assignments() {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            for &var in &state.assigned_units {
                state.var_values[var as usize] = 0;
            }
            state.assigned_units.clear();
        }
    });
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
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        let cls = clause_init(lits, nb_lits);
        let old = state.clause_table.insert(id, cls.clone());
        let mut ok = old.is_none();
        if !ok && state.lenient {
            if let Some(old_cls) = old.as_ref() {
                ok = clauses_equivalent(old_cls, &cls);
            }
        }
        if !ok {
            if let Some(previous) = old {
                state.clause_table.insert(id, previous);
            }
            trusted_utils_set_msg(&format!(
                "Insertion of clause {} unsuccessful - already present?",
                id
            ));
            return false;
        }
        if nb_lits == 0 {
            state.unsat_proven = true;
        }
        true
    })
}

pub fn check_clause(base_id: u64, lits: &[i32], nb_lits: i32, hints: &[u64], nb_hints: i32) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        trusted_utils_set_msg("");
        state.assigned_units.reserve((nb_lits + nb_hints) as usize);

        for &lit in lits.iter().take(nb_lits as usize) {
            let var = lit.unsigned_abs() as usize;
            state.var_values[var] = if lit > 0 { -1 } else { 1 };
            state.assigned_units.push(var as i32);
        }

        let mut ok = true;
        for (i, &hint_id) in hints.iter().take(nb_hints as usize).enumerate() {
            let Some(cls) = state.clause_table.get(&hint_id).cloned() else {
                trusted_utils_set_msg(&format!("Derivation {}: hint {} not found", base_id, hint_id));
                ok = false;
                break;
            };

            let mut new_unit = 0;
            for &lit in &cls {
                if lit == 0 {
                    break;
                }
                let var = lit.unsigned_abs() as usize;
                if state.var_values[var] == 0 {
                    if new_unit != 0 {
                        trusted_utils_set_msg(&format!(
                            "Derivation {}: multiple literals unassigned",
                            base_id
                        ));
                        ok = false;
                        break;
                    }
                    new_unit = lit;
                    continue;
                }
                let sign = state.var_values[var] > 0;
                if sign == (lit > 0) {
                    trusted_utils_set_msg(&format!(
                        "Derivation {}: dependency {} is satisfied",
                        base_id, hint_id
                    ));
                    ok = false;
                    break;
                }
            }
            if !ok {
                break;
            }

            if new_unit == 0 {
                if i + 1 < nb_hints as usize {
                    trusted_utils_set_msg(&format!(
                        "Derivation {}: empty clause produced at non-final hint {}",
                        base_id, hint_id
                    ));
                    ok = false;
                    break;
                }
                for &var in &state.assigned_units {
                    state.var_values[var as usize] = 0;
                }
                state.assigned_units.clear();
                return true;
            }

            let var = new_unit.unsigned_abs() as usize;
            state.var_values[var] = if new_unit > 0 { 1 } else { -1 };
            state.assigned_units.push(var as i32);
        }

        if trusted_utils_get_msg().is_empty() {
            trusted_utils_set_msg(&format!(
                "Derivation {}: no empty clause was produced",
                base_id
            ));
        }
        for &var in &state.assigned_units {
            state.var_values[var as usize] = 0;
        }
        state.assigned_units.clear();
        false
    })
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        if !state.clause_to_add.is_empty() {
            trusted_utils_set_msg("literals left in unterminated clause");
            return false;
        }
        state.formula_hasher.siphash_pad(2);
        *out_sig = Some(state.formula_hasher.siphash_digest());
        state.done_loading = true;
        state.nb_loaded_clauses = state.id_to_add - 1;
        true
    })
}

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        for &id in ids.iter().take(nb_ids as usize) {
            if !state.clause_table.contains_key(&id) {
                trusted_utils_set_msg(&format!("Clause deletion: ID {} not found", id));
                return false;
            }
            if state.check_model && id <= state.nb_loaded_clauses {
                continue;
            }
            state.clause_table.remove(&id);
        }
        true
    })
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let left: Vec<_> = left_cls.iter().copied().take_while(|lit| *lit != 0).collect();
    let right: Vec<_> = right_cls.iter().copied().take_while(|lit| *lit != 0).collect();
    left.len() == right.len() && left.iter().all(|lit| right.contains(lit))
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();

        if !state.done_loading {
            trusted_utils_set_msg("SAT validation illegal - loading formula was not concluded");
            return false;
        }
        if !state.check_model {
            trusted_utils_set_msg("SAT validation illegal - not executed to explicitly support this");
            return false;
        }

        let mut model = model.to_vec();
        for id in 1..=state.nb_loaded_clauses {
            let Some(cls) = state.clause_table.get(&id) else {
                trusted_utils_set_msg(&format!("SAT validation: original ID {} not found", id));
                return false;
            };
            let mut satisfied = false;
            for &lit in cls {
                if lit == 0 {
                    break;
                }
                let var = lit.unsigned_abs() as usize;
                if (var as u64).saturating_sub(1) >= size {
                    trusted_utils_set_msg(&format!(
                        "SAT validation: model does not cover variable {}",
                        var
                    ));
                    return false;
                }
                let mut model_lit = model[var - 1];
                if model_lit != var as i32 && model_lit != -(var as i32) && model_lit != 0 {
                    trusted_utils_set_msg(&format!(
                        "SAT validation: unexpected literal {} in assignment of variable {}",
                        model_lit, var
                    ));
                    return false;
                }
                if model_lit == 0 {
                    model_lit = lit;
                    model[var - 1] = lit;
                }
                if model_lit == lit {
                    satisfied = true;
                    break;
                }
            }
            if !satisfied {
                trusted_utils_set_msg(&format!(
                    "SAT validation: original clause {} not satisfied",
                    id
                ));
                return false;
            }
        }
        true
    })
}

pub fn lrat_check_load(lit: i32) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        if lit == 0 {
            let clause = clause_init(&state.clause_to_add, state.clause_to_add.len() as i32);
            let old = state.clause_table.insert(state.id_to_add, clause.clone());
            if old.is_some() {
                trusted_utils_set_msg(&format!(
                    "Insertion of clause {} unsuccessful - already present?",
                    state.id_to_add
                ));
                return false;
            }
            if clause.len() == 1 {
                state.unsat_proven = true;
            }
            state.id_to_add += 1;
            let bytes = ints_to_bytes(&clause);
            state.formula_hasher.siphash_update(&bytes, bytes.len() as u64);
            state.clause_to_add.clear();
            return true;
        }
        state.clause_to_add.push(lit);
        true
    })
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    STATE.with(|state| {
        *state.borrow_mut() = Some(LratState {
            clause_table: HashMap::new(),
            var_values: vec![0; (nb_vars + 1) as usize],
            assigned_units: Vec::with_capacity(512),
            check_model: opt_check_model,
            lenient: opt_lenient,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            clause_to_add: Vec::with_capacity(512),
            done_loading: false,
            unsat_proven: false,
            formula_hasher: SipHash::siphash_init(&SECRET_KEY),
        });
    });
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls = data.iter().copied().take(nb_lits as usize).collect::<Vec<_>>();
    cls.push(0);
    cls
}

pub fn lrat_check_validate_unsat() -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        if !state.done_loading {
            trusted_utils_set_msg("UNSAT validation illegal - loading formula was not concluded");
            return false;
        }
        if !state.unsat_proven {
            trusted_utils_set_msg(
                "UNSAT validation unsuccessful - did not derive or import empty clause",
            );
            return false;
        }
        true
    })
}

fn ints_to_bytes(values: &[i32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * std::mem::size_of::<i32>());
    for value in values {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
    bytes
}
