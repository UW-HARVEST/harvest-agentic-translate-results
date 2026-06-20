use crate::fst::ArcData;
use crate::queue::Queue;

const EPS: u32 = 0;

fn allowed(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..a.len() {
        for j in 0..b.len() {
            if a[i].olabel == b[j].ilabel && allowed(a, b, i, j) {
                q.enqueue((a[i].clone(), b[j].clone()));
            }
        }
    }
}

pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..a.len() {
        let mut l = 0usize;
        if b.is_empty() {
            break;
        }
        let mut h = b.len() - 1;
        while l <= h {
            let m = (l + h) >> 1;
            if a[i].olabel > b[m].ilabel {
                l = m + 1;
            } else if a[i].olabel < b[m].ilabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && a[i].olabel == b[ll - 1].ilabel {
                    ll -= 1;
                }
                while hh < h && a[i].olabel == b[hh + 1].ilabel {
                    hh += 1;
                }
                while ll <= hh {
                    if allowed(a, b, i, ll) {
                        q.enqueue((a[i].clone(), b[ll].clone()));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}

pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..b.len() {
        let mut l = 0usize;
        if a.is_empty() {
            break;
        }
        let mut h = a.len() - 1;
        while l <= h {
            let m = (l + h) >> 1;
            if b[i].ilabel > a[m].olabel {
                l = m + 1;
            } else if b[i].ilabel < a[m].olabel {
                if m == 0 {
                    break;
                }
                h = m - 1;
            } else {
                let mut ll = m;
                let mut hh = m;
                while ll > l && b[i].ilabel == a[ll - 1].olabel {
                    ll -= 1;
                }
                while hh < h && b[i].ilabel == a[hh + 1].olabel {
                    hh += 1;
                }
                while ll <= hh {
                    if allowed(a, b, ll, i) {
                        q.enqueue((a[ll].clone(), b[i].clone()));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}

pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let mut i = 0usize;
    let mut j = 0usize;
    while i < a.len() && j < b.len() {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < b.len() && a[i].olabel == b[t].ilabel {
                if allowed(a, b, i, t) {
                    q.enqueue((a[i].clone(), b[t].clone()));
                }
                t += 1;
            }
            i += 1;
        }
    }
}
