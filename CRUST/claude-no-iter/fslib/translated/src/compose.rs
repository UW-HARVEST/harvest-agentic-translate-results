use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// Flags
const OSORT: u32 = 0x1;
const ISORT: u32 = 0x2;

// A trait to represent your semiring
pub trait Semiring: Clone {
    fn zero() -> Self;
    fn one() -> Self;
    fn plus(&self, rhs: &Self) -> Self;
    fn prod(&self, rhs: &Self) -> Self;
}

// Example semiring for Weighted FST with float weights (Tropical semiring-like)
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
        // tropical: min
        if self.0 < rhs.0 { FloatSemiring(self.0) } else { FloatSemiring(rhs.0) }
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
        Fst { states: Vec::new(), start: 0, flags: 0 }
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

pub const EPS: u32 = 0;

fn arc_match<W: Semiring>(arcs_a: &[Arc<W>], _arcs_b: &[Arc<W>], i: usize, j: usize) -> bool {
    let al = arcs_a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

fn match_full_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let m = arcs_a.len();
    let n = arcs_b.len();
    let mut result = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if arcs_a[i].olabel < arcs_b[j].ilabel {
            i += 1;
        } else if arcs_a[i].olabel > arcs_b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && arcs_a[i].olabel == arcs_b[t].ilabel {
                if arc_match(arcs_a, arcs_b, i, t) {
                    result.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[t].clone() });
                }
                t += 1;
            }
            i += 1;
        }
    }
    result
}

fn match_half_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let n = arcs_b.len();
    let mut result = Vec::new();
    if n == 0 {
        return result;
    }
    for i in 0..arcs_a.len() {
        let mut l: usize = 0;
        let mut h: usize = n - 1;
        loop {
            if l > h { break; }
            let m = (l + h) >> 1;
            if arcs_a[i].olabel > arcs_b[m].ilabel {
                l = m + 1;
            } else if arcs_a[i].olabel < arcs_b[m].ilabel {
                if m == 0 { break; }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && arcs_a[i].olabel == arcs_b[ll - 1].ilabel { ll -= 1; }
                while hh < h && arcs_a[i].olabel == arcs_b[hh + 1].ilabel { hh += 1; }
                let mut k = ll;
                while k <= hh {
                    if arc_match(arcs_a, arcs_b, i, k) {
                        result.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[k].clone() });
                    }
                    k += 1;
                }
                break;
            }
        }
    }
    result
}

fn match_half_sorted_rev<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let m = arcs_a.len();
    let mut result = Vec::new();
    if m == 0 {
        return result;
    }
    for i in 0..arcs_b.len() {
        let mut l: usize = 0;
        let mut h: usize = m - 1;
        loop {
            if l > h { break; }
            let mid = (l + h) >> 1;
            if arcs_b[i].ilabel > arcs_a[mid].olabel {
                l = mid + 1;
            } else if arcs_b[i].ilabel < arcs_a[mid].olabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && arcs_b[i].ilabel == arcs_a[ll - 1].olabel { ll -= 1; }
                while hh < h && arcs_b[i].ilabel == arcs_a[hh + 1].olabel { hh += 1; }
                let mut k = ll;
                while k <= hh {
                    if arc_match(arcs_a, arcs_b, k, i) {
                        result.push(ArcPair { a: arcs_a[k].clone(), b: arcs_b[i].clone() });
                    }
                    k += 1;
                }
                break;
            }
        }
    }
    result
}

fn match_unsorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut result = Vec::new();
    for i in 0..arcs_a.len() {
        for j in 0..arcs_b.len() {
            if arcs_a[i].olabel == arcs_b[j].ilabel && arc_match(arcs_a, arcs_b, i, j) {
                result.push(ArcPair { a: arcs_a[i].clone(), b: arcs_b[j].clone() });
            }
        }
    }
    result
}

fn match_arcs<W: Semiring>(
    fst_a: &Fst<W>,
    fst_b: &Fst<W>,
    pair: &StatePair,
    sr: &W,
) -> Vec<ArcPair<W>> {
    let state_a = &fst_a.states[pair.a];
    let state_b = &fst_b.states[pair.b];

    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;

    let loop_a = Arc { state: pair.a, ilabel: EPS, olabel: EPS, weight: sr.clone() };
    let loop_b = Arc { state: pair.b, ilabel: EPS, olabel: EPS, weight: sr.clone() };

    let mut arcs_a: Vec<Arc<W>> = Vec::with_capacity(state_a.arcs.len() + 1);
    arcs_a.push(loop_a);
    for a in state_a.arcs.iter() {
        arcs_a.push(a.clone());
    }
    let mut arcs_b: Vec<Arc<W>> = Vec::with_capacity(state_b.arcs.len() + 1);
    arcs_b.push(loop_b);
    for a in state_b.arcs.iter() {
        arcs_b.push(a.clone());
    }
    if isort && osort {
        match_full_sorted(&arcs_a, &arcs_b)
    } else if isort || osort {
        if isort {
            match_half_sorted(&arcs_a, &arcs_b)
        } else {
            match_half_sorted_rev(&arcs_a, &arcs_b)
        }
    } else {
        match_unsorted(&arcs_a, &arcs_b)
    }
}

pub fn fst_compose<W: Semiring>(fst_a: &Fst<W>, fst_b: &Fst<W>, sr: &W) -> Fst<W> {
    let mut result: Fst<W> = Fst::new();
    if fst_a.states.is_empty() || fst_b.states.is_empty() {
        return result;
    }
    let mut marked: HashMap<StatePair, usize> = HashMap::new();
    let mut q: std::collections::VecDeque<StatePair> = std::collections::VecDeque::new();

    let initial = StatePair { a: fst_a.start, b: fst_b.start };
    q.push_back(initial);

    while let Some(pair) = q.pop_front() {
        let sc = if let Some(&id) = marked.get(&pair) {
            id
        } else {
            let id = result.add_state();
            let sa = &fst_a.states[pair.a];
            let sb = &fst_b.states[pair.b];
            if let (Some(_), Some(_)) = (&sa.final_weight, &sb.final_weight) {
                result.set_final(id, W::one());
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                result.start = id;
            }
            marked.insert(pair, id);
            id
        };

        let pairs = match_arcs(fst_a, fst_b, &pair, sr);
        for ap in pairs {
            let new_pair = StatePair { a: ap.a.state, b: ap.b.state };
            let dst_sc = if let Some(&id) = marked.get(&new_pair) {
                id
            } else {
                let id = result.add_state();
                let sa = &fst_a.states[new_pair.a];
                let sb = &fst_b.states[new_pair.b];
                if let (Some(_), Some(_)) = (&sa.final_weight, &sb.final_weight) {
                    result.set_final(id, W::one());
                }
                q.push_back(new_pair);
                marked.insert(new_pair, id);
                id
            };
            let weight = ap.a.weight.prod(&ap.b.weight);
            result.add_arc(sc, Arc {
                state: dst_sc,
                ilabel: ap.a.ilabel,
                olabel: ap.b.olabel,
                weight,
            });
        }
    }
    result
}
