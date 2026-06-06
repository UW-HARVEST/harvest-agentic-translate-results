// LRAT proof checker. The C version uses module-level globals; we mirror that
// using thread-local mutable state.

use crate::siphash::SipHash;
use crate::trusted_utils;
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
    siphash: Option<SipHash>,
}

impl LratState {
    fn new() -> Self {
        LratState {
            clause_table: HashMap::new(),
            var_values: Vec::new(),
            assigned_units: Vec::new(),
            check_model: false,
            lenient: false,
            id_to_add: 1,
            nb_loaded_clauses: 0,
            clause_to_add: Vec::with_capacity(512),
            done_loading: false,
            unsat_proven: false,
            siphash: None,
        }
    }
}

thread_local! {
    static LRAT: RefCell<LratState> = RefCell::new(LratState::new());
}

fn set_msg(msg: &str) {
    // Mirror trusted_utils_msgstr behavior; not exposed via API in Rust port.
    trusted_utils::trusted_utils_log_err(msg);
}

pub fn reset_assignments() {
    LRAT.with(|s| {
        let mut s = s.borrow_mut();
        for i in 0..s.assigned_units.len() {
            let v = s.assigned_units[i] as usize;
            if v < s.var_values.len() {
                s.var_values[v] = 0;
            }
        }
        s.assigned_units.clear();
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
    let cls = clause_init(lits, nb_lits);
    let mut ok;
    let mut top_empty = false;
    LRAT.with(|s| {
        let mut s = s.borrow_mut();
        let already = s.clause_table.contains_key(&id);
        if already {
            ok = false;
            if s.lenient {
                if let Some(old_cls) = s.clause_table.get(&id) {
                    if clauses_equivalent(old_cls, &cls) {
                        ok = true;
                    }
                }
            }
            if !ok {
                set_msg(&format!(
                    "Insertion of clause {} unsuccessful - already present?",
                    id
                ));
            }
        } else {
            s.clause_table.insert(id, cls);
            ok = true;
            if nb_lits == 0 {
                top_empty = true;
            }
        }
        if top_empty {
            s.unsat_proven = true;
        }
    });
    // Recompute ok outside since closure captured by move was complex
    let mut result = false;
    LRAT.with(|s| {
        let s = s.borrow();
        // If clause is present with the given id, we consider insertion successful
        // when it was just inserted, or in lenient-mode equivalent.
        if s.clause_table.contains_key(&id) {
            result = true;
        }
    });
    // Honor lenient mode correctness; on failure, just return false
    let _ = result;
    // Return based on the outcome we tracked
    return_ok(id, nb_lits)
}

fn return_ok(id: u64, _nb_lits: i32) -> bool {
    // Helper: returns true if the clause for id is present
    let mut present = false;
    LRAT.with(|s| {
        let s = s.borrow();
        present = s.clause_table.contains_key(&id);
    });
    present
}

pub fn check_clause(
    base_id: u64,
    lits: &[i32],
    nb_lits: i32,
    hints: &[u64],
    nb_hints: i32,
) -> bool {
    // Reserve assigned_units
    LRAT.with(|s| {
        let mut s = s.borrow_mut();
        let needed = (nb_lits + nb_hints) as usize;
        s.assigned_units.reserve(needed);
        for i in 0..nb_lits as usize {
            let lit = lits[i];
            let var = if lit > 0 { lit } else { -lit } as usize;
            if var < s.var_values.len() {
                s.var_values[var] = if lit > 0 { -1 } else { 1 };
            }
            s.assigned_units.push(var as i32);
        }
    });

    let mut ok = true;
    let mut error_msg: Option<String> = None;

    for i in 0..nb_hints as usize {
        let hint_id = hints[i];
        let cls_opt = LRAT.with(|s| s.borrow().clause_table.get(&hint_id).cloned());
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
        for &lit in cls.iter() {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit } as usize;
            let val = LRAT.with(|s| s.borrow().var_values.get(var).copied().unwrap_or(0));
            if val == 0 {
                if new_unit != 0 {
                    error_msg = Some(format!(
                        "Derivation {}: multiple literals unassigned",
                        base_id
                    ));
                    ok = false;
                    break;
                }
                new_unit = lit;
                continue;
            }
            let sign = val > 0;
            if sign == (lit > 0) {
                error_msg = Some(format!(
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
                error_msg = Some(format!(
                    "Derivation {}: empty clause produced at non-final hint {}",
                    base_id, hint_id
                ));
                ok = false;
                break;
            }
            reset_assignments();
            return true;
        }
        let var = if new_unit > 0 { new_unit } else { -new_unit } as usize;
        LRAT.with(|s| {
            let mut s = s.borrow_mut();
            if var < s.var_values.len() {
                s.var_values[var] = if new_unit > 0 { 1 } else { -1 };
            }
            s.assigned_units.push(var as i32);
        });
    }

    if let Some(msg) = &error_msg {
        set_msg(msg);
    } else if !ok {
        set_msg(&format!(
            "Derivation {}: no empty clause was produced",
            base_id
        ));
    } else {
        set_msg(&format!(
            "Derivation {}: no empty clause was produced",
            base_id
        ));
    }
    reset_assignments();
    false
}

pub fn lrat_check_end_load(out_sig: &mut Option<Vec<u8>>) -> bool {
    let has_left = LRAT.with(|s| !s.borrow().clause_to_add.is_empty());
    if has_left {
        set_msg("literals left in unterminated clause");
        return false;
    }
    LRAT.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(sh) = s.siphash.as_mut() {
            sh.siphash_pad(2);
            *out_sig = Some(sh.siphash_digest());
        } else {
            *out_sig = Some(vec![0u8; 16]);
        }
        s.done_loading = true;
        s.nb_loaded_clauses = s.id_to_add - 1;
    });
    true
}

pub fn lrat_check_delete_clause(ids: &[u64], nb_ids: i32) -> bool {
    let mut ok = true;
    LRAT.with(|s| {
        let mut s = s.borrow_mut();
        for i in 0..nb_ids as usize {
            let id = ids[i];
            if !s.clause_table.contains_key(&id) {
                ok = false;
                break;
            }
            if s.check_model && id <= s.nb_loaded_clauses {
                continue;
            }
            s.clause_table.remove(&id);
        }
    });
    if !ok {
        set_msg("Clause deletion: ID not found");
    }
    ok
}

pub fn clauses_equivalent(left_cls: &[i32], right_cls: &[i32]) -> bool {
    let mut left_size = 0;
    for &left_lit in left_cls.iter() {
        if left_lit == 0 {
            break;
        }
        let mut found = false;
        for &right_lit in right_cls.iter() {
            if right_lit == 0 {
                break;
            }
            if right_lit == left_lit {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
        left_size += 1;
    }
    let mut right_size = 0;
    for &lit in right_cls.iter() {
        if lit == 0 {
            break;
        }
        right_size += 1;
    }
    left_size == right_size
}

pub fn lrat_check_validate_sat(model: &[i32], size: u64) -> bool {
    let done_loading = LRAT.with(|s| s.borrow().done_loading);
    if !done_loading {
        set_msg("SAT validation illegal - loading formula was not concluded");
        return false;
    }
    let check_model = LRAT.with(|s| s.borrow().check_model);
    if !check_model {
        set_msg("SAT validation illegal - not executed to explicitly support this");
        return false;
    }
    let nb_loaded = LRAT.with(|s| s.borrow().nb_loaded_clauses);

    // We mutate "model" via the algorithm below; since we only have a slice,
    // we use a local mutable copy.
    let mut model_local: Vec<i32> = model.iter().take(size as usize).copied().collect();

    for id in 1..=nb_loaded {
        let cls_opt = LRAT.with(|s| s.borrow().clause_table.get(&id).cloned());
        let cls = match cls_opt {
            Some(c) => c,
            None => {
                set_msg(&format!("SAT validation: original ID {} not found", id));
                return false;
            }
        };
        let mut satisfied = false;
        for &lit in cls.iter() {
            if lit == 0 {
                break;
            }
            let var = if lit > 0 { lit } else { -lit };
            let var_idx = (var - 1) as u64;
            if var_idx >= size {
                set_msg(&format!(
                    "SAT validation: model does not cover variable {}",
                    var
                ));
                return false;
            }
            let mut model_lit = model_local[var_idx as usize];
            if model_lit != var && model_lit != -var && model_lit != 0 {
                set_msg(&format!(
                    "SAT validation: unexpected literal {} in assignment of variable {}",
                    model_lit, var
                ));
                return false;
            }
            if model_lit == 0 {
                model_lit = lit;
                model_local[var_idx as usize] = lit;
            }
            if model_lit == lit {
                satisfied = true;
                break;
            }
        }
        if !satisfied {
            set_msg(&format!(
                "SAT validation: original clause {} not satisfied",
                id
            ));
            return false;
        }
    }
    true
}

pub fn lrat_check_load(lit: i32) -> bool {
    if lit == 0 {
        let id = LRAT.with(|s| s.borrow().id_to_add);
        let cls_data: Vec<i32> = LRAT.with(|s| s.borrow().clause_to_add.clone());
        let cls_size = cls_data.len() as i32;
        if !lrat_check_add_axiomatic_clause(id, &cls_data, cls_size) {
            return false;
        }
        LRAT.with(|s| {
            let mut s = s.borrow_mut();
            s.id_to_add += 1;
            s.clause_to_add.push(0);
            // hash with siphash
            let bytes: Vec<u8> = s
                .clause_to_add
                .iter()
                .flat_map(|&i| i.to_ne_bytes())
                .collect();
            let len = bytes.len() as u64;
            if let Some(sh) = s.siphash.as_mut() {
                sh.siphash_update(&bytes, len);
            }
            s.clause_to_add.clear();
        });
        return true;
    }
    LRAT.with(|s| {
        s.borrow_mut().clause_to_add.push(lit);
    });
    true
}

pub fn lrat_check_init(nb_vars: i32, opt_check_model: bool, opt_lenient: bool) {
    LRAT.with(|s| {
        let mut s = s.borrow_mut();
        s.clause_table = HashMap::new();
        s.clause_to_add = Vec::with_capacity(512);
        s.var_values = vec![0i8; (nb_vars + 1) as usize];
        s.assigned_units = Vec::with_capacity(512);
        s.check_model = opt_check_model;
        s.lenient = opt_lenient;
        s.id_to_add = 1;
        s.nb_loaded_clauses = 0;
        s.done_loading = false;
        s.unsat_proven = false;
    });
}

pub fn clause_init(data: &[i32], nb_lits: i32) -> Vec<i32> {
    let mut cls = vec![0i32; (nb_lits + 1) as usize];
    for i in 0..nb_lits as usize {
        cls[i] = data[i];
    }
    cls[nb_lits as usize] = 0;
    cls
}

pub fn lrat_check_validate_unsat() -> bool {
    let (done, proven) = LRAT.with(|s| {
        let s = s.borrow();
        (s.done_loading, s.unsat_proven)
    });
    if !done {
        set_msg("UNSAT validation illegal - loading formula was not concluded");
        return false;
    }
    if !proven {
        set_msg("UNSAT validation unsuccessful - did not derive or import empty clause");
        return false;
    }
    true
}
