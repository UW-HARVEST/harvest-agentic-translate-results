use std::fs::File;
use std::io::{self, Read as IoRead, Write};
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
impl InternalNode {
    pub fn new(bf: u32) -> Self {
        let bf_us = bf as usize;
        let length_m1 = bf_us.saturating_sub(1);
        let mut child_nodes: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(bf_us * bf_us);
        for _ in 0..bf_us * bf_us {
            child_nodes.push(make_null_leaf_rc());
        }
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; length_m1],
            m2: vec![0.0; bf_us * length_m1],
            child_nodes,
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
        let cap = bf as usize;
        LeafNode {
            node_type: NodeType::LeafNode,
            sv1: None,
            sv2: None,
            points: Vec::with_capacity(cap),
            d1: Vec::with_capacity(cap),
            d2: Vec::with_capacity(cap),
            nbpoints: 0,
        }
    }
}

fn make_null_leaf() -> LeafNode {
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

fn make_null_leaf_rc() -> Rc<RefCell<Node>> {
    Rc::new(RefCell::new(Node::Leaf(make_null_leaf())))
}

fn is_null_leaf_node(n: &Node) -> bool {
    match n {
        Node::Leaf(l) => l.sv1.is_none() && l.sv2.is_none() && l.nbpoints == 0,
        _ => false,
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

fn check_dist(d: f32) -> bool {
    !d.is_nan() && d >= 0.0
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

    fn select_vp(&self, points: &[MVPDatapoint]) -> Result<(i32, i32), ()> {
        let nb = points.len();
        if nb == 0 {
            return Err(());
        }
        let dist = self.distance_function;
        let mut sv1_pos: i32 = if nb >= 1 { 0 } else { -1 };
        let mut sv2_pos: i32 = -1;
        let mut max_dist: f32 = 0.0;
        for i in 0..nb {
            for j in (i + 1)..nb {
                let d = dist(&points[i], &points[j]);
                if !check_dist(d) {
                    return Err(());
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

    fn find_splits_for(&self, points: &[MVPDatapoint], vp: &MVPDatapoint, length_m: usize) -> Result<Vec<f32>, ()> {
        let nb = points.len();
        if nb == 0 || length_m == 0 {
            return Err(());
        }
        let dist = self.distance_function;
        let mut dists: Vec<f32> = Vec::with_capacity(nb);
        for p in points {
            let d = dist(p, vp);
            if !check_dist(d) {
                return Err(());
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
        Ok(m)
    }

    fn create_leaf_from_points(
        &self,
        mut points: Vec<MVPDatapoint>,
        lvl: usize,
        error: &mut MVPError,
    ) -> Option<Rc<RefCell<Node>>> {
        let dist = self.distance_function;
        let pl = self.path_length;
        let nb = points.len();

        let (sv1_pos, sv2_pos) = match self.select_vp(&points) {
            Ok(v) => v,
            Err(_) => {
                *error = MVPError::VpNoSelect;
                return None;
            }
        };

        let sv1_idx = sv1_pos as usize;
        let sv1_data = points[sv1_idx].clone();

        // distance from sv1 to all points
        for p in &mut points {
            let d = dist(&sv1_data, p);
            if !check_dist(d) {
                *error = MVPError::NoSv1Range;
                return None;
            }
            if lvl < pl {
                p.path[lvl] = d;
            }
        }

        let sv2_data: Option<MVPDatapoint> = if sv2_pos >= 0 {
            let s2 = points[sv2_pos as usize].clone();
            for p in &mut points {
                let d = dist(&s2, p);
                if !check_dist(d) {
                    *error = MVPError::NoSv2Range;
                    return None;
                }
                if lvl + 1 < pl {
                    p.path[lvl + 1] = d;
                }
            }
            Some(s2)
        } else {
            None
        };

        let mut leaf = LeafNode::new(self.leaf_capacity as u32);
        leaf.sv1 = Some(Arc::new(sv1_data.clone()));
        if let Some(ref s2) = sv2_data {
            leaf.sv2 = Some(Arc::new(s2.clone()));
        }

        let mut count = 0usize;
        for (i, p) in points.into_iter().enumerate() {
            if i as i32 == sv1_pos || i as i32 == sv2_pos {
                continue;
            }
            let d1 = dist(&p, &sv1_data);
            let d2 = if let Some(ref s2) = sv2_data {
                dist(&p, s2)
            } else {
                0.0
            };
            leaf.d1.push(d1);
            leaf.d2.push(d2);
            leaf.points.push(Arc::new(p));
            count += 1;
        }
        leaf.nbpoints = count;
        let _ = nb;

        Some(Rc::new(RefCell::new(Node::Leaf(leaf))))
    }

    fn create_internal_from_points(
        &self,
        mut points: Vec<MVPDatapoint>,
        lvl: usize,
        error: &mut MVPError,
    ) -> Option<Rc<RefCell<Node>>> {
        let dist = self.distance_function;
        let bf = self.branch_factor;
        let length_m1 = bf - 1;
        let pl = self.path_length;

        let (sv1_pos, sv2_pos) = match self.select_vp(&points) {
            Ok(v) => v,
            Err(_) => {
                *error = MVPError::VpNoSelect;
                return None;
            }
        };
        if sv1_pos < 0 || sv2_pos < 0 {
            *error = MVPError::VpNoSelect;
            return None;
        }
        let sv1_data = points[sv1_pos as usize].clone();

        // sv1 distance for all points (updates path)
        for p in &mut points {
            let d = dist(&sv1_data, p);
            if !check_dist(d) {
                *error = MVPError::NoSv1Range;
                return None;
            }
            if lvl < pl {
                p.path[lvl] = d;
            }
        }

        let m1 = match self.find_splits_for(&points, &sv1_data, length_m1) {
            Ok(m) => m,
            Err(_) => {
                *error = MVPError::NoSplits;
                return None;
            }
        };

        let sv2_data = points[sv2_pos as usize].clone();

        // Bin points by sv1 distance, skipping sv1 and sv2
        let mut bins: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
        for (i, p) in points.into_iter().enumerate() {
            if i as i32 == sv1_pos || i as i32 == sv2_pos {
                continue;
            }
            let d = dist(&sv1_data, &p);
            if !check_dist(d) {
                *error = MVPError::NoSort;
                return None;
            }
            let mut bin_idx = length_m1;
            for k in 0..length_m1 {
                if d <= m1[k] {
                    bin_idx = k;
                    break;
                }
            }
            bins[bin_idx].push(p);
        }

        let mut internal = InternalNode::new(bf as u32);
        internal.sv1 = Some(Arc::new(sv1_data));
        internal.sv2 = Some(Arc::new(sv2_data.clone()));
        internal.m1 = m1;
        let mut m2: Vec<f32> = vec![0.0; bf * length_m1];

        for (i, mut bin) in bins.into_iter().enumerate() {
            if bin.is_empty() {
                continue;
            }
            // Compute distances to sv2
            for p in &mut bin {
                let d = dist(&sv2_data, p);
                if !check_dist(d) {
                    *error = MVPError::NoSv2Range;
                    return None;
                }
                if lvl + 1 < pl {
                    p.path[lvl + 1] = d;
                }
            }
            let bin_m2 = match self.find_splits_for(&bin, &sv2_data, length_m1) {
                Ok(m) => m,
                Err(_) => {
                    *error = MVPError::NoSplits;
                    return None;
                }
            };
            for k in 0..length_m1 {
                m2[i * length_m1 + k] = bin_m2[k];
            }

            // Sort into bf bins by sv2 distance
            let mut bins2: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
            for p in bin {
                let d = dist(&sv2_data, &p);
                if !check_dist(d) {
                    *error = MVPError::NoSort;
                    return None;
                }
                let mut bin_idx = length_m1;
                for k in 0..length_m1 {
                    if d <= bin_m2[k] {
                        bin_idx = k;
                        break;
                    }
                }
                bins2[bin_idx].push(p);
            }

            for (j, bin2) in bins2.into_iter().enumerate() {
                let child = self.add_internal(None, bin2, lvl + 2, error);
                if let Some(c) = child {
                    internal.child_nodes[i * bf + j] = c;
                }
                if *error != MVPError::Success {
                    return None;
                }
            }
        }
        internal.m2 = m2;

        Some(Rc::new(RefCell::new(Node::Internal(internal))))
    }

    fn create_new_node(
        &self,
        points: Vec<MVPDatapoint>,
        lvl: usize,
        error: &mut MVPError,
    ) -> Option<Rc<RefCell<Node>>> {
        if points.is_empty() {
            return None;
        }
        let nb = points.len();
        if nb <= self.leaf_capacity + 2 {
            self.create_leaf_from_points(points, lvl, error)
        } else {
            // Try internal; if vantage points can't be distinguished, fall back to leaf
            let result = self.create_internal_from_points(points.clone(), lvl, error);
            if *error == MVPError::VpNoSelect {
                *error = MVPError::Success;
                return self.create_leaf_from_points(points, lvl, error);
            }
            result
        }
    }

    fn add_internal(
        &self,
        existing: Option<Rc<RefCell<Node>>>,
        points: Vec<MVPDatapoint>,
        lvl: usize,
        error: &mut MVPError,
    ) -> Option<Rc<RefCell<Node>>> {
        if points.is_empty() {
            return existing;
        }

        let existing_rc = match existing {
            None => return self.create_new_node(points, lvl, error),
            Some(rc) => rc,
        };

        // Check null marker
        let is_null = is_null_leaf_node(&*existing_rc.borrow());
        if is_null {
            return self.create_new_node(points, lvl, error);
        }

        let is_leaf = matches!(&*existing_rc.borrow(), Node::Leaf(_));
        if is_leaf {
            // Determine if we have room for in-place add
            let (current_nb, has_sv1, has_sv2) = {
                let n = existing_rc.borrow();
                if let Node::Leaf(leaf) = &*n {
                    (leaf.nbpoints, leaf.sv1.is_some(), leaf.sv2.is_some())
                } else {
                    unreachable!()
                }
            };

            let has_room = current_nb + points.len() <= self.leaf_capacity;
            if has_room {
                // Append in place
                self.add_to_leaf_in_place(&existing_rc, points, lvl, error);
                Some(existing_rc)
            } else {
                // Restructure: extract current points + sv1 + sv2 + new, recurse
                let inner =
                    std::mem::replace(&mut *existing_rc.borrow_mut(), Node::Leaf(make_null_leaf()));
                if let Node::Leaf(leaf) = inner {
                    let mut all_points: Vec<MVPDatapoint> =
                        Vec::with_capacity(current_nb + points.len() + 2);
                    if let Some(sv1) = leaf.sv1 {
                        all_points.push((*sv1).clone());
                    }
                    if let Some(sv2) = leaf.sv2 {
                        all_points.push((*sv2).clone());
                    }
                    for p in leaf.points {
                        all_points.push((*p).clone());
                    }
                    for p in points {
                        all_points.push(p);
                    }
                    let _ = (has_sv1, has_sv2);
                    self.create_new_node(all_points, lvl, error)
                } else {
                    unreachable!()
                }
            }
        } else {
            // Internal node
            self.add_to_internal_in_place(&existing_rc, points, lvl, error);
            Some(existing_rc)
        }
    }

    fn add_to_leaf_in_place(
        &self,
        node_rc: &Rc<RefCell<Node>>,
        mut points: Vec<MVPDatapoint>,
        lvl: usize,
        error: &mut MVPError,
    ) {
        let dist = self.distance_function;
        let pl = self.path_length;

        // Get clone of sv1
        let sv1_clone = {
            let n = node_rc.borrow();
            if let Node::Leaf(leaf) = &*n {
                leaf.sv1.as_ref().map(|a| (**a).clone())
            } else {
                None
            }
        };
        let sv1_clone = match sv1_clone {
            Some(s) => s,
            None => {
                *error = MVPError::NoSv1Range;
                return;
            }
        };
        for p in &mut points {
            let d = dist(&sv1_clone, p);
            if !check_dist(d) {
                *error = MVPError::NoSv1Range;
                return;
            }
            if lvl < pl {
                p.path[lvl] = d;
            }
        }

        // sv2 might be None - use first point if so
        let mut start_pos = 0usize;
        let sv2_clone = {
            let mut n = node_rc.borrow_mut();
            if let Node::Leaf(leaf) = &mut *n {
                if leaf.sv2.is_none() && !points.is_empty() {
                    leaf.sv2 = Some(Arc::new(points[0].clone()));
                    start_pos = 1;
                }
                leaf.sv2.as_ref().map(|a| (**a).clone())
            } else {
                None
            }
        };
        let sv2_clone = match sv2_clone {
            Some(s) => s,
            None => {
                // No points to add (empty)
                return;
            }
        };

        for p in &mut points {
            let d = dist(&sv2_clone, p);
            if !check_dist(d) {
                *error = MVPError::NoSv2Range;
                return;
            }
            if lvl + 1 < pl {
                p.path[lvl + 1] = d;
            }
        }

        // Now append
        let mut n = node_rc.borrow_mut();
        if let Node::Leaf(leaf) = &mut *n {
            for (idx, p) in points.into_iter().enumerate() {
                if idx < start_pos {
                    continue;
                }
                let d1 = dist(&p, &sv1_clone);
                let d2 = dist(&p, &sv2_clone);
                leaf.d1.push(d1);
                leaf.d2.push(d2);
                leaf.points.push(Arc::new(p));
                leaf.nbpoints += 1;
            }
        }
    }

    fn add_to_internal_in_place(
        &self,
        node_rc: &Rc<RefCell<Node>>,
        mut points: Vec<MVPDatapoint>,
        lvl: usize,
        error: &mut MVPError,
    ) {
        let dist = self.distance_function;
        let pl = self.path_length;
        let bf = self.branch_factor;
        let length_m1 = bf - 1;

        // Clone sv1, sv2, m1, m2
        let (sv1_clone, sv2_clone, m1, m2) = {
            let n = node_rc.borrow();
            if let Node::Internal(internal) = &*n {
                let s1 = internal.sv1.as_ref().map(|a| (**a).clone());
                let s2 = internal.sv2.as_ref().map(|a| (**a).clone());
                (s1, s2, internal.m1.clone(), internal.m2.clone())
            } else {
                return;
            }
        };
        let sv1_clone = match sv1_clone {
            Some(s) => s,
            None => {
                *error = MVPError::NoSv1Range;
                return;
            }
        };
        let sv2_clone = match sv2_clone {
            Some(s) => s,
            None => {
                *error = MVPError::NoSv2Range;
                return;
            }
        };

        // Compute sv1 distance for all points
        for p in &mut points {
            let d = dist(&sv1_clone, p);
            if !check_dist(d) {
                *error = MVPError::NoSv1Range;
                return;
            }
            if lvl < pl {
                p.path[lvl] = d;
            }
        }

        // Bin by sv1 distance
        let mut bins: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
        for p in points {
            let d = dist(&sv1_clone, &p);
            let mut bin_idx = length_m1;
            for k in 0..length_m1 {
                if d <= m1[k] {
                    bin_idx = k;
                    break;
                }
            }
            bins[bin_idx].push(p);
        }

        for (i, mut bin) in bins.into_iter().enumerate() {
            if bin.is_empty() {
                continue;
            }
            for p in &mut bin {
                let d = dist(&sv2_clone, p);
                if !check_dist(d) {
                    *error = MVPError::NoSv2Range;
                    return;
                }
                if lvl + 1 < pl {
                    p.path[lvl + 1] = d;
                }
            }

            // Bin by sv2 distance using m2[i*length_m1..]
            let mut bins2: Vec<Vec<MVPDatapoint>> = (0..bf).map(|_| Vec::new()).collect();
            for p in bin {
                let d = dist(&sv2_clone, &p);
                let mut bin_idx = length_m1;
                for k in 0..length_m1 {
                    if d <= m2[i * length_m1 + k] {
                        bin_idx = k;
                        break;
                    }
                }
                bins2[bin_idx].push(p);
            }

            for (j, bin2) in bins2.into_iter().enumerate() {
                if bin2.is_empty() {
                    continue;
                }
                let child_idx = i * bf + j;
                let child_rc = {
                    let n = node_rc.borrow();
                    if let Node::Internal(internal) = &*n {
                        Rc::clone(&internal.child_nodes[child_idx])
                    } else {
                        return;
                    }
                };
                let new_child = self.add_internal(Some(child_rc), bin2, lvl + 2, error);
                {
                    let mut n = node_rc.borrow_mut();
                    if let Node::Internal(internal) = &mut *n {
                        if let Some(c) = new_child {
                            internal.child_nodes[child_idx] = c;
                        } else {
                            internal.child_nodes[child_idx] = make_null_leaf_rc();
                        }
                    }
                }
                if *error != MVPError::Success {
                    return;
                }
            }
        }
    }

    pub fn add(&mut self, points: Vec<MVPDatapoint>) -> MVPError {
        if points.is_empty() {
            return MVPError::Success;
        }
        if self.datatype != points[0].data_type {
            return MVPError::TypeMismatch;
        }
        let pl = self.path_length;
        let owned: Vec<MVPDatapoint> = points
            .into_iter()
            .map(|mut p| {
                p.path = vec![0.0f32; pl];
                p
            })
            .collect();
        let existing = self.node.take();
        let mut error = MVPError::Success;
        let new_node = self.add_internal(existing, owned, 0, &mut error);
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
        let mut results: Vec<Arc<MVPDatapoint>> = Vec::new();
        let mut target_path: Vec<f32> = vec![0.0f32; self.path_length];
        let err = self.retrieve_internal(
            &node,
            target,
            &mut target_path,
            radius,
            knearest,
            &mut results,
            0,
        );
        match err {
            MVPError::Success | MVPError::KNearestCap => {
                Ok(results.into_iter().map(|a| (*a).clone()).collect())
            }
            e => Err(e),
        }
    }

    fn retrieve_internal(
        &self,
        node_rc: &Rc<RefCell<Node>>,
        target: &MVPDatapoint,
        target_path: &mut [f32],
        radius: f32,
        k: usize,
        results: &mut Vec<Arc<MVPDatapoint>>,
        lvl: usize,
    ) -> MVPError {
        let dist = self.distance_function;
        let bf = self.branch_factor;
        let length_m1 = bf - 1;
        let pl = self.path_length;

        let n = node_rc.borrow();
        match &*n {
            Node::Leaf(leaf) => {
                if leaf.sv1.is_none() && leaf.sv2.is_none() && leaf.nbpoints == 0 {
                    return MVPError::Success;
                }
                let sv1 = match &leaf.sv1 {
                    Some(s) => s,
                    None => return MVPError::Success,
                };
                let d1 = dist(target, sv1);
                if !check_dist(d1) {
                    return MVPError::BadDistVal;
                }
                if lvl < pl {
                    target_path[lvl] = d1;
                }
                if d1 <= radius {
                    results.push(Arc::clone(sv1));
                    if results.len() >= k {
                        return MVPError::KNearestCap;
                    }
                }
                if let Some(sv2) = &leaf.sv2 {
                    let d2 = dist(target, sv2);
                    if !check_dist(d2) {
                        return MVPError::BadDistVal;
                    }
                    if d2 <= radius {
                        results.push(Arc::clone(sv2));
                        if results.len() >= k {
                            return MVPError::KNearestCap;
                        }
                    }
                    if lvl + 1 < pl {
                        target_path[lvl + 1] = d2;
                    }
                    for i in 0..leaf.nbpoints {
                        if d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i] {
                            if d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i] {
                                let endpath = if lvl + 1 < pl { lvl + 1 } else { pl };
                                let mut skip = false;
                                for j in 0..endpath {
                                    let pp = if j < leaf.points[i].path.len() {
                                        leaf.points[i].path[j]
                                    } else {
                                        0.0
                                    };
                                    if !(target_path[j] - radius <= pp
                                        && target_path[j] + radius >= pp)
                                    {
                                        skip = true;
                                        break;
                                    }
                                }
                                if !skip {
                                    let d = dist(target, &leaf.points[i]);
                                    if !check_dist(d) {
                                        return MVPError::BadDistVal;
                                    }
                                    if d <= radius {
                                        results.push(Arc::clone(&leaf.points[i]));
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
            }
            Node::Internal(internal) => {
                let sv1 = match &internal.sv1 {
                    Some(s) => s,
                    None => return MVPError::Success,
                };
                let d1 = dist(target, sv1);
                if !check_dist(d1) {
                    return MVPError::BadDistVal;
                }
                if d1 <= radius {
                    results.push(Arc::clone(sv1));
                    if results.len() >= k {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl < pl {
                    target_path[lvl] = d1;
                }
                let sv2 = match &internal.sv2 {
                    Some(s) => s,
                    None => return MVPError::Success,
                };
                let d2 = dist(target, sv2);
                if !check_dist(d2) {
                    return MVPError::BadDistVal;
                }
                if d2 <= radius {
                    results.push(Arc::clone(sv2));
                    if results.len() >= k {
                        return MVPError::KNearestCap;
                    }
                }
                if lvl + 1 < pl {
                    target_path[lvl + 1] = d2;
                }

                // Collect children to recurse on (clone Rcs so we drop borrow)
                let m1 = internal.m1.clone();
                let m2 = internal.m2.clone();
                let mut children_to_recurse: Vec<Rc<RefCell<Node>>> = Vec::new();
                for i in 0..length_m1 {
                    if d1 - radius <= m1[i] {
                        for j in 0..length_m1 {
                            if d2 - radius <= m2[i * length_m1 + j] {
                                children_to_recurse
                                    .push(Rc::clone(&internal.child_nodes[i * bf + j]));
                            }
                        }
                        if length_m1 > 0
                            && d2 + radius >= m2[i * length_m1 + length_m1 - 1]
                        {
                            children_to_recurse
                                .push(Rc::clone(&internal.child_nodes[i * bf + length_m1]));
                        }
                    }
                }
                if length_m1 > 0 && d1 + radius >= m1[length_m1 - 1] {
                    for j in 0..length_m1 {
                        if d2 - radius <= m2[length_m1 * length_m1 + j] {
                            children_to_recurse
                                .push(Rc::clone(&internal.child_nodes[bf * length_m1 + j]));
                        }
                    }
                    if length_m1 > 0
                        && d2 + radius >= m2[length_m1 * length_m1 + length_m1 - 1]
                    {
                        children_to_recurse.push(Rc::clone(
                            &internal.child_nodes[bf * length_m1 + length_m1],
                        ));
                    }
                }
                drop(n);

                for child in children_to_recurse {
                    let err = self.retrieve_internal(
                        &child,
                        target,
                        target_path,
                        radius,
                        k,
                        results,
                        lvl + 2,
                    );
                    if err != MVPError::Success {
                        return err;
                    }
                }
                MVPError::Success
            }
        }
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        let _ = mode;
        if self.node.is_none() {
            return MVPError::ArgErr;
        }
        let mut buf: Vec<u8> = Vec::new();
        // Header
        let tag_bytes = TAG.as_bytes();
        buf.extend_from_slice(tag_bytes);
        buf.push(0); // null term
        buf.extend_from_slice(&VERSION.to_le_bytes());
        buf.push(self.branch_factor as u8);
        buf.push(self.path_length as u8);
        buf.push(self.leaf_capacity as u8);
        buf.push(self.datatype as u8);
        while buf.len() < HEADER_SIZE {
            buf.push(0);
        }

        let node_rc = self.node.as_ref().unwrap();
        let mut error = MVPError::Success;
        write_node(
            &*node_rc.borrow(),
            &mut buf,
            self.path_length,
            self.datatype as u8 as usize,
            &mut error,
        );
        if error != MVPError::Success {
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
        let node = match &self.node {
            Some(n) => n,
            None => return MVPError::EmptyTree,
        };
        print_node(stream, &*node.borrow(), 0, self.branch_factor)
    }

    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        let _ = node;
        self.node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        // No-op in pure-Rust impl; reserve more space if needed.
        self.size += self.pgsize;
        0
    }
}

fn write_dp_opt(
    dp: &Option<Arc<MVPDatapoint>>,
    buf: &mut Vec<u8>,
    pl: usize,
    type_size: usize,
) {
    match dp {
        None => {
            buf.push(0);
            buf.extend_from_slice(&0u32.to_le_bytes());
        }
        Some(d) => {
            buf.push(1);
            let id_bytes = d.id.as_bytes();
            // Truncate to fit u8 length
            let idlen = std::cmp::min(id_bytes.len(), 255) as u8;
            let datalength = d.datalen as u32;
            // We don't actually use bytelength in read but write a placeholder
            let bytelength: u32 = (1 + idlen as u32) + 4 + (datalength * type_size as u32) + (pl as u32 * 4);
            buf.extend_from_slice(&bytelength.to_le_bytes());
            buf.push(idlen);
            buf.extend_from_slice(&id_bytes[..idlen as usize]);
            buf.extend_from_slice(&datalength.to_le_bytes());
            // data length in bytes = datalen * type_size
            let total_bytes = d.datalen * type_size;
            let mut data = d.data.clone();
            if data.len() < total_bytes {
                data.resize(total_bytes, 0);
            }
            buf.extend_from_slice(&data[..total_bytes]);
            // path
            for i in 0..pl {
                let p = if i < d.path.len() { d.path[i] } else { 0.0 };
                buf.extend_from_slice(&p.to_le_bytes());
            }
        }
    }
}

fn write_node(
    node: &Node,
    buf: &mut Vec<u8>,
    pl: usize,
    type_size: usize,
    error: &mut MVPError,
) {
    match node {
        Node::Leaf(leaf) => {
            if leaf.sv1.is_none() && leaf.sv2.is_none() && leaf.nbpoints == 0 {
                buf.push(0); // null marker
                return;
            }
            buf.push(2); // LEAF_NODE
            write_dp_opt(&leaf.sv1, buf, pl, type_size);
            write_dp_opt(&leaf.sv2, buf, pl, type_size);
            buf.extend_from_slice(&(leaf.nbpoints as u32).to_le_bytes());
            for i in 0..leaf.nbpoints {
                buf.extend_from_slice(&leaf.d1[i].to_le_bytes());
                buf.extend_from_slice(&leaf.d2[i].to_le_bytes());
                let dp = Some(Arc::clone(&leaf.points[i]));
                write_dp_opt(&dp, buf, pl, type_size);
            }
        }
        Node::Internal(internal) => {
            buf.push(1); // INTERNAL_NODE
            write_dp_opt(&internal.sv1, buf, pl, type_size);
            write_dp_opt(&internal.sv2, buf, pl, type_size);
            buf.extend_from_slice(&(internal.m1.len() as u32).to_le_bytes());
            for &m in &internal.m1 {
                buf.extend_from_slice(&m.to_le_bytes());
            }
            buf.extend_from_slice(&(internal.m2.len() as u32).to_le_bytes());
            for &m in &internal.m2 {
                buf.extend_from_slice(&m.to_le_bytes());
            }
            buf.extend_from_slice(&(internal.child_nodes.len() as u32).to_le_bytes());
            for child in &internal.child_nodes {
                write_node(&*child.borrow(), buf, pl, type_size, error);
                if *error != MVPError::Success {
                    return;
                }
            }
        }
    }
}

fn print_node(
    stream: &mut dyn Write,
    node: &Node,
    lvl: usize,
    bf: usize,
) -> MVPError {
    match node {
        Node::Leaf(leaf) => {
            if leaf.sv1.is_none() && leaf.sv2.is_none() && leaf.nbpoints == 0 {
                let _ = writeln!(stream, "NULL{}", lvl);
                return MVPError::Success;
            }
            let _ = writeln!(stream, "LEAF{}  ({} points)", lvl, leaf.nbpoints);
            if let Some(s) = &leaf.sv1 {
                let _ = writeln!(stream, "    sv1: {}", s.id);
            }
            if let Some(s) = &leaf.sv2 {
                let _ = writeln!(stream, "    sv2: {}", s.id);
            }
            for (i, p) in leaf.points.iter().enumerate() {
                let _ = writeln!(stream, "        point[{}]: {}", i, p.id);
            }
            MVPError::Success
        }
        Node::Internal(internal) => {
            let _ = writeln!(stream, "INTERNAL{}", lvl);
            if let Some(s) = &internal.sv1 {
                let _ = writeln!(stream, "  sv1: {}", s.id);
            }
            if let Some(s) = &internal.sv2 {
                let _ = writeln!(stream, "  sv2: {}", s.id);
            }
            for (i, &v) in internal.m1.iter().enumerate() {
                let _ = write!(stream, "  M1[{}] = {:.4};", i, v);
            }
            for (i, &v) in internal.m2.iter().enumerate() {
                let _ = write!(stream, "  M2[{}] = {:.4};", i, v);
            }
            let _ = writeln!(stream);
            for child in &internal.child_nodes {
                let err = print_node(stream, &*child.borrow(), lvl + 2, bf);
                if err != MVPError::Success {
                    return err;
                }
            }
            MVPError::Success
        }
    }
}

fn read_u8(buf: &[u8], pos: &mut usize) -> Result<u8, MVPError> {
    if *pos >= buf.len() {
        return Err(MVPError::FileOpen);
    }
    let v = buf[*pos];
    *pos += 1;
    Ok(v)
}

fn read_u32(buf: &[u8], pos: &mut usize) -> Result<u32, MVPError> {
    if *pos + 4 > buf.len() {
        return Err(MVPError::FileOpen);
    }
    let bytes: [u8; 4] = buf[*pos..*pos + 4].try_into().unwrap();
    *pos += 4;
    Ok(u32::from_le_bytes(bytes))
}

fn read_f32(buf: &[u8], pos: &mut usize) -> Result<f32, MVPError> {
    if *pos + 4 > buf.len() {
        return Err(MVPError::FileOpen);
    }
    let bytes: [u8; 4] = buf[*pos..*pos + 4].try_into().unwrap();
    *pos += 4;
    Ok(f32::from_le_bytes(bytes))
}

fn read_dp_opt(
    buf: &[u8],
    pos: &mut usize,
    pl: usize,
    type_size: usize,
    datatype: MVPDataType,
) -> Result<Option<Arc<MVPDatapoint>>, MVPError> {
    let active = read_u8(buf, pos)?;
    let _bytelength = read_u32(buf, pos)?;
    if active == 0 {
        return Ok(None);
    }
    let idlen = read_u8(buf, pos)? as usize;
    if *pos + idlen > buf.len() {
        return Err(MVPError::FileOpen);
    }
    let id = String::from_utf8(buf[*pos..*pos + idlen].to_vec())
        .map_err(|_| MVPError::FileOpen)?;
    *pos += idlen;
    let datalength = read_u32(buf, pos)? as usize;
    let total_bytes = datalength * type_size;
    if *pos + total_bytes > buf.len() {
        return Err(MVPError::FileOpen);
    }
    let data = buf[*pos..*pos + total_bytes].to_vec();
    *pos += total_bytes;
    let mut path = Vec::with_capacity(pl);
    for _ in 0..pl {
        path.push(read_f32(buf, pos)?);
    }
    Ok(Some(Arc::new(MVPDatapoint {
        id,
        data,
        path,
        datalen: datalength,
        data_type: datatype,
    })))
}

fn read_node(
    buf: &[u8],
    pos: &mut usize,
    pl: usize,
    type_size: usize,
    datatype: MVPDataType,
    bf: usize,
    leafcap: usize,
) -> Result<Option<Rc<RefCell<Node>>>, MVPError> {
    let node_type = read_u8(buf, pos)?;
    match node_type {
        0 => Ok(None),
        2 => {
            let mut leaf = LeafNode::new(leafcap as u32);
            leaf.sv1 = read_dp_opt(buf, pos, pl, type_size, datatype)?;
            leaf.sv2 = read_dp_opt(buf, pos, pl, type_size, datatype)?;
            let nbpoints = read_u32(buf, pos)? as usize;
            for _ in 0..nbpoints {
                let d1 = read_f32(buf, pos)?;
                let d2 = read_f32(buf, pos)?;
                let dp = read_dp_opt(buf, pos, pl, type_size, datatype)?;
                if let Some(d) = dp {
                    leaf.d1.push(d1);
                    leaf.d2.push(d2);
                    leaf.points.push(d);
                }
            }
            leaf.nbpoints = leaf.points.len();
            Ok(Some(Rc::new(RefCell::new(Node::Leaf(leaf)))))
        }
        1 => {
            let mut internal = InternalNode::new(bf as u32);
            internal.sv1 = read_dp_opt(buf, pos, pl, type_size, datatype)?;
            internal.sv2 = read_dp_opt(buf, pos, pl, type_size, datatype)?;
            let m1_len = read_u32(buf, pos)? as usize;
            let mut m1 = Vec::with_capacity(m1_len);
            for _ in 0..m1_len {
                m1.push(read_f32(buf, pos)?);
            }
            internal.m1 = m1;
            let m2_len = read_u32(buf, pos)? as usize;
            let mut m2 = Vec::with_capacity(m2_len);
            for _ in 0..m2_len {
                m2.push(read_f32(buf, pos)?);
            }
            internal.m2 = m2;
            let child_count = read_u32(buf, pos)? as usize;
            let mut child_nodes: Vec<Rc<RefCell<Node>>> = Vec::with_capacity(child_count);
            for _ in 0..child_count {
                let child = read_node(buf, pos, pl, type_size, datatype, bf, leafcap)?;
                match child {
                    Some(rc) => child_nodes.push(rc),
                    None => child_nodes.push(make_null_leaf_rc()),
                }
            }
            internal.child_nodes = child_nodes;
            Ok(Some(Rc::new(RefCell::new(Node::Internal(internal)))))
        }
        _ => Err(MVPError::Unrecognized),
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let mut f = File::open(filename).map_err(|_| MVPError::FileNotFound)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|_| MVPError::FileOpen)?;
    if buf.len() < HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }
    let tag_len = TAG.len() + 1;
    let mut pos = tag_len;
    let _version = read_u32(&buf, &mut pos)?;
    let bf = read_u8(&buf, &mut pos)? as usize;
    let pl = read_u8(&buf, &mut pos)? as usize;
    let lc = read_u8(&buf, &mut pos)? as usize;
    let ht = read_u8(&buf, &mut pos)?;
    let datatype = match ht {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => return Err(MVPError::ArgErr),
    };
    let type_size = ht as usize;

    let mut tree = MVPTree::new(bf, pl, lc, datatype, distance_function);
    let mut pos = HEADER_SIZE;
    let node = read_node(&buf, &mut pos, pl, type_size, datatype, bf, lc)?;
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
    pub fn select_vantage_points(&mut self, nb: u32, sv1_pos: i32, sv2_pos: i32, dist: DistanceFunction) -> i32 {
        let _ = (nb, sv1_pos, sv2_pos, dist);
        0
    }
    pub fn find_splits(&mut self, nb: u32, vp: &MVPDatapoint, tree: &MVPTree, lengthM: u32) -> f32 {
        let _ = (nb, vp, tree, lengthM);
        0.0
    }
    pub fn sort_points(
        &mut self,
        nb: u32,
        sv1_pos: i32,
        sv2_pos: i32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        counts: &mut Vec<Vec<i32>>,
        pivots: Vec<f32>,
    ) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        let _ = (nb, sv1_pos, sv2_pos, vp, tree, counts, pivots);
        Vec::new()
    }
    pub fn find_distance_range_for_vp(
        &mut self,
        nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        level: i32,
    ) -> i32 {
        let _ = (nb, vp, tree, level);
        0
    }
    pub fn write(&self, tree: &MVPTree) -> i64 {
        let _ = tree;
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
