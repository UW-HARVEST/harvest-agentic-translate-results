use std::fs::File;
use std::io::{self, Read, Write};
use std::os::raw::c_int;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::convert::TryInto;

pub const TAG: &str = "phashmvp2010";
pub const VERSION: u32 = 0x01000000;
pub const HEADER_SIZE: usize = 32;
pub const FILE_OFFSET_BITS: usize = 64;
pub const ERROR_MSGS: [&str; 25] = [
    "no error",
    "bad argument",
    "no distance function found",
    "mem alloc error",
    "no leaf node created",
    "no internal node created",
    "no path array alloc'd",
    "could not select vantage points",
    "could not calculate range from an sv1",
    "could not calculate range from an sv2",
    "points too compact",
    "could not sort points",
    "could not open file",
    "could not close file",
    "mmap error",
    "unmap error",
    "no write",
    "could not extend file",
    "could not remap file",
    "datatypes in conflict",
    "no. retrieved exceeds k",
    "empty tree",
    "distance value either NaN or less than zero",
    "could not open file",
    "unrecognized node",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MVPDataType {
    ByteArray = 1,
    UInt16Array = 2,
    UInt32Array = 4,
    UInt64Array = 8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    InternalNode = 1,
    LeafNode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MVPError {
    Success,
    ArgErr,
    NoDistanceFunc,
    MemAlloc,
    NoLeaf,
    NoInternal,
    PathAlloc,
    VpNoSelect,
    NoSv1Range,
    NoSv2Range,
    NoSpace,
    NoSort,
    FileOpen,
    FileClose,
    MemMap,
    Munmap,
    NoWrite,
    FileTruncate,
    MremapFail,
    TypeMismatch,
    KNearestCap,
    EmptyTree,
    NoSplits,
    BadDistVal,
    FileNotFound,
    Unrecognized,
}
#[derive(Debug, Clone)]
pub struct MVPDatapoint {
    pub id: String,
    pub data: Vec<u8>,
    pub path: Vec<f32>,
    pub datalen: usize,
    pub data_type: MVPDataType,
}
pub type DistanceFunction = fn(&MVPDatapoint, &MVPDatapoint) -> f32;
pub struct InternalNode {
    pub node_type: NodeType,
    pub sv1: Option<Arc<MVPDatapoint>>,
    pub sv2: Option<Arc<MVPDatapoint>>,
    pub m1: Vec<f32>,
    pub m2: Vec<f32>,
    pub child_nodes: Vec<Rc<RefCell<Node>>>,
}
impl InternalNode {
    pub fn new(bf: u32) -> Self {
        let bf = bf as usize;
        let m1_len = if bf >= 1 { bf - 1 } else { 0 };
        let m2_len = bf * m1_len;
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; m1_len],
            m2: vec![0.0; m2_len],
            child_nodes: Vec::new(),
        }
    }
}
pub struct LeafNode {
    pub node_type: NodeType,
    pub sv1: Option<Arc<MVPDatapoint>>,
    pub sv2: Option<Arc<MVPDatapoint>>,
    pub points: Vec<Arc<MVPDatapoint>>,
    pub d1: Vec<f32>,
    pub d2: Vec<f32>,
    pub nbpoints: usize,
}
impl LeafNode {
    pub fn new(_bf: u32) -> Self {
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::new(),
            d1: Vec::new(),
            d2: Vec::new(),
            nbpoints: 0,
        }
    }
}
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}
pub struct MVPTree {
    pub branch_factor: usize,
    pub path_length: usize,
    pub leaf_capacity: usize,
    pub datatype: MVPDataType,
    pub pos: i64,
    pub size: i64,
    pub pgsize: i64,
    pub buf: Vec<u8>,
    pub node: Option<Rc<RefCell<Node>>>,
    pub distance_function: DistanceFunction,
}

fn is_bad_dist(d: f32) -> bool {
    d.is_nan() || d < 0.0
}

fn make_empty_node() -> Rc<RefCell<Node>> {
    Rc::new(RefCell::new(Node::Leaf(LeafNode {
        node_type: NodeType::LeafNode,
        sv1: None,
        sv2: None,
        points: Vec::new(),
        d1: Vec::new(),
        d2: Vec::new(),
        nbpoints: 0,
    })))
}

fn is_empty_node(node: &Rc<RefCell<Node>>) -> bool {
    match &*node.borrow() {
        Node::Leaf(l) => l.sv1.is_none() && l.sv2.is_none() && l.nbpoints == 0,
        _ => false,
    }
}

fn select_vantage_points_helper(
    points: &[MVPDatapoint],
    dist: DistanceFunction,
) -> Result<(Option<usize>, Option<usize>), ()> {
    let nb = points.len();
    if nb == 0 {
        return Err(());
    }
    let mut sv1_pos: Option<usize> = Some(0);
    let mut sv2_pos: Option<usize> = None;
    let mut max_dist = 0.0f32;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if is_bad_dist(d) {
                return Err(());
            }
            if d > max_dist {
                max_dist = d;
                sv1_pos = Some(i);
                sv2_pos = Some(j);
            }
        }
    }
    Ok((sv1_pos, sv2_pos))
}

