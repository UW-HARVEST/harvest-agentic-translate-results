// This file contains the public stub Compose API used by tests for fst.rs
// (Fst::compose), and a thin trait wrapper.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::fst::{ArcData, Fst as RFst, Spair, EPS, ISORT, OSORT};
use crate::queue::Queue;
use crate::sr::{self, Sr};

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
        if self.0 < rhs.0 {
            self.clone()
        } else {
            rhs.clone()
        }
    }
    fn prod(&self, rhs: &Self) -> Self {
        FloatSemiring(self.0 + rhs.0)
    }
}

#[derive(Clone, Debug)]
pub struct Arc<W: Semiring> {
    pub state: usize,
    pub ilabel: u32,
    pub olabel: u32,
    pub weight: W,
}

#[derive(Clone, Debug)]
pub struct State<W: Semiring> {
    pub arcs: Vec<Arc<W>>,
    pub final_weight: Option<W>,
}

#[derive(Clone, Debug)]
pub struct Fst<W: Semiring> {
    pub states: Vec<State<W>>,
    pub start: usize,
    pub flags: u32,
}

impl<W: Semiring> Fst<W> {
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            start: 0,
            flags: 0,
        }
    }
    pub fn add_state(&mut self) -> usize {
        self.states.push(State { arcs: Vec::new(), final_weight: None });
        self.states.len() - 1
    }
    pub fn set_final(&mut self, st: usize, weight: W) {
        if st < self.states.len() {
            self.states[st].final_weight = Some(weight);
        }
    }
    pub fn add_arc(&mut self, src: usize, arc: Arc<W>) {
        if src < self.states.len() {
            self.states[src].arcs.push(arc);
        }
    }
}

#[derive(Copy, Clone, Debug, Eq)]
pub struct StatePair {
    pub a: usize,
    pub b: usize,
}

impl PartialEq for StatePair {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}
impl Hash for StatePair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.a);
        state.write_usize(self.b);
    }
}

#[derive(Clone, Debug)]
pub struct ArcPair<W: Semiring> {
    pub a: Arc<W>,
    pub b: Arc<W>,
}

pub const EPS_LBL: u32 = 0;

fn match_full_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    let m = arcs_a.len();
    let n = arcs_b.len();
    let mut i = 0;
    let mut j = 0;
    while i < m && j < n {
        if arcs_a[i].olabel < arcs_b[j].ilabel {
            i += 1;
        } else if arcs_a[i].olabel > arcs_b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && arcs_a[i].olabel == arcs_b[t].ilabel {
                out.push(ArcPair {
                    a: arcs_a[i].clone(),
                    b: arcs_b[t].clone(),
                });
                t += 1;
            }
            i += 1;
        }
    }
    out
}

fn match_half_sorted<W: Semiring>(_arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    Vec::new()
}
fn match_half_sorted_rev<W: Semiring>(_arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    Vec::new()
}
fn match_unsorted<W: Semiring>(_arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    Vec::new()
}

fn match_arcs<W: Semiring>(_fst_a: &Fst<W>, _fst_b: &Fst<W>, _pair: &StatePair, _sr: &W) -> Vec<ArcPair<W>> {
    Vec::new()
}

pub fn fst_compose<W: Semiring>(_fst_a: &Fst<W>, _fst_b: &Fst<W>, _sr: &W) -> Fst<W> {
    Fst::new()
}

// ----- Internal helpers used by main RFst type -----

pub const EPS_U: u32 = EPS;

pub fn match_arcs_impl(
    fst_a: &RFst,
    fst_b: &RFst,
    pair: &Spair,
    sr: &Sr,
    mq: &mut Queue<(ArcData, ArcData)>,
) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];

    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;

    // Construct arcs_a / arcs_b with eps self-loop at index 0
    let loop_a = ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    };
    let loop_b = ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    };
    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(state_a.arcs.len() + 1);
    arcs_a.push(loop_a);
    for arc in &state_a.arcs {
        arcs_a.push(arc.clone());
    }
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(state_b.arcs.len() + 1);
    arcs_b.push(loop_b);
    for arc in &state_b.arcs {
        arcs_b.push(arc.clone());
    }

    if isort && osort {
        crate::matcher::match_full_sorted(&arcs_a, &arcs_b, mq);
    } else if isort {
        crate::matcher::match_half_sorted(&arcs_a, &arcs_b, mq);
    } else if osort {
        crate::matcher::match_half_sorted_rev(&arcs_a, &arcs_b, mq);
    } else {
        crate::matcher::match_unsorted(&arcs_a, &arcs_b, mq);
    }
}

pub fn fst_compose_into(fst_a: &RFst, fst_b: &RFst, fst_c: &mut RFst) {
    fst_c.empty();
    let sr_struct = sr::sr_get(fst_a.sr_type);
    fst_c.sr_type = fst_a.sr_type;

    let mut q: Queue<Spair> = Queue::new();
    let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
    let mut marked: HashMap<(u32, u32), u32> = HashMap::new();

    let mut pair = Spair {
        a: fst_a.start,
        b: fst_b.start,
    };

    q.enqueue(Spair {
        a: pair.a,
        b: pair.b,
    });

    while let Some(p) = q.dequeue() {
        pair.a = p.a;
        pair.b = p.b;

        let state_a = &fst_a.states[pair.a as usize];
        let state_b = &fst_b.states[pair.b as usize];

        let sc: u32;
        if let Some(&existing) = marked.get(&(pair.a, pair.b)) {
            sc = existing;
        } else {
            sc = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(sc, sr_struct.one);
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = sc;
            }
            marked.insert((pair.a, pair.b), sc);
        }

        match_arcs_impl(fst_a, fst_b, &pair, &sr_struct, &mut mq);

        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let new_pair_a = arc_a.state;
            let new_pair_b = arc_b.state;
            let dst_sc: u32;
            if let Some(&existing) = marked.get(&(new_pair_a, new_pair_b)) {
                dst_sc = existing;
            } else {
                let dst_state_a = &fst_a.states[new_pair_a as usize];
                let dst_state_b = &fst_b.states[new_pair_b as usize];
                dst_sc = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(dst_sc, sr_struct.one);
                }
                q.enqueue(Spair {
                    a: new_pair_a,
                    b: new_pair_b,
                });
                marked.insert((new_pair_a, new_pair_b), dst_sc);
            }
            fst_c.add_arc(
                sc,
                dst_sc,
                arc_a.ilabel,
                arc_b.olabel,
                (sr_struct.prod)(arc_a.weight, arc_b.weight),
            );
        }
    }
}
