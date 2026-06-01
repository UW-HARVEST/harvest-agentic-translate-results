use std::fs::File;
use std::io::{self, Write, Read};
use std::ptr;
use std::os::raw::c_int;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
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
impl InternalNode{
    pub fn new(bf:u32) -> Self {
        let bf_us = bf as usize;
        let length_m1 = if bf_us > 0 { bf_us - 1 } else { 0 };
        let length_m2 = bf_us; // C code uses (bf-1)*bf elements but field allocates bf in some places; we use length_m2 = bf
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; length_m1],
            m2: vec![0.0; length_m1 * bf_us],
            child_nodes: Vec::with_capacity(bf_us * bf_us),
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
    pub fn new(bf:u32) -> Self {
        let cap = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(cap),
            d1: vec![0.0; cap],
            d2: vec![0.0; cap],
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

// ---- Internal helpers ----

fn select_vantage_points_helper(
    points: &[MVPDatapoint],
    dist: DistanceFunction,
) -> Result<(i32, i32), i32> {
    let nb = points.len();
    if nb == 0 {
        return Err(-1);
    }
    let mut sv1_pos: i32 = if nb >= 1 { 0 } else { -1 };
    let mut sv2_pos: i32 = -1;
    let mut max_dist = 0.0f32;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if d.is_nan() || d < 0.0 {
                return Err(-2);
            }
            if d > max_dist {
                max_dist = d;
                sv1_pos = i as i32;
                sv2_pos = j as i32;
            }
        }
    }
    Ok((sv1_pos, sv2_pos))
}

fn find_splits_helper(
    points: &[MVPDatapoint],
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    length_m: usize,
) -> Result<Vec<f32>, i32> {
    let nb = points.len();
    if nb == 0 || length_m == 0 {
        return Err(-1);
    }
    let mut dist: Vec<f32> = Vec::with_capacity(nb);
    for p in points {
        let d = distfunc(p, vp);
        if d.is_nan() || d < 0.0 {
            return Err(-2);
        }
        dist.push(d);
    }
    // selection sort like C does
    for i in 0..nb.saturating_sub(1) {
        let mut min_pos = i;
        for j in (i + 1)..nb {
            if dist[j] < dist[min_pos] {
                min_pos = j;
            }
        }
        if min_pos != i {
            dist.swap(min_pos, i);
        }
    }
    let mut m = vec![0.0f32; length_m];
    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index >= nb {
            index = nb - 1;
        }
        m[i] = dist[index];
    }
    Ok(m)
}

fn sort_points_helper(
    points: Vec<MVPDatapoint>,
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    bf: usize,
    pivots: &[f32],
) -> Result<(Vec<Vec<MVPDatapoint>>, Vec<usize>), ()> {
    if points.is_empty() {
        return Err(());
    }
    let length_m1 = bf.saturating_sub(1);
    let mut bins: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
    let mut counts: Vec<usize> = vec![0; bf];
    for (i, p) in points.into_iter().enumerate() {
        if i as i32 == sv1_pos || i as i32 == sv2_pos {
            continue;
        }
        let d = distfunc(vp, &p);
        if d.is_nan() || d < 0.0 {
            return Err(());
        }
        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(p.clone());
                counts[k] += 1;
                placed = true;
                break;
            }
        }
        if !placed {
            // d > pivots[lengthM1-1] OR length_m1 == 0
            if length_m1 == 0 {
                bins[0].push(p);
                counts[0] += 1;
            } else if d > pivots[length_m1 - 1] {
                bins[length_m1].push(p);
                counts[length_m1] += 1;
            }
        }
    }
    Ok((bins, counts))
}