fn find_splits_helper(
    points: &[MVPDatapoint],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    length_m: usize,
) -> Option<Vec<f32>> {
    let nb = points.len();
    if nb == 0 || length_m == 0 {
        return None;
    }
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points.iter() {
        let d = dist(p, vp);
        if is_bad_dist(d) {
            return None;
        }
        dists.push(d);
    }
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut m = vec![0.0f32; length_m];
    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index >= nb {
            index = nb - 1;
        }
        m[i] = dists[index];
    }
    Some(m)
}

fn find_distance_range_helper(
    points: &mut [MVPDatapoint],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    lvl: usize,
    path_length: usize,
) -> Result<(), ()> {
    for p in points.iter_mut() {
        let d = dist(vp, p);
        if is_bad_dist(d) {
            return Err(());
        }
        if lvl < path_length {
            if p.path.len() < path_length {
                p.path.resize(path_length, 0.0);
            }
            p.path[lvl] = d;
        }
    }
    Ok(())
}

fn sort_points_helper(
    points: Vec<MVPDatapoint>,
    sv1_pos: Option<usize>,
    sv2_pos: Option<usize>,
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    pivots: &[f32],
    bf: usize,
) -> Option<Vec<Vec<MVPDatapoint>>> {
    let length_m1 = bf - 1;
    let mut bins: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
    for (i, p) in points.into_iter().enumerate() {
        if Some(i) == sv1_pos || Some(i) == sv2_pos {
            continue;
        }
        let d = dist(vp, &p);
        if is_bad_dist(d) {
            return None;
        }
        let mut target_bin = length_m1;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                target_bin = k;
                break;
            }
        }
        bins[target_bin].push(p);
    }
    Some(bins)
}

fn build_new_node(
    tree: &MVPTree,
    points: Vec<MVPDatapoint>,
    lvl: usize,
    error: &mut MVPError,
) -> Option<Rc<RefCell<Node>>> {
    if points.is_empty() {
        return None;
    }
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;
    let dist = tree.distance_function;

    if points.len() <= tree.leaf_capacity + 2 {
        let mut points = points;
        let (sv1_pos, sv2_pos) = match select_vantage_points_helper(&points, dist) {
            Ok(v) => v,
            Err(_) => {
                *error = MVPError::VpNoSelect;
                return None;
            }
        };
        let sv1 = sv1_pos.map(|i| points[i].clone());
        let sv2 = sv2_pos.map(|i| points[i].clone());
        if let Some(ref s1) = sv1 {
            if find_distance_range_helper(&mut points, s1, dist, lvl, tree.path_length).is_err() {
                *error = MVPError::NoSv1Range;
                return None;
            }
        }
        if let Some(ref s2) = sv2 {
            if find_distance_range_helper(&mut points, s2, dist, lvl + 1, tree.path_length)
                .is_err()
            {
                *error = MVPError::NoSv2Range;
                return None;
            }
        }
        let mut leaf = LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::new(),
            d1: Vec::new(),
            d2: Vec::new(),
            nbpoints: 0,
        };
        for (i, p) in points.iter().enumerate() {
            if Some(i) == sv1_pos || Some(i) == sv2_pos {
                continue;
            }
            let d1v = if let Some(ref s1) = sv1 { dist(p, s1) } else { 0.0 };
            let d2v = if let Some(ref s2) = sv2 { dist(p, s2) } else { 0.0 };
            leaf.d1.push(d1v);
            leaf.d2.push(d2v);
            leaf.points.push(Arc::new(p.clone()));
        }
        leaf.nbpoints = leaf.points.len();
        leaf.sv1 = sv1.map(Arc::new);
        leaf.sv2 = sv2.map(Arc::new);
        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else {
        let mut points = points;
        let (sv1_pos, sv2_pos) = match select_vantage_points_helper(&points, dist) {
            Ok(v) => v,
            Err(_) => {
                *error = MVPError::VpNoSelect;
                return None;
            }
        };
        let s1_idx = sv1_pos.unwrap_or(0);
        let s2_idx = sv2_pos.unwrap_or(0);
        let sv1 = points[s1_idx].clone();
        let sv2 = points[s2_idx].clone();
        if find_distance_range_helper(&mut points, &sv1, dist, lvl, tree.path_length).is_err() {
            *error = MVPError::NoSv1Range;
            return None;
        }
        let m1 = match find_splits_helper(&points, &sv1, dist, length_m1) {
            Some(m) => m,
            None => {
                *error = MVPError::NoSplits;
                return None;
            }
        };
        let bins = match sort_points_helper(points, sv1_pos, sv2_pos, &sv1, dist, &m1, bf) {
            Some(b) => b,
            None => {
                *error = MVPError::NoSort;
                return None;
            }
        };
        let mut m2 = vec![0.0f32; bf * length_m1];
        let mut child_nodes: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(bf * bf);
        for (i, mut bin) in bins.into_iter().enumerate() {
            if !bin.is_empty() {
                if find_distance_range_helper(&mut bin, &sv2, dist, lvl + 1, tree.path_length)
                    .is_err()
                {
                    *error = MVPError::NoSv2Range;
                    return None;
                }
            }
            let m2_chunk = if bin.is_empty() {
                vec![0.0; length_m1]
            } else {
                match find_splits_helper(&bin, &sv2, dist, length_m1) {
                    Some(m) => m,
                    None => {
                        *error = MVPError::NoSplits;
                        return None;
                    }
                }
            };
            for k in 0..length_m1 {
                m2[i * length_m1 + k] = m2_chunk[k];
            }
            let bins2 = if bin.is_empty() {
                (0..bf).map(|_| Vec::new()).collect::<Vec<_>>()
            } else {
                match sort_points_helper(bin, None, None, &sv2, dist, &m2_chunk, bf) {
                    Some(b) => b,
                    None => {
                        *error = MVPError::NoSort;
                        return None;
                    }
                }
            };
            for sub_bin in bins2 {
                if sub_bin.is_empty() {
                    child_nodes.push(make_empty_node());
                } else {
                    let child = build_new_node(tree, sub_bin, lvl + 2, error);
                    if !matches!(*error, MVPError::Success) {
                        return None;
                    }
                    if let Some(c) = child {
                        child_nodes.push(c);
                    } else {
                        child_nodes.push(make_empty_node());
                    }
                }
            }
        }
        let internal = InternalNode {
            node_type: NodeType::InternalNode,
            sv1: Some(Arc::new(sv1)),
            sv2: Some(Arc::new(sv2)),
            m1,
            m2,
            child_nodes,
        };
        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    }
}

