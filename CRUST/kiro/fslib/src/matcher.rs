use crate::fst::ArcData;
use crate::queue::Queue;

fn _match(a: &[ArcData], b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == 0 {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..a.len() {
        for j in 0..b.len() {
            if a[i].olabel == b[j].ilabel && _match(a, b, i, j) {
                q.enqueue((a[i].clone(), b[j].clone()));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], q: &mut Queue<(ArcData, ArcData)>) {
    let n = b.len();
    for i in 0..a.len() {
        if n == 0 { continue; }
        let mut l: usize = 0;
        let mut h: usize = n - 1;
        while l <= h {
            let mid = (l + h) >> 1;
            if a[i].olabel > b[mid].ilabel {
                l = mid + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && a[i].olabel == b[ll - 1].ilabel { ll -= 1; }
                while hh < h && a[i].olabel == b[hh + 1].ilabel { hh += 1; }
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
    let m = a.len();
    for i in 0..b.len() {
        if m == 0 { continue; }
        let mut l: usize = 0;
        let mut h: usize = m - 1;
        while l <= h {
            let mid = (l + h) >> 1;
            if b[i].ilabel > a[mid].olabel {
                l = mid + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && b[i].ilabel == a[ll - 1].olabel { ll -= 1; }
                while hh < h && b[i].ilabel == a[hh + 1].olabel { hh += 1; }
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
    let mut i = 0usize;
    let mut j = 0usize;
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
