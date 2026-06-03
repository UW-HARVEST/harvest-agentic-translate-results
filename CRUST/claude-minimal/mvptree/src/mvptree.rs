use std::fs::{File, OpenOptions};
use std::io::{self, Write, Read};
use std::os::unix::fs::OpenOptionsExt;
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

impl MVPDataType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(MVPDataType::ByteArray),
            2 => Some(MVPDataType::UInt16Array),
            4 => Some(MVPDataType::UInt32Array),
            8 => Some(MVPDataType::UInt64Array),
            _ => None,
        }
    }
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
        let lengthm1 = if bf > 0 { bf - 1 } else { 0 };
        let lengthm2 = bf * lengthm1;
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; lengthm1],
            m2: vec![0.0; lengthm2],
            child_nodes: Vec::with_capacity(bf * bf),
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
    pub fn new(leafcap: u32) -> Self {
        let leafcap = leafcap as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(leafcap),
            d1: vec![0.0; leafcap],
            d2: vec![0.0; leafcap],
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

// ---------------- private helpers ----------------

fn is_bad_dist(d: f32) -> bool {
    d.is_nan() || d < 0.0
}

/// Select two vantage points: the pair with the maximum distance between them.
/// Returns (sv1_pos, sv2_pos). sv1_pos defaults to Some(0) if at least one point exists.
fn select_vantage_points(
    points: &[MVPDatapoint],
    dist: DistanceFunction,
) -> Result<(Option<usize>, Option<usize>), MVPError> {
    let nb = points.len();
    if nb == 0 {
        return Err(MVPError::ArgErr);
    }
    let mut sv1_pos: Option<usize> = Some(0);
    let mut sv2_pos: Option<usize> = None;
    let mut max_dist = 0.0f32;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if is_bad_dist(d) {
                return Err(MVPError::BadDistVal);
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

/// Compute the split pivots used to bin points relative to a vantage point.
fn find_splits(
    points: &[MVPDatapoint],
    vp: &MVPDatapoint,
    tree: &MVPTree,
    length_m: usize,
) -> Result<Vec<f32>, MVPError> {
    let nb = points.len();
    if nb == 0 || length_m == 0 {
        return Err(MVPError::ArgErr);
    }
    let dist = tree.distance_function;
    let mut distances: Vec<f32> = Vec::with_capacity(nb);
    for p in points {
        let d = dist(p, vp);
        if is_bad_dist(d) {
            return Err(MVPError::BadDistVal);
        }
        distances.push(d);
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut m = vec![0.0f32; length_m];
    for i in 0..length_m {
        let mut index = (i + 1) * nb / (length_m + 1);
        if index >= nb {
            index = nb - 1;
        }
        m[i] = distances[index];
    }
    Ok(m)
}

/// Sort points into bf bins based on distance to vp using the provided pivot points.
/// Returns bins (Vec<Vec<MVPDatapoint>>).
fn sort_points(
    points: Vec<MVPDatapoint>,
    vp: &MVPDatapoint,
    tree: &MVPTree,
    pivots: &[f32],
) -> Result<Vec<Vec<MVPDatapoint>>, MVPError> {
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;
    let dist = tree.distance_function;
    let mut bins: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
    for p in points.into_iter() {
        let d = dist(vp, &p);
        if is_bad_dist(d) {
            return Err(MVPError::BadDistVal);
        }
        let mut placed = false;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                bins[k].push(p.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            // d > pivots[length_m1 - 1]
            bins[length_m1].push(p);
        }
    }
    Ok(bins)
}

/// Compute distances from vp to each point, write distance to point.path[lvl] if room,
/// and return the distances.
fn compute_distances_and_set_path(
    points: &mut [MVPDatapoint],
    vp: &MVPDatapoint,
    tree: &MVPTree,
    lvl: usize,
) -> Result<Vec<f32>, MVPError> {
    let dist = tree.distance_function;
    let mut out = Vec::with_capacity(points.len());
    for p in points.iter_mut() {
        let d = dist(vp, p);
        if is_bad_dist(d) {
            return Err(MVPError::BadDistVal);
        }
        if lvl < tree.path_length {
            if p.path.len() < tree.path_length {
                p.path.resize(tree.path_length, 0.0);
            }
            p.path[lvl] = d;
        }
        out.push(d);
    }
    Ok(out)
}

fn ensure_path_capacity(p: &mut MVPDatapoint, path_length: usize) {
    if p.path.len() < path_length {
        p.path.resize(path_length, 0.0);
    }
}

// ---------------- node creation ----------------

fn create_leaf_from_points(
    tree: &MVPTree,
    mut points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<Node, MVPError> {
    if points.is_empty() {
        return Ok(Node::Leaf(LeafNode::new(tree.leaf_capacity as u32)));
    }
    let dist = tree.distance_function;
    let (sv1_pos_opt, sv2_pos_opt) = select_vantage_points(&points, dist)?;

    // Extract sv1 and sv2 from the points vector. We must remove indices in
    // descending order to keep remaining indices valid.
    let mut sv1: Option<MVPDatapoint> = None;
    let mut sv2: Option<MVPDatapoint> = None;

    match (sv1_pos_opt, sv2_pos_opt) {
        (Some(s1), Some(s2)) => {
            // Use stable removal: collect indices to remove in descending order.
            let (lo, hi, swap) = if s1 < s2 {
                (s1, s2, false)
            } else {
                (s2, s1, true)
            };
            let hi_pt = points.remove(hi);
            let lo_pt = points.remove(lo);
            if swap {
                sv1 = Some(hi_pt);
                sv2 = Some(lo_pt);
            } else {
                sv1 = Some(lo_pt);
                sv2 = Some(hi_pt);
            }
        }
        (Some(s1), None) => {
            sv1 = Some(points.remove(s1));
        }
        _ => {}
    }

    let mut leaf = LeafNode::new(tree.leaf_capacity as u32);

    // Compute distances of remaining points to sv1 and update paths.
    let mut d1_vec: Vec<f32> = Vec::with_capacity(points.len());
    let mut d2_vec: Vec<f32> = Vec::with_capacity(points.len());

    if let Some(ref sv1_pt) = sv1 {
        d1_vec = compute_distances_and_set_path(&mut points, sv1_pt, tree, lvl)?;
    } else {
        d1_vec = vec![0.0; points.len()];
    }

    if let Some(ref sv2_pt) = sv2 {
        d2_vec = compute_distances_and_set_path(&mut points, sv2_pt, tree, lvl + 1)?;
    } else {
        d2_vec = vec![0.0; points.len()];
    }

    // Update sv1's and sv2's own path entries.
    if let Some(ref mut sv1_mut) = sv1 {
        ensure_path_capacity(sv1_mut, tree.path_length);
        if lvl < tree.path_length {
            sv1_mut.path[lvl] = 0.0;
        }
        if let Some(ref sv2_pt) = sv2 {
            let d = dist(sv2_pt, sv1_mut);
            if is_bad_dist(d) {
                return Err(MVPError::BadDistVal);
            }
            if lvl + 1 < tree.path_length {
                sv1_mut.path[lvl + 1] = d;
            }
        }
    }
    if let Some(ref mut sv2_mut) = sv2 {
        ensure_path_capacity(sv2_mut, tree.path_length);
        if lvl + 1 < tree.path_length {
            sv2_mut.path[lvl + 1] = 0.0;
        }
        if let Some(ref sv1_pt) = sv1 {
            let d = dist(sv1_pt, sv2_mut);
            if is_bad_dist(d) {
                return Err(MVPError::BadDistVal);
            }
            if lvl < tree.path_length {
                sv2_mut.path[lvl] = d;
            }
        }
    }

    // Pad d1, d2 to leaf_capacity
    let lc = tree.leaf_capacity;
    if d1_vec.len() < lc {
        d1_vec.resize(lc, 0.0);
    }
    if d2_vec.len() < lc {
        d2_vec.resize(lc, 0.0);
    }

    leaf.nbpoints = points.len();
    leaf.points = points.into_iter().map(Arc::new).collect();
    leaf.d1 = d1_vec;
    leaf.d2 = d2_vec;
    leaf.sv1 = sv1.map(Arc::new);
    leaf.sv2 = sv2.map(Arc::new);

    Ok(Node::Leaf(leaf))
}

fn create_internal_from_points(
    tree: &MVPTree,
    mut points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<Node, MVPError> {
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;
    let dist = tree.distance_function;

    let (sv1_pos_opt, sv2_pos_opt) = select_vantage_points(&points, dist)?;
    let s1 = sv1_pos_opt.ok_or(MVPError::VpNoSelect)?;
    let s2 = sv2_pos_opt.ok_or(MVPError::VpNoSelect)?;

    // Extract sv1, sv2 (removing higher index first)
    let (lo, hi, swap) = if s1 < s2 {
        (s1, s2, false)
    } else {
        (s2, s1, true)
    };
    let hi_pt = points.remove(hi);
    let lo_pt = points.remove(lo);
    let (mut sv1, mut sv2) = if swap {
        (hi_pt, lo_pt)
    } else {
        (lo_pt, hi_pt)
    };

    // Compute distances from sv1 to all remaining points (and update their paths)
    let _d_sv1 = compute_distances_and_set_path(&mut points, &sv1, tree, lvl)?;

    // sv1's path
    ensure_path_capacity(&mut sv1, tree.path_length);
    if lvl < tree.path_length {
        sv1.path[lvl] = 0.0;
    }
    let d_sv1_sv2 = dist(&sv1, &sv2);
    if is_bad_dist(d_sv1_sv2) {
        return Err(MVPError::BadDistVal);
    }
    ensure_path_capacity(&mut sv2, tree.path_length);
    if lvl < tree.path_length {
        sv2.path[lvl] = d_sv1_sv2;
    }

    // Compute M1 from points and sv1
    let m1 = find_splits(&points, &sv1, tree, length_m1).map_err(|_| MVPError::NoSplits)?;

    // Sort points into bins based on M1
    let bins = sort_points(points, &sv1, tree, &m1).map_err(|_| MVPError::NoSort)?;

    // For each bin, compute distances to sv2 (lvl+1), then split into bins via M2.
    let mut m2 = vec![0.0f32; bf * length_m1];
    let mut child_bins: Vec<Vec<Vec<MVPDatapoint>>> = Vec::with_capacity(bf);

    for (i, mut bin) in bins.into_iter().enumerate() {
        if bin.is_empty() {
            // Empty bin still needs M2 entries (zeros) and bf empty children.
            let empty: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
            child_bins.push(empty);
            continue;
        }
        let _d_sv2_bin = compute_distances_and_set_path(&mut bin, &sv2, tree, lvl + 1)?;
        let m2_bin =
            find_splits(&bin, &sv2, tree, length_m1).map_err(|_| MVPError::NoSplits)?;
        for (k, val) in m2_bin.iter().enumerate() {
            m2[i * length_m1 + k] = *val;
        }
        let bin2 = sort_points(bin, &sv2, tree, &m2_bin).map_err(|_| MVPError::NoSort)?;
        child_bins.push(bin2);
    }

    // sv2's own path[lvl+1]
    if lvl + 1 < tree.path_length {
        sv2.path[lvl + 1] = 0.0;
    }

    let mut internal = InternalNode::new(bf as u32);
    internal.sv1 = Some(Arc::new(sv1));
    internal.sv2 = Some(Arc::new(sv2));
    internal.m1 = m1;
    internal.m2 = m2;

    // Recursively construct child nodes
    for bin2 in child_bins {
        for sub_bin in bin2 {
            if sub_bin.is_empty() {
                internal.child_nodes.push(Rc::new(RefCell::new(
                    Node::Leaf(LeafNode::new(tree.leaf_capacity as u32)),
                )));
                // Mark empty leaf as logically "None" via empty leaf - but C uses NULL.
                // Use a sentinel: empty optional. We'll use Option semantics via storing
                // a placeholder leaf with no sv1; this is treated as empty in retrieval.
            } else {
                let child = build_subtree(tree, sub_bin, lvl + 2)?;
                internal.child_nodes.push(Rc::new(RefCell::new(child)));
            }
        }
    }

    Ok(Node::Internal(internal))
}

/// Recursively build a subtree node from a fresh set of points.
fn build_subtree(
    tree: &MVPTree,
    points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<Node, MVPError> {
    if points.is_empty() {
        return Ok(Node::Leaf(LeafNode::new(tree.leaf_capacity as u32)));
    }
    // Pre-check: if we can't find 2 distinct vantage points (e.g. all points
    // are identical), we must build a leaf regardless of the count.
    let (_sv1_opt, sv2_opt) = select_vantage_points(&points, tree.distance_function)?;
    if points.len() <= tree.leaf_capacity + 2 || sv2_opt.is_none() {
        create_leaf_from_points(tree, points, lvl)
    } else {
        create_internal_from_points(tree, points, lvl)
    }
}

// ---------------- adding to existing node ----------------

/// Try to extract a Vec<MVPDatapoint> from a vector of Arc<MVPDatapoint>. If an Arc
/// has more than one strong reference, clones the inner data.
fn unwrap_arcs(arcs: Vec<Arc<MVPDatapoint>>) -> Vec<MVPDatapoint> {
    arcs.into_iter()
        .map(|a| match Arc::try_unwrap(a) {
            Ok(v) => v,
            Err(arc) => (*arc).clone(),
        })
        .collect()
}

fn add_to_existing_node(
    tree: &MVPTree,
    node: Node,
    new_points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<Node, MVPError> {
    match node {
        Node::Leaf(mut leaf) => {
            if leaf.nbpoints + new_points.len() <= tree.leaf_capacity {
                // Plenty of room: append to leaf
                add_to_leaf_in_place(tree, &mut leaf, new_points, lvl)?;
                Ok(Node::Leaf(leaf))
            } else {
                // Overflow: extract everything and rebuild
                let mut combined: Vec<MVPDatapoint> = Vec::new();
                if let Some(sv1) = leaf.sv1.take() {
                    combined.push(match Arc::try_unwrap(sv1) {
                        Ok(v) => v,
                        Err(a) => (*a).clone(),
                    });
                }
                if let Some(sv2) = leaf.sv2.take() {
                    combined.push(match Arc::try_unwrap(sv2) {
                        Ok(v) => v,
                        Err(a) => (*a).clone(),
                    });
                }
                let pts = std::mem::take(&mut leaf.points);
                combined.extend(unwrap_arcs(pts));
                combined.extend(new_points);

                build_subtree(tree, combined, lvl)
            }
        }
        Node::Internal(mut internal) => {
            add_to_internal_in_place(tree, &mut internal, new_points, lvl)?;
            Ok(Node::Internal(internal))
        }
    }
}

fn add_to_leaf_in_place(
    tree: &MVPTree,
    leaf: &mut LeafNode,
    mut points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<(), MVPError> {
    let dist = tree.distance_function;

    // Update paths via sv1
    if let Some(ref sv1_arc) = leaf.sv1 {
        let sv1 = sv1_arc.as_ref();
        compute_distances_and_set_path(&mut points, sv1, tree, lvl)
            .map_err(|_| MVPError::NoSv1Range)?;
    }

    let mut start = 0usize;
    if leaf.sv2.is_none() && !points.is_empty() {
        // Promote first incoming point to sv2
        let new_sv2 = points.remove(0);
        let new_sv2_arc = Arc::new(new_sv2);
        leaf.sv2 = Some(new_sv2_arc);
        start = 0; // Already removed; iterate the rest below
        // Reset points iterator. Setting start = 0 since points has shifted.
        let _ = start;
    }

    // Update paths via sv2
    if let Some(ref sv2_arc) = leaf.sv2 {
        let sv2 = sv2_arc.as_ref();
        compute_distances_and_set_path(&mut points, sv2, tree, lvl + 1)
            .map_err(|_| MVPError::NoSv2Range)?;
    }

    // Append points to leaf with their d1/d2 distances
    for p in points.into_iter() {
        let mut p = p;
        ensure_path_capacity(&mut p, tree.path_length);
        let d1 = if let Some(ref sv1_arc) = leaf.sv1 {
            let d = dist(&p, sv1_arc.as_ref());
            if is_bad_dist(d) {
                return Err(MVPError::BadDistVal);
            }
            d
        } else {
            0.0
        };
        let d2 = if let Some(ref sv2_arc) = leaf.sv2 {
            let d = dist(&p, sv2_arc.as_ref());
            if is_bad_dist(d) {
                return Err(MVPError::BadDistVal);
            }
            d
        } else {
            0.0
        };
        let count = leaf.nbpoints;
        if count >= leaf.d1.len() {
            leaf.d1.resize(count + 1, 0.0);
            leaf.d2.resize(count + 1, 0.0);
        }
        leaf.d1[count] = d1;
        leaf.d2[count] = d2;
        leaf.points.push(Arc::new(p));
        leaf.nbpoints += 1;
    }

    Ok(())
}

fn add_to_internal_in_place(
    tree: &MVPTree,
    internal: &mut InternalNode,
    mut points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<(), MVPError> {
    let bf = tree.branch_factor;
    let length_m1 = bf - 1;

    // Update paths via sv1
    let sv1_arc = internal.sv1.clone().ok_or(MVPError::ArgErr)?;
    compute_distances_and_set_path(&mut points, sv1_arc.as_ref(), tree, lvl)
        .map_err(|_| MVPError::NoSv1Range)?;

    // Sort points into bins via M1 around sv1
    let bins = sort_points(points, sv1_arc.as_ref(), tree, &internal.m1)
        .map_err(|_| MVPError::NoSort)?;

    let sv2_arc = internal.sv2.clone().ok_or(MVPError::ArgErr)?;

    for (i, mut bin) in bins.into_iter().enumerate() {
        if bin.is_empty() {
            continue;
        }
        // Update paths via sv2
        compute_distances_and_set_path(&mut bin, sv2_arc.as_ref(), tree, lvl + 1)
            .map_err(|_| MVPError::NoSv2Range)?;

        // Sort by M2 slice for this i
        let m2_slice: Vec<f32> = (0..length_m1)
            .map(|k| internal.m2[i * length_m1 + k])
            .collect();
        let bin2 = sort_points(bin, sv2_arc.as_ref(), tree, &m2_slice)
            .map_err(|_| MVPError::NoSort)?;

        for (j, sub_bin) in bin2.into_iter().enumerate() {
            if sub_bin.is_empty() {
                continue;
            }
            let child_idx = i * bf + j;
            // Recursively add sub_bin to child node
            let child_rc = internal.child_nodes[child_idx].clone();
            // Take node out, add, put back
            let new_child = {
                let cell = &*child_rc;
                let node = std::mem::replace(
                    &mut *cell.borrow_mut(),
                    Node::Leaf(LeafNode::new(tree.leaf_capacity as u32)),
                );
                // Determine if this is an "empty" placeholder leaf (no sv1).
                let is_empty_placeholder = matches!(
                    &node,
                    Node::Leaf(l) if l.sv1.is_none() && l.nbpoints == 0
                );
                if is_empty_placeholder {
                    build_subtree(tree, sub_bin, lvl + 2)?
                } else {
                    add_to_existing_node(tree, node, sub_bin, lvl + 2)?
                }
            };
            *child_rc.borrow_mut() = new_child;
        }
    }

    Ok(())
}

// ---------------- retrieve ----------------

fn retrieve_recursive(
    tree: &MVPTree,
    node_rc: &Rc<RefCell<Node>>,
    target: &mut MVPDatapoint,
    radius: f32,
    knearest: usize,
    results: &mut Vec<Arc<MVPDatapoint>>,
    lvl: usize,
) -> Result<(), MVPError> {
    let dist = tree.distance_function;
    let node_ref = node_rc.borrow();
    match &*node_ref {
        Node::Leaf(leaf) => {
            // If the leaf is a placeholder (no sv1, no points), treat as empty.
            if leaf.sv1.is_none() && leaf.nbpoints == 0 {
                return Ok(());
            }
            let sv1_arc = match &leaf.sv1 {
                Some(a) => a.clone(),
                None => return Ok(()),
            };
            let d1 = dist(target, sv1_arc.as_ref());
            if is_bad_dist(d1) {
                return Err(MVPError::BadDistVal);
            }
            ensure_path_capacity(target, tree.path_length);
            if lvl < tree.path_length {
                target.path[lvl] = d1;
            }
            if d1 <= radius {
                results.push(sv1_arc.clone());
                if results.len() >= knearest {
                    return Err(MVPError::KNearestCap);
                }
            }
            if let Some(sv2_arc) = leaf.sv2.clone() {
                let d2 = dist(target, sv2_arc.as_ref());
                if is_bad_dist(d2) {
                    return Err(MVPError::BadDistVal);
                }
                if d2 <= radius {
                    results.push(sv2_arc.clone());
                    if results.len() >= knearest {
                        return Err(MVPError::KNearestCap);
                    }
                }
                if lvl + 1 < tree.path_length {
                    target.path[lvl + 1] = d2;
                }
                for i in 0..leaf.nbpoints {
                    let pt_d1 = leaf.d1[i];
                    let pt_d2 = leaf.d2[i];
                    if d1 - radius <= pt_d1 && d1 + radius >= pt_d1 {
                        if d2 - radius <= pt_d2 && d2 + radius >= pt_d2 {
                            let endpath = if lvl + 1 < tree.path_length {
                                lvl + 1
                            } else {
                                tree.path_length
                            };
                            let mut skip = false;
                            for j in 0..endpath {
                                let tgt = target.path[j];
                                let pp = leaf.points[i].path.get(j).copied().unwrap_or(0.0);
                                if tgt - radius <= pp && tgt + radius >= pp {
                                    continue;
                                } else {
                                    skip = true;
                                    break;
                                }
                            }
                            if !skip {
                                let d = dist(target, leaf.points[i].as_ref());
                                if is_bad_dist(d) {
                                    return Err(MVPError::BadDistVal);
                                }
                                if d <= radius {
                                    results.push(leaf.points[i].clone());
                                    if results.len() >= knearest {
                                        return Err(MVPError::KNearestCap);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        Node::Internal(internal) => {
            let bf = tree.branch_factor;
            let length_m1 = bf - 1;
            let sv1_arc = match &internal.sv1 {
                Some(a) => a.clone(),
                None => return Ok(()),
            };
            let d1 = dist(target, sv1_arc.as_ref());
            if is_bad_dist(d1) {
                return Err(MVPError::BadDistVal);
            }
            if d1 <= radius {
                results.push(sv1_arc.clone());
                if results.len() >= knearest {
                    return Err(MVPError::KNearestCap);
                }
            }
            ensure_path_capacity(target, tree.path_length);
            if lvl < tree.path_length {
                target.path[lvl] = d1;
            }
            let sv2_arc = match &internal.sv2 {
                Some(a) => a.clone(),
                None => return Ok(()),
            };
            let d2 = dist(target, sv2_arc.as_ref());
            if is_bad_dist(d2) {
                return Err(MVPError::BadDistVal);
            }
            if d2 <= radius {
                results.push(sv2_arc.clone());
                if results.len() >= knearest {
                    return Err(MVPError::KNearestCap);
                }
            }
            if lvl + 1 < tree.path_length {
                target.path[lvl + 1] = d2;
            }

            // Drop borrow before recursion
            let child_nodes = internal.child_nodes.clone();
            let m1 = internal.m1.clone();
            let m2 = internal.m2.clone();
            drop(node_ref);

            for i in 0..length_m1 {
                if d1 - radius <= m1[i] {
                    for j in 0..length_m1 {
                        if d2 - radius <= m2[i * length_m1 + j] {
                            retrieve_recursive(
                                tree,
                                &child_nodes[i * bf + j],
                                target,
                                radius,
                                knearest,
                                results,
                                lvl + 2,
                            )?;
                        }
                    }
                    // last 2nd-level bin
                    if d2 + radius >= m2[i * length_m1 + length_m1 - 1] {
                        retrieve_recursive(
                            tree,
                            &child_nodes[i * bf + length_m1],
                            target,
                            radius,
                            knearest,
                            results,
                            lvl + 2,
                        )?;
                    }
                }
            }
            // last 1st-level bin
            if d1 + radius >= m1[length_m1 - 1] {
                for j in 0..length_m1 {
                    if d2 - radius <= m2[length_m1 * length_m1 + j] {
                        retrieve_recursive(
                            tree,
                            &child_nodes[bf * length_m1 + j],
                            target,
                            radius,
                            knearest,
                            results,
                            lvl + 2,
                        )?;
                    }
                }
                if d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1] {
                    retrieve_recursive(
                        tree,
                        &child_nodes[bf * length_m1 + length_m1],
                        target,
                        radius,
                        knearest,
                        results,
                        lvl + 2,
                    )?;
                }
            }
            Ok(())
        }
    }
}

// ---------------- write/read binary format ----------------

fn write_u8(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
}
fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn write_i64_le(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn write_f32_le(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn read_u8(buf: &[u8], pos: &mut usize) -> Option<u8> {
    if *pos >= buf.len() {
        return None;
    }
    let v = buf[*pos];
    *pos += 1;
    Some(v)
}
fn read_u32_le(buf: &[u8], pos: &mut usize) -> Option<u32> {
    if *pos + 4 > buf.len() {
        return None;
    }
    let v = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().ok()?);
    *pos += 4;
    Some(v)
}
fn read_i64_le(buf: &[u8], pos: &mut usize) -> Option<i64> {
    if *pos + 8 > buf.len() {
        return None;
    }
    let v = i64::from_le_bytes(buf[*pos..*pos + 8].try_into().ok()?);
    *pos += 8;
    Some(v)
}
fn read_f32_le(buf: &[u8], pos: &mut usize) -> Option<f32> {
    if *pos + 4 > buf.len() {
        return None;
    }
    let v = f32::from_le_bytes(buf[*pos..*pos + 4].try_into().ok()?);
    *pos += 4;
    Some(v)
}

fn write_datapoint(
    buf: &mut Vec<u8>,
    dp: Option<&MVPDatapoint>,
    path_length: usize,
) -> usize {
    let start = buf.len();
    match dp {
        None => {
            // active=0, bytelength=0
            write_u8(buf, 0u8);
            write_u32_le(buf, 0u32);
        }
        Some(dp) => {
            let active: u8 = 1;
            let idlen = dp.id.len() as u8;
            let datalength = dp.datalen as u32;
            let type_size = dp.data_type as u8;
            let data_bytes = (datalength as usize) * (type_size as usize);
            let bytelength = (1u32) // idlen byte
                + (idlen as u32)
                + 4u32 // datalength
                + (data_bytes as u32)
                + (path_length as u32) * 4u32;
            write_u8(buf, active);
            write_u32_le(buf, bytelength);
            write_u8(buf, idlen);
            buf.extend_from_slice(&dp.id.as_bytes()[..idlen as usize]);
            write_u32_le(buf, datalength);
            // ensure data has at least data_bytes
            let actual_data = &dp.data[..data_bytes.min(dp.data.len())];
            buf.extend_from_slice(actual_data);
            // pad if necessary
            if actual_data.len() < data_bytes {
                buf.resize(buf.len() + (data_bytes - actual_data.len()), 0);
            }
            // write path
            for i in 0..path_length {
                let v = dp.path.get(i).copied().unwrap_or(0.0);
                write_f32_le(buf, v);
            }
        }
    }
    start
}

fn read_datapoint(
    buf: &[u8],
    pos: &mut usize,
    path_length: usize,
    datatype: MVPDataType,
) -> Result<Option<MVPDatapoint>, MVPError> {
    let active = read_u8(buf, pos).ok_or(MVPError::NoWrite)?;
    let bytelength = read_u32_le(buf, pos).ok_or(MVPError::NoWrite)?;
    if active == 0 && bytelength == 0 {
        return Ok(None);
    }
    let idlen = read_u8(buf, pos).ok_or(MVPError::NoWrite)? as usize;
    if *pos + idlen > buf.len() {
        return Err(MVPError::NoWrite);
    }
    let id = String::from_utf8_lossy(&buf[*pos..*pos + idlen]).into_owned();
    *pos += idlen;
    let datalength = read_u32_le(buf, pos).ok_or(MVPError::NoWrite)? as usize;
    let type_size = datatype as u8 as usize;
    let data_bytes = datalength * type_size;
    if *pos + data_bytes > buf.len() {
        return Err(MVPError::NoWrite);
    }
    let data = buf[*pos..*pos + data_bytes].to_vec();
    *pos += data_bytes;
    let mut path: Vec<f32> = Vec::with_capacity(path_length);
    for _ in 0..path_length {
        path.push(read_f32_le(buf, pos).ok_or(MVPError::NoWrite)?);
    }
    Ok(Some(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    }))
}

fn write_node_recursive(
    buf: &mut Vec<u8>,
    tree: &MVPTree,
    node_rc: &Rc<RefCell<Node>>,
) -> Result<usize, MVPError> {
    let start = buf.len();
    let node_ref = node_rc.borrow();
    match &*node_ref {
        Node::Leaf(leaf) => {
            // For empty placeholder leaf (no sv1, no points), serialize as null.
            // But that would conflict with reading. Actually we'll serialize regardless.
            let node_type: u8 = 2; // LEAF_NODE
            write_u8(buf, node_type);
            write_datapoint(buf, leaf.sv1.as_ref().map(|a| a.as_ref()), tree.path_length);
            write_datapoint(buf, leaf.sv2.as_ref().map(|a| a.as_ref()), tree.path_length);
            let nbpoints = leaf.nbpoints as u32;
            write_u32_le(buf, nbpoints);
            // Reserve space for the d1/d2/offset table.
            // The table is normally leaf_capacity entries, but if a leaf holds
            // more (e.g. many identical points that cannot be split), expand
            // it so we don't overlap the datapoint area.
            let lc = tree.leaf_capacity;
            let table_entries = lc.max(leaf.nbpoints);
            let table_size = table_entries * (4 + 4 + 8);
            let table_start = buf.len();
            buf.resize(buf.len() + table_size, 0);

            // Now write the points and fill in the table
            for i in 0..leaf.nbpoints {
                let pt_offset = buf.len();
                write_datapoint(buf, Some(leaf.points[i].as_ref()), tree.path_length);
                // Fill in table at table_start + i*16
                let d1 = leaf.d1.get(i).copied().unwrap_or(0.0);
                let d2 = leaf.d2.get(i).copied().unwrap_or(0.0);
                let entry_pos = table_start + i * 16;
                buf[entry_pos..entry_pos + 4].copy_from_slice(&d1.to_le_bytes());
                buf[entry_pos + 4..entry_pos + 8].copy_from_slice(&d2.to_le_bytes());
                buf[entry_pos + 8..entry_pos + 16]
                    .copy_from_slice(&(pt_offset as i64).to_le_bytes());
            }
            Ok(start)
        }
        Node::Internal(internal) => {
            let bf = tree.branch_factor;
            let length_m1 = bf - 1;
            let length_m2 = bf * length_m1;
            let fanout = bf * bf;

            let node_type: u8 = 1; // INTERNAL_NODE
            write_u8(buf, node_type);
            write_datapoint(
                buf,
                internal.sv1.as_ref().map(|a| a.as_ref()),
                tree.path_length,
            );
            write_datapoint(
                buf,
                internal.sv2.as_ref().map(|a| a.as_ref()),
                tree.path_length,
            );
            // M1
            for v in &internal.m1[..length_m1] {
                write_f32_le(buf, *v);
            }
            // M2
            for k in 0..length_m2 {
                write_f32_le(buf, internal.m2.get(k).copied().unwrap_or(0.0));
            }
            // Reserve fanout * (1 byte fileno + 8 bytes off_t)
            let table_start = buf.len();
            let table_size = fanout * (1 + 8);
            buf.resize(buf.len() + table_size, 0);

            // Drop borrow before recursing
            let children = internal.child_nodes.clone();
            drop(node_ref);

            for (i, child_rc) in children.iter().enumerate() {
                let offset = write_node_recursive(buf, tree, child_rc)?;
                let entry_pos = table_start + i * 9;
                buf[entry_pos] = 0u8; // fileno
                buf[entry_pos + 1..entry_pos + 9]
                    .copy_from_slice(&(offset as i64).to_le_bytes());
            }
            Ok(start)
        }
    }
}

fn read_node_recursive(
    buf: &[u8],
    pos: &mut usize,
    tree: &MVPTree,
) -> Result<Node, MVPError> {
    let node_type = read_u8(buf, pos).ok_or(MVPError::NoWrite)?;
    let path_length = tree.path_length;
    let datatype = tree.datatype;
    if node_type == 2 {
        // LEAF_NODE
        let sv1 = read_datapoint(buf, pos, path_length, datatype)?;
        let sv2 = read_datapoint(buf, pos, path_length, datatype)?;
        let nbpoints = read_u32_le(buf, pos).ok_or(MVPError::NoWrite)? as usize;
        let lc = tree.leaf_capacity;
        let table_start = *pos;
        let mut leaf = LeafNode::new(lc as u32);
        leaf.sv1 = sv1.map(Arc::new);
        leaf.sv2 = sv2.map(Arc::new);
        leaf.nbpoints = nbpoints;
        if leaf.d1.len() < nbpoints {
            leaf.d1.resize(nbpoints, 0.0);
        }
        if leaf.d2.len() < nbpoints {
            leaf.d2.resize(nbpoints, 0.0);
        }
        let mut points: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(nbpoints);
        for i in 0..nbpoints {
            let entry_pos = table_start + i * 16;
            let d1 = f32::from_le_bytes(
                buf[entry_pos..entry_pos + 4]
                    .try_into()
                    .map_err(|_| MVPError::NoWrite)?,
            );
            let d2 = f32::from_le_bytes(
                buf[entry_pos + 4..entry_pos + 8]
                    .try_into()
                    .map_err(|_| MVPError::NoWrite)?,
            );
            let offset = i64::from_le_bytes(
                buf[entry_pos + 8..entry_pos + 16]
                    .try_into()
                    .map_err(|_| MVPError::NoWrite)?,
            ) as usize;
            leaf.d1[i] = d1;
            leaf.d2[i] = d2;
            let mut child_pos = offset;
            let dp = read_datapoint(buf, &mut child_pos, path_length, datatype)?
                .ok_or(MVPError::NoWrite)?;
            points.push(Arc::new(dp));
        }
        leaf.points = points;
        // Advance pos past the entire table. Use max(lc, nbpoints) for the
        // case where the leaf holds more than leaf_capacity points (e.g. many
        // identical points that cannot be split).
        let table_entries = lc.max(nbpoints);
        *pos = table_start + table_entries * 16;
        // Find end of last child (advance pos to the max of children offsets + sizes).
        // For simplicity: scan max child end from the bytes we know about.
        // Actually our writer always writes children sequentially after the table,
        // so we can find the maximum used offset and put pos there. But child sizes
        // depend on type. The simplest: don't try to advance past, since this is
        // the last node usually. Reading is offset-driven for children, and the
        // main caller (mvptree_read) doesn't rely on sequential pos. So leaving
        // pos at table_start + lc*16 is OK for our use (caller only reads root).
        Ok(Node::Leaf(leaf))
    } else if node_type == 1 {
        // INTERNAL_NODE
        let bf = tree.branch_factor;
        let length_m1 = bf - 1;
        let length_m2 = bf * length_m1;
        let fanout = bf * bf;

        let sv1 = read_datapoint(buf, pos, path_length, datatype)?;
        let sv2 = read_datapoint(buf, pos, path_length, datatype)?;

        let mut internal = InternalNode::new(bf as u32);
        internal.sv1 = sv1.map(Arc::new);
        internal.sv2 = sv2.map(Arc::new);

        // M1
        let mut m1 = vec![0.0f32; length_m1];
        for v in m1.iter_mut() {
            *v = read_f32_le(buf, pos).ok_or(MVPError::NoWrite)?;
        }
        internal.m1 = m1;
        // M2
        let mut m2 = vec![0.0f32; length_m2];
        for v in m2.iter_mut() {
            *v = read_f32_le(buf, pos).ok_or(MVPError::NoWrite)?;
        }
        internal.m2 = m2;

        let table_start = *pos;
        let mut children: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(fanout);
        for i in 0..fanout {
            let entry_pos = table_start + i * 9;
            let _fileno = buf[entry_pos];
            let offset = i64::from_le_bytes(
                buf[entry_pos + 1..entry_pos + 9]
                    .try_into()
                    .map_err(|_| MVPError::NoWrite)?,
            ) as usize;
            if offset == 0 {
                children.push(Rc::new(RefCell::new(Node::Leaf(LeafNode::new(
                    tree.leaf_capacity as u32,
                )))));
            } else {
                let mut child_pos = offset;
                let child = read_node_recursive(buf, &mut child_pos, tree)?;
                children.push(Rc::new(RefCell::new(child)));
            }
        }
        internal.child_nodes = children;
        *pos = table_start + fanout * 9;
        Ok(Node::Internal(internal))
    } else {
        Err(MVPError::Unrecognized)
    }
}

// ---------------- MVPTree impl ----------------

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

    pub fn add(&mut self, mut points: Vec<MVPDatapoint>) -> MVPError {
        if points.is_empty() {
            return MVPError::Success;
        }
        // Type check
        if points[0].data_type != self.datatype {
            // If the tree is "empty" treat first add as setting type. But our
            // datatype is always set in new(), so just check equality.
            return MVPError::TypeMismatch;
        }
        // Reset path arrays for incoming points
        for p in &mut points {
            p.path = vec![0.0; self.path_length];
        }

        let node_opt = self.node.take();
        let result: Result<Node, MVPError> = match node_opt {
            None => build_subtree(self, points, 0),
            Some(node_rc) => {
                let node = match Rc::try_unwrap(node_rc) {
                    Ok(refcell) => refcell.into_inner(),
                    Err(rc) => {
                        // Should not happen since we just took it. But handle anyway:
                        // We need ownership. As a fallback, re-store and bail.
                        self.node = Some(rc);
                        return MVPError::ArgErr;
                    }
                };
                add_to_existing_node(self, node, points, 0)
            }
        };

        match result {
            Ok(new_node) => {
                self.node = Some(Rc::new(RefCell::new(new_node)));
                MVPError::Success
            }
            Err(e) => e,
        }
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
        let node_rc = match &self.node {
            Some(n) => n.clone(),
            None => return Err(MVPError::EmptyTree),
        };
        let mut target_local = target.clone();
        target_local.path = vec![0.0; self.path_length];
        let mut results: Vec<Arc<MVPDatapoint>> = Vec::new();
        let res = retrieve_recursive(self, &node_rc, &mut target_local, radius, knearest, &mut results, 0);
        match res {
            Ok(()) | Err(MVPError::KNearestCap) => {
                Ok(results.into_iter().map(|a| (*a).clone()).collect())
            }
            Err(e) => Err(e),
        }
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        let node_rc = match &self.node {
            Some(n) => n.clone(),
            None => return MVPError::ArgErr,
        };

        let mut buf: Vec<u8> = Vec::new();
        // Write header (32 bytes)
        // tag with null terminator
        let tag_bytes = TAG.as_bytes();
        buf.extend_from_slice(tag_bytes);
        buf.push(0u8);
        // version (4 bytes)
        write_u32_le(&mut buf, VERSION);
        // bf, pl, lc, ht
        buf.push(self.branch_factor as u8);
        buf.push(self.path_length as u8);
        buf.push(self.leaf_capacity as u8);
        let ht = match &*node_rc.borrow() {
            Node::Leaf(l) => l
                .sv1
                .as_ref()
                .map(|a| a.data_type as u8)
                .unwrap_or(self.datatype as u8),
            Node::Internal(i) => i
                .sv1
                .as_ref()
                .map(|a| a.data_type as u8)
                .unwrap_or(self.datatype as u8),
        };
        buf.push(ht);
        // Pad to HEADER_SIZE
        if buf.len() < HEADER_SIZE {
            buf.resize(HEADER_SIZE, 0);
        } else if buf.len() > HEADER_SIZE {
            // Truncate header back to HEADER_SIZE? Should not happen since
            // tag(13) + version(4) + 4 = 21 <= 32.
        }

        // Write nodes starting at HEADER_SIZE
        if let Err(e) = write_node_recursive(&mut buf, self, &node_rc) {
            return e;
        }

        // Write to file. Ensure readable mode bits are set, since Rust does not
        // parse C-style octal literals (e.g. `00755` is decimal 755 in Rust),
        // which can result in non-readable file permissions.
        let safe_mode = (mode as u32) | 0o600;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(safe_mode)
            .open(filename);
        let mut file = match file {
            Ok(f) => f,
            Err(_) => return MVPError::FileOpen,
        };
        if file.write_all(&buf).is_err() {
            return MVPError::NoWrite;
        }
        MVPError::Success
    }

    pub fn print(&self, _stream: &mut dyn Write) -> MVPError {
        MVPError::Success
    }

    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        self.size += self.pgsize;
        0
    }
}

pub fn mvptree_read(
    filename: &str,
    distance_function: DistanceFunction,
) -> Result<MVPTree, MVPError> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => return Err(MVPError::FileNotFound),
    };
    let mut buf: Vec<u8> = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return Err(MVPError::NoWrite);
    }
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::NoWrite);
    }
    let tag_bytes = TAG.as_bytes();
    let mut pos = 0usize;
    // Read tag (tag.len() + 1 bytes)
    pos += tag_bytes.len() + 1;
    // Read version
    pos += 4;
    let bf = buf[pos] as usize;
    pos += 1;
    let pl = buf[pos] as usize;
    pos += 1;
    let lc = buf[pos] as usize;
    pos += 1;
    let ht = buf[pos];
    pos += 1;
    let datatype = MVPDataType::from_u8(ht).unwrap_or(MVPDataType::ByteArray);

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);
    pos = HEADER_SIZE;
    let node = read_node_recursive(&buf, &mut pos, &tree)?;
    tree.node = Some(Rc::new(RefCell::new(node)));
    Ok(tree)
}

// ---------------- MVPDatapoint impl ----------------

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