fn add_to_existing(
    tree: &MVPTree,
    node_rc: Rc<RefCell<Node>>,
    points: Vec<MVPDatapoint>,
    lvl: usize,
    error: &mut MVPError,
) -> Rc<RefCell<Node>> {
    if points.is_empty() {
        return node_rc;
    }
    if is_empty_node(&node_rc) {
        return match build_new_node(tree, points, lvl, error) {
            Some(n) => n,
            None => node_rc,
        };
    }
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;
    let dist = tree.distance_function;

    let is_leaf = matches!(*node_rc.borrow(), Node::Leaf(_));

    if is_leaf {
        let (existing_count, can_fit) = {
            let b = node_rc.borrow();
            if let Node::Leaf(l) = &*b {
                (l.nbpoints, l.nbpoints + points.len() <= tree.leaf_capacity)
            } else {
                unreachable!()
            }
        };
        let _ = existing_count;
        if can_fit {
            let mut points = points;
            let (sv1_clone, sv2_clone) = {
                let b = node_rc.borrow();
                if let Node::Leaf(l) = &*b {
                    (l.sv1.clone(), l.sv2.clone())
                } else {
                    unreachable!()
                }
            };
            if let Some(ref s1) = sv1_clone {
                if find_distance_range_helper(&mut points, s1, dist, lvl, tree.path_length)
                    .is_err()
                {
                    *error = MVPError::NoSv1Range;
                    return node_rc;
                }
            }
            let mut start_pos = 0usize;
            let new_sv2: Option<Arc<MVPDatapoint>> = if sv2_clone.is_none() {
                if !points.is_empty() {
                    let np = points[0].clone();
                    start_pos = 1;
                    Some(Arc::new(np))
                } else {
                    None
                }
            } else {
                sv2_clone.clone()
            };
            if let Some(ref s2) = new_sv2 {
                if find_distance_range_helper(&mut points, s2, dist, lvl + 1, tree.path_length)
                    .is_err()
                {
                    *error = MVPError::NoSv2Range;
                    return node_rc;
                }
            }
            {
                let mut bm = node_rc.borrow_mut();
                if let Node::Leaf(leaf) = &mut *bm {
                    if leaf.sv2.is_none() {
                        leaf.sv2 = new_sv2.clone();
                    }
                    let s1_ref = leaf.sv1.clone();
                    let s2_ref = leaf.sv2.clone();
                    for i in start_pos..points.len() {
                        let p = &points[i];
                        let d1v = if let Some(ref s1) = s1_ref {
                            dist(p, s1)
                        } else {
                            0.0
                        };
                        let d2v = if let Some(ref s2) = s2_ref {
                            dist(p, s2)
                        } else {
                            0.0
                        };
                        leaf.d1.push(d1v);
                        leaf.d2.push(d2v);
                        leaf.points.push(Arc::new(p.clone()));
                    }
                    leaf.nbpoints = leaf.points.len();
                }
            }
            node_rc
        } else {
            // rebuild
            let mut all: Vec<MVPDatapoint> = Vec::new();
            {
                let b = node_rc.borrow();
                if let Node::Leaf(leaf) = &*b {
                    if let Some(s1) = &leaf.sv1 {
                        all.push((**s1).clone());
                    }
                    if let Some(s2) = &leaf.sv2 {
                        all.push((**s2).clone());
                    }
                    for p in &leaf.points {
                        all.push((**p).clone());
                    }
                }
            }
            for p in points {
                all.push(p);
            }
            match build_new_node(tree, all, lvl, error) {
                Some(n) => n,
                None => node_rc,
            }
        }
    } else {
        let mut points = points;
        let (sv1_arc, sv2_arc, m1_clone, m2_clone) = {
            let b = node_rc.borrow();
            if let Node::Internal(i) = &*b {
                (i.sv1.clone(), i.sv2.clone(), i.m1.clone(), i.m2.clone())
            } else {
                unreachable!()
            }
        };
        if let Some(ref s1) = sv1_arc {
            if find_distance_range_helper(&mut points, s1, dist, lvl, tree.path_length).is_err() {
                *error = MVPError::NoSv1Range;
                return node_rc;
            }
        }
        let sv1_ref = match &sv1_arc {
            Some(s) => s.as_ref(),
            None => return node_rc,
        };
        let bins = match sort_points_helper(points, None, None, sv1_ref, dist, &m1_clone, bf) {
            Some(b) => b,
            None => {
                *error = MVPError::NoSort;
                return node_rc;
            }
        };
        for (i, mut bin) in bins.into_iter().enumerate() {
            if bin.is_empty() {
                continue;
            }
            if let Some(ref s2) = sv2_arc {
                if find_distance_range_helper(&mut bin, s2, dist, lvl + 1, tree.path_length)
                    .is_err()
                {
                    *error = MVPError::NoSv2Range;
                    return node_rc;
                }
            }
            let m2_chunk: Vec<f32> = m2_clone[i * length_m1..(i + 1) * length_m1].to_vec();
            let sv2_ref = match &sv2_arc {
                Some(s) => s.as_ref(),
                None => return node_rc,
            };
            let bins2 = match sort_points_helper(bin, None, None, sv2_ref, dist, &m2_chunk, bf) {
                Some(b) => b,
                None => {
                    *error = MVPError::NoSort;
                    return node_rc;
                }
            };
            for (j, sub_bin) in bins2.into_iter().enumerate() {
                if sub_bin.is_empty() {
                    continue;
                }
                let child_idx = i * bf + j;
                let child_rc = {
                    let b = node_rc.borrow();
                    if let Node::Internal(int_n) = &*b {
                        int_n.child_nodes[child_idx].clone()
                    } else {
                        unreachable!()
                    }
                };
                let new_child = add_to_existing(tree, child_rc, sub_bin, lvl + 2, error);
                {
                    let mut bm = node_rc.borrow_mut();
                    if let Node::Internal(int_n) = &mut *bm {
                        int_n.child_nodes[child_idx] = new_child;
                    }
                }
                if !matches!(*error, MVPError::Success) {
                    return node_rc;
                }
            }
        }
        node_rc
    }
}

