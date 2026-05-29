use fslib::fst::ArcData;
use fslib::matcher::{match_full_sorted, match_half_sorted, match_half_sorted_rev, match_unsorted};
use fslib::queue::Queue;

fn arc(ilabel: u32, olabel: u32) -> ArcData {
    ArcData {
        state: 0,
        ilabel,
        olabel,
        weight: 0.0,
    }
}

#[test]
fn test_match_full_sorted_basic() {
    // a sorted by olabel, b sorted by ilabel
    // a olabels: 1, 2, 3, 5, 7
    // b ilabels: 2, 3, 3, 5
    // matches: (2,2), (3,3), (3,3), (5,5)
    let a = vec![arc(0, 1), arc(0, 2), arc(0, 3), arc(0, 5), arc(0, 7)];
    let b = vec![arc(2, 0), arc(3, 0), arc(3, 0), arc(5, 0)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_full_sorted(&a, &b, &mut q);
    let mut out: Vec<(u32, u32)> = Vec::new();
    while let Some((aa, bb)) = q.dequeue() {
        out.push((aa.olabel, bb.ilabel));
    }
    assert_eq!(out.len(), 4);
    assert_eq!(out, vec![(2, 2), (3, 3), (3, 3), (5, 5)]);
}

#[test]
fn test_match_full_sorted_no_overlap() {
    let a = vec![arc(0, 1), arc(0, 2)];
    let b = vec![arc(3, 0), arc(4, 0)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_full_sorted(&a, &b, &mut q);
    assert_eq!(q.len(), 0);
}

#[test]
fn test_match_full_sorted_eps_special() {
    // a[0] has olabel=0 (EPS), b[0] also has ilabel=0
    // arc_match: when olabel==EPS, fail if (i!=0 && j!=0) or (i==0 && j==0).
    // For i=0,j=0 -> fail. For i=0, j>0: if a[0].olabel==EPS and b[j].ilabel==EPS too, succeed.
    let a = vec![arc(0, 0), arc(0, 0)]; // both EPS
    let b = vec![arc(0, 0), arc(0, 0)]; // both EPS
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_full_sorted(&a, &b, &mut q);
    // The matching: a[0].olabel == b[0].ilabel == 0. arc_match: i=0,j=0,al=EPS -> fail.
    // a[0].olabel == b[1].ilabel == 0. arc_match: i=0,j=1,al=EPS -> al==EPS, but i==0,j!=0 condition: (i!=0 && j!=0)||(i==0&&j==0) = false || false = false -> succeed.
    // a[1].olabel: when i=1, t starts at j=0, only matches as j increases.
    // After matching a[0]'s sequence, t=2, i becomes 1.
    // For i=1, j=2 in old loop or restart? Looking at code:
    // i is incremented after inner loop. Then while loop continues, j is still where it was.
    // Actually j stays where it was, after inner loop. So i=1, j=2 (out of bounds)
    let mut out: Vec<(usize, u32, u32)> = Vec::new();
    while let Some((aa, bb)) = q.dequeue() {
        out.push((out.len(), aa.olabel, bb.ilabel));
    }
    // a[0] matches b[1] (i=0,j=1: arc_match true)
    // when i increments to 1, j is still 0 (it was stuck at 0 because a[0].olabel<=b[0].ilabel and they were equal).
    // Actually: in match_full_sorted, after match, only i is incremented.
    // For i=1 (a[1].olabel=0): inner while: a[1].olabel == b[0].ilabel (both 0), so we enter else branch.
    //   t=0: arc_match(a, b, 1, 0)? a[1].olabel==EPS, (i=1!=0 && j=0!=0)? No. (i==0&&j==0)? No. So pass.
    //   t=1: arc_match(a, b, 1, 1)? both !=0, fail.
    // So a[1] matches b[0].
    // Total matches: (a[0], b[1]) and (a[1], b[0]) -> 2 matches
    assert_eq!(out.len(), 2);
}

#[test]
fn test_match_unsorted_basic() {
    // a: [eps loop, (0,2), (0,3)]
    // b: [eps loop, (2,0), (3,0)]
    // Position 0: (0,0): EPS but i==0&&j==0 -> fail
    // (0,1): a[0].olabel=0=b[1].ilabel=2? No.
    // (0,2): a[0].olabel=0=b[2].ilabel=3? No.
    // (1,0): a[1].olabel=2=b[0].ilabel=0? No.
    // (1,1): a[1].olabel=2=b[1].ilabel=2? Yes. arc_match: a[1].olabel=2!=EPS, pass. -> match
    // (1,2): a[1].olabel=2=b[2].ilabel=3? No.
    // (2,1): a[2].olabel=3=b[1].ilabel=2? No.
    // (2,2): a[2].olabel=3=b[2].ilabel=3? Yes. arc_match: pass. -> match
    let a = vec![arc(0, 0), arc(0, 2), arc(0, 3)];
    let b = vec![arc(0, 0), arc(2, 0), arc(3, 0)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_unsorted(&a, &b, &mut q);
    let mut out: Vec<(u32, u32)> = Vec::new();
    while let Some((aa, bb)) = q.dequeue() {
        out.push((aa.olabel, bb.ilabel));
    }
    assert_eq!(out, vec![(2, 2), (3, 3)]);
}

#[test]
fn test_match_half_sorted() {
    // a unsorted, b sorted by ilabel
    // a olabels: 3, 5, 2
    // b ilabels: 1, 2, 3, 3, 4
    // For a[0]=3: matches b[2]=3 and b[3]=3
    // For a[1]=5: no match
    // For a[2]=2: matches b[1]=2
    let a = vec![arc(0, 3), arc(0, 5), arc(0, 2)];
    let b = vec![arc(1, 0), arc(2, 0), arc(3, 0), arc(3, 0), arc(4, 0)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_half_sorted(&a, &b, &mut q);
    let mut out: Vec<(u32, u32)> = Vec::new();
    while let Some((aa, bb)) = q.dequeue() {
        out.push((aa.olabel, bb.ilabel));
    }
    // Per probe: count=3, (3,3), (3,3), (2,2)
    assert_eq!(out.len(), 3);
    assert_eq!(out, vec![(3, 3), (3, 3), (2, 2)]);
}

#[test]
fn test_match_half_sorted_rev() {
    // a sorted by olabel, b unsorted
    // a olabels: 1, 2, 3, 3, 4
    // b ilabels: 2, 3, 5
    let a = vec![arc(0, 1), arc(0, 2), arc(0, 3), arc(0, 3), arc(0, 4)];
    let b = vec![arc(2, 0), arc(3, 0), arc(5, 0)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_half_sorted_rev(&a, &b, &mut q);
    let mut out: Vec<(u32, u32)> = Vec::new();
    while let Some((aa, bb)) = q.dequeue() {
        out.push((aa.olabel, bb.ilabel));
    }
    // From probe: (2,2), (3,3), (3,3) [order depends on how items are added]
    assert_eq!(out.len(), 3);
    // Verify all matches present (sorted to be order-independent in case)
    let mut sorted = out.clone();
    sorted.sort();
    assert_eq!(sorted, vec![(2, 2), (3, 3), (3, 3)]);
}

#[test]
fn test_match_unsorted_empty() {
    let a: Vec<ArcData> = vec![];
    let b: Vec<ArcData> = vec![];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_unsorted(&a, &b, &mut q);
    assert_eq!(q.len(), 0);
}

#[test]
fn test_match_half_sorted_empty_b() {
    let a = vec![arc(0, 1)];
    let b: Vec<ArcData> = vec![];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_half_sorted(&a, &b, &mut q);
    assert_eq!(q.len(), 0);
}

#[test]
fn test_match_half_sorted_rev_empty_a() {
    let a: Vec<ArcData> = vec![];
    let b = vec![arc(1, 0)];
    let mut q: Queue<(ArcData, ArcData)> = Queue::new();
    match_half_sorted_rev(&a, &b, &mut q);
    assert_eq!(q.len(), 0);
}

fn main() {}
