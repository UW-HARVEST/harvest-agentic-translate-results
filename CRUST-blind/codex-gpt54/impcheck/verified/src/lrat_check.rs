use std::cell::RefCell;
use std::collections::HashMap;

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
    hasher: crate::siphash::SipHash,
}

thread_local! {
    static LRAT_STATE: RefCell<Option<LratState>> = const { RefCell::new(None) };
}

fn with_state<R>(f: impl FnOnce(&mut LratState) -> R) -> R {
    LRAT_STATE.with(|state| f(state.borrow_mut().as_mut().expect("lrat_check_init must be called first")))
}

pub fn reset_assignments() {
    with_state(|state| {
        for var in state.assigned_units.drain(..) {
            if let Some(slot) = state.var_values.get_mut(var as usize) {
                *slot = 0;
            }
        }
    });
}
pub fn lrat_check_add_clause(id: u64, lits: &[i32], nb_lits: i32, hints: &[u64], nb_hints: i32) -> bool {
    if !check_clause(id, lits, nb_lits, hints, nb_hints) {
        return false;
    }
    lrat_check_add_axiomatic_clause(id, lits, nb_lits)
}
pub fn lrat_check_add_axiomatic_clause(id: u64, lits: &[i32], nb_lits: i32) -> bool {
    with_state(|state| {
        let cls = clause_init(lits, nb_lits);
        if let Some(old_cls) = state.clause_table.get(&id) {
            if state.lenient && clauses_equivalent(old_cls, &cls) {
                return true;
            }
            return false;
        }
        if nb_lits == 0 {
            state.unsat_proven = true;
        }
        state.clause_table.insert(id, cls);
        true
    })
}
pub fn check_clause(base_id: u64, lits: &[i32], nb_lits: i32, hints: &[u64], nb_hints: i32) -> bool {
    with_state(|state| {
        let _ = base_id;
        state.assigned_units.reserve((nb_lits + nb_hints) as usize);
        for lit in lits.iter().take(nb_lits as usize) {
            let var = lit.abs() as usize;
            if var < state.var_values.len() {
                state.var_values[var] = if *lit > 0 { -1 } else { 1 };
                state.assigned_units.push(var as i32);
            }
        }

        for (hint_idx, hint_id) in hints.iter().take(nb_hints as usize).enumerate() {
            let Some(cls) = state.clause_table.get(hint_id).cloned() else {
                for var in state.assigned_units.drain(..) {
                    if let Some(slot) = state.var_values.get_mut(var as usize) {
                        *slot = 0;
                    }
                }
                return false;
            };
            let mut new_unit = 0;
            for lit in cls.into_iter().take_while(|lit| *lit != 0) {
                let var = lit.abs() as usize;
                let current = *state.var_values.get(var).unwrap_or(&0);
                if current == 0 {
                    if new_unit != 0 {
                        for var in state.assigned_units.drain(..) {
                            if let Some(slot) = state.var_values.get_mut(var as usize) {
                                *slot = 0;
                            }
                        }
                        return false;
                    }
                    new_unit = lit;
                    continue;
                }
                let sign = current > 0;
                if sign == (lit > 0) {
                    for var in state.assigned_units.drain(..) {
                        if let Some(slot) = state.var_values.get_mut(var as usize) {
                            *slot = 0;
                        }
                    }
                    return false;
                }
            }
            if new_unit == 0 {
                let ok = hint_idx + 1 == nb_hints as usize;
                for var in state.assigned_units.drain(..) {
                    if let Some(slot) = state.var_values.get_mut(var as usize) {
                        *slot = 0;
                    }
                }
                return ok;
            }
            let var = new_unit.abs() as usize;
            if var < state.var_values.len() {
                state.var_values[var] = if new_unit > 0 { 1 } else { -1 };
                state.assigned_units.push(var as i32);
            }
        }

        for var in state.assigned_units.drain(..) {
            if let Some(slot) = state.var_values.get_mut(var as usize) {
                *slot = 0;
            }
        }
        false
    })
}
pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    with_state(|state| {
        if !state.clause_to_add.is_empty() {
            return false;
        }
        state.hasher.siphash_pad(2);
        *out_sig = Some(state.hasher.siphash_digest());
        state.done_loading = true;
        state.nb_loaded_clauses = state.id_to_add - 1;
        true
    })
}
pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    with_state(|state| {
        for id in ids.iter().take(nb_ids as usize) {
            if state.check_model && *id <= state.nb_loaded_clauses {
                continue;
            }
            if state.clause_table.remove(id).is_none() {
                return false;
            }
        }
        true
    })
}
pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let left: Vec<i32> = left_cls.iter().copied().take_while(|lit| *lit != 0).collect();
    let right: Vec<i32> = right_cls.iter().copied().take_while(|lit| *lit != 0).collect();
    left.len() == right.len() && left.iter().all(|lit| right.contains(lit))
}
pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    with_state(|state| {
        if !state.done_loading || !state.check_model {
            return false;
        }
        let mut model = model[..model.len().min(size as usize)].to_vec();
        for id in 1..=state.nb_loaded_clauses {
            let Some(cls) = state.clause_table.get(&id) else {
                return false;
            };
            let mut satisfied = false;
            for lit in cls.iter().copied().take_while(|lit| *lit != 0) {
                let var = lit.abs() as usize;
                if var == 0 || var > model.len() {
                    return false;
                }
                let mut model_lit = model[var - 1];
                if model_lit != var as i32 && model_lit != -(var as i32) && model_lit != 0 {
                    return false;
                }
                if model_lit == 0 {
                    model[var - 1] = lit;
                    model_lit = lit;
                }
                if model_lit == lit {
                    satisfied = true;
                    break;
                }
            }
            if !satisfied {
                return false;
            }
        }
        true
    })
}
pub fn lrat_check_load(lit: i32) -> bool {
    with_state(|state| {
        if lit == 0 {
            let cls = clause_init(&state.clause_to_add, state.clause_to_add.len() as i32);
            if let Some(old_cls) = state.clause_table.get(&state.id_to_add) {
                if !(state.lenient && clauses_equivalent(old_cls, &cls)) {
                    return false;
                }
            } else {
                state.clause_table.insert(state.id_to_add, cls);
            }
            if state.clause_to_add.is_empty() {
                state.unsat_proven = true;
            }
            if !state.clause_table.contains_key(&state.id_to_add) {
                return false;
            }
            state.id_to_add += 1;
            let mut serialized = state.clause_to_add.clone();
            serialized.push(0);
            let mut bytes = Vec::with_capacity(serialized.len() * std::mem::size_of::<i32>());
            for value in serialized {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
            state.hasher.siphash_update(&bytes, bytes.len() as u64);
            state.clause_to_add.clear();
            return true;
        }
        state.clause_to_add.push(lit);
        true
    })
}
pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    let key = [
        86_u8, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
    ];
    LRAT_STATE.with(|state| {
        *state.borrow_mut() = Some(LratState {
            clause_table: HashMap::new(),
            var_values: vec![0; (nb_vars.max(0) as usize) + 1],
            assigned_units: Vec::with_capacity(512),
            check_model: opt_check_model,
            lenient: opt_lenient,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            clause_to_add: Vec::with_capacity(512),
            done_loading: false,
            unsat_proven: false,
            hasher: crate::siphash::SipHash::siphash_init(&key),
        });
    });
}
pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls = data[..data.len().min(nb_lits.max(0) as usize)].to_vec();
    cls.push(0);
    cls
}
pub fn lrat_check_validate_unsat() -> bool {
    with_state(|state| state.done_loading && state.unsat_proven)
}
