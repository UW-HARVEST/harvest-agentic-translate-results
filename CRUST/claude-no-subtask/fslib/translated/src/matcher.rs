use crate::fst::ArcData;
use crate::queue::Queue;

const EPS: u32 = 0;

// Mirrors C `_match`: returns 1 if pair (i,j) is a valid match.
// In C, the check is on the olabel of `a`. If it's EPS:
//   if ((i != 0 && j != 0) || (i == 0 && j == 0)) -> return 0
// else return 1
fn matchok(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let m = a.len();
    let n = b.len();
    for i in 0..m {
        for j in 0..n {
            if a[i].olabel == b[j].ilabel && matchok(a, b, i, j) {
                q.enqueue((clone_arc(&a[i]), clone_arc(&b[j])));
            }
        }
    }
}

pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let m = a.len();
    let n = b.len();
    if n == 0 {
        return;
    }
    for i in 0..m {
        let mut l: usize = 0;
        let mut h: usize = n - 1;
        loop {
            if l > h {
                break;
            }
            let mid = (l + h) >> 1;
            if a[i].olabel > b[mid].ilabel {
                l = mid + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 {
                    break;
                }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && a[i].olabel == b[ll - 1].ilabel {
                    ll -= 1;
                }
                while hh < h && a[i].olabel == b[hh + 1].ilabel {
                    hh += 1;
                }
                while ll <= hh {
                    if matchok(a, b, i, ll) {
                        q.enqueue((clone_arc(&a[i]), clone_arc(&b[ll])));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}

pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let m = a.len();
    let n = b.len();
    if m == 0 {
        return;
    }
    for i in 0..n {
        let mut l: usize = 0;
        let mut h: usize = m - 1;
        loop {
            if l > h {
                break;
            }
            let mid = (l + h) >> 1;
            if b[i].ilabel > a[mid].olabel {
                l = mid + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 {
                    break;
                }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && b[i].ilabel == a[ll - 1].olabel {
                    ll -= 1;
                }
                while hh < h && b[i].ilabel == a[hh + 1].olabel {
                    hh += 1;
                }
                while ll <= hh {
                    if matchok(a, b, ll, i) {
                        q.enqueue((clone_arc(&a[ll]), clone_arc(&b[i])));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}

pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let m = a.len();
    let n = b.len();
    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < m && j < n {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && a[i].olabel == b[t].ilabel {
                if matchok(a, b, i, t) {
                    q.enqueue((clone_arc(&a[i]), clone_arc(&b[t])));
                }
                t += 1;
            }
            i += 1;
        }
    }
}

fn clone_arc(a: &ArcData) -> ArcData {
    ArcData {
        state: a.state,
        weight: a.weight,
        ilabel: a.ilabel,
        olabel: a.olabel,
    }
}