fn retrieve_helper(
    tree: &MVPTree,
    node: &Rc<RefCell<Node>>,
    target: &mut MVPDatapoint,
    radius: f32,
    results: &mut Vec<Arc<MVPDatapoint>>,
    lvl: usize,
    k: usize,
) -> MVPError {
    if is_empty_node(node) {
        return MVPError::Success;
    }
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;
    let dist = tree.distance_function;

    let (is_leaf, leaf_data, internal_data) = {
        let b = node.borrow();
        match &*b {
            Node::Leaf(l) => (
                true,
                Some((
                    l.sv1.clone(),
                    l.sv2.clone(),
                    l.d1.clone(),
                    l.d2.clone(),
                    l.points.clone(),
                    l.nbpoints,
                )),
                None,
            ),
            Node::Internal(i) => (
                false,
                None,
                Some((
                    i.sv1.clone(),
                    i.sv2.clone(),
                    i.m1.clone(),
                    i.m2.clone(),
                    i.child_nodes.clone(),
                )),
            ),
        }
    };

    if is_leaf {
        let (sv1_opt, sv2_opt, d1_arr, d2_arr, points_arr, nbpoints) = leaf_data.unwrap();
        let sv1 = match sv1_opt {
            Some(s) => s,
            None => return MVPError::Success,
        };
        let d1 = dist(target, &sv1);
        if is_bad_dist(d1) {
            return MVPError::BadDistVal;
        }
        if lvl < tree.path_length {
            target.path[lvl] = d1;
        }
        if d1 <= radius {
            results.push(sv1.clone());
            if results.len() >= k {
                return MVPError::KNearestCap;
            }
        }
        if let Some(sv2) = sv2_opt {
            let d2 = dist(target, &sv2);
            if is_bad_dist(d2) {
                return MVPError::BadDistVal;
            }
            if d2 <= radius {
                results.push(sv2.clone());
                if results.len() >= k {
                    return MVPError::KNearestCap;
                }
            }
            if lvl + 1 < tree.path_length {
                target.path[lvl + 1] = d2;
            }
            for i in 0..nbpoints {
                if d1 - radius <= d1_arr[i] && d1 + radius >= d1_arr[i] {
                    if d2 - radius <= d2_arr[i] && d2 + radius >= d2_arr[i] {
                        let endpath = std::cmp::min(lvl + 1, tree.path_length);
                        let mut skip = false;
                        for j in 0..endpath {
                            let pj = if j < points_arr[i].path.len() {
                                points_arr[i].path[j]
                            } else {
                                0.0
                            };
                            if !(target.path[j] - radius <= pj && target.path[j] + radius >= pj) {
                                skip = true;
                                break;
                            }
                        }
                        if !skip {
                            let d = dist(target, &points_arr[i]);
                            if is_bad_dist(d) {
                                return MVPError::BadDistVal;
                            }
                            if d <= radius {
                                results.push(points_arr[i].clone());
                                if results.len() >= k {
                                    return MVPError::KNearestCap;
                                }
                            }
                        }
                    }
                }
            }
        }
        MVPError::Success
    } else {
        let (sv1_opt, sv2_opt, m1, m2, children) = internal_data.unwrap();
        let sv1 = match sv1_opt {
            Some(s) => s,
            None => return MVPError::Success,
        };
        let sv2 = match sv2_opt {
            Some(s) => s,
            None => return MVPError::Success,
        };
        let d1 = dist(target, &sv1);
        if is_bad_dist(d1) {
            return MVPError::BadDistVal;
        }
        if d1 <= radius {
            results.push(sv1.clone());
            if results.len() >= k {
                return MVPError::KNearestCap;
            }
        }
        if lvl < tree.path_length {
            target.path[lvl] = d1;
        }
        let d2 = dist(target, &sv2);
        if is_bad_dist(d2) {
            return MVPError::BadDistVal;
        }
        if d2 <= radius {
            results.push(sv2.clone());
            if results.len() >= k {
                return MVPError::KNearestCap;
            }
        }
        if lvl + 1 < tree.path_length {
            target.path[lvl + 1] = d2;
        }
        // first level bins
        for i in 0..length_m1 {
            if d1 - radius <= m1[i] {
                for j in 0..length_m1 {
                    if d2 - radius <= m2[i * length_m1 + j] {
                        let err = retrieve_helper(
                            tree,
                            &children[i * bf + j],
                            target,
                            radius,
                            results,
                            lvl + 2,
                            k,
                        );
                        if !matches!(err, MVPError::Success) {
                            return err;
                        }
                    }
                }
                if d2 + radius >= m2[i * length_m1 + length_m1 - 1] {
                    let err = retrieve_helper(
                        tree,
                        &children[i * bf + length_m1],
                        target,
                        radius,
                        results,
                        lvl + 2,
                        k,
                    );
                    if !matches!(err, MVPError::Success) {
                        return err;
                    }
                }
            }
        }
        // last 1st level bin
        if d1 + radius >= m1[length_m1 - 1] {
            for j in 0..length_m1 {
                if d2 - radius <= m2[length_m1 * length_m1 + j] {
                    let err = retrieve_helper(
                        tree,
                        &children[bf * length_m1 + j],
                        target,
                        radius,
                        results,
                        lvl + 2,
                        k,
                    );
                    if !matches!(err, MVPError::Success) {
                        return err;
                    }
                }
            }
            if d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1] {
                let err = retrieve_helper(
                    tree,
                    &children[bf * length_m1 + length_m1],
                    target,
                    radius,
                    results,
                    lvl + 2,
                    k,
                );
                if !matches!(err, MVPError::Success) {
                    return err;
                }
            }
        }
        MVPError::Success
    }
}

