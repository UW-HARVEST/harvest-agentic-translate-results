use crate::fst::ArcData;
use crate::queue::Queue;
pub fn match_unsorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    for arc_a in a {
        for arc_b in b {
            if arc_a.olabel == arc_b.ilabel {
                q.enqueue((clone_arc(arc_a), clone_arc(arc_b)));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    for arc_a in a {
        let mut l = 0usize;
        let mut h = b.len().saturating_sub(1);
        while l <= h && !b.is_empty() {
            let m = (l + h) >> 1;
            if arc_a.olabel > b[m].ilabel {
                l = m + 1;
            } else if arc_a.olabel < b[m].ilabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && arc_a.olabel == b[ll - 1].ilabel {
                    ll -= 1;
                }
                while hh < h && arc_a.olabel == b[hh + 1].ilabel {
                    hh += 1;
                }
                while ll <= hh {
                    q.enqueue((clone_arc(arc_a), clone_arc(&b[ll])));
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    for arc_b in b {
        let mut l = 0usize;
        let mut h = a.len().saturating_sub(1);
        while l <= h && !a.is_empty() {
            let m = (l + h) >> 1;
            if arc_b.ilabel > a[m].olabel {
                l = m + 1;
            } else if arc_b.ilabel < a[m].olabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && arc_b.ilabel == a[ll - 1].olabel {
                    ll -= 1;
                }
                while hh < h && arc_b.ilabel == a[hh + 1].olabel {
                    hh += 1;
                }
                while ll <= hh {
                    q.enqueue((clone_arc(&a[ll]), clone_arc(arc_b)));
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let (mut i, mut j) = (0usize, 0usize);
    while i < a.len() && j < b.len() {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < b.len() && a[i].olabel == b[t].ilabel {
                q.enqueue((clone_arc(&a[i]), clone_arc(&b[t])));
                t += 1;
            }
            i += 1;
        }
    }
}
fn clone_arc(arc: &ArcData) -> ArcData {
    ArcData {
        state: arc.state,
        weight: arc.weight,
        ilabel: arc.ilabel,
        olabel: arc.olabel,
    }
}
