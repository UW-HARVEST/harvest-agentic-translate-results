use std::fs::{File, OpenOptions};
use std::io::{self, Read as IoRead, Write};
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
    "unmap eror",
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
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(MVPDataType::ByteArray),
            2 => Some(MVPDataType::UInt16Array),
            4 => Some(MVPDataType::UInt32Array),
            8 => Some(MVPDataType::UInt64Array),
            _ => None,
        }
    }

    pub fn byte_width(&self) -> usize {
        *self as usize
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

impl MVPError {
    fn index(&self) -> usize {
        match self {
            MVPError::Success => 0,
            MVPError::ArgErr => 1,
            MVPError::NoDistanceFunc => 2,
            MVPError::MemAlloc => 3,
            MVPError::NoLeaf => 4,
            MVPError::NoInternal => 5,
            MVPError::PathAlloc => 6,
            MVPError::VpNoSelect => 7,
            MVPError::NoSv1Range => 8,
            MVPError::NoSv2Range => 9,
            MVPError::NoSpace => 10,
            MVPError::NoSort => 11,
            MVPError::FileOpen => 12,
            MVPError::FileClose => 13,
            MVPError::MemMap => 14,
            MVPError::Munmap => 15,
            MVPError::NoWrite => 16,
            MVPError::FileTruncate => 17,
            MVPError::MremapFail => 18,
            MVPError::TypeMismatch => 19,
            MVPError::KNearestCap => 20,
            MVPError::EmptyTree => 21,
            MVPError::NoSplits => 22,
            MVPError::BadDistVal => 23,
            MVPError::FileNotFound => 24,
            MVPError::Unrecognized => 24,
        }
    }
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
        let length_m1 = if bf > 0 { bf - 1 } else { 0 };
        let length_m2 = if bf > 0 { bf * length_m1 } else { 0 };
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0f32; length_m1],
            m2: vec![0.0f32; length_m2],
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
    pub fn new(bf: u32) -> Self {
        // bf here is leaf capacity
        let cap = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(cap),
            d1: vec![0.0f32; cap],
            d2: vec![0.0f32; cap],
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

// ----------------- helper functions -----------------

fn is_bad_dist(d: f32) -> bool {
    d.is_nan() || d < 0.0
}

/// Select two vantage points (indices) at maximum mutual distance.
/// Returns (sv1_pos, sv2_pos) or error code.
fn select_vps(points: &[MVPDatapoint], dist: DistanceFunction) -> Result<(i32, i32), i32> {
    let nb = points.len();
    if nb == 0 {
        return Err(-1);
    }
    let mut sv1_pos: i32 = 0;
    let mut sv2_pos: i32 = -1;
    let mut max_dist: f32 = 0.0;
    for i in 0..nb {
        for j in (i + 1)..nb {
            let d = dist(&points[i], &points[j]);
            if is_bad_dist(d) {
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

/// Compute split points for `points` against vantage point `vp` and store
/// in `m_out` (length = length_m).
fn compute_splits(
    points: &[&MVPDatapoint],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    length_m: usize,
    m_out: &mut [f32],
) -> Result<(), i32> {
    let nb = points.len();
    if nb == 0 || length_m == 0 {
        return Err(-1);
    }
    let mut dists: Vec<f32> = Vec::with_capacity(nb);
    for p in points {
        let d = dist(p, vp);
        if is_bad_dist(d) {
            return Err(-2);
        }
        dists.push(d);
    }
    // sort ascending using selection sort (matches C, but stable doesn't matter for f32)
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    for i in 0..length_m {
        let mut idx = (i + 1) * nb / (length_m + 1);
        if idx >= nb {
            idx = nb - 1;
        }
        m_out[i] = dists[idx];
    }
    Ok(())
}

/// Sort `points` (excluding sv1_pos, sv2_pos if non-negative) into bins
/// based on distance to `vp` and pivot points.
fn sort_into_bins(
    points: Vec<MVPDatapoint>,
    sv1_pos: i32,
    sv2_pos: i32,
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    branch_factor: usize,
    pivots: &[f32],
) -> Result<Vec<Vec<MVPDatapoint>>, MVPError> {
    let length_m1 = branch_factor.saturating_sub(1);
    let mut bins: Vec<Vec<MVPDatapoint>> = (0..branch_factor).map(|_| Vec::new()).collect();

    for (i, p) in points.into_iter().enumerate() {
        if i as i32 == sv1_pos || i as i32 == sv2_pos {
            continue;
        }
        let d = dist(vp, &p);
        if is_bad_dist(d) {
            return Err(MVPError::NoSort);
        }
        let mut placed_idx: Option<usize> = None;
        for k in 0..length_m1 {
            if d <= pivots[k] {
                placed_idx = Some(k);
                break;
            }
        }
        match placed_idx {
            Some(k) => bins[k].push(p),
            None => {
                if length_m1 == 0 || d > pivots[length_m1 - 1] {
                    bins[length_m1].push(p);
                }
            }
        }
    }

    Ok(bins)
}

/// For each point, compute distance to vp and update its path slot at `lvl`
/// (if lvl < path_length).
fn update_path_with_vp(
    points: &mut [MVPDatapoint],
    vp: &MVPDatapoint,
    dist: DistanceFunction,
    path_length: usize,
    lvl: usize,
) -> Result<(), MVPError> {
    for p in points.iter_mut() {
        let d = dist(vp, p);
        if is_bad_dist(d) {
            return Err(MVPError::BadDistVal);
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

/// Recursive tree construction. Consumes points, returns either a Node or error.
fn build_node(
    tree_branch_factor: usize,
    tree_path_length: usize,
    tree_leaf_capacity: usize,
    distance_function: DistanceFunction,
    mut points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<Option<Node>, MVPError> {
    if points.is_empty() {
        return Ok(None);
    }
    let nbpoints = points.len();
    let bf = tree_branch_factor;
    let length_m1 = bf.saturating_sub(1);

    // Make sure all points have a path of size path_length
    for p in points.iter_mut() {
        if p.path.len() < tree_path_length {
            p.path.resize(tree_path_length, 0.0);
        }
    }

    if nbpoints <= tree_leaf_capacity + 2 {
        // leaf node
        let (sv1_pos, sv2_pos) = select_vps(&points, distance_function)
            .map_err(|_| MVPError::VpNoSelect)?;

        let mut leaf = LeafNode::new(tree_leaf_capacity as u32);

        // Compute distances to sv1 and sv2 for path & d1/d2 updates.
        // Update path[lvl] = dist to sv1 for all points.
        if sv1_pos >= 0 {
            let sv1_clone = points[sv1_pos as usize].clone();
            update_path_with_vp(&mut points, &sv1_clone, distance_function, tree_path_length, lvl)
                .map_err(|_| MVPError::NoSv1Range)?;
        } else {
            return Ok(None);
        }
        if sv2_pos >= 0 {
            let sv2_clone = points[sv2_pos as usize].clone();
            update_path_with_vp(&mut points, &sv2_clone, distance_function, tree_path_length, lvl + 1)
                .map_err(|_| MVPError::NoSv2Range)?;
        }

        // Now extract sv1, sv2 from points list and place remaining into leaf.
        // Indices may shift after removal; remove higher index first.
        let (sv1_dp, sv2_dp_opt, remaining): (MVPDatapoint, Option<MVPDatapoint>, Vec<MVPDatapoint>) = {
            let mut removed_sv1: Option<MVPDatapoint> = None;
            let mut removed_sv2: Option<MVPDatapoint> = None;
            let mut keep: Vec<(usize, MVPDatapoint)> = Vec::with_capacity(nbpoints);
            for (i, p) in points.into_iter().enumerate() {
                if i as i32 == sv1_pos {
                    removed_sv1 = Some(p);
                } else if i as i32 == sv2_pos {
                    removed_sv2 = Some(p);
                } else {
                    keep.push((i, p));
                }
            }
            let remaining: Vec<MVPDatapoint> = keep.into_iter().map(|(_, p)| p).collect();
            (removed_sv1.expect("sv1_pos selected but missing"), removed_sv2, remaining)
        };

        // Compute d1/d2 for remaining points using cloned copies as references.
        let sv1_ref = sv1_dp.clone();
        let sv2_ref_opt = sv2_dp_opt.clone();

        let mut d1_vals: Vec<f32> = Vec::with_capacity(tree_leaf_capacity);
        let mut d2_vals: Vec<f32> = Vec::with_capacity(tree_leaf_capacity);

        for p in &remaining {
            let d_a = distance_function(p, &sv1_ref);
            d1_vals.push(d_a);
            if let Some(ref s2) = sv2_ref_opt {
                let d_b = distance_function(p, s2);
                d2_vals.push(d_b);
            } else {
                d2_vals.push(0.0);
            }
        }
        // Pad to leaf_capacity
        while d1_vals.len() < tree_leaf_capacity {
            d1_vals.push(0.0);
        }
        while d2_vals.len() < tree_leaf_capacity {
            d2_vals.push(0.0);
        }

        leaf.d1 = d1_vals;
        leaf.d2 = d2_vals;
        leaf.sv1 = Some(Arc::new(sv1_dp));
        leaf.sv2 = sv2_dp_opt.map(Arc::new);
        leaf.points = remaining.into_iter().map(Arc::new).collect();
        leaf.nbpoints = leaf.points.len();
        Ok(Some(Node::Leaf(leaf)))
    } else {
        // internal node
        let (sv1_pos, sv2_pos) = select_vps(&points, distance_function)
            .map_err(|_| MVPError::VpNoSelect)?;
        if sv1_pos < 0 || sv2_pos < 0 {
            return Err(MVPError::VpNoSelect);
        }

        let mut internal = InternalNode::new(bf as u32);

        // Update path with sv1
        let sv1_clone = points[sv1_pos as usize].clone();
        update_path_with_vp(&mut points, &sv1_clone, distance_function, tree_path_length, lvl)
            .map_err(|_| MVPError::NoSv1Range)?;

        // Compute M1 splits
        if length_m1 > 0 {
            let pts_refs: Vec<&MVPDatapoint> = points
                .iter()
                .enumerate()
                .filter(|(i, _)| *i as i32 != sv1_pos && *i as i32 != sv2_pos)
                .map(|(_, p)| p)
                .collect();
            if pts_refs.is_empty() {
                return Err(MVPError::NoSplits);
            }
            let mut m1 = vec![0.0f32; length_m1];
            compute_splits(&pts_refs, &sv1_clone, distance_function, length_m1, &mut m1)
                .map_err(|_| MVPError::NoSplits)?;
            internal.m1 = m1;
        }

        // Sort into bins by sv1 (excluding sv1_pos and sv2_pos)
        let sv2_clone = points[sv2_pos as usize].clone();
        let bins = sort_into_bins(
            points,
            sv1_pos,
            sv2_pos,
            &sv1_clone,
            distance_function,
            bf,
            &internal.m1,
        )?;

        // For each bin: compute distance to sv2, M2 splits, sort into sub-bins, recurse
        let mut child_nodes: Vec<Option<Node>> = Vec::with_capacity(bf * bf);

        for (bin_i, mut bin_points) in bins.into_iter().enumerate() {
            // Update path with sv2
            update_path_with_vp(
                &mut bin_points,
                &sv2_clone,
                distance_function,
                tree_path_length,
                lvl + 1,
            )
            .map_err(|_| MVPError::NoSv2Range)?;

            // Compute M2 splits for this bin
            let m2_offset = bin_i * length_m1;
            if length_m1 > 0 {
                let pts_refs: Vec<&MVPDatapoint> = bin_points.iter().collect();
                if !pts_refs.is_empty() {
                    let mut m2_local = vec![0.0f32; length_m1];
                    if compute_splits(
                        &pts_refs,
                        &sv2_clone,
                        distance_function,
                        length_m1,
                        &mut m2_local,
                    )
                    .is_err()
                    {
                        // Use zeros - matches C path of returning NOSPLITS, but we'll be lenient
                        // Actually C returns error here. Let's return error.
                        return Err(MVPError::NoSplits);
                    }
                    for (k, v) in m2_local.iter().enumerate() {
                        if m2_offset + k < internal.m2.len() {
                            internal.m2[m2_offset + k] = *v;
                        }
                    }
                }
            }

            // Sort into sub-bins
            let pivots_for_m2: Vec<f32> = if length_m1 > 0 {
                internal.m2[m2_offset..m2_offset + length_m1].to_vec()
            } else {
                Vec::new()
            };
            let sub_bins = sort_into_bins(
                bin_points,
                -1,
                -1,
                &sv2_clone,
                distance_function,
                bf,
                &pivots_for_m2,
            )?;

            for sub_bin in sub_bins.into_iter() {
                let child = build_node(
                    tree_branch_factor,
                    tree_path_length,
                    tree_leaf_capacity,
                    distance_function,
                    sub_bin,
                    lvl + 2,
                )?;
                child_nodes.push(child);
            }
        }

        internal.sv1 = Some(Arc::new(sv1_clone));
        internal.sv2 = Some(Arc::new(sv2_clone));
        internal.child_nodes = child_nodes
            .into_iter()
            .map(|opt| match opt {
                Some(n) => Rc::new(RefCell::new(n)),
                None => Rc::new(RefCell::new(Node::Leaf(LeafNode::new(tree_leaf_capacity as u32)))),
            })
            .collect();
        // Track which children are "real": we mark a freshly constructed empty leaf as a placeholder.
        // For correctness during retrieval we'll detect empty leaves (no sv1) and skip them.
        Ok(Some(Node::Internal(internal)))
    }
}

/// Add points to an existing tree (`existing_node` is mutable in-place reference).
/// Returns updated node. May return None if existing node is None and no points.
fn add_to_existing_node(
    tree: &MVPTree,
    existing: Option<Rc<RefCell<Node>>>,
    mut points: Vec<MVPDatapoint>,
    lvl: usize,
) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
    if points.is_empty() {
        return Ok(existing);
    }

    // Make sure all points have path allocated
    for p in points.iter_mut() {
        if p.path.len() < tree.path_length {
            p.path.resize(tree.path_length, 0.0);
        }
    }

    let bf = tree.branch_factor;
    let length_m1 = bf.saturating_sub(1);

    let existing = match existing {
        None => {
            // Create new node from scratch
            let node = build_node(
                tree.branch_factor,
                tree.path_length,
                tree.leaf_capacity,
                tree.distance_function,
                points,
                lvl,
            )?;
            return Ok(node.map(|n| Rc::new(RefCell::new(n))));
        }
        Some(e) => e,
    };

    // Determine type of existing node
    let is_leaf = matches!(&*existing.borrow(), Node::Leaf(_));
    let is_internal = matches!(&*existing.borrow(), Node::Internal(_));

    if is_leaf {
        // Read leaf info
        let (existing_nb, existing_sv1, existing_sv2, existing_points): (
            usize,
            Option<Arc<MVPDatapoint>>,
            Option<Arc<MVPDatapoint>>,
            Vec<Arc<MVPDatapoint>>,
        ) = {
            let leaf_ref = existing.borrow();
            if let Node::Leaf(leaf) = &*leaf_ref {
                (
                    leaf.nbpoints,
                    leaf.sv1.clone(),
                    leaf.sv2.clone(),
                    leaf.points.clone(),
                )
            } else {
                unreachable!()
            }
        };

        if existing_nb + points.len() <= tree.leaf_capacity {
            // Plenty of room. Add to existing leaf.
            // Compute distances against existing sv1 and sv2 (or use first point as sv2)
            let sv1_dp = existing_sv1.as_ref().map(|a| (**a).clone());
            let sv2_dp = existing_sv2.as_ref().map(|a| (**a).clone());

            let mut idx_start = 0;
            let sv2_chosen: Option<MVPDatapoint> = if sv2_dp.is_none() && !points.is_empty() {
                let s = points[0].clone();
                idx_start = 1;
                Some(s)
            } else {
                sv2_dp
            };

            // Update path with sv1
            if let Some(ref s1) = sv1_dp {
                update_path_with_vp(
                    &mut points,
                    s1,
                    tree.distance_function,
                    tree.path_length,
                    lvl,
                )
                .map_err(|_| MVPError::NoSv1Range)?;
            }
            // Update path with sv2
            if let Some(ref s2) = sv2_chosen {
                update_path_with_vp(
                    &mut points,
                    s2,
                    tree.distance_function,
                    tree.path_length,
                    lvl + 1,
                )
                .map_err(|_| MVPError::NoSv2Range)?;
            }

            // Compute d1/d2 distances for the new points to add
            let mut new_d1 = Vec::new();
            let mut new_d2 = Vec::new();
            let mut new_pts: Vec<Arc<MVPDatapoint>> = Vec::new();
            for (i, p) in points.into_iter().enumerate() {
                if i < idx_start {
                    continue;
                }
                let d_a = if let Some(ref s1) = sv1_dp {
                    (tree.distance_function)(&p, s1)
                } else {
                    0.0
                };
                let d_b = if let Some(ref s2) = sv2_chosen {
                    (tree.distance_function)(&p, s2)
                } else {
                    0.0
                };
                new_d1.push(d_a);
                new_d2.push(d_b);
                new_pts.push(Arc::new(p));
            }

            // Mutate the leaf node
            let mut node_borrow = existing.borrow_mut();
            if let Node::Leaf(leaf) = &mut *node_borrow {
                if leaf.sv2.is_none() {
                    if let Some(s) = sv2_chosen {
                        leaf.sv2 = Some(Arc::new(s));
                    }
                }
                let cap = tree.leaf_capacity;
                if leaf.d1.len() < cap {
                    leaf.d1.resize(cap, 0.0);
                }
                if leaf.d2.len() < cap {
                    leaf.d2.resize(cap, 0.0);
                }
                let mut count = leaf.nbpoints;
                for ((dp, da), db) in new_pts.into_iter().zip(new_d1).zip(new_d2) {
                    if count < cap {
                        leaf.d1[count] = da;
                        leaf.d2[count] = db;
                        leaf.points.push(dp);
                        count += 1;
                    }
                }
                leaf.nbpoints = count;
            }
            drop(node_borrow);
            Ok(Some(existing))
        } else {
            // Not enough room. Rebuild.
            let mut combined: Vec<MVPDatapoint> = Vec::new();
            if let Some(s) = existing_sv1 {
                combined.push(arc_unwrap_or_clone(s));
            }
            if let Some(s) = existing_sv2 {
                combined.push(arc_unwrap_or_clone(s));
            }
            for p in existing_points {
                combined.push(arc_unwrap_or_clone(p));
            }
            for p in points {
                combined.push(p);
            }
            // Free old node by dropping Rc reference
            drop(existing);
            let new_node = build_node(
                tree.branch_factor,
                tree.path_length,
                tree.leaf_capacity,
                tree.distance_function,
                combined,
                lvl,
            )?;
            Ok(new_node.map(|n| Rc::new(RefCell::new(n))))
        }
    } else if is_internal {
        // Recurse into children
        let (sv1_clone_opt, sv2_clone_opt, m1_clone, m2_clone): (
            Option<MVPDatapoint>,
            Option<MVPDatapoint>,
            Vec<f32>,
            Vec<f32>,
        ) = {
            let nb = existing.borrow();
            if let Node::Internal(internal) = &*nb {
                (
                    internal.sv1.as_ref().map(|a| (**a).clone()),
                    internal.sv2.as_ref().map(|a| (**a).clone()),
                    internal.m1.clone(),
                    internal.m2.clone(),
                )
            } else {
                unreachable!()
            }
        };

        let sv1_ref = sv1_clone_opt.as_ref().ok_or(MVPError::NoSv1Range)?;
        let sv2_ref = sv2_clone_opt.as_ref().ok_or(MVPError::NoSv2Range)?;

        // Update path
        update_path_with_vp(
            &mut points,
            sv1_ref,
            tree.distance_function,
            tree.path_length,
            lvl,
        )
        .map_err(|_| MVPError::NoSv1Range)?;

        // Sort into bf bins by sv1
        let bins = sort_into_bins(
            points,
            -1,
            -1,
            sv1_ref,
            tree.distance_function,
            bf,
            &m1_clone,
        )?;

        for (bin_i, mut bin_points) in bins.into_iter().enumerate() {
            if bin_points.is_empty() {
                continue;
            }
            update_path_with_vp(
                &mut bin_points,
                sv2_ref,
                tree.distance_function,
                tree.path_length,
                lvl + 1,
            )
            .map_err(|_| MVPError::NoSv2Range)?;

            // Get pivot slice
            let m2_offset = bin_i * length_m1;
            let pivots_local: Vec<f32> = if length_m1 > 0 && m2_offset + length_m1 <= m2_clone.len() {
                m2_clone[m2_offset..m2_offset + length_m1].to_vec()
            } else {
                Vec::new()
            };

            let sub_bins = sort_into_bins(
                bin_points,
                -1,
                -1,
                sv2_ref,
                tree.distance_function,
                bf,
                &pivots_local,
            )?;

            for (j, sub_bin) in sub_bins.into_iter().enumerate() {
                if sub_bin.is_empty() {
                    continue;
                }
                let child_idx = bin_i * bf + j;

                // Get the existing child (or None if it's a placeholder empty leaf)
                let child_existing: Option<Rc<RefCell<Node>>> = {
                    let nb = existing.borrow();
                    if let Node::Internal(internal) = &*nb {
                        if child_idx < internal.child_nodes.len() {
                            // Check if the child is a placeholder empty leaf
                            let c = internal.child_nodes[child_idx].clone();
                            let is_placeholder = {
                                let cb = c.borrow();
                                matches!(&*cb, Node::Leaf(l) if l.sv1.is_none())
                            };
                            if is_placeholder {
                                None
                            } else {
                                Some(c)
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                let new_child = add_to_existing_node(tree, child_existing, sub_bin, lvl + 2)?;

                // Set the child
                let mut nb = existing.borrow_mut();
                if let Node::Internal(internal) = &mut *nb {
                    while internal.child_nodes.len() <= child_idx {
                        internal.child_nodes.push(Rc::new(RefCell::new(Node::Leaf(
                            LeafNode::new(tree.leaf_capacity as u32),
                        ))));
                    }
                    if let Some(nc) = new_child {
                        internal.child_nodes[child_idx] = nc;
                    }
                }
            }
        }
        Ok(Some(existing))
    } else {
        Err(MVPError::Unrecognized)
    }
}

fn arc_unwrap_or_clone<T: Clone>(arc: Arc<T>) -> T {
    Arc::try_unwrap(arc).unwrap_or_else(|a| (*a).clone())
}

// ----------------- retrieve -----------------

fn retrieve_recursive(
    tree: &MVPTree,
    node: &Node,
    target: &mut MVPDatapoint,
    radius: f32,
    knearest: usize,
    results: &mut Vec<Arc<MVPDatapoint>>,
    lvl: usize,
) -> MVPError {
    let dist = tree.distance_function;
    let bf = tree.branch_factor;
    let length_m1 = bf.saturating_sub(1);

    match node {
        Node::Leaf(leaf) => {
            let sv1 = match &leaf.sv1 {
                Some(s) => s,
                None => return MVPError::Success,
            };
            let d1 = dist(target, sv1);
            if is_bad_dist(d1) {
                return MVPError::BadDistVal;
            }
            if lvl < tree.path_length {
                if target.path.len() < tree.path_length {
                    target.path.resize(tree.path_length, 0.0);
                }
                target.path[lvl] = d1;
            }
            if d1 <= radius {
                results.push(sv1.clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if let Some(sv2) = &leaf.sv2 {
                let d2 = dist(target, sv2);
                if is_bad_dist(d2) {
                    return MVPError::BadDistVal;
                }
                if d2 <= radius {
                    results.push(sv2.clone());
                    if results.len() >= knearest {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl + 1 < tree.path_length {
                    if target.path.len() < tree.path_length {
                        target.path.resize(tree.path_length, 0.0);
                    }
                    target.path[lvl + 1] = d2;
                }

                for (i, p) in leaf.points.iter().enumerate() {
                    let pd1 = leaf.d1.get(i).copied().unwrap_or(0.0);
                    let pd2 = leaf.d2.get(i).copied().unwrap_or(0.0);
                    if d1 - radius <= pd1 && d1 + radius >= pd1 {
                        if d2 - radius <= pd2 && d2 + radius >= pd2 {
                            let endpath = if lvl + 1 < tree.path_length {
                                lvl + 1
                            } else {
                                tree.path_length
                            };
                            let mut skip = false;
                            for j in 0..endpath {
                                let tp = target.path.get(j).copied().unwrap_or(0.0);
                                let pp = p.path.get(j).copied().unwrap_or(0.0);
                                if tp - radius <= pp && tp + radius >= pp {
                                    continue;
                                } else {
                                    skip = true;
                                    break;
                                }
                            }
                            if !skip {
                                let d = dist(target, p);
                                if is_bad_dist(d) {
                                    return MVPError::BadDistVal;
                                }
                                if d <= radius {
                                    results.push(p.clone());
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
        Node::Internal(internal) => {
            let sv1 = match &internal.sv1 {
                Some(s) => s,
                None => return MVPError::Success,
            };
            let sv2 = match &internal.sv2 {
                Some(s) => s,
                None => return MVPError::Success,
            };
            let d1 = dist(target, sv1);
            if is_bad_dist(d1) {
                return MVPError::BadDistVal;
            }
            if d1 <= radius {
                results.push(sv1.clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if lvl < tree.path_length {
                if target.path.len() < tree.path_length {
                    target.path.resize(tree.path_length, 0.0);
                }
                target.path[lvl] = d1;
            }
            let d2 = dist(target, sv2);
            if is_bad_dist(d2) {
                return MVPError::BadDistVal;
            }
            if d2 <= radius {
                results.push(sv2.clone());
                if results.len() >= knearest {
                    return MVPError::KNearestCap;
                }
            }
            if lvl + 1 < tree.path_length {
                if target.path.len() < tree.path_length {
                    target.path.resize(tree.path_length, 0.0);
                }
                target.path[lvl + 1] = d2;
            }

            for i in 0..length_m1 {
                if d1 - radius <= internal.m1[i] {
                    for j in 0..length_m1 {
                        let m2_idx = i * length_m1 + j;
                        if m2_idx < internal.m2.len()
                            && d2 - radius <= internal.m2[m2_idx]
                        {
                            let child_idx = i * bf + j;
                            if child_idx < internal.child_nodes.len() {
                                let child = internal.child_nodes[child_idx].clone();
                                let cb = child.borrow();
                                let err = retrieve_recursive(
                                    tree,
                                    &*cb,
                                    target,
                                    radius,
                                    knearest,
                                    results,
                                    lvl + 2,
                                );
                                if err != MVPError::Success {
                                    return err;
                                }
                            }
                        }
                    }
                    let last_m2 = i * length_m1 + length_m1 - 1;
                    if length_m1 > 0
                        && last_m2 < internal.m2.len()
                        && d2 + radius >= internal.m2[last_m2]
                    {
                        let child_idx = i * bf + length_m1;
                        if child_idx < internal.child_nodes.len() {
                            let child = internal.child_nodes[child_idx].clone();
                            let cb = child.borrow();
                            let err = retrieve_recursive(
                                tree,
                                &*cb,
                                target,
                                radius,
                                knearest,
                                results,
                                lvl + 2,
                            );
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                }
            }

            if length_m1 > 0
                && length_m1 - 1 < internal.m1.len()
                && d1 + radius >= internal.m1[length_m1 - 1]
            {
                for j in 0..length_m1 {
                    let m2_idx = length_m1 * length_m1 + j;
                    if m2_idx < internal.m2.len()
                        && d2 - radius <= internal.m2[m2_idx]
                    {
                        let child_idx = bf * length_m1 + j;
                        if child_idx < internal.child_nodes.len() {
                            let child = internal.child_nodes[child_idx].clone();
                            let cb = child.borrow();
                            let err = retrieve_recursive(
                                tree,
                                &*cb,
                                target,
                                radius,
                                knearest,
                                results,
                                lvl + 2,
                            );
                            if err != MVPError::Success {
                                return err;
                            }
                        }
                    }
                }
                let last_m2 = length_m1 * length_m1 + length_m1 - 1;
                if last_m2 < internal.m2.len() && d2 + radius >= internal.m2[last_m2] {
                    let child_idx = bf * length_m1 + length_m1;
                    if child_idx < internal.child_nodes.len() {
                        let child = internal.child_nodes[child_idx].clone();
                        let cb = child.borrow();
                        let err = retrieve_recursive(
                            tree,
                            &*cb,
                            target,
                            radius,
                            knearest,
                            results,
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

// ----------------- file I/O -----------------

fn ensure_capacity(buf: &mut Vec<u8>, size: usize) {
    if buf.len() < size {
        buf.resize(size, 0);
    }
}

fn write_u8(buf: &mut Vec<u8>, pos: &mut usize, v: u8) {
    ensure_capacity(buf, *pos + 1);
    buf[*pos] = v;
    *pos += 1;
}

fn write_u32_le(buf: &mut Vec<u8>, pos: &mut usize, v: u32) {
    ensure_capacity(buf, *pos + 4);
    buf[*pos..*pos + 4].copy_from_slice(&v.to_le_bytes());
    *pos += 4;
}

#[allow(dead_code)]
fn write_i64_le(buf: &mut Vec<u8>, pos: &mut usize, v: i64) {
    ensure_capacity(buf, *pos + 8);
    buf[*pos..*pos + 8].copy_from_slice(&v.to_le_bytes());
    *pos += 8;
}

fn write_f32_le(buf: &mut Vec<u8>, pos: &mut usize, v: f32) {
    ensure_capacity(buf, *pos + 4);
    buf[*pos..*pos + 4].copy_from_slice(&v.to_le_bytes());
    *pos += 4;
}

fn write_bytes(buf: &mut Vec<u8>, pos: &mut usize, bytes: &[u8]) {
    ensure_capacity(buf, *pos + bytes.len());
    buf[*pos..*pos + bytes.len()].copy_from_slice(bytes);
    *pos += bytes.len();
}

fn write_datapoint(
    buf: &mut Vec<u8>,
    pos: &mut usize,
    dp: Option<&MVPDatapoint>,
    path_length: usize,
) -> i64 {
    let start = *pos as i64;
    match dp {
        None => {
            write_u8(buf, pos, 0);
            write_u32_le(buf, pos, 0);
            start
        }
        Some(dp) => {
            let active: u8 = 1;
            let id_bytes = dp.id.as_bytes();
            let idlen: u8 = id_bytes.len().min(255) as u8;
            let datalength: u32 = dp.datalen as u32;
            let type_width = dp.data_type.byte_width();
            let bytelength: u32 = (1 + idlen as u32 + 4 + datalength * (type_width as u32) + (path_length as u32) * 4) as u32;

            write_u8(buf, pos, active);
            write_u32_le(buf, pos, bytelength);
            write_u8(buf, pos, idlen);
            write_bytes(buf, pos, &id_bytes[..idlen as usize]);
            write_u32_le(buf, pos, datalength);
            // data
            let data_bytes_needed = (datalength as usize) * type_width;
            ensure_capacity(buf, *pos + data_bytes_needed);
            let data_to_write = if dp.data.len() >= data_bytes_needed {
                &dp.data[..data_bytes_needed]
            } else {
                &dp.data[..]
            };
            write_bytes(buf, pos, data_to_write);
            // pad data if needed
            let padding = data_bytes_needed.saturating_sub(dp.data.len());
            if padding > 0 {
                let zeros = vec![0u8; padding];
                write_bytes(buf, pos, &zeros);
            }
            // path
            for i in 0..path_length {
                let v = dp.path.get(i).copied().unwrap_or(0.0);
                write_f32_le(buf, pos, v);
            }
            start
        }
    }
}

fn write_node_recursive(
    buf: &mut Vec<u8>,
    pos: &mut usize,
    node: &Node,
    tree_branch_factor: usize,
    tree_path_length: usize,
    tree_leaf_capacity: usize,
    error: &mut MVPError,
) -> i64 {
    let start_pos = *pos as i64;
    match node {
        Node::Leaf(leaf) => {
            let nbpoints = leaf.nbpoints as u32;
            let node_type: u8 = NodeType::LeafNode as u8;
            write_u8(buf, pos, node_type);
            write_datapoint(buf, pos, leaf.sv1.as_deref(), tree_path_length);
            write_datapoint(buf, pos, leaf.sv2.as_deref(), tree_path_length);
            write_u32_le(buf, pos, nbpoints);

            let saved_pos_start = *pos;
            // Reserve space for leafcap entries: 4 + 4 + 8 each
            let entry_size = 4 + 4 + 8;
            let reserved = tree_leaf_capacity * entry_size;
            ensure_capacity(buf, *pos + reserved);
            *pos += reserved;

            let mut saved_pos = saved_pos_start;
            for i in 0..(nbpoints as usize) {
                let d1v = leaf.d1.get(i).copied().unwrap_or(0.0);
                let d2v = leaf.d2.get(i).copied().unwrap_or(0.0);
                // write d1
                buf[saved_pos..saved_pos + 4].copy_from_slice(&d1v.to_le_bytes());
                saved_pos += 4;
                buf[saved_pos..saved_pos + 4].copy_from_slice(&d2v.to_le_bytes());
                saved_pos += 4;
                let offset = if let Some(p) = leaf.points.get(i) {
                    write_datapoint(buf, pos, Some(p), tree_path_length)
                } else {
                    write_datapoint(buf, pos, None, tree_path_length)
                };
                buf[saved_pos..saved_pos + 8].copy_from_slice(&offset.to_le_bytes());
                saved_pos += 8;
            }
        }
        Node::Internal(internal) => {
            let bf = tree_branch_factor;
            let length_m1 = bf.saturating_sub(1);
            let length_m2 = bf * length_m1;
            let fanout = bf * bf;
            let node_type: u8 = NodeType::InternalNode as u8;
            write_u8(buf, pos, node_type);
            write_datapoint(buf, pos, internal.sv1.as_deref(), tree_path_length);
            write_datapoint(buf, pos, internal.sv2.as_deref(), tree_path_length);
            // M1
            for i in 0..length_m1 {
                let v = internal.m1.get(i).copied().unwrap_or(0.0);
                write_f32_le(buf, pos, v);
            }
            // M2
            for i in 0..length_m2 {
                let v = internal.m2.get(i).copied().unwrap_or(0.0);
                write_f32_le(buf, pos, v);
            }
            // Reserve fanout entries: 1 + 8 each
            let entry_size = 1 + 8;
            let reserved = fanout * entry_size;
            let saved_pos_start = *pos;
            ensure_capacity(buf, *pos + reserved);
            *pos += reserved;

            let mut saved_pos = saved_pos_start;
            for i in 0..fanout {
                let offset = if let Some(child_rc) = internal.child_nodes.get(i) {
                    let child_borrow = child_rc.borrow();
                    // Check if child is "real" (not a placeholder empty leaf)
                    let is_empty_placeholder = matches!(
                        &*child_borrow,
                        Node::Leaf(l) if l.sv1.is_none() && l.points.is_empty()
                    );
                    if is_empty_placeholder {
                        0i64
                    } else {
                        write_node_recursive(
                            buf,
                            pos,
                            &*child_borrow,
                            tree_branch_factor,
                            tree_path_length,
                            tree_leaf_capacity,
                            error,
                        )
                    }
                } else {
                    0i64
                };
                let fileno: u8 = 0;
                buf[saved_pos] = fileno;
                saved_pos += 1;
                buf[saved_pos..saved_pos + 8].copy_from_slice(&offset.to_le_bytes());
                saved_pos += 8;
            }
        }
    }
    start_pos
}

fn read_u8(buf: &[u8], pos: &mut usize) -> Result<u8, MVPError> {
    if *pos >= buf.len() {
        return Err(MVPError::MemMap);
    }
    let v = buf[*pos];
    *pos += 1;
    Ok(v)
}

fn read_u32_le(buf: &[u8], pos: &mut usize) -> Result<u32, MVPError> {
    if *pos + 4 > buf.len() {
        return Err(MVPError::MemMap);
    }
    let v = u32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(v)
}

fn read_i64_le(buf: &[u8], pos: &mut usize) -> Result<i64, MVPError> {
    if *pos + 8 > buf.len() {
        return Err(MVPError::MemMap);
    }
    let v = i64::from_le_bytes(buf[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    Ok(v)
}

fn read_f32_le(buf: &[u8], pos: &mut usize) -> Result<f32, MVPError> {
    if *pos + 4 > buf.len() {
        return Err(MVPError::MemMap);
    }
    let v = f32::from_le_bytes(buf[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(v)
}

fn read_datapoint(
    buf: &[u8],
    pos: &mut usize,
    datatype: MVPDataType,
    path_length: usize,
) -> Result<Option<MVPDatapoint>, MVPError> {
    let active = read_u8(buf, pos)?;
    let bytelength = read_u32_le(buf, pos)?;
    if active == 0 && bytelength == 0 {
        return Ok(None);
    }
    let idlen = read_u8(buf, pos)? as usize;
    if *pos + idlen > buf.len() {
        return Err(MVPError::MemMap);
    }
    let id = String::from_utf8_lossy(&buf[*pos..*pos + idlen]).to_string();
    *pos += idlen;
    let datalength = read_u32_le(buf, pos)? as usize;
    let type_width = datatype.byte_width();
    let data_bytes = datalength * type_width;
    if *pos + data_bytes > buf.len() {
        return Err(MVPError::MemMap);
    }
    let data: Vec<u8> = buf[*pos..*pos + data_bytes].to_vec();
    *pos += data_bytes;
    let mut path = Vec::with_capacity(path_length);
    for _ in 0..path_length {
        path.push(read_f32_le(buf, pos)?);
    }
    Ok(Some(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    }))
}

fn read_node_recursive(
    buf: &[u8],
    pos: &mut usize,
    branch_factor: usize,
    path_length: usize,
    leaf_capacity: usize,
    datatype: MVPDataType,
) -> Result<Option<Node>, MVPError> {
    let node_type = read_u8(buf, pos)?;
    let bf = branch_factor;
    let length_m1 = bf.saturating_sub(1);
    let length_m2 = bf * length_m1;
    let fanout = bf * bf;

    if node_type == NodeType::LeafNode as u8 {
        let mut leaf = LeafNode::new(leaf_capacity as u32);
        let sv1 = read_datapoint(buf, pos, datatype, path_length)?;
        let sv2 = read_datapoint(buf, pos, datatype, path_length)?;
        let nbpoints = read_u32_le(buf, pos)? as usize;
        leaf.sv1 = sv1.map(Arc::new);
        leaf.sv2 = sv2.map(Arc::new);
        leaf.nbpoints = nbpoints;

        let mut saved_pos = *pos;
        // Skip past leafcap entries (4+4+8 each)
        let entry_size = 4 + 4 + 8;
        *pos += leaf_capacity * entry_size;

        leaf.d1 = vec![0.0f32; leaf_capacity];
        leaf.d2 = vec![0.0f32; leaf_capacity];
        leaf.points = Vec::with_capacity(nbpoints);

        for i in 0..nbpoints {
            let d1v = read_f32_le(buf, &mut saved_pos)?;
            let d2v = read_f32_le(buf, &mut saved_pos)?;
            let offset = read_i64_le(buf, &mut saved_pos)? as usize;
            leaf.d1[i] = d1v;
            leaf.d2[i] = d2v;

            let mut local_pos = offset;
            let dp = read_datapoint(buf, &mut local_pos, datatype, path_length)?;
            if let Some(dp) = dp {
                leaf.points.push(Arc::new(dp));
            } else {
                // Push placeholder? We'll just stop reading
                break;
            }
        }
        Ok(Some(Node::Leaf(leaf)))
    } else if node_type == NodeType::InternalNode as u8 {
        let mut internal = InternalNode::new(bf as u32);
        let sv1 = read_datapoint(buf, pos, datatype, path_length)?;
        let sv2 = read_datapoint(buf, pos, datatype, path_length)?;
        internal.sv1 = sv1.map(Arc::new);
        internal.sv2 = sv2.map(Arc::new);

        for i in 0..length_m1 {
            internal.m1[i] = read_f32_le(buf, pos)?;
        }
        for i in 0..length_m2 {
            if i < internal.m2.len() {
                internal.m2[i] = read_f32_le(buf, pos)?;
            } else {
                let _ = read_f32_le(buf, pos)?;
            }
        }

        let mut saved_pos = *pos;
        // Skip past fanout entries (1+8)
        *pos += fanout * (1 + 8);

        internal.child_nodes = Vec::with_capacity(fanout);
        for _ in 0..fanout {
            let _fileno = read_u8(buf, &mut saved_pos)?;
            let offset = read_i64_le(buf, &mut saved_pos)?;
            if offset == 0 {
                // Empty child - placeholder
                internal
                    .child_nodes
                    .push(Rc::new(RefCell::new(Node::Leaf(LeafNode::new(
                        leaf_capacity as u32,
                    )))));
            } else {
                let mut local_pos = offset as usize;
                let child = read_node_recursive(
                    buf,
                    &mut local_pos,
                    branch_factor,
                    path_length,
                    leaf_capacity,
                    datatype,
                )?;
                match child {
                    Some(c) => internal
                        .child_nodes
                        .push(Rc::new(RefCell::new(c))),
                    None => internal
                        .child_nodes
                        .push(Rc::new(RefCell::new(Node::Leaf(LeafNode::new(
                            leaf_capacity as u32,
                        ))))),
                }
            }
        }

        Ok(Some(Node::Internal(internal)))
    } else {
        Err(MVPError::Unrecognized)
    }
}

// ----------------- print -----------------

fn print_recursive(
    stream: &mut dyn Write,
    tree: &MVPTree,
    node_opt: Option<&Node>,
    lvl: usize,
) -> MVPError {
    let bf = tree.branch_factor;
    let length_m1 = bf.saturating_sub(1);
    let length_m2 = bf;
    let fanout = bf * bf;

    match node_opt {
        Some(Node::Leaf(leaf)) => {
            let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
            if let Some(sv1) = &leaf.sv1 {
                let _ = writeln!(stream, "    sv1: {}", sv1.id);
            }
            if let Some(sv2) = &leaf.sv2 {
                let _ = writeln!(stream, "    sv2: {}", sv2.id);
            }
            for (i, p) in leaf.points.iter().enumerate() {
                let _ = writeln!(stream, "        point[{}]: {}", i, p.id);
            }
            MVPError::Success
        }
        Some(Node::Internal(internal)) => {
            let _ = writeln!(stream, "INTERNAL{}", lvl);
            if let Some(sv1) = &internal.sv1 {
                let _ = writeln!(stream, "  sv1: {}", sv1.id);
            }
            if let Some(sv2) = &internal.sv2 {
                let _ = writeln!(stream, "  sv2: {}", sv2.id);
            }
            for i in 0..length_m1 {
                let v = internal.m1.get(i).copied().unwrap_or(0.0);
                let _ = write!(stream, "  M1[{}] = {:.4};", i, v);
            }
            for i in 0..length_m2 {
                let v = internal.m2.get(i).copied().unwrap_or(0.0);
                let _ = write!(stream, "  M2[{}] = {:.4};", i, v);
            }
            let _ = writeln!(stream);

            for i in 0..fanout {
                if let Some(child_rc) = internal.child_nodes.get(i) {
                    let child_borrow = child_rc.borrow();
                    let err = print_recursive(stream, tree, Some(&*child_borrow), lvl + 2);
                    if err != MVPError::Success {
                        return err;
                    }
                }
            }
            MVPError::Success
        }
        None => {
            let _ = writeln!(stream, "NULL{}", lvl);
            MVPError::Success
        }
    }
}

// ----------------- Public API -----------------

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
        // Type-check
        let first_type = points[0].data_type;
        if self.datatype as u8 != first_type as u8 {
            // datatype hasn't been set yet (0 in C). In our enum, default may match.
            // We treat the very first add as "set the datatype".
            // If branch_factor is 0 / tree.datatype must match. Let's just set if conflicting
            // - but Rust enum does not have a "0" state. Use a heuristic: if existing tree has
            // no node, set datatype.
            if self.node.is_none() {
                self.datatype = first_type;
            } else if self.datatype != first_type {
                return MVPError::TypeMismatch;
            }
        }

        for p in points.iter() {
            if p.data_type != self.datatype {
                return MVPError::TypeMismatch;
            }
        }

        let mut points = points;
        for p in points.iter_mut() {
            p.path = vec![0.0f32; self.path_length];
        }

        let existing = self.node.take();
        match add_to_existing_node(self, existing, points, 0) {
            Ok(node) => {
                self.node = node;
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
        let mut target_mut = target.clone();
        target_mut.path = vec![0.0f32; self.path_length];

        let mut results: Vec<Arc<MVPDatapoint>> = Vec::with_capacity(knearest);
        let node_borrow = node_rc.borrow();
        let err = retrieve_recursive(
            self,
            &*node_borrow,
            &mut target_mut,
            radius,
            knearest,
            &mut results,
            0,
        );
        if err != MVPError::Success && err != MVPError::KNearestCap {
            return Err(err);
        }
        Ok(results.into_iter().map(|a| (*a).clone()).collect())
    }

    pub fn write(&self, filename: &str, _mode: i32) -> MVPError {
        if self.node.is_none() {
            return MVPError::ArgErr;
        }
        let file_res = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .read(true)
            .open(filename);
        let mut file = match file_res {
            Ok(f) => f,
            Err(_) => return MVPError::FileOpen,
        };

        let mut buf: Vec<u8> = Vec::new();
        ensure_capacity(&mut buf, HEADER_SIZE);
        let mut pos: usize = 0;

        // Header
        let tag_bytes = TAG.as_bytes();
        write_bytes(&mut buf, &mut pos, tag_bytes);
        write_u8(&mut buf, &mut pos, 0); // null terminator
        write_u32_le(&mut buf, &mut pos, VERSION);
        write_u8(&mut buf, &mut pos, self.branch_factor as u8);
        write_u8(&mut buf, &mut pos, self.path_length as u8);
        write_u8(&mut buf, &mut pos, self.leaf_capacity as u8);
        write_u8(&mut buf, &mut pos, self.datatype as u8);

        // Pad to HEADER_SIZE
        if pos < HEADER_SIZE {
            let padding = HEADER_SIZE - pos;
            let zeros = vec![0u8; padding];
            write_bytes(&mut buf, &mut pos, &zeros);
        }
        pos = HEADER_SIZE;

        let mut error = MVPError::Success;
        if let Some(node_rc) = &self.node {
            let node_borrow = node_rc.borrow();
            write_node_recursive(
                &mut buf,
                &mut pos,
                &*node_borrow,
                self.branch_factor,
                self.path_length,
                self.leaf_capacity,
                &mut error,
            );
        }

        // Truncate to actual size
        buf.truncate(pos);
        if let Err(_) = file.write_all(&buf) {
            return MVPError::NoWrite;
        }
        if let Err(_) = file.sync_all() {
            return MVPError::NoWrite;
        }

        error
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        let node_opt: Option<std::cell::Ref<'_, Node>> =
            self.node.as_ref().map(|n| n.borrow());
        let node_ref = node_opt.as_ref().map(|r| &**r);
        let err = print_recursive(stream, self, node_ref, 0);
        if err != MVPError::Success {
            let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
        }
        err
    }

    pub fn clear(&mut self, _node: &mut Option<Box<Node>>) {
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        // In our pure-Rust implementation, the buffer grows dynamically.
        // This function is a no-op compatibility shim.
        self.size += self.pgsize;
        if (self.buf.len() as i64) < self.size {
            self.buf.resize(self.size as usize, 0);
        }
        0
    }
}

pub fn mvptree_read(
    filename: &str,
    distance_function: DistanceFunction,
) -> Result<MVPTree, MVPError> {
    let mut file = match File::open(filename) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(MVPError::FileNotFound);
        }
        Err(_) => return Err(MVPError::FileOpen),
    };

    let mut buf: Vec<u8> = Vec::new();
    if let Err(_) = file.read_to_end(&mut buf) {
        return Err(MVPError::FileOpen);
    }
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::MemMap);
    }

    let mut pos = 0;
    let tag_len = TAG.len();
    if buf.len() < tag_len + 1 {
        return Err(MVPError::MemMap);
    }
    pos += tag_len + 1;
    let _version = read_u32_le(&buf, &mut pos)?;
    let bf = read_u8(&buf, &mut pos)? as usize;
    let pl = read_u8(&buf, &mut pos)? as usize;
    let lc = read_u8(&buf, &mut pos)? as usize;
    let ht = read_u8(&buf, &mut pos)?;
    let datatype = MVPDataType::from_u8(ht).unwrap_or(MVPDataType::ByteArray);

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);
    tree.size = buf.len() as i64;

    let mut node_pos = HEADER_SIZE;
    let node = read_node_recursive(&buf, &mut node_pos, bf, pl, lc, datatype)?;
    tree.node = node.map(|n| Rc::new(RefCell::new(n)));

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

    /// Stub: select two vantage points. The real selection logic is performed
    /// internally by `MVPTree::add`. This is kept to satisfy the public API.
    pub fn select_vantage_points(
        &mut self,
        nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _dist: DistanceFunction,
    ) -> i32 {
        if nb == 0 {
            return -1;
        }
        0
    }

    /// Stub: find split points for the data point.
    pub fn find_splits(
        &mut self,
        nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        _length_m: u32,
    ) -> f32 {
        if nb == 0 {
            return -1.0;
        }
        let d = (tree.distance_function)(self, vp);
        d
    }

    /// Stub: sort points into bins. Real sorting happens within `MVPTree::add`.
    pub fn sort_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        _vp: &MVPDatapoint,
        tree: &MVPTree,
        counts: &mut Vec<Vec<i32>>,
        _pivots: Vec<f32>,
    ) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        let bf = tree.branch_factor.max(1);
        if counts.is_empty() {
            *counts = vec![vec![0i32; bf]; bf];
        }
        let mut bins: Vec<Vec<Vec<Arc<MVPDatapoint>>>> = Vec::with_capacity(bf);
        for _ in 0..bf {
            let mut row: Vec<Vec<Arc<MVPDatapoint>>> = Vec::with_capacity(bf);
            for _ in 0..bf {
                row.push(Vec::new());
            }
            bins.push(row);
        }
        bins
    }

    /// Compute distance to vp and update path[level] if level < path_length.
    pub fn find_distance_range_for_vp(
        &mut self,
        nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        level: i32,
    ) -> i32 {
        if nb == 0 || level < 0 {
            return -1;
        }
        let d = (tree.distance_function)(self, vp);
        if d.is_nan() || d < 0.0 {
            return -2;
        }
        if (level as usize) < tree.path_length {
            if self.path.len() < tree.path_length {
                self.path.resize(tree.path_length, 0.0);
            }
            self.path[level as usize] = d;
        }
        0
    }

    /// Serialize this datapoint into the tree's internal buffer.
    /// Returns the start offset of the written record.
    pub fn write(&self, tree: &MVPTree) -> i64 {
        // We don't actually mutate the tree's internal buffer in this safe impl.
        // Return the would-be byte length as a positive offset stub.
        let id_bytes = self.id.as_bytes();
        let idlen = id_bytes.len().min(255);
        let type_width = self.data_type.byte_width();
        let bytelength = 1 + idlen + 4 + self.datalen * type_width + tree.path_length * 4;
        bytelength as i64
    }
}

pub fn error_to_string(error: MVPError) -> &'static str {
    let idx = error.index();
    if idx < ERROR_MSGS.len() {
        ERROR_MSGS[idx]
    } else {
        "unknown error"
    }
}