fn write_datapoint_buf(
    buf: &mut Vec<u8>,
    dp: Option<&MVPDatapoint>,
    path_length: usize,
    type_size: usize,
) -> u64 {
    let start = buf.len() as u64;
    match dp {
        None => {
            buf.push(0u8);
            buf.extend_from_slice(&0u32.to_ne_bytes());
        }
        Some(dp) => {
            let active = 1u8;
            let id_bytes = dp.id.as_bytes();
            let idlen = id_bytes.len() as u8;
            let datalength = dp.datalen as u32;
            let bytelength: u32 = 1u32
                + idlen as u32
                + 4
                + datalength * type_size as u32
                + (path_length as u32) * 4;
            buf.push(active);
            buf.extend_from_slice(&bytelength.to_ne_bytes());
            buf.push(idlen);
            buf.extend_from_slice(id_bytes);
            buf.extend_from_slice(&datalength.to_ne_bytes());
            // write data: datalength * type_size bytes; for ByteArray, dp.data is exactly that
            let need_bytes = datalength as usize * type_size;
            if dp.data.len() >= need_bytes {
                buf.extend_from_slice(&dp.data[..need_bytes]);
            } else {
                buf.extend_from_slice(&dp.data);
                for _ in 0..(need_bytes - dp.data.len()) {
                    buf.push(0);
                }
            }
            // path
            for k in 0..path_length {
                let v = if k < dp.path.len() { dp.path[k] } else { 0.0 };
                buf.extend_from_slice(&v.to_ne_bytes());
            }
        }
    }
    start
}

