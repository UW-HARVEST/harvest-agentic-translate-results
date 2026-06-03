use crate::fst::{ArcData, EPS};
use crate::queue::Queue;
fn _match(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
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
            if a[i].olabel == b[j].ilabel && _match(a, b, i, j) {
                q.enqueue((a[i].clone(), b[j].clone()));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let m = a.len() as i64;
    let n = b.len() as i64;
    for i in 0..m as usize {
        let mut l: i64 = 0;
        let mut h: i64 = n - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if a[i].olabel > b[mid].ilabel {
                l = mid as i64 + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 { break; }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && a[i].olabel == b[ll - 1].ilabel { ll -= 1; }
                while (hh as i64) < h && a[i].olabel == b[hh + 1].ilabel { hh += 1; }
                while ll <= hh {
                    if _match(a, b, i, ll) {
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
    let m = a.len() as i64;
    let n = b.len() as i64;
    for i in 0..n as usize {
        let mut l: i64 = 0;
        let mut h: i64 = m - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if b[i].ilabel > a[mid].olabel {
                l = mid as i64 + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 { break; }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && b[i].ilabel == a[ll - 1].olabel { ll -= 1; }
                while (hh as i64) < h && b[i].ilabel == a[hh + 1].olabel { hh += 1; }
                while ll <= hh {
                    if _match(a, b, ll, i) {
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
                if _match(a, b, i, t) {
                    q.enqueue((a[i].clone(), b[t].clone()));
                }
                t += 1;
            }
            i += 1;
        }
    }
}
