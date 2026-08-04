use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
// Flags (just examples; adapt to your constants)
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
// Arc representation
#[derive(Clone, Debug)]
pub struct Arc<W: Semiring> {
    pub state: usize,
    pub ilabel: u32,
    pub olabel: u32,
    pub weight: W,
}
// State representation
#[derive(Clone, Debug)]
pub struct State<W: Semiring> {
    pub arcs: Vec<Arc<W>>,
    pub final_weight: Option<W>,
}
// Fst representation
#[derive(Clone, Debug)]
pub struct Fst<W: Semiring> {
    pub states: Vec<State<W>>,
    pub start: usize,
    pub flags: u32,   // Might store OSORT/ISORT flags, etc.
    // sr_type, etc. could go here if needed
}
impl<W: Semiring> Fst<W> {
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            start: 0,
            flags: 0,
        }
    }
    // Add a new state, returns its index
    pub fn add_state(&mut self) -> usize {
        self.states.push(State {
            arcs: Vec::new(),
            final_weight: None,
        });
        self.states.len() - 1
    }
    // Set final weight
    pub fn set_final(&mut self, st: usize, weight: W) {
        self.states[st].final_weight = Some(weight);
    }
    // Add arc
    pub fn add_arc(&mut self, src: usize, arc: Arc<W>) {
        self.states[src].arcs.push(arc);
    }
}
// A pair of states (a,b)
#[derive(Copy, Clone, Debug, Eq)]
pub struct StatePair {
    pub a: usize,
    pub b: usize,
}
// We need to implement PartialEq and Hash for StatePair
impl PartialEq for StatePair {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}
impl Hash for StatePair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // A simple combination for demonstration
        // for fewer collisions you can do something stronger
        state.write_usize(self.a);
        state.write_usize(self.b);
    }
}
// A pair of arcs matched together
#[derive(Clone, Debug)]
pub struct ArcPair<W: Semiring> {
    pub a: Arc<W>,
    pub b: Arc<W>,
}
// The “EPS” label constant
pub const EPS: u32 = 0;
fn match_full_sorted<W: Semiring>(
    arcs_a: &[Arc<W>],
    arcs_b: &[Arc<W>],
) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < arcs_a.len() && j < arcs_b.len() {
        if arcs_a[i].olabel < arcs_b[j].ilabel {
            i += 1;
        } else if arcs_a[i].olabel > arcs_b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < arcs_b.len() && arcs_a[i].olabel == arcs_b[t].ilabel {
                if match_pair(arcs_a, arcs_b, i, t) {
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
fn match_half_sorted<W: Semiring>(
    arcs_a: &[Arc<W>],
    arcs_b: &[Arc<W>],
) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    for (i, arc_a) in arcs_a.iter().enumerate() {
        let mut l = 0usize;
        let mut h = arcs_b.len().saturating_sub(1);
        while l <= h && !arcs_b.is_empty() {
            let m = (l + h) >> 1;
            if arc_a.olabel > arcs_b[m].ilabel {
                l = m + 1;
            } else if arc_a.olabel < arcs_b[m].ilabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && arc_a.olabel == arcs_b[ll - 1].ilabel {
                    ll -= 1;
                }
                while hh < h && arc_a.olabel == arcs_b[hh + 1].ilabel {
                    hh += 1;
                }
                while ll <= hh {
                    if match_pair(arcs_a, arcs_b, i, ll) {
                        out.push(ArcPair {
                            a: arc_a.clone(),
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
fn match_half_sorted_rev<W: Semiring>(
    arcs_a: &[Arc<W>],
    arcs_b: &[Arc<W>],
) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    for (i, arc_b) in arcs_b.iter().enumerate() {
        let mut l = 0usize;
        let mut h = arcs_a.len().saturating_sub(1);
        while l <= h && !arcs_a.is_empty() {
            let m = (l + h) >> 1;
            if arc_b.ilabel > arcs_a[m].olabel {
                l = m + 1;
            } else if arc_b.ilabel < arcs_a[m].olabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && arc_b.ilabel == arcs_a[ll - 1].olabel {
                    ll -= 1;
                }
                while hh < h && arc_b.ilabel == arcs_a[hh + 1].olabel {
                    hh += 1;
                }
                while ll <= hh {
                    if match_pair(arcs_a, arcs_b, ll, i) {
                        out.push(ArcPair {
                            a: arcs_a[ll].clone(),
                            b: arc_b.clone(),
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
fn match_unsorted<W: Semiring>(
    arcs_a: &[Arc<W>],
    arcs_b: &[Arc<W>],
) -> Vec<ArcPair<W>> {
    let mut out = Vec::new();
    for (i, arc_a) in arcs_a.iter().enumerate() {
        for (j, arc_b) in arcs_b.iter().enumerate() {
            if arc_a.olabel == arc_b.ilabel && match_pair(arcs_a, arcs_b, i, j) {
                out.push(ArcPair {
                    a: arc_a.clone(),
                    b: arc_b.clone(),
                });
            }
        }
    }
    out
}
/// Matches arcs from two states (given by `pair`) and enqueues matched pairs
fn match_arcs<W: Semiring>(
    fst_a: &Fst<W>,
    fst_b: &Fst<W>,
    pair: &StatePair,
    sr: &W,
) -> Vec<ArcPair<W>> {
    let state_a = &fst_a.states[pair.a];
    let state_b = &fst_b.states[pair.b];
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

    let osort = fst_a.flags & OSORT != 0;
    let isort = fst_b.flags & ISORT != 0;
    if osort && isort {
        match_full_sorted(&arcs_a, &arcs_b)
    } else if isort {
        match_half_sorted(&arcs_a, &arcs_b)
    } else if osort {
        match_half_sorted_rev(&arcs_a, &arcs_b)
    } else {
        match_unsorted(&arcs_a, &arcs_b)
    }
}
/// Compose two FSTs into a third. 
pub fn fst_compose<W: Semiring>(
    fst_a: &Fst<W>,
    fst_b: &Fst<W>,
    sr: &W,             // e.g., sr = FloatSemiring::one() or something
) -> Fst<W> {
    let mut fst_c = Fst::new();
    let mut agenda = VecDeque::new();
    let mut marked = HashMap::new();

    let start = StatePair {
        a: fst_a.start,
        b: fst_b.start,
    };
    agenda.push_back(start);

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

        for mi in match_arcs(fst_a, fst_b, &pair, sr) {
            let next_pair = StatePair {
                a: mi.a.state,
                b: mi.b.state,
            };
            let dst = if let Some(dst) = marked.get(&next_pair).copied() {
                dst
            } else {
                let dst = fst_c.add_state();
                if fst_a.states[next_pair.a].final_weight.is_some()
                    && fst_b.states[next_pair.b].final_weight.is_some()
                {
                    fst_c.set_final(dst, W::one());
                }
                marked.insert(next_pair, dst);
                agenda.push_back(next_pair);
                dst
            };

            fst_c.add_arc(
                sc,
                Arc {
                    state: dst,
                    ilabel: mi.a.ilabel,
                    olabel: mi.b.olabel,
                    weight: mi.a.weight.prod(&mi.b.weight),
                },
            );
        }
    }

    fst_c
}
fn match_pair<W: Semiring>(a: &[Arc<W>], b: &[Arc<W>], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS && ((i != 0 && j != 0) || (i == 0 && j == 0)) {
        return false;
    }
    true
}