fn write_node_buf(
    buf: &mut Vec<u8>,
    tree: &MVPTree,
    node: &Rc<RefCell<Node>>,
    error: &mut MVPError,
) -> u64 {
    let start_pos = buf.len() as u64;
    if is_empty_node(node) {
        return 0;
    }
    let type_size = tree.datatype as usize;
    let path_length = tree.path_length;
    let leafcap = tree.leaf_capacity;
    let bf = tree.branch_factor;

    // Take snapshot of node data without holding borrow during recursion
    enum NodeSnapshot {
        Leaf {
            sv1: Option<Arc<MVPDatapoint>>,
            sv2: Option<Arc<MVPDatapoint>>,
            d1: Vec<f32>,
            d2: Vec<f32>,
            points: Vec<Arc<MVPDatapoint>>,
            nbpoints: usize,
        },
        Internal {
            sv1: Option<Arc<MVPDatapoint>>,
            sv2: Option<Arc<MVPDatapoint>>,
            m1: Vec<f32>,
            m2: Vec<f32>,
            children: Vec<Rc<RefCell<Node>>>,
        },
    }
    let snap = {
        let b = node.borrow();
        match &*b {
            Node::Leaf(l) => NodeSnapshot::Leaf {
                sv1: l.sv1.clone(),
                sv2: l.sv2.clone(),
                d1: l.d1.clone(),
                d2: l.d2.clone(),
                points: l.points.clone(),
                nbpoints: l.nbpoints,
            },
            Node::Internal(i) => NodeSnapshot::Internal {
                sv1: i.sv1.clone(),
                sv2: i.sv2.clone(),
                m1: i.m1.clone(),
                m2: i.m2.clone(),
                children: i.child_nodes.clone(),
            },
        }
    };

    match snap {
        NodeSnapshot::Leaf {
            sv1,
            sv2,
            d1,
            d2,
            points,
            nbpoints,
        } => {
            buf.push(NodeType::LeafNode as u8);
            write_datapoint_buf(buf, sv1.as_deref(), path_length, type_size);
            write_datapoint_buf(buf, sv2.as_deref(), path_length, type_size);
            let nbp = nbpoints as u32;
            buf.extend_from_slice(&nbp.to_ne_bytes());

            let entry_size: usize = 4 + 4 + 8;
            let saved_pos = buf.len();
            for _ in 0..(leafcap * entry_size) {
                buf.push(0);
            }

            for i in 0..nbpoints {
                let off = write_datapoint_buf(buf, Some(&points[i]), path_length, type_size);
                let p = saved_pos + i * entry_size;
                buf[p..p + 4].copy_from_slice(&d1[i].to_ne_bytes());
                buf[p + 4..p + 8].copy_from_slice(&d2[i].to_ne_bytes());
                buf[p + 8..p + 16].copy_from_slice(&off.to_ne_bytes());
            }
        }
        NodeSnapshot::Internal {
            sv1,
            sv2,
            m1,
            m2,
            children,
        } => {
            let length_m1 = bf - 1;
            let length_m2 = (bf - 1) * bf;
            let fanout = bf * bf;

            buf.push(NodeType::InternalNode as u8);
            write_datapoint_buf(buf, sv1.as_deref(), path_length, type_size);
            write_datapoint_buf(buf, sv2.as_deref(), path_length, type_size);

            for i in 0..length_m1 {
                let v = if i < m1.len() { m1[i] } else { 0.0 };
                buf.extend_from_slice(&v.to_ne_bytes());
            }
            for i in 0..length_m2 {
                let v = if i < m2.len() { m2[i] } else { 0.0 };
                buf.extend_from_slice(&v.to_ne_bytes());
            }

            let entry_size: usize = 1 + 8;
            let saved_pos = buf.len();
            for _ in 0..(fanout * entry_size) {
                buf.push(0);
            }

            for i in 0..fanout {
                let off = write_node_buf(buf, tree, &children[i], error);
                let p = saved_pos + i * entry_size;
                buf[p] = 0u8;
                buf[p + 1..p + 9].copy_from_slice(&off.to_ne_bytes());
            }
        }
    }

    start_pos
}

struct ReadState<'a> {
    buf: &'a [u8],
    pos: usize,
    datatype: usize,
    path_length: usize,
    branch_factor: usize,
    leaf_capacity: usize,
}

fn read_datapoint_state(state: &mut ReadState) -> Option<MVPDatapoint> {
    let active = state.buf[state.pos];
    state.pos += 1;
    let bytelength = u32::from_ne_bytes(
        state.buf[state.pos..state.pos + 4].try_into().unwrap(),
    );
    state.pos += 4;
    let _ = bytelength;
    if active == 0 {
        return None;
    }
    let idlen = state.buf[state.pos] as usize;
    state.pos += 1;
    let id =
        String::from_utf8_lossy(&state.buf[state.pos..state.pos + idlen]).into_owned();
    state.pos += idlen;
    let datalength = u32::from_ne_bytes(
        state.buf[state.pos..state.pos + 4].try_into().unwrap(),
    ) as usize;
    state.pos += 4;
    let data_size = datalength * state.datatype;
    let data = state.buf[state.pos..state.pos + data_size].to_vec();
    state.pos += data_size;
    let mut path = Vec::with_capacity(state.path_length);
    for _ in 0..state.path_length {
        let v = f32::from_ne_bytes(
            state.buf[state.pos..state.pos + 4].try_into().unwrap(),
        );
        state.pos += 4;
        path.push(v);
    }
    let dt = match state.datatype {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => MVPDataType::ByteArray,
    };
    Some(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: dt,
    })
}