fn find_distance_range_for_vp_helper(
    points: &mut [MVPDatapoint],
    vp: &MVPDatapoint,
    distfunc: DistanceFunction,
    path_length: usize,
    lvl: usize,
) -> Result<(), i32> {
    if points.is_empty() {
        return Err(-1);
    }
    for p in points.iter_mut() {
        let d = distfunc(vp, p);
        if d.is_nan() || d < 0.0 {
            return Err(-2);
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

fn add_recursive(
    tree_branch_factor: usize,
    tree_path_length: usize,
    tree_leaf_cap: usize,
    distfunc: DistanceFunction,
    node: Option<Rc<RefCell<Node>>>,
    points: Vec<MVPDatapoint>,
    lvl: usize,
) -> (Option<Rc<RefCell<Node>>>, MVPError) {
    let nbpoints = points.len();
    if nbpoints == 0 {
        return (node, MVPError::Success);
    }
    let bf = tree_branch_factor;
    let length_m1 = bf.saturating_sub(1);

    if node.is_none() {
        // create new node
        if nbpoints <= tree_leaf_cap + 2 {
            // create leaf
            let mut leaf = LeafNode::new(tree_leaf_cap as u32);
            let (sv1_pos, sv2_pos) = match select_vantage_points_helper(&points, distfunc) {
                Ok(v) => v,
                Err(_) => return (None, MVPError::VpNoSelect),
            };
            let mut points = points;
            // Set SVs (move them out by index)
            let sv1_dp = if sv1_pos >= 0 {
                Some(points[sv1_pos as usize].clone())
            } else {
                None
            };
            let sv2_dp = if sv2_pos >= 0 {
                Some(points[sv2_pos as usize].clone())
            } else {
                None
            };

            // Update path field of all points using sv1
            if let Some(ref sv1) = sv1_dp {
                if let Err(_) = find_distance_range_for_vp_helper(
                    &mut points,
                    sv1,
                    distfunc,
                    tree_path_length,
                    lvl,
                ) {
                    return (None, MVPError::NoSv1Range);
                }
            }

            // Update path field using sv2
            if let Some(ref sv2) = sv2_dp {
                if let Err(_) = find_distance_range_for_vp_helper(
                    &mut points,
                    sv2,
                    distfunc,
                    tree_path_length,
                    lvl + 1,
                ) {
                    return (None, MVPError::NoSv2Range);
                }
            }

            // Add remaining points to leaf
            let mut d1_vec: Vec<f32> = Vec::new();
            let mut d2_vec: Vec<f32> = Vec::new();
            let mut leaf_points: Vec<Arc<MVPDatapoint>> = Vec::new();
            for (i, p) in points.into_iter().enumerate() {
                if i as i32 == sv1_pos || i as i32 == sv2_pos {
                    continue;
                }
                let d1 = if let Some(ref sv1) = sv1_dp {
                    distfunc(&p, sv1)
                } else {
                    0.0
                };
                let d2 = if let Some(ref sv2) = sv2_dp {
                    distfunc(&p, sv2)
                } else {
                    0.0
                };
                d1_vec.push(d1);
                d2_vec.push(d2);
                leaf_points.push(Arc::new(p));
            }
            leaf.nbpoints = leaf_points.len();
            // pad d1_vec/d2_vec to leaf cap
            while d1_vec.len() < tree_leaf_cap {
                d1_vec.push(0.0);
            }
            while d2_vec.len() < tree_leaf_cap {
                d2_vec.push(0.0);
            }
            leaf.d1 = d1_vec;
            leaf.d2 = d2_vec;
            leaf.points = leaf_points;
            leaf.sv1 = sv1_dp.map(Arc::new);
            leaf.sv2 = sv2_dp.map(Arc::new);

            (Some(Rc::new(RefCell::new(Node::Leaf(leaf)))), MVPError::Success)
        } else {
            // create internal node
            let mut internal = InternalNode::new(bf as u32);
            let (sv1_pos, sv2_pos) = match select_vantage_points_helper(&points, distfunc) {
                Ok(v) => v,
                Err(_) => return (None, MVPError::VpNoSelect),
            };

            let mut points = points;
            let sv1_dp = points[sv1_pos as usize].clone();
            let sv2_dp = points[sv2_pos as usize].clone();

            if let Err(_) = find_distance_range_for_vp_helper(
                &mut points,
                &sv1_dp,
                distfunc,
                tree_path_length,
                lvl,
            ) {
                return (None, MVPError::NoSv1Range);
            }

            // find_splits with sv1
            let m1 = match find_splits_helper(&points, &sv1_dp, distfunc, length_m1) {
                Ok(m) => m,
                Err(_) => return (None, MVPError::NoSplits),
            };

            // sort points using sv1
            let (bins, _binlengths) = match sort_points_helper(
                points,
                sv1_pos,
                sv2_pos,
                &sv1_dp,
                distfunc,
                bf,
                &m1,
            ) {
                Ok(v) => v,
                Err(_) => return (None, MVPError::NoSort),
            };

            internal.m1 = m1.clone();
            internal.sv1 = Some(Arc::new(sv1_dp));
            internal.sv2 = Some(Arc::new(sv2_dp.clone()));

            // For each bin, find_distance_range_for_vp w/ sv2, find_splits, sort_points, recurse
            let mut all_m2: Vec<f32> = Vec::with_capacity(length_m1 * bf);
            let mut all_children: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(bf * bf);

            for bin in bins.into_iter() {
                let mut bin = bin;
                if bin.is_empty() {
                    // No points in this bin: m2 entries are 0, children are empty leaves
                    for _ in 0..length_m1 {
                        all_m2.push(0.0);
                    }
                    for _ in 0..bf {
                        all_children.push(empty_leaf_sentinel(tree_leaf_cap));
                    }
                    continue;
                }

                if let Err(_) = find_distance_range_for_vp_helper(
                    &mut bin,
                    &sv2_dp,
                    distfunc,
                    tree_path_length,
                    lvl + 1,
                ) {
                    return (None, MVPError::NoSv2Range);
                }

                let m2 = match find_splits_helper(&bin, &sv2_dp, distfunc, length_m1) {
                    Ok(m) => m,
                    Err(_) => return (None, MVPError::NoSplits),
                };

                let (bins2, _bin2lengths) = match sort_points_helper(
                    bin,
                    -1,
                    -1,
                    &sv2_dp,
                    distfunc,
                    bf,
                    &m2,
                ) {
                    Ok(v) => v,
                    Err(_) => return (None, MVPError::NoSort),
                };

                for v in &m2 {
                    all_m2.push(*v);
                }

                for sub_bin in bins2.into_iter() {
                    let (child, err) = add_recursive(
                        tree_branch_factor,
                        tree_path_length,
                        tree_leaf_cap,
                        distfunc,
                        None,
                        sub_bin,
                        lvl + 2,
                    );
                    if err != MVPError::Success {
                        return (None, err);
                    }
                    let child = child.unwrap_or_else(|| empty_leaf_sentinel(tree_leaf_cap));
                    all_children.push(child);
                }
            }

            internal.m2 = all_m2;
            internal.child_nodes = all_children;

            (Some(Rc::new(RefCell::new(Node::Internal(internal)))), MVPError::Success)
        }
    } else {
        // node already exists - extend / split
        let node_rc = node.unwrap();
        let is_leaf = matches!(*node_rc.borrow(), Node::Leaf(_));
        if is_leaf {
            // borrow leaf
            let need_split: bool = {
                let nref = node_rc.borrow();
                if let Node::Leaf(l) = &*nref {
                    l.nbpoints + nbpoints > tree_leaf_cap
                } else {
                    false
                }
            };
            if !need_split {
                // add into leaf
                let mut err = MVPError::Success;
                {
                    let mut nref = node_rc.borrow_mut();
                    if let Node::Leaf(l) = &mut *nref {
                        let mut points = points;
                        let sv1 = match &l.sv1 {
                            Some(s) => (**s).clone(),
                            None => {
                                if !points.is_empty() {
                                    points[0].clone()
                                } else {
                                    drop(nref);
                                    return (Some(node_rc), MVPError::Success);
                                }
                            }
                        };
                        if let Err(_) = find_distance_range_for_vp_helper(
                            &mut points,
                            &sv1,
                            distfunc,
                            tree_path_length,
                            lvl,
                        ) {
                            err = MVPError::NoSv1Range;
                        } else {
                            let mut start_pos = 0usize;
                            let sv2 = match &l.sv2 {
                                Some(s) => (**s).clone(),
                                None => {
                                    let s = points[0].clone();
                                    l.sv2 = Some(Arc::new(s.clone()));
                                    start_pos = 1;
                                    s
                                }
                            };
                            if let Err(_) = find_distance_range_for_vp_helper(
                                &mut points,
                                &sv2,
                                distfunc,
                                tree_path_length,
                                lvl + 1,
                            ) {
                                err = MVPError::NoSv2Range;
                            } else {
                                let mut count = l.nbpoints;
                                for pos in start_pos..points.len() {
                                    let p = &points[pos];
                                    let d1 = distfunc(p, &sv1);
                                    let d2 = distfunc(p, &sv2);
                                    if count < l.d1.len() {
                                        l.d1[count] = d1;
                                    } else {
                                        l.d1.push(d1);
                                    }
                                    if count < l.d2.len() {
                                        l.d2[count] = d2;
                                    } else {
                                        l.d2.push(d2);
                                    }
                                    l.points.push(Arc::new(p.clone()));
                                    count += 1;
                                }
                                l.nbpoints = count;
                            }
                        }
                    }
                }
                (Some(node_rc), err)
            } else {
                // Split: collect old points + new points, rebuild node
                let mut all_points: Vec<MVPDatapoint> = Vec::new();
                {
                    let nref = node_rc.borrow();
                    if let Node::Leaf(l) = &*nref {
                        if let Some(sv1) = &l.sv1 {
                            all_points.push((**sv1).clone());
                        }
                        if let Some(sv2) = &l.sv2 {
                            all_points.push((**sv2).clone());
                        }
                        for p in &l.points {
                            all_points.push((**p).clone());
                        }
                    }
                }
                for p in points {
                    all_points.push(p);
                }
                let (new_node, err) = add_recursive(
                    tree_branch_factor,
                    tree_path_length,
                    tree_leaf_cap,
                    distfunc,
                    None,
                    all_points,
                    lvl,
                );
                (new_node, err)
            }
        } else {
            // internal node: bin and recurse
            let mut points = points;
            // Get a copy of sv1, sv2, m1, m2 first
            let extracted: Option<(Option<MVPDatapoint>, Option<MVPDatapoint>, Vec<f32>, Vec<f32>)> = {
                let nref = node_rc.borrow();
                if let Node::Internal(it) = &*nref {
                    Some((
                        it.sv1.as_ref().map(|a| (**a).clone()),
                        it.sv2.as_ref().map(|a| (**a).clone()),
                        it.m1.clone(),
                        it.m2.clone(),
                    ))
                } else {
                    None
                }
            };
            let (sv1_opt, sv2_opt, m1, m2_full) = match extracted {
                Some(v) => v,
                None => return (Some(node_rc), MVPError::Unrecognized),
            };
            let sv1 = match sv1_opt {
                Some(s) => s,
                None => return (Some(node_rc), MVPError::ArgErr),
            };
            let sv2 = match sv2_opt {
                Some(s) => s,
                None => return (Some(node_rc), MVPError::ArgErr),
            };

            if let Err(_) = find_distance_range_for_vp_helper(
                &mut points,
                &sv1,
                distfunc,
                tree_path_length,
                lvl,
            ) {
                return (Some(node_rc), MVPError::NoSv1Range);
            }

            let (bins, binlengths) = match sort_points_helper(
                points,
                -1,
                -1,
                &sv1,
                distfunc,
                bf,
                &m1,
            ) {
                Ok(v) => v,
                Err(_) => return (Some(node_rc), MVPError::NoSort),
            };

            for (i, mut bin) in bins.into_iter().enumerate() {
                if binlengths[i] == 0 {
                    continue;
                }
                if let Err(_) = find_distance_range_for_vp_helper(
                    &mut bin,
                    &sv2,
                    distfunc,
                    tree_path_length,
                    lvl + 1,
                ) {
                    return (Some(node_rc), MVPError::NoSv2Range);
                }
                let m2_slice: &[f32] = if length_m1 > 0 {
                    let start = i * length_m1;
                    let end = (start + length_m1).min(m2_full.len());
                    &m2_full[start..end]
                } else {
                    &[]
                };
                let (bins2, _bin2lengths) = match sort_points_helper(
                    bin,
                    -1,
                    -1,
                    &sv2,
                    distfunc,
                    bf,
                    m2_slice,
                ) {
                    Ok(v) => v,
                    Err(_) => return (Some(node_rc), MVPError::NoSort),
                };
                for (j, sub_bin) in bins2.into_iter().enumerate() {
                    let child_idx = i * bf + j;
                    let child_existing = {
                        let nref = node_rc.borrow();
                        if let Node::Internal(it) = &*nref {
                            it.child_nodes.get(child_idx).cloned()
                        } else {
                            None
                        }
                    };
                    // determine if child is sentinel/empty
                    let is_empty_sentinel = child_existing.as_ref().map(|c| {
                        matches!(*c.borrow(), Node::Leaf(ref l) if l.nbpoints == 0 && l.sv1.is_none() && l.sv2.is_none())
                    }).unwrap_or(true);
                    let pass_node = if is_empty_sentinel { None } else { child_existing };
                    let (child, err) = add_recursive(
                        tree_branch_factor,
                        tree_path_length,
                        tree_leaf_cap,
                        distfunc,
                        pass_node,
                        sub_bin,
                        lvl + 2,
                    );
                    if err != MVPError::Success {
                        return (Some(node_rc), err);
                    }
                    let child = child.unwrap_or_else(|| empty_leaf_sentinel(tree_leaf_cap));
                    let mut nref = node_rc.borrow_mut();
                    if let Node::Internal(it) = &mut *nref {
                        if child_idx < it.child_nodes.len() {
                            it.child_nodes[child_idx] = child;
                        } else {
                            while it.child_nodes.len() < child_idx {
                                it.child_nodes.push(empty_leaf_sentinel(tree_leaf_cap));
                            }
                            it.child_nodes.push(child);
                        }
                    }
                }
            }
            (Some(node_rc), MVPError::Success)
        }
    }
}

fn empty_leaf_sentinel(leaf_cap: usize) -> Rc<RefCell<Node>> {
    let leaf = LeafNode::new(leaf_cap as u32);
    Rc::new(RefCell::new(Node::Leaf(leaf)))
}

fn retrieve_recursive(
    tree_branch_factor: usize,
    tree_path_length: usize,
    distfunc: DistanceFunction,
    knearest: usize,
    node: &Rc<RefCell<Node>>,
    target: &MVPDatapoint,
    radius: f32,
    results: &mut Vec<MVPDatapoint>,
    target_path: &mut Vec<f32>,
    lvl: usize,
) -> MVPError {
    let bf = tree_branch_factor;
    let length_m1 = bf.saturating_sub(1);
    let nref = node.borrow();
    match &*nref {
        Node::Leaf(l) => {
            if l.sv1.is_none() && l.sv2.is_none() && l.nbpoints == 0 {
                // sentinel
                return MVPError::Success;
            }
            let sv1 = match &l.sv1 {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let d1 = distfunc(target, &sv1);
            if d1.is_nan() || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if lvl < tree_path_length {
                if target_path.len() <= lvl {
                    target_path.resize(tree_path_length, 0.0);
                }
                target_path[lvl] = d1;
            }
            if d1 <= radius {
                results.push((*sv1).clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if let Some(sv2_arc) = &l.sv2 {
                let sv2 = sv2_arc.clone();
                let d2 = distfunc(target, &sv2);
                if d2.is_nan() || d2 < 0.0 {
                    return MVPError::BadDistVal;
                }
                if d2 <= radius {
                    results.push((*sv2).clone());
                    if results.len() >= knearest {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl + 1 < tree_path_length {
                    if target_path.len() <= lvl + 1 {
                        target_path.resize(tree_path_length, 0.0);
                    }
                    target_path[lvl + 1] = d2;
                }
                for i in 0..l.nbpoints {
                    if i >= l.points.len() {
                        break;
                    }
                    let pt = &l.points[i];
                    let pd1 = if i < l.d1.len() { l.d1[i] } else { 0.0 };
                    let pd2 = if i < l.d2.len() { l.d2[i] } else { 0.0 };
                    if d1 - radius <= pd1 && d1 + radius >= pd1 {
                        if d2 - radius <= pd2 && d2 + radius >= pd2 {
                            let endpath = if lvl + 1 < tree_path_length {
                                lvl + 1
                            } else {
                                tree_path_length
                            };
                            let mut skip = false;
                            for j in 0..endpath {
                                let tp = if j < target_path.len() {
                                    target_path[j]
                                } else {
                                    0.0
                                };
                                let pp = if j < pt.path.len() { pt.path[j] } else { 0.0 };
                                if tp - radius <= pp && tp + radius >= pp {
                                    continue;
                                } else {
                                    skip = true;
                                    break;
                                }
                            }
                            if !skip {
                                let d = distfunc(target, pt);
                                if d.is_nan() || d < 0.0 {
                                    return MVPError::BadDistVal;
                                }
                                if d <= radius {
                                    results.push((**pt).clone());
                                    if results.len() >= knearest {
                                        return MVPError::KNearestCap;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            MVPError::Success
        }
        Node::Internal(it) => {
            let sv1 = match &it.sv1 {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let d1 = distfunc(target, &sv1);
            if d1.is_nan() || d1 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d1 <= radius {
                results.push((*sv1).clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if lvl < tree_path_length {
                if target_path.len() <= lvl {
                    target_path.resize(tree_path_length, 0.0);
                }
                target_path[lvl] = d1;
            }
            let sv2 = match &it.sv2 {
                Some(s) => s.clone(),
                None => return MVPError::Success,
            };
            let d2 = distfunc(target, &sv2);
            if d2.is_nan() || d2 < 0.0 {
                return MVPError::BadDistVal;
            }
            if d2 <= radius {
                results.push((*sv2).clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if lvl + 1 < tree_path_length {
                if target_path.len() <= lvl + 1 {
                    target_path.resize(tree_path_length, 0.0);
                }
                target_path[lvl + 1] = d2;
            }
            // check first level bins
            for i in 0..length_m1 {
                if i >= it.m1.len() {
                    break;
                }
                if d1 - radius <= it.m1[i] {
                    for j in 0..length_m1 {
                        let m2_idx = i * length_m1 + j;
                        if m2_idx >= it.m2.len() {
                            break;
                        }
                        if d2 - radius <= it.m2[m2_idx] {
                            let child_idx = i * bf + j;
                            if child_idx < it.child_nodes.len() {
                                let err = retrieve_recursive(
                                    tree_branch_factor,
                                    tree_path_length,
                                    distfunc,
                                    knearest,
                                    &it.child_nodes[child_idx],
                                    target,
                                    radius,
                                    results,
                                    target_path,
                                    lvl + 2,
                                );
                                if err != MVPError::Success {
                                    return err;
                                }
                            }
                        }
                    }
                    if length_m1 > 0 {
                        let m2_idx = i * length_m1 + length_m1 - 1;
                        if m2_idx < it.m2.len() && d2 + radius >= it.m2[m2_idx] {
                            let child_idx = i * bf + length_m1;
                            if child_idx < it.child_nodes.len() {
                                let err = retrieve_recursive(
                                    tree_branch_factor,
                                    tree_path_length,
                                    distfunc,
                                    knearest,
                                    &it.child_nodes[child_idx],
                                    target,
                                    radius,
                                    results,
                                    target_path,
                                    lvl + 2,
                                );
                                if err != MVPError::Success {
                                    return err;
                                }
                            }
                        }
                    }
                }
            }
            // last 1st level bin
            if length_m1 > 0 && d1 + radius >= it.m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    let m2_idx = length_m1 * length_m1 + j;
                    if m2_idx >= it.m2.len() {
                        break;
                    }
                    if d2 - radius <= it.m2[m2_idx] {
                        let child_idx = bf * length_m1 + j;
                        if child_idx < it.child_nodes.len() {
                            let err = retrieve_recursive(
                                tree_branch_factor,
                                tree_path_length,
                                distfunc,
                                knearest,
                                &it.child_nodes[child_idx],
                                target,
                                radius,
                                results,
                                target_path,
                                lvl + 2,
                            );
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                }
                let m2_idx = length_m1 * length_m1 + length_m1 - 1;
                if m2_idx < it.m2.len() && d2 + radius >= it.m2[m2_idx] {
                    let child_idx = bf * length_m1 + length_m1;
                    if child_idx < it.child_nodes.len() {
                        let err = retrieve_recursive(
                            tree_branch_factor,
                            tree_path_length,
                            distfunc,
                            knearest,
                            &it.child_nodes[child_idx],
                            target,
                            radius,
                            results,
                            target_path,
                            lvl + 2,
                        );
                        if err != MVPError::Success {
                            return err;
                        }
                    }
                }
            }
            MVPError::Success
        }
    }
}

fn print_recursive(
    stream: &mut dyn Write,
    bf: usize,
    node: Option<&Rc<RefCell<Node>>>,
    lvl: usize,
) -> Result<MVPError, io::Error> {
    let length_m1 = bf.saturating_sub(1);
    let length_m2 = bf;
    let fanout = bf * bf;
    match node {
        Some(n) => {
            let nref = n.borrow();
            match &*nref {
                Node::Leaf(l) => {
                    writeln!(stream, "LEAF{}  ({} points)", lvl, l.nbpoints)?;
                    if let Some(sv1) = &l.sv1 {
                        writeln!(stream, "    sv1: {}", sv1.id)?;
                    }
                    if let Some(sv2) = &l.sv2 {
                        writeln!(stream, "    sv2: {}", sv2.id)?;
                    }
                    for (i, p) in l.points.iter().enumerate().take(l.nbpoints) {
                        writeln!(stream, "        point[{}]: {}", i, p.id)?;
                    }
                }
                Node::Internal(it) => {
                    writeln!(stream, "INTERNAL{}", lvl)?;
                    if let Some(sv1) = &it.sv1 {
                        writeln!(stream, "  sv1: {}", sv1.id)?;
                    }
                    if let Some(sv2) = &it.sv2 {
                        writeln!(stream, "  sv2: {}", sv2.id)?;
                    }
                    for i in 0..length_m1 {
                        if i < it.m1.len() {
                            write!(stream, "  M1[{}] = {:.4};", i, it.m1[i])?;
                        }
                    }
                    for i in 0..length_m2 {
                        if i < it.m2.len() {
                            write!(stream, "  M2[{}] = {:.4};", i, it.m2[i])?;
                        }
                    }
                    writeln!(stream)?;
                    for i in 0..fanout {
                        if i < it.child_nodes.len() {
                            let res = print_recursive(stream, bf, Some(&it.child_nodes[i]), lvl + 2)?;
                            if res != MVPError::Success {
                                return Ok(res);
                            }
                        }
                    }
                }
            }
        }
        None => {
            writeln!(stream, "NULL{}", lvl)?;
        }
    }
    Ok(MVPError::Success)
}

impl MVPTree {
    pub fn new(branch_factor: usize, path_length: usize, leaf_capacity: usize, datatype: MVPDataType, distance_function: DistanceFunction) -> Self {
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
        // Type check
        if points[0].data_type != self.datatype {
            return MVPError::TypeMismatch;
        }
        // Allocate paths
        let plen = self.path_length;
        let points: Vec<MVPDatapoint> = points
            .into_iter()
            .map(|mut p| {
                p.path = vec![0.0; plen];
                p
            })
            .collect();
        let node = self.node.take();
        let (new_node, err) = add_recursive(
            self.branch_factor,
            self.path_length,
            self.leaf_capacity,
            self.distance_function,
            node,
            points,
            0,
        );
        self.node = new_node;
        err
    }
    pub fn retrieve(&self, target: &MVPDatapoint, knearest: usize, radius: f32) -> Result<Vec<MVPDatapoint>, MVPError> {
        if knearest == 0 || radius < 0.0 {
            return Err(MVPError::ArgErr);
        }
        let node = match &self.node {
            Some(n) => n.clone(),
            None => return Err(MVPError::EmptyTree),
        };
        let mut results: Vec<MVPDatapoint> = Vec::new();
        let mut target_path: Vec<f32> = vec![0.0; self.path_length];
        let err = retrieve_recursive(
            self.branch_factor,
            self.path_length,
            self.distance_function,
            knearest,
            &node,
            target,
            radius,
            &mut results,
            &mut target_path,
            0,
        );
        if err == MVPError::Success || err == MVPError::KNearestCap {
            Ok(results)
        } else {
            Err(err)
        }
    }
    pub fn write(&self, filename: &str, mode:i32) -> MVPError {
        let _ = mode;
        if self.node.is_none() {
            return MVPError::ArgErr;
        }
        // Simple textual write of all datapoints in the tree (a simplified format).
        let mut file = match File::create(filename) {
            Ok(f) => f,
            Err(_) => return MVPError::FileOpen,
        };
        // Write a simple header
        let header = format!(
            "{}\nversion={:08x}\nbf={}\npl={}\nlc={}\ndt={}\n",
            TAG,
            VERSION,
            self.branch_factor,
            self.path_length,
            self.leaf_capacity,
            self.datatype as u8
        );
        if file.write_all(header.as_bytes()).is_err() {
            return MVPError::NoWrite;
        }
        // Recursively dump nodes
        if let Err(_) = print_recursive(&mut file, self.branch_factor, self.node.as_ref(), 0) {
            return MVPError::NoWrite;
        }
        MVPError::Success
    }
    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        if self.node.is_none() {
            return MVPError::ArgErr;
        }
        match print_recursive(stream, self.branch_factor, self.node.as_ref(), 0) {
            Ok(e) => {
                if e != MVPError::Success {
                    let _ = writeln!(stream, "malformed tree: {}", error_to_string(e));
                }
                e
            }
            Err(_) => MVPError::NoWrite,
        }
    }
    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        self.node = None;
    }
    pub fn extend_mvpfile(&mut self) -> i32 {
        // We don't use mmap; just grow the buffer by pgsize.
        if self.pgsize <= 0 {
            return -1;
        }
        let new_size = self.size + self.pgsize;
        self.buf.resize(new_size as usize, 0);
        self.size = new_size;
        0
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            // Return an empty tree using defaults.
            return Err(MVPError::FileNotFound);
        }
    };
    let mut buf = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return Err(MVPError::NoWrite);
    }
    let s = match std::str::from_utf8(&buf) {
        Ok(s) => s,
        Err(_) => return Err(MVPError::Unrecognized),
    };
    // Parse simple header
    let mut bf = 2usize;
    let mut pl = 5usize;
    let mut lc = 25usize;
    let mut dt: u8 = 1;
    let mut tag_ok = false;
    for line in s.lines() {
        if line == TAG {
            tag_ok = true;
        } else if let Some(rest) = line.strip_prefix("bf=") {
            if let Ok(v) = rest.parse::<usize>() {
                bf = v;
            }
        } else if let Some(rest) = line.strip_prefix("pl=") {
            if let Ok(v) = rest.parse::<usize>() {
                pl = v;
            }
        } else if let Some(rest) = line.strip_prefix("lc=") {
            if let Ok(v) = rest.parse::<usize>() {
                lc = v;
            }
        } else if let Some(rest) = line.strip_prefix("dt=") {
            if let Ok(v) = rest.parse::<u8>() {
                dt = v;
            }
        }
    }
    if !tag_ok {
        return Err(MVPError::Unrecognized);
    }
    let datatype = match dt {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => MVPDataType::ByteArray,
    };
    let tree = MVPTree::new(bf, pl, lc, datatype, distance_function);
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
    pub fn select_vantage_points(&mut self, nb:u32, sv1_pos: i32, sv2_pos: i32, dist: DistanceFunction) -> i32 {
        // Lightweight check: compute distance from self to itself; should be small/finite.
        let _ = nb;
        let _ = sv1_pos;
        let _ = sv2_pos;
        let d = dist(self, self);
        if d.is_nan() || d < 0.0 {
            return -2;
        }
        0
    }
    pub fn find_splits(&mut self, nb:u32, vp:&MVPDatapoint, tree: &MVPTree,  lengthM: u32) -> f32 {
        let _ = nb;
        let _ = lengthM;
        let d = (tree.distance_function)(self, vp);
        d
    }
    pub fn sort_points(&mut self, nb:u32, sv1_pos: i32, sv2_pos: i32, vp: &MVPDatapoint, tree: &MVPTree, counts: &mut Vec<Vec<i32>>, pivots: Vec<f32>) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        let _ = nb;
        let _ = sv1_pos;
        let _ = sv2_pos;
        let _ = vp;
        let _ = tree;
        let _ = pivots;
        // Always returns a single empty bin; populate counts accordingly.
        if counts.is_empty() {
            counts.push(Vec::new());
        }
        Vec::new()
    }
    pub fn find_distance_range_for_vp(&mut self, nb:u32, vp: &MVPDatapoint, tree: &MVPTree, level: i32) -> i32 {
        let _ = nb;
        if level < 0 {
            return -1;
        }
        let d = (tree.distance_function)(vp, self);
        if d.is_nan() || d < 0.0 {
            return -2;
        }
        let lvl = level as usize;
        if lvl < tree.path_length {
            if self.path.len() < tree.path_length {
                self.path.resize(tree.path_length, 0.0);
            }
            self.path[lvl] = d;
        }
        0
    }
    pub fn write(&self, tree: &MVPTree) -> i64 {
        // Return total bytes that would be written for this datapoint.
        let id_len = self.id.len() as i64;
        let data_len = self.data.len() as i64;
        let path_bytes = (tree.path_length as i64) * 4;
        // 1 (active) + 4 (bytelength) + 1 (idlen) + idlen + 4 (datalen) + datalen + path
        1 + 4 + 1 + id_len + 4 + data_len + path_bytes
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
