use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

const OSORT: u32 = 0x1;
const ISORT: u32 = 0x2;

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
        Self(f32::MAX)
    }

    fn one() -> Self {
        Self(0.0)
    }

    fn plus(&self, rhs: &Self) -> Self {
        Self(self.0.min(rhs.0))
    }

    fn prod(&self, rhs: &Self) -> Self {
        Self(self.0 + rhs.0)
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
        self.states.push(State {
            arcs: Vec::new(),
            final_weight: None,
        });
        self.states.len() - 1
    }

    pub fn set_final(&mut self, st: usize, weight: W) {
        self.states[st].final_weight = Some(weight);
    }

    pub fn add_arc(&mut self, src: usize, arc: Arc<W>) {
        self.states[src].arcs.push(arc);
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

fn allowed<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>], i: usize, j: usize) -> bool {
    let al = arcs_a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

fn match_full_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < arcs_a.len() && j < arcs_b.len() {
        if arcs_a[i].olabel < arcs_b[j].ilabel {
            i += 1;
        } else if arcs_a[i].olabel > arcs_b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < arcs_b.len() && arcs_a[i].olabel == arcs_b[t].ilabel {
                if allowed(arcs_a, arcs_b, i, t) {
                    out.push(ArcPair {
                        a: arcs_a[i].clone(),
                        b: arcs_b[t].clone(),
                    });
                }
                t += 1;
            }
            i += 1;
        }
    }
    out
}

fn match_half_sorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    for i in 0..arcs_a.len() {
        if arcs_b.is_empty() {
            break;
        }
        let mut l = 0usize;
        let mut h = arcs_b.len() - 1;
        while l <= h {
            let m = (l + h) >> 1;
            if arcs_a[i].olabel > arcs_b[m].ilabel {
                l = m + 1;
            } else if arcs_a[i].olabel < arcs_b[m].ilabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && arcs_a[i].olabel == arcs_b[ll - 1].ilabel {
                    ll -= 1;
                }
                while hh < h && arcs_a[i].olabel == arcs_b[hh + 1].ilabel {
                    hh += 1;
                }
                while ll <= hh {
                    if allowed(arcs_a, arcs_b, i, ll) {
                        out.push(ArcPair {
                            a: arcs_a[i].clone(),
                            b: arcs_b[ll].clone(),
                        });
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
    out
}

fn match_half_sorted_rev<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    for i in 0..arcs_b.len() {
        if arcs_a.is_empty() {
            break;
        }
        let mut l = 0usize;
        let mut h = arcs_a.len() - 1;
        while l <= h {
            let m = (l + h) >> 1;
            if arcs_b[i].ilabel > arcs_a[m].olabel {
                l = m + 1;
            } else if arcs_b[i].ilabel < arcs_a[m].olabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && arcs_b[i].ilabel == arcs_a[ll - 1].olabel {
                    ll -= 1;
                }
                while hh < h && arcs_b[i].ilabel == arcs_a[hh + 1].olabel {
                    hh += 1;
                }
                while ll <= hh {
                    if allowed(arcs_a, arcs_b, ll, i) {
                        out.push(ArcPair {
                            a: arcs_a[ll].clone(),
                            b: arcs_b[i].clone(),
                        });
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
    out
}

fn match_unsorted<W: Semiring>(arcs_a: &[Arc<W>], arcs_b: &[Arc<W>]) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    for i in 0..arcs_a.len() {
        for j in 0..arcs_b.len() {
            if arcs_a[i].olabel == arcs_b[j].ilabel && allowed(arcs_a, arcs_b, i, j) {
                out.push(ArcPair {
                    a: arcs_a[i].clone(),
                    b: arcs_b[j].clone(),
                });
            }
        }
    }
    out
}

fn match_arcs<W: Semiring>(fst_a: &Fst<W>, fst_b: &Fst<W>, pair: &StatePair, sr: &W) -> Vec<ArcPair<W>> {
    let state_a = &fst_a.states[pair.a];
    let state_b = &fst_b.states[pair.b];
    let osort = fst_a.flags & OSORT != 0;
    let isort = fst_b.flags & ISORT != 0;

    let mut arcs_a = Vec::with_capacity(state_a.arcs.len() + 1);
    let mut arcs_b = Vec::with_capacity(state_b.arcs.len() + 1);

    arcs_a.push(Arc {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.clone(),
    });
    arcs_b.push(Arc {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.clone(),
    });
    arcs_a.extend(state_a.arcs.iter().cloned());
    arcs_b.extend(state_b.arcs.iter().cloned());

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
    let mut fst_c = Fst::new();
    let mut agenda = VecDeque::new();
    let mut marked = HashMap::<StatePair, usize>::new();

    agenda.push_back(StatePair {
        a: fst_a.start,
        b: fst_b.start,
    });

    while let Some(pair) = agenda.pop_front() {
        let sc = if let Some(sc) = marked.get(&pair).copied() {
            sc
        } else {
            let sc = fst_c.add_state();
            if fst_a.states[pair.a].final_weight.is_some() && fst_b.states[pair.b].final_weight.is_some() {
                fst_c.set_final(sc, W::one());
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = sc;
            }
            marked.insert(pair, sc);
            sc
        };

        for matched in match_arcs(fst_a, fst_b, &pair, sr) {
            let dst_pair = StatePair {
                a: matched.a.state,
                b: matched.b.state,
            };
            let dst = if let Some(dst) = marked.get(&dst_pair).copied() {
                dst
            } else {
                let dst = fst_c.add_state();
                if fst_a.states[dst_pair.a].final_weight.is_some()
                    && fst_b.states[dst_pair.b].final_weight.is_some()
                {
                    fst_c.set_final(dst, W::one());
                }
                agenda.push_back(dst_pair);
                marked.insert(dst_pair, dst);
                dst
            };

            fst_c.add_arc(
                sc,
                Arc {
                    state: dst,
                    ilabel: matched.a.ilabel,
                    olabel: matched.b.olabel,
                    weight: matched.a.weight.prod(&matched.b.weight),
                },
            );
        }
    }

    fst_c
}