fn read_node_state(state: &mut ReadState, error: &mut MVPError) -> Option<Rc<RefCell<Node>>> {
    let nt = state.buf[state.pos];
    state.pos += 1;
    if nt == NodeType::LeafNode as u8 {
        let mut leaf = LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::new(),
            d1: Vec::new(),
            d2: Vec::new(),
            nbpoints: 0,
        };
        leaf.sv1 = read_datapoint_state(state).map(Arc::new);
        leaf.sv2 = read_datapoint_state(state).map(Arc::new);
        let nbpoints = u32::from_ne_bytes(
            state.buf[state.pos..state.pos + 4].try_into().unwrap(),
        ) as usize;
        state.pos += 4;
        leaf.nbpoints = nbpoints;

        let entry_size: usize = 4 + 4 + 8;
        let mut saved_pos = state.pos;
        // skip the table region
        state.pos += state.leaf_capacity * entry_size;

        for _i in 0..nbpoints {
            let d1v = f32::from_ne_bytes(
                state.buf[saved_pos..saved_pos + 4].try_into().unwrap(),
            );
            saved_pos += 4;
            let d2v = f32::from_ne_bytes(
                state.buf[saved_pos..saved_pos + 4].try_into().unwrap(),
            );
            saved_pos += 4;
            let off = u64::from_ne_bytes(
                state.buf[saved_pos..saved_pos + 8].try_into().unwrap(),
            );
            saved_pos += 8;

            leaf.d1.push(d1v);
            leaf.d2.push(d2v);
            state.pos = off as usize;
            let p = read_datapoint_state(state);
            if let Some(p) = p {
                leaf.points.push(Arc::new(p));
            }
        }
        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    } else if nt == NodeType::InternalNode as u8 {
        let bf = state.branch_factor;
        let length_m1 = bf - 1;
        let length_m2 = (bf - 1) * bf;
        let fanout = bf * bf;

        let mut internal = InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; length_m1],
            m2: vec![0.0; length_m2],
            child_nodes: Vec::with_capacity(fanout),
        };
        internal.sv1 = read_datapoint_state(state).map(Arc::new);
        internal.sv2 = read_datapoint_state(state).map(Arc::new);

        for i in 0..length_m1 {
            internal.m1[i] = f32::from_ne_bytes(
                state.buf[state.pos..state.pos + 4].try_into().unwrap(),
            );
            state.pos += 4;
        }
        for i in 0..length_m2 {
            internal.m2[i] = f32::from_ne_bytes(
                state.buf[state.pos..state.pos + 4].try_into().unwrap(),
            );
            state.pos += 4;
        }

        let entry_size: usize = 1 + 8;
        let mut saved_pos = state.pos;
        for _i in 0..fanout {
            let _fileno = state.buf[saved_pos];
            saved_pos += 1;
            let off = u64::from_ne_bytes(
                state.buf[saved_pos..saved_pos + 8].try_into().unwrap(),
            );
            saved_pos += 8;
            if off == 0 {
                internal.child_nodes.push(make_empty_node());
            } else {
                state.pos = off as usize;
                match read_node_state(state, error) {
                    Some(c) => internal.child_nodes.push(c),
                    None => internal.child_nodes.push(make_empty_node()),
                }
                if !matches!(*error, MVPError::Success) {
                    return None;
                }
            }
        }

        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    } else {
        *error = MVPError::Unrecognized;
        None
    }
}

impl MVPTree {
    pub fn new(
        branch_factor: usize,
        path_length: usize,
        leaf_capacity: usize,
        datatype: MVPDataType,
        distance_function: DistanceFunction,
    ) -> Self {
        MVPTree {
            branch_factor,
            path_length,
            leaf_capacity,
            datatype,
            pos: 0,
            size: 0,
            pgsize: 4096,
            buf: Vec::new(),
            node: None,
            distance_function,
        }
    }

    pub fn add(&mut self, points: Vec<MVPDatapoint>) -> MVPError {
        if points.is_empty() {
            return MVPError::Success;
        }
        // type check
        if self.datatype != points[0].data_type {
            return MVPError::TypeMismatch;
        }
        let mut points = points;
        for p in points.iter_mut() {
            p.path = vec![0.0; self.path_length];
        }
        let mut error = MVPError::Success;
        let new_node = match self.node.take() {
            Some(node_rc) => Some(add_to_existing(self, node_rc, points, 0, &mut error)),
            None => build_new_node(self, points, 0, &mut error),
        };
        self.node = new_node;
        error
    }

    pub fn retrieve(
        &self,
        target: &MVPDatapoint,
        knearest: usize,
        radius: f32,
    ) -> Result<Vec<MVPDatapoint>, MVPError> {
        if knearest == 0 || radius < 0.0 {
            return Err(MVPError::ArgErr);
        }
        let node = match &self.node {
            Some(n) => n.clone(),
            None => return Err(MVPError::EmptyTree),
        };
        let mut target = target.clone();
        target.path = vec![0.0; self.path_length];
        let mut results: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(knearest);
        let err = retrieve_helper(self, &node, &mut target, radius, &mut results, 0, knearest);
        match err {
            MVPError::Success | MVPError::KNearestCap => {
                Ok(results.into_iter().map(|a| (*a).clone()).collect())
            }
            e => Err(e),
        }
    }

    pub fn write(&self, filename: &str, _mode: i32) -> MVPError {
        let node_rc = match &self.node {
            Some(n) => n.clone(),
            None => return MVPError::ArgErr,
        };
        let mut buf: Vec<u8> = Vec::new();

        // Header
        let tag_bytes = TAG.as_bytes();
        buf.extend_from_slice(tag_bytes);
        buf.push(0u8); // null terminator after tag
        buf.extend_from_slice(&VERSION.to_ne_bytes());
        let bf = self.branch_factor as u8;
        let pl = self.path_length as u8;
        let lc = self.leaf_capacity as u8;
        let ht = self.datatype as u8;
        buf.push(bf);
        buf.push(pl);
        buf.push(lc);
        buf.push(ht);
        while buf.len() < HEADER_SIZE {
            buf.push(0);
        }

        let mut error = MVPError::Success;
        write_node_buf(&mut buf, self, &node_rc, &mut error);
        if !matches!(error, MVPError::Success) {
            return error;
        }

        match File::create(filename) {
            Ok(mut f) => {
                if f.write_all(&buf).is_err() {
                    return MVPError::NoWrite;
                }
                MVPError::Success
            }
            Err(_) => MVPError::FileOpen,
        }
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        print_node(stream, self, &self.node, 0)
    }

    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        *node = None;
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        0
    }
}

