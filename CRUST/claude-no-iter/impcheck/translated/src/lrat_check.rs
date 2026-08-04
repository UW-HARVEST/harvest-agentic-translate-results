// Note: This file isn't included in `lib.rs`, so it isn't compiled
// as part of the library. It is provided for completeness.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;

use crate::secret::SECRET_KEY;
use crate::siphash::SipHash;
use crate::trusted_utils;

thread_local! {
    static STATE: RefCell<LratState> = RefCell::new(LratState::new());
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
    sip: SipHash,
    msgstr: String,
}

impl LratState {
    fn new() -> Self {
        Self {
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
            sip: SipHash::siphash_init(&SECRET_KEY),
            msgstr: String::new(),
        }
    }
}

fn with_state<R>(f: impl FnOnce(&mut LratState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

pub fn reset_assignments() {
    with_state(|s| {
        for &v in &s.assigned_units {
            s.var_values[v as usize] = 0;
        }
        s.assigned_units.clear();
    });
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls = Vec::with_capacity(nb_lits as usize + 1);
    for i in 0..nb_lits as usize {
        cls.push(data[i]);
    }
    cls.push(0);
    cls
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut lit_idx = 0usize;
    while left_cls[lit_idx] != 0 {
        let left_lit = left_cls[lit_idx];
        let mut found = false;
        let mut right_lit_idx = 0usize;
        while right_cls[right_lit_idx] != 0 {
            if right_cls[right_lit_idx] == left_lit {
                found = true;
                break;
            }
            right_lit_idx += 1;
        }
        if !found {
            return false;
        }
        lit_idx += 1;
    }
    let left_size = lit_idx;
    let mut idx = 0usize;
    while right_cls[idx] != 0 {
        idx += 1;
    }
    left_size == idx
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    with_state(|s| {
        s.clause_table.clear();
        s.var_values = vec![0i8; (nb_vars + 1) as usize];
        s.assigned_units.clear();
        s.check_model = opt_check_model;
        s.lenient = opt_lenient;
        s.id_to_add = 1;
        s.nb_loaded_clauses = 0;
        s.clause_to_add.clear();
        s.done_loading = false;
        s.unsat_proven = false;
        s.sip = SipHash::siphash_init(&SECRET_KEY);
        s.msgstr.clear();
    });
}

pub fn lrat_check_add_axiomatic_clause(id: u64, lits: &[i32], nb_lits: i32) -> bool {
    with_state(|s| {
        let cls = clause_init(lits, nb_lits);
        let already_present = s.clause_table.contains_key(&id);
        if already_present {
            if s.lenient {
                let old = s.clause_table.get(&id).cloned();
                if let Some(old_cls) = old {
                    if clauses_equivalent(&old_cls, &cls) {
                        return true;
                    }
                }
            }
            s.msgstr = format!(
                "Insertion of clause {} unsuccessful - already present?",
                id
            );
            return false;
        }
        s.clause_table.insert(id, cls);
        if nb_lits == 0 {
            s.unsat_proven = true;
        }
        true
    })
}

pub fn check_clause(
    base_id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    let result = with_state(|s| -> Result<bool, ()> {
        if s.assigned_units.capacity() < (nb_lits + nb_hints) as usize {
            s.assigned_units
                .reserve((nb_lits + nb_hints) as usize - s.assigned_units.capacity());
        }
        for i in 0..nb_lits as usize {
            let lit = lits[i];
            let var = if lit > 0 { lit } else { -lit };
            s.var_values[var as usize] = if lit > 0 { -1 } else { 1 };
            s.assigned_units.push(var);
        }
        let mut ok = true;
        let mut produced_empty = false;
        for i in 0..nb_hints as usize {
            let hint_id = hints[i];
            let cls = match s.clause_table.get(&hint_id).cloned() {
                Some(c) => c,
                None => {
                    s.msgstr = format!(
                        "Derivation {}: hint {} not found",
                        base_id, hint_id
                    );
                    ok = false;
                    break;
                }
            };
            let mut new_unit = 0i32;
            let mut bad = false;
            let mut lit_idx = 0usize;
            loop {
                let lit = cls[lit_idx];
                if lit == 0 {
                    break;
                }
                let var = if lit > 0 { lit } else { -lit };
                if s.var_values[var as usize] == 0 {
                    if new_unit != 0 {
                        s.msgstr = format!(
                            "Derivation {}: multiple literals unassigned",
                            base_id
                        );
                        ok = false;
                        bad = true;
                        break;
                    }
                    new_unit = lit;
                } else {
                    let sign = s.var_values[var as usize] > 0;
                    if sign == (lit > 0) {
                        s.msgstr = format!(
                            "Derivation {}: dependency {} is satisfied",
                            base_id, hint_id
                        );
                        ok = false;
                        bad = true;
                        break;
                    }
                }
                lit_idx += 1;
            }
            if bad {
                break;
            }
            if !ok {
                break;
            }
            if new_unit == 0 {
                if (i + 1) < nb_hints as usize {
                    s.msgstr = format!(
                        "Derivation {}: empty clause produced at non-final hint {}",
                        base_id, hint_id
                    );
                    break;
                }
                produced_empty = true;
                break;
            }
            let var = if new_unit > 0 { new_unit } else { -new_unit };
            s.var_values[var as usize] = if new_unit > 0 { 1 } else { -1 };
            s.assigned_units.push(var);
        }
        if produced_empty {
            for &v in &s.assigned_units {
                s.var_values[v as usize] = 0;
            }
            s.assigned_units.clear();
            return Ok(true);
        }
        if s.msgstr.is_empty() {
            s.msgstr = format!("Derivation {}: no empty clause was produced", base_id);
        }
        for &v in &s.assigned_units {
            s.var_values[v as usize] = 0;
        }
        s.assigned_units.clear();
        Ok(false)
    });
    result.unwrap_or(false)
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        let (id, lits) = with_state(|s| {
            let cls = s.clause_to_add.clone();
            (s.id_to_add, cls)
        });
        let nb_lits = lits.len() as i32;
        if !lrat_check_add_axiomatic_clause(id, &lits, nb_lits) {
            return false;
        }
        with_state(|s| {
            s.id_to_add += 1;
            s.clause_to_add.push(0);
            // Hash bytes of the clause
            let bytes_vec: Vec<u8> = s
                .clause_to_add
                .iter()
                .flat_map(|i| i.to_ne_bytes())
                .collect();
            s.sip.siphash_update(&bytes_vec, bytes_vec.len() as u64);
            s.clause_to_add.clear();
        });
        return true;
    }
    with_state(|s| s.clause_to_add.push(lit));
    true
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    with_state(|s| {
        if !s.clause_to_add.is_empty() {
            s.msgstr = "literals left in unterminated clause".to_string();
            return false;
        }
        s.sip.siphash_pad(2);
        let sig = s.sip.siphash_digest();
        *out_sig = Some(sig);
        s.done_loading = true;
        s.nb_loaded_clauses = s.id_to_add - 1;
        true
    })
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

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    with_state(|s| {
        for i in 0..nb_ids as usize {
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
    })
}

pub fn lrat_check_validate_unsat() -> bool {
    with_state(|s| {
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
    })
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    with_state(|s| {
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
        let mut model_local = model.to_vec();
        for id in 1..=s.nb_loaded_clauses {
            let cls = match s.clause_table.get(&id).cloned() {
                Some(c) => c,
                None => {
                    s.msgstr = format!("SAT validation: original ID {} not found", id);
                    return false;
                }
            };
            let mut satisfied = false;
            let mut lit_idx = 0usize;
            while cls[lit_idx] != 0 {
                let lit = cls[lit_idx];
                let var = if lit > 0 { lit } else { -lit };
                if (var as u64 - 1) >= size {
                    s.msgstr = format!(
                        "SAT validation: model does not cover variable {}",
                        var
                    );
                    return false;
                }
                let mut model_lit = model_local[(var - 1) as usize];
                if model_lit != var && model_lit != -var && model_lit != 0 {
                    s.msgstr = format!(
                        "SAT validation: unexpected literal {} in assignment of variable {}",
                        model_lit, var
                    );
                    return false;
                }
                if model_lit == 0 {
                    model_lit = lit;
                    model_local[(var - 1) as usize] = lit;
                }
                if model_lit == lit {
                    satisfied = true;
                    break;
                }
                lit_idx += 1;
            }
            if !satisfied {
                s.msgstr =
                    format!("SAT validation: original clause {} not satisfied", id);
                return false;
            }
        }
        true
    })
}

pub fn _ensure_used() {
    let _ = trusted_utils::SIG_SIZE_BYTES;
}
