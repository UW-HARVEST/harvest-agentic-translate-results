use std::collections::HashMap;
use crate::fst::{Fst, ArcData, Spair, OSORT, ISORT, EPS};
use crate::queue::Queue;
use crate::sr::{Sr, sr_get};

// Trait to represent your semiring (kept for compatibility)
pub trait Semiring: Clone {
    fn zero() -> Self;
    fn one() -> Self;
    fn plus(&self, rhs: &Self) -> Self;
    fn prod(&self, rhs: &Self) -> Self;
}
#[derive(Clone, Debug)]
pub struct FloatSemiring(pub f32);
impl Semiring for FloatSemiring {
    fn zero() -> Self {
        FloatSemiring(f32::MAX)
    }
    fn one() -> Self {
        FloatSemiring(0.0)
    }
    fn plus(&self, rhs: &Self) -> Self {
        FloatSemiring(if self.0 < rhs.0 { self.0 } else { rhs.0 })
    }
    fn prod(&self, rhs: &Self) -> Self {
        FloatSemiring(self.0 + rhs.0)
    }
}

// `match_arcs` exposed on real Fst for use by compose
pub fn match_arcs(
    fst_a: &Fst,
    fst_b: &Fst,
    pair: &Spair,
    sr: &Sr,
    mq: &mut Queue<(ArcData, ArcData)>,
) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];

    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;

    // Build arcs lists prepended with eps loop
    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(state_a.arcs.len() + 1);
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(state_b.arcs.len() + 1);

    arcs_a.push(ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    arcs_a.extend_from_slice(&state_a.arcs);

    arcs_b.push(ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    arcs_b.extend_from_slice(&state_b.arcs);

    if isort && osort {
        crate::matcher::match_full_sorted(&arcs_a, &arcs_b, mq);
    } else if isort || osort {
        if isort {
            crate::matcher::match_half_sorted(&arcs_a, &arcs_b, mq);
        } else {
            crate::matcher::match_half_sorted_rev(&arcs_a, &arcs_b, mq);
        }
    } else {
        crate::matcher::match_unsorted(&arcs_a, &arcs_b, mq);
    }
}

pub fn fst_compose_real(fst_a: &Fst, fst_b: &Fst, fst_c: &mut Fst) {
    let sr = sr_get(fst_a.sr_type);
    let mut q: Queue<(u32, u32)> = Queue::new();
    let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
    let mut marked: HashMap<(u32, u32), u32> = HashMap::new();

    let init_pair = (fst_a.start, fst_b.start);
    q.enqueue(init_pair);

    while let Some(pair) = q.dequeue() {
        let state_a = &fst_a.states[pair.0 as usize];
        let state_b = &fst_b.states[pair.1 as usize];

        let sc = if let Some(&existing) = marked.get(&pair) {
            existing
        } else {
            let new_state = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(new_state, sr.one);
            }
            if pair.0 == fst_a.start && pair.1 == fst_b.start {
                fst_c.start = new_state;
            }
            marked.insert(pair, new_state);
            new_state
        };

        // Match arcs
        let spair = Spair { a: pair.0, b: pair.1 };
        match_arcs(fst_a, fst_b, &spair, &sr, &mut mq);

        while let Some(mi) = mq.dequeue() {
            let arc_a = mi.0;
            let arc_b = mi.1;

            let new_pair = (arc_a.state, arc_b.state);

            let dst_sc = if let Some(&existing) = marked.get(&new_pair) {
                existing
            } else {
                let dst_state_a = &fst_a.states[new_pair.0 as usize];
                let dst_state_b = &fst_b.states[new_pair.1 as usize];

                let new_state = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(new_state, sr.one);
                }
                q.enqueue(new_pair);
                marked.insert(new_pair, new_state);
                new_state
            };

            fst_c.add_arc(
                sc,
                dst_sc,
                arc_a.ilabel,
                arc_b.olabel,
                (sr.prod)(arc_a.weight, arc_b.weight),
            );
        }
    }
}