fn print_node(
    stream: &mut dyn Write,
    tree: &MVPTree,
    node: &Option<Rc<RefCell<Node>>>,
    lvl: usize,
) -> MVPError {
    let bf = tree.branch_factor;
    let length_m1 = if bf >= 1 { bf - 1 } else { 0 };
    let length_m2 = bf * length_m1;
    let fanout = bf * bf;
    if let Some(n) = node {
        if is_empty_node(n) {
            let _ = writeln!(stream, "NULL{}", lvl);
            return MVPError::Success;
        }
        let b = n.borrow();
        match &*b {
            Node::Leaf(leaf) => {
                let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
                if let Some(s1) = &leaf.sv1 {
                    let _ = writeln!(stream, "    sv1: {}", s1.id);
                }
                if let Some(s2) = &leaf.sv2 {
                    let _ = writeln!(stream, "    sv2: {}", s2.id);
                }
                for (i, p) in leaf.points.iter().enumerate() {
                    let _ = writeln!(stream, "        point[{}]: {}", i, p.id);
                }
            }
            Node::Internal(internal) => {
                let _ = writeln!(stream, "INTERNAL{}", lvl);
                if let Some(s1) = &internal.sv1 {
                    let _ = writeln!(stream, "  sv1: {}", s1.id);
                }
                if let Some(s2) = &internal.sv2 {
                    let _ = writeln!(stream, "  sv2: {}", s2.id);
                }
                for i in 0..length_m1 {
                    let _ = write!(stream, "  M1[{}] = {:.4};", i, internal.m1[i]);
                }
                for i in 0..length_m2.min(internal.m2.len()) {
                    let _ = write!(stream, "  M2[{}] = {:.4};", i, internal.m2[i]);
                }
                let _ = writeln!(stream);
                let children = internal.child_nodes.clone();
                drop(b);
                for i in 0..fanout.min(children.len()) {
                    let child_opt = Some(children[i].clone());
                    let _ = print_node(stream, tree, &child_opt, lvl + 2);
                }
            }
        }
    } else {
        let _ = writeln!(stream, "NULL{}", lvl);
    }
    MVPError::Success
}

pub fn mvptree_read(
    filename: &str,
    distance_function: DistanceFunction,
) -> Result<MVPTree, MVPError> {
    let mut buf: Vec<u8> = Vec::new();
    match File::open(filename) {
        Ok(mut f) => {
            if f.read_to_end(&mut buf).is_err() {
                return Err(MVPError::FileOpen);
            }
        }
        Err(_) => return Err(MVPError::FileNotFound),
    }

    let tag_len = TAG.len() + 1;
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }
    let mut pos = tag_len;
    pos += 4; // version
    let bf = buf[pos] as usize;
    pos += 1;
    let pl = buf[pos] as usize;
    pos += 1;
    let lc = buf[pos] as usize;
    pos += 1;
    let ht = buf[pos] as usize;
    pos += 1;
    let _ = pos;

    let datatype = match ht {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => MVPDataType::ByteArray,
    };

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);

    let mut state = ReadState {
        buf: &buf,
        pos: HEADER_SIZE,
        datatype: ht,
        path_length: pl,
        branch_factor: bf,
        leaf_capacity: lc,
    };
    let mut error = MVPError::Success;
    let node = read_node_state(&mut state, &mut error);
    if !matches!(error, MVPError::Success) {
        return Err(error);
    }
    tree.node = node;
    Ok(tree)
}

impl MVPDatapoint {
    pub fn new(id: String, data: Vec<u8>, data_type: MVPDataType) -> Self {
        let datalen = data.len();
        MVPDatapoint {
            id,
            data,
            path: vec![],
            datalen,
            data_type,
        }
    }
    pub fn select_vantage_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _dist: DistanceFunction,
    ) -> i32 {
        0
    }
    pub fn find_splits(
        &mut self,
        _nb: u32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _length_m: u32,
    ) -> f32 {
        0.0
    }
    pub fn sort_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _counts: &mut Vec<Vec<i32>>,
        _pivots: Vec<f32>,
    ) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        Vec::new()
    }
    pub fn find_distance_range_for_vp(
        &mut self,
        _nb: u32,
        _vp: &MVPDatapoint,
        _tree: &MVPTree,
        _level: i32,
    ) -> i32 {
        0
    }
    pub fn write(&self, _tree: &MVPTree) -> i64 {
        0
    }
}

pub fn error_to_string(error: MVPError) -> &'static str {
    let idx = error as usize;
    if idx < ERROR_MSGS.len() {
        ERROR_MSGS[idx]
    } else {
        "unknown error"
    }
}
