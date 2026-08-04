use std::cell::RefCell;
use std::fs::{self, OpenOptions};
use std::io::{self, Cursor, Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::rc::Rc;
use std::sync::Arc;

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
        let len_m1 = bf.saturating_sub(1);
        let fanout = bf.saturating_mul(bf);
        InternalNode {
            node_type: NodeType::InternalNode,
            sv1: None,
            sv2: None,
            m1: vec![0.0; len_m1],
            m2: vec![0.0; bf.saturating_mul(len_m1)],
            child_nodes: (0..fanout)
                .map(|_| Rc::new(RefCell::new(Node::Leaf(LeafNode::empty()))))
                .collect(),
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

    fn empty() -> Self {
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

        if points[0].data_type != self.datatype {
            return MVPError::TypeMismatch;
        }

        for point in &mut points {
            if point.data_type != self.datatype {
                return MVPError::TypeMismatch;
            }
            point.path = vec![0.0; self.path_length];
        }

        match add_points_to_node(self, self.node.clone(), points, 0) {
            Ok(node) => {
                self.node = Some(node);
                MVPError::Success
            }
            Err(err) => err,
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
        if self.node.is_none() {
            return Err(MVPError::EmptyTree);
        }

        let mut query = target.clone();
        query.path = vec![0.0; self.path_length];
        let mut results = Vec::with_capacity(knearest);
        let err = retrieve_from_node(
            self,
            self.node.as_ref().expect("checked is_some"),
            &mut query,
            radius,
            knearest,
            &mut results,
            0,
        );

        match err {
            MVPError::Success | MVPError::KNearestCap => Ok(results),
            other => Err(other),
        }
    }

    pub fn write(&self, filename: &str, mode: i32) -> MVPError {
        if filename.is_empty() || self.node.is_none() {
            return MVPError::ArgErr;
        }

        let mut buf = vec![0_u8; HEADER_SIZE];
        write_header(self, &mut buf);

        if let Err(err) = write_node_bytes(self, self.node.as_ref().expect("checked is_some"), &mut buf) {
            return err;
        }

        let mut options = OpenOptions::new();
        options.create(true).write(true).truncate(true);
        #[cfg(unix)]
        options.mode(normalize_mode(mode));

        match options.open(filename) {
            Ok(mut file) => {
                #[cfg(unix)]
                {
                    let _ = file.set_permissions(fs::Permissions::from_mode(normalize_mode(mode)));
                }
                if file.write_all(&buf).is_err() {
                    return MVPError::NoWrite;
                }
                if file.flush().is_err() {
                    return MVPError::FileClose;
                }
                MVPError::Success
            }
            Err(_) => MVPError::FileOpen,
        }
    }

    pub fn print(&self, stream: &mut dyn Write) -> MVPError {
        match &self.node {
            Some(node) => {
                let err = print_node(stream, self, node, 0);
                if err != MVPError::Success {
                    let _ = writeln!(stream, "malformed tree: {}", error_to_string(err));
                }
                err
            }
            None => {
                let _ = writeln!(stream, "NULL0");
                MVPError::Success
            }
        }
    }

    pub fn clear(&mut self, node: &mut Option<Box<Node>>) {
        self.node = None;
        *node = None;
    }

    pub fn extend_mvpfile(&mut self) -> i32 {
        let grow_by = if self.pgsize > 0 { self.pgsize as usize } else { 4096 };
        self.buf.resize(self.buf.len() + grow_by, 0);
        self.size = self.buf.len() as i64;
        0
    }
}

pub fn mvptree_read(filename: &str, distance_function: DistanceFunction) -> Result<MVPTree, MVPError> {
    let bytes = match fs::read(filename) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(MVPTree::new(2, 5, 25, MVPDataType::ByteArray, distance_function));
        }
        Err(_) => return Err(MVPError::FileOpen),
    };

    if bytes.len() < HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }

    let mut cursor = Cursor::new(bytes);
    let (_, branch_factor, path_length, leaf_capacity, datatype) = read_header(&mut cursor)?;
    let node = read_node_bytes(&mut cursor, branch_factor, path_length, leaf_capacity, datatype)?;

    let mut tree = MVPTree::new(
        branch_factor,
        path_length,
        leaf_capacity,
        datatype,
        distance_function,
    );
    tree.size = cursor.get_ref().len() as i64;
    tree.node = Some(node);
    Ok(tree)
}

impl MVPDatapoint {
    pub fn new(id: String, data: Vec<u8>, data_type: MVPDataType) -> Self {
        let width = datatype_width(data_type);
        let datalen = if width > 0 && data.len().is_multiple_of(width) {
            data.len() / width
        } else {
            data.len()
        };
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
        nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        dist: DistanceFunction,
    ) -> i32 {
        if nb == 0 {
            return -1;
        }
        let d = dist(self, self);
        if d.is_nan() || d < 0.0 {
            -2
        } else {
            0
        }
    }

    pub fn find_splits(&mut self, _nb: u32, vp: &MVPDatapoint, tree: &MVPTree, _length_m: u32) -> f32 {
        (tree.distance_function)(self, vp)
    }

    pub fn sort_points(
        &mut self,
        _nb: u32,
        _sv1_pos: i32,
        _sv2_pos: i32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        counts: &mut Vec<Vec<i32>>,
        pivots: Vec<f32>,
    ) -> Vec<Vec<Vec<Arc<MVPDatapoint>>>> {
        let distance = (tree.distance_function)(self, vp);
        let mut bins = vec![Vec::<Vec<Arc<MVPDatapoint>>>::new(); counts.len().max(1)];
        let mut slot = pivots.len();
        for (idx, pivot) in pivots.iter().enumerate() {
            if distance <= *pivot {
                slot = idx;
                break;
            }
        }
        if slot >= bins.len() {
            bins.resize(slot + 1, Vec::new());
        }
        bins[slot].push(vec![Arc::new(self.clone())]);
        bins
    }

    pub fn find_distance_range_for_vp(
        &mut self,
        _nb: u32,
        vp: &MVPDatapoint,
        tree: &MVPTree,
        level: i32,
    ) -> i32 {
        let d = (tree.distance_function)(vp, self);
        if d.is_nan() || d < 0.0 {
            return -2;
        }
        let level = level.max(0) as usize;
        if level < self.path.len() {
            self.path[level] = d;
        }
        0
    }

    pub fn write(&self, tree: &MVPTree) -> i64 {
        let mut buf = Vec::new();
        if write_datapoint_bytes(Some(self), tree.path_length, &mut buf).is_err() {
            return -1;
        }
        buf.len() as i64
    }
}

pub fn error_to_string(error: MVPError) -> &'static str {
    ERROR_MSGS[error as usize]
}

fn datatype_width(data_type: MVPDataType) -> usize {
    data_type as usize
}

#[cfg(unix)]
fn normalize_mode(mode: i32) -> u32 {
    u32::from_str_radix(&mode.to_string(), 8).unwrap_or(mode as u32)
}

#[cfg(not(unix))]
fn normalize_mode(mode: i32) -> u32 {
    mode as u32
}

fn make_arc(point: MVPDatapoint) -> Arc<MVPDatapoint> {
    Arc::new(point)
}

fn clone_arc(opt: &Option<Arc<MVPDatapoint>>) -> Option<Arc<MVPDatapoint>> {
    opt.as_ref().map(Arc::clone)
}

fn validated_distance(tree: &MVPTree, a: &MVPDatapoint, b: &MVPDatapoint) -> Result<f32, MVPError> {
    let d = (tree.distance_function)(a, b);
    if d.is_nan() || d < 0.0 {
        Err(MVPError::BadDistVal)
    } else {
        Ok(d)
    }
}

fn select_vantage_positions(
    points: &[MVPDatapoint],
    dist: DistanceFunction,
) -> Result<(usize, Option<usize>), MVPError> {
    if points.is_empty() {
        return Err(MVPError::ArgErr);
    }

    let mut sv1 = 0_usize;
    let mut sv2 = None;
    let mut max_dist = 0.0_f32;

    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = dist(&points[i], &points[j]);
            if d.is_nan() || d < 0.0 {
                return Err(MVPError::BadDistVal);
            }
            if d > max_dist {
                max_dist = d;
                sv1 = i;
                sv2 = Some(j);
            }
        }
    }

    if sv2.is_none() && points.len() > 1 {
        sv2 = Some(1);
    }

    Ok((sv1, sv2))
}

fn set_path_distances(
    tree: &MVPTree,
    points: &mut [MVPDatapoint],
    vp: &MVPDatapoint,
    level: usize,
) -> Result<(), MVPError> {
    for point in points {
        let d = validated_distance(tree, vp, point)?;
        if level < point.path.len() {
            point.path[level] = d;
        }
    }
    Ok(())
}

fn find_splits_for_points(
    tree: &MVPTree,
    points: &[MVPDatapoint],
    vp: &MVPDatapoint,
    count: usize,
) -> Result<Vec<f32>, MVPError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if points.is_empty() {
        return Err(MVPError::NoSplits);
    }

    let mut distances = Vec::with_capacity(points.len());
    for point in points {
        distances.push(validated_distance(tree, point, vp)?);
    }
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut pivots = vec![0.0; count];
    for (i, pivot) in pivots.iter_mut().enumerate() {
        let mut index = (i + 1) * points.len() / (count + 1);
        if index >= points.len() {
            index = points.len() - 1;
        }
        *pivot = distances[index];
    }
    Ok(pivots)
}

fn sort_points_into_bins(
    tree: &MVPTree,
    points: Vec<MVPDatapoint>,
    skip: &[usize],
    vp: &MVPDatapoint,
    pivots: &[f32],
) -> Result<Vec<Vec<MVPDatapoint>>, MVPError> {
    let bf = tree.branch_factor;
    if bf == 0 {
        return Ok(Vec::new());
    }

    let mut bins = vec![Vec::<MVPDatapoint>::new(); bf];
    for (idx, point) in points.into_iter().enumerate() {
        if skip.contains(&idx) {
            continue;
        }
        let d = validated_distance(tree, vp, &point)?;
        let mut bin = bf - 1;
        for (pivot_idx, pivot) in pivots.iter().enumerate() {
            if d <= *pivot {
                bin = pivot_idx;
                break;
            }
        }
        bins[bin].push(point);
    }

    Ok(bins)
}

fn build_leaf_node(tree: &MVPTree, mut points: Vec<MVPDatapoint>, level: usize) -> Result<Rc<RefCell<Node>>, MVPError> {
    if points.is_empty() {
        return Ok(Rc::new(RefCell::new(Node::Leaf(LeafNode::empty()))));
    }

    let (sv1_idx, sv2_idx) = select_vantage_positions(&points, tree.distance_function)?;
    let sv1 = points[sv1_idx].clone();
    set_path_distances(tree, &mut points, &sv1, level).map_err(|_| MVPError::NoSv1Range)?;

    let sv2 = sv2_idx.map(|idx| points[idx].clone());
    if let Some(ref sv2_point) = sv2 {
        set_path_distances(tree, &mut points, sv2_point, level + 1).map_err(|_| MVPError::NoSv2Range)?;
    }

    let mut leaf = LeafNode::new(tree.leaf_capacity.max(points.len()) as u32);
    leaf.sv1 = Some(make_arc(sv1));
    leaf.sv2 = sv2.map(make_arc);

    for (idx, point) in points.into_iter().enumerate() {
        if idx == sv1_idx || Some(idx) == sv2_idx {
            continue;
        }

        let d1 = validated_distance(tree, leaf.sv1.as_ref().expect("present"), &point).map_err(|_| MVPError::NoSv1Range)?;
        let d2 = if let Some(ref sv2_point) = leaf.sv2 {
            validated_distance(tree, sv2_point, &point).map_err(|_| MVPError::NoSv2Range)?
        } else {
            0.0
        };

        leaf.points.push(make_arc(point));
        leaf.d1.push(d1);
        leaf.d2.push(d2);
    }
    leaf.nbpoints = leaf.points.len();

    Ok(Rc::new(RefCell::new(Node::Leaf(leaf))))
}

fn build_internal_node(tree: &MVPTree, mut points: Vec<MVPDatapoint>, level: usize) -> Result<Rc<RefCell<Node>>, MVPError> {
    if points.len() <= tree.leaf_capacity + 2 || tree.branch_factor < 2 {
        return build_leaf_node(tree, points, level);
    }

    let (sv1_idx, sv2_idx) = select_vantage_positions(&points, tree.distance_function)?;
    let sv2_idx = match sv2_idx {
        Some(idx) => idx,
        None => return build_leaf_node(tree, points, level),
    };

    let sv1 = points[sv1_idx].clone();
    let sv2 = points[sv2_idx].clone();

    set_path_distances(tree, &mut points, &sv1, level).map_err(|_| MVPError::NoSv1Range)?;

    let mut internal = InternalNode::new(tree.branch_factor as u32);
    internal.sv1 = Some(make_arc(sv1.clone()));
    internal.sv2 = Some(make_arc(sv2.clone()));
    internal.m1 = find_splits_for_points(tree, &points, &sv1, tree.branch_factor.saturating_sub(1))
        .map_err(|_| MVPError::NoSplits)?;

    let bins1 = sort_points_into_bins(tree, points, &[sv1_idx, sv2_idx], &sv1, &internal.m1).map_err(|_| MVPError::NoSort)?;
    let len_m1 = tree.branch_factor.saturating_sub(1);

    for (i, mut bin) in bins1.into_iter().enumerate() {
        if !bin.is_empty() {
            set_path_distances(tree, &mut bin, &sv2, level + 1).map_err(|_| MVPError::NoSv2Range)?;
            let splits = find_splits_for_points(tree, &bin, &sv2, len_m1).map_err(|_| MVPError::NoSplits)?;
            let start = i * len_m1;
            let end = start + splits.len();
            if end <= internal.m2.len() {
                internal.m2[start..end].copy_from_slice(&splits);
            }

            let bins2 = sort_points_into_bins(tree, bin, &[], &sv2, &splits).map_err(|_| MVPError::NoSort)?;
            for (j, child_bin) in bins2.into_iter().enumerate() {
                let child = if child_bin.is_empty() {
                    Rc::new(RefCell::new(Node::Leaf(LeafNode::empty())))
                } else {
                    build_internal_node(tree, child_bin, level + 2)?
                };
                internal.child_nodes[i * tree.branch_factor + j] = child;
            }
        }
    }

    Ok(Rc::new(RefCell::new(Node::Internal(internal))))
}

fn add_points_to_node(
    tree: &MVPTree,
    node: Option<Rc<RefCell<Node>>>,
    points: Vec<MVPDatapoint>,
    level: usize,
) -> Result<Rc<RefCell<Node>>, MVPError> {
    if points.is_empty() {
        return node.ok_or(MVPError::ArgErr);
    }

    match node {
        None => build_internal_node(tree, points, level),
        Some(node_ref) => {
            let is_leaf = matches!(*node_ref.borrow(), Node::Leaf(_));
            if is_leaf {
                let mut combined = points;
                {
                    let borrowed = node_ref.borrow();
                    if let Node::Leaf(leaf) = &*borrowed {
                        if let Some(sv1) = &leaf.sv1 {
                            combined.push((**sv1).clone());
                        }
                        if let Some(sv2) = &leaf.sv2 {
                            combined.push((**sv2).clone());
                        }
                        combined.extend(leaf.points.iter().map(|point| (**point).clone()));
                    }
                }
                build_internal_node(tree, combined, level)
            } else {
                let (sv1, sv2, m1) = {
                    let borrowed = node_ref.borrow();
                    let Node::Internal(internal) = &*borrowed else {
                        return Err(MVPError::Unrecognized);
                    };
                    (
                        internal.sv1.as_ref().ok_or(MVPError::Unrecognized)?.clone(),
                        internal.sv2.as_ref().ok_or(MVPError::Unrecognized)?.clone(),
                        internal.m1.clone(),
                    )
                };
                let bins1 = sort_points_into_bins(tree, points, &[], &sv1, &m1).map_err(|_| MVPError::NoSort)?;

                for (i, mut bin) in bins1.into_iter().enumerate() {
                    if bin.is_empty() {
                        continue;
                    }

                    set_path_distances(tree, &mut bin, &sv2, level + 1).map_err(|_| MVPError::NoSv2Range)?;

                    let borrowed = node_ref.borrow();
                    let Node::Internal(internal) = &*borrowed else {
                        return Err(MVPError::Unrecognized);
                    };
                    let len_m1 = tree.branch_factor.saturating_sub(1);
                    let start = i * len_m1;
                    let end = start + len_m1;
                    let pivots = internal.m2.get(start..end).unwrap_or(&[]);
                    let bins2 = sort_points_into_bins(tree, bin, &[], &sv2, pivots).map_err(|_| MVPError::NoSort)?;
                    drop(borrowed);

                    for (j, child_bin) in bins2.into_iter().enumerate() {
                        if child_bin.is_empty() {
                            continue;
                        }
                        let child_index = i * tree.branch_factor + j;
                        let current_child = {
                            let borrowed = node_ref.borrow();
                            let Node::Internal(internal) = &*borrowed else {
                                return Err(MVPError::Unrecognized);
                            };
                            internal.child_nodes.get(child_index).cloned().ok_or(MVPError::Unrecognized)?
                        };
                        let new_child = add_points_to_node(tree, Some(current_child), child_bin, level + 2)?;
                        let mut borrowed = node_ref.borrow_mut();
                        let Node::Internal(internal) = &mut *borrowed else {
                            return Err(MVPError::Unrecognized);
                        };
                        internal.child_nodes[child_index] = new_child;
                    }
                }

                Ok(node_ref)
            }
        }
    }
}

fn retrieve_from_node(
    tree: &MVPTree,
    node: &Rc<RefCell<Node>>,
    target: &mut MVPDatapoint,
    radius: f32,
    knearest: usize,
    results: &mut Vec<MVPDatapoint>,
    level: usize,
) -> MVPError {
    let borrowed = node.borrow();
    match &*borrowed {
        Node::Leaf(leaf) => retrieve_from_leaf(tree, leaf, target, radius, knearest, results, level),
        Node::Internal(internal) => retrieve_from_internal(tree, internal, target, radius, knearest, results, level),
    }
}

fn push_result(results: &mut Vec<MVPDatapoint>, point: &Arc<MVPDatapoint>, knearest: usize) -> MVPError {
    results.push((**point).clone());
    if results.len() >= knearest {
        MVPError::KNearestCap
    } else {
        MVPError::Success
    }
}

fn retrieve_from_leaf(
    tree: &MVPTree,
    leaf: &LeafNode,
    target: &mut MVPDatapoint,
    radius: f32,
    knearest: usize,
    results: &mut Vec<MVPDatapoint>,
    level: usize,
) -> MVPError {
    let Some(sv1) = leaf.sv1.as_ref() else {
        return MVPError::Success;
    };

    let d1 = match validated_distance(tree, target, sv1) {
        Ok(d) => d,
        Err(err) => return err,
    };
    if level < target.path.len() {
        target.path[level] = d1;
    }
    if d1 <= radius {
        let err = push_result(results, sv1, knearest);
        if err != MVPError::Success {
            return err;
        }
    }

    let Some(sv2) = leaf.sv2.as_ref() else {
        return MVPError::Success;
    };

    let d2 = match validated_distance(tree, target, sv2) {
        Ok(d) => d,
        Err(err) => return err,
    };
    if level + 1 < target.path.len() {
        target.path[level + 1] = d2;
    }
    if d2 <= radius {
        let err = push_result(results, sv2, knearest);
        if err != MVPError::Success {
            return err;
        }
    }

    for i in 0..leaf.nbpoints {
        if !(d1 - radius <= leaf.d1[i] && d1 + radius >= leaf.d1[i]) {
            continue;
        }
        if !(d2 - radius <= leaf.d2[i] && d2 + radius >= leaf.d2[i]) {
            continue;
        }

        let end_path = (level + 1).min(target.path.len());
        let mut skip = false;
        for j in 0..end_path {
            let point_path = leaf.points[i].path.get(j).copied().unwrap_or(0.0);
            if !(target.path[j] - radius <= point_path && target.path[j] + radius >= point_path) {
                skip = true;
                break;
            }
        }

        if skip {
            continue;
        }

        let d = match validated_distance(tree, target, &leaf.points[i]) {
            Ok(d) => d,
            Err(err) => return err,
        };
        if d <= radius {
            let err = push_result(results, &leaf.points[i], knearest);
            if err != MVPError::Success {
                return err;
            }
        }
    }

    MVPError::Success
}

fn retrieve_from_internal(
    tree: &MVPTree,
    internal: &InternalNode,
    target: &mut MVPDatapoint,
    radius: f32,
    knearest: usize,
    results: &mut Vec<MVPDatapoint>,
    level: usize,
) -> MVPError {
    let Some(sv1) = internal.sv1.as_ref() else {
        return MVPError::Success;
    };
    let Some(sv2) = internal.sv2.as_ref() else {
        return MVPError::Success;
    };

    let d1 = match validated_distance(tree, target, sv1) {
        Ok(d) => d,
        Err(err) => return err,
    };
    if d1 <= radius {
        let err = push_result(results, sv1, knearest);
        if err != MVPError::Success {
            return err;
        }
    }
    if level < target.path.len() {
        target.path[level] = d1;
    }

    let d2 = match validated_distance(tree, target, sv2) {
        Ok(d) => d,
        Err(err) => return err,
    };
    if d2 <= radius {
        let err = push_result(results, sv2, knearest);
        if err != MVPError::Success {
            return err;
        }
    }
    if level + 1 < target.path.len() {
        target.path[level + 1] = d2;
    }

    let bf = tree.branch_factor;
    let len_m1 = bf.saturating_sub(1);

    for i in 0..len_m1 {
        if d1 - radius <= internal.m1[i] {
            for j in 0..len_m1 {
                if d2 - radius <= internal.m2[i * len_m1 + j] {
                    let err = retrieve_from_node(tree, &internal.child_nodes[i * bf + j], target, radius, knearest, results, level + 2);
                    if err != MVPError::Success {
                        return err;
                    }
                }
            }
            if len_m1 > 0 && d2 + radius >= internal.m2[i * len_m1 + (len_m1 - 1)] {
                let err = retrieve_from_node(tree, &internal.child_nodes[i * bf + len_m1], target, radius, knearest, results, level + 2);
                if err != MVPError::Success {
                    return err;
                }
            }
        }
    }

    if len_m1 > 0 && d1 + radius >= internal.m1[len_m1 - 1] {
        for j in 0..len_m1 {
            if d2 - radius <= internal.m2[len_m1 * len_m1 + j] {
                let err = retrieve_from_node(tree, &internal.child_nodes[bf * len_m1 + j], target, radius, knearest, results, level + 2);
                if err != MVPError::Success {
                    return err;
                }
            }
        }
        if d2 + radius >= internal.m2[len_m1 * len_m1 + (len_m1 - 1)] {
            let err = retrieve_from_node(tree, &internal.child_nodes[bf * len_m1 + len_m1], target, radius, knearest, results, level + 2);
            if err != MVPError::Success {
                return err;
            }
        }
    }

    MVPError::Success
}

fn write_header(tree: &MVPTree, buf: &mut Vec<u8>) {
    buf[..HEADER_SIZE].fill(0);
    let tag_bytes = TAG.as_bytes();
    let tag_end = tag_bytes.len().min(HEADER_SIZE.saturating_sub(1));
    buf[..tag_end].copy_from_slice(&tag_bytes[..tag_end]);
    buf[tag_end] = 0;
    let mut pos = tag_end + 1;
    buf[pos..pos + 4].copy_from_slice(&VERSION.to_le_bytes());
    pos += 4;
    buf[pos] = tree.branch_factor as u8;
    pos += 1;
    buf[pos] = tree.path_length as u8;
    pos += 1;
    buf[pos] = tree.leaf_capacity as u8;
    pos += 1;
    buf[pos] = tree.datatype as u8;
}

fn write_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_f32(buf: &mut Vec<u8>, value: f32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_datapoint_bytes(dp: Option<&MVPDatapoint>, path_length: usize, buf: &mut Vec<u8>) -> Result<u64, MVPError> {
    let start = buf.len() as u64;
    match dp {
        None => {
            buf.push(0);
            write_u32(buf, 0);
        }
        Some(dp) => {
            let id_bytes = dp.id.as_bytes();
            if id_bytes.len() > u8::MAX as usize {
                return Err(MVPError::NoWrite);
            }
            let bytelength = 1_u32
                .saturating_add(id_bytes.len() as u32)
                .saturating_add(4)
                .saturating_add(dp.data.len() as u32)
                .saturating_add((path_length * 4) as u32);

            buf.push(1);
            write_u32(buf, bytelength);
            buf.push(id_bytes.len() as u8);
            buf.extend_from_slice(id_bytes);
            write_u32(buf, dp.datalen as u32);
            buf.extend_from_slice(&dp.data);
            for i in 0..path_length {
                write_f32(buf, dp.path.get(i).copied().unwrap_or(0.0));
            }
        }
    }
    Ok(start)
}

fn write_node_bytes(
    tree: &MVPTree,
    node: &Rc<RefCell<Node>>,
    buf: &mut Vec<u8>,
) -> Result<u64, MVPError> {
    let start = buf.len() as u64;
    let borrowed = node.borrow();
    match &*borrowed {
        Node::Leaf(leaf) => {
            buf.push(NodeType::LeafNode as u8);
            write_datapoint_bytes(leaf.sv1.as_deref(), tree.path_length, buf)?;
            write_datapoint_bytes(leaf.sv2.as_deref(), tree.path_length, buf)?;
            write_u32(buf, leaf.nbpoints as u32);
            let table_start = buf.len();
            buf.resize(table_start + leaf.nbpoints * (4 + 4 + 8), 0);
            let mut entry = table_start;
            for i in 0..leaf.nbpoints {
                buf[entry..entry + 4].copy_from_slice(&leaf.d1[i].to_le_bytes());
                entry += 4;
                buf[entry..entry + 4].copy_from_slice(&leaf.d2[i].to_le_bytes());
                entry += 4;
                let point_offset = write_datapoint_bytes(Some(&leaf.points[i]), tree.path_length, buf)?;
                buf[entry..entry + 8].copy_from_slice(&point_offset.to_le_bytes());
                entry += 8;
            }
        }
        Node::Internal(internal) => {
            buf.push(NodeType::InternalNode as u8);
            write_datapoint_bytes(internal.sv1.as_deref(), tree.path_length, buf)?;
            write_datapoint_bytes(internal.sv2.as_deref(), tree.path_length, buf)?;
            for value in &internal.m1 {
                write_f32(buf, *value);
            }
            for value in &internal.m2 {
                write_f32(buf, *value);
            }
            let table_start = buf.len();
            let fanout = internal.child_nodes.len();
            buf.resize(table_start + fanout * (1 + 8), 0);
            let mut entry = table_start;
            for child in &internal.child_nodes {
                buf[entry] = 0;
                entry += 1;
                let child_offset = write_node_bytes(tree, child, buf)?;
                buf[entry..entry + 8].copy_from_slice(&child_offset.to_le_bytes());
                entry += 8;
            }
        }
    }
    Ok(start)
}

fn read_exact<const N: usize>(cursor: &mut Cursor<Vec<u8>>) -> Result<[u8; N], MVPError> {
    let mut buf = [0_u8; N];
    cursor.read_exact(&mut buf).map_err(|_| MVPError::FileOpen)?;
    Ok(buf)
}

fn read_u8(cursor: &mut Cursor<Vec<u8>>) -> Result<u8, MVPError> {
    Ok(read_exact::<1>(cursor)?[0])
}

fn read_u32(cursor: &mut Cursor<Vec<u8>>) -> Result<u32, MVPError> {
    Ok(u32::from_le_bytes(read_exact::<4>(cursor)?))
}

fn read_u64(cursor: &mut Cursor<Vec<u8>>) -> Result<u64, MVPError> {
    Ok(u64::from_le_bytes(read_exact::<8>(cursor)?))
}

fn read_f32(cursor: &mut Cursor<Vec<u8>>) -> Result<f32, MVPError> {
    Ok(f32::from_le_bytes(read_exact::<4>(cursor)?))
}

fn read_header(
    cursor: &mut Cursor<Vec<u8>>,
) -> Result<(String, usize, usize, usize, MVPDataType), MVPError> {
    let mut header = vec![0_u8; HEADER_SIZE];
    cursor.read_exact(&mut header).map_err(|_| MVPError::FileOpen)?;
    let nul = header.iter().position(|b| *b == 0).unwrap_or(TAG.len());
    let tag = String::from_utf8_lossy(&header[..nul]).to_string();
    let version_start = nul + 1;
    if version_start + 8 > HEADER_SIZE {
        return Err(MVPError::FileOpen);
    }

    let branch_factor = header[version_start + 4] as usize;
    let path_length = header[version_start + 5] as usize;
    let leaf_capacity = header[version_start + 6] as usize;
    let datatype = match header[version_start + 7] {
        1 => MVPDataType::ByteArray,
        2 => MVPDataType::UInt16Array,
        4 => MVPDataType::UInt32Array,
        8 => MVPDataType::UInt64Array,
        _ => MVPDataType::ByteArray,
    };

    Ok((tag, branch_factor, path_length, leaf_capacity, datatype))
}

fn read_datapoint_bytes(
    cursor: &mut Cursor<Vec<u8>>,
    path_length: usize,
    datatype: MVPDataType,
) -> Result<Option<Arc<MVPDatapoint>>, MVPError> {
    let active = read_u8(cursor)?;
    let bytelength = read_u32(cursor)?;
    if active == 0 && bytelength == 0 {
        return Ok(None);
    }

    let id_len = read_u8(cursor)? as usize;
    let mut id = vec![0_u8; id_len];
    cursor.read_exact(&mut id).map_err(|_| MVPError::FileOpen)?;
    let datalen = read_u32(cursor)? as usize;
    let mut data = vec![0_u8; datalen.saturating_mul(datatype_width(datatype))];
    cursor.read_exact(&mut data).map_err(|_| MVPError::FileOpen)?;
    let mut path = vec![0.0; path_length];
    for value in &mut path {
        *value = read_f32(cursor)?;
    }

    Ok(Some(Arc::new(MVPDatapoint {
        id: String::from_utf8_lossy(&id).to_string(),
        data,
        path,
        datalen,
        data_type: datatype,
    })))
}

fn read_node_at(
    bytes: &Vec<u8>,
    offset: u64,
    branch_factor: usize,
    path_length: usize,
    leaf_capacity: usize,
    datatype: MVPDataType,
) -> Result<Rc<RefCell<Node>>, MVPError> {
    let mut cursor = Cursor::new(bytes.clone());
    cursor.set_position(offset);
    read_node_bytes(&mut cursor, branch_factor, path_length, leaf_capacity, datatype)
}

fn read_node_bytes(
    cursor: &mut Cursor<Vec<u8>>,
    branch_factor: usize,
    path_length: usize,
    leaf_capacity: usize,
    datatype: MVPDataType,
) -> Result<Rc<RefCell<Node>>, MVPError> {
    let node_type = read_u8(cursor)?;
    match node_type {
        x if x == NodeType::LeafNode as u8 => {
            let sv1 = read_datapoint_bytes(cursor, path_length, datatype)?;
            let sv2 = read_datapoint_bytes(cursor, path_length, datatype)?;
            let nbpoints = read_u32(cursor)? as usize;
            let mut leaf = LeafNode::new(leaf_capacity.max(nbpoints) as u32);
            leaf.sv1 = clone_arc(&sv1);
            leaf.sv2 = clone_arc(&sv2);
            leaf.nbpoints = nbpoints;

            let bytes = cursor.get_ref().clone();
            let mut offsets = Vec::with_capacity(nbpoints);
            for _ in 0..nbpoints {
                leaf.d1.push(read_f32(cursor)?);
                leaf.d2.push(read_f32(cursor)?);
                offsets.push(read_u64(cursor)?);
            }
            for offset in offsets {
                let point = read_datapoint_at(&bytes, offset, path_length, datatype)?;
                leaf.points.push(point);
            }
            Ok(Rc::new(RefCell::new(Node::Leaf(leaf))))
        }
        x if x == NodeType::InternalNode as u8 => {
            let sv1 = read_datapoint_bytes(cursor, path_length, datatype)?;
            let sv2 = read_datapoint_bytes(cursor, path_length, datatype)?;
            let len_m1 = branch_factor.saturating_sub(1);
            let mut internal = InternalNode::new(branch_factor as u32);
            internal.sv1 = clone_arc(&sv1);
            internal.sv2 = clone_arc(&sv2);
            internal.m1 = vec![0.0; len_m1];
            internal.m2 = vec![0.0; branch_factor.saturating_mul(len_m1)];

            for value in &mut internal.m1 {
                *value = read_f32(cursor)?;
            }
            for value in &mut internal.m2 {
                *value = read_f32(cursor)?;
            }

            let bytes = cursor.get_ref().clone();
            internal.child_nodes.clear();
            for _ in 0..branch_factor.saturating_mul(branch_factor) {
                let _fileno = read_u8(cursor)?;
                let offset = read_u64(cursor)?;
                internal.child_nodes.push(read_node_at(
                    &bytes,
                    offset,
                    branch_factor,
                    path_length,
                    leaf_capacity,
                    datatype,
                )?);
            }

            Ok(Rc::new(RefCell::new(Node::Internal(internal))))
        }
        _ => Err(MVPError::Unrecognized),
    }
}

fn read_datapoint_at(
    bytes: &Vec<u8>,
    offset: u64,
    path_length: usize,
    datatype: MVPDataType,
) -> Result<Arc<MVPDatapoint>, MVPError> {
    let mut cursor = Cursor::new(bytes.clone());
    cursor.set_position(offset);
    read_datapoint_bytes(&mut cursor, path_length, datatype)?.ok_or(MVPError::FileOpen)
}

fn print_node(stream: &mut dyn Write, tree: &MVPTree, node: &Rc<RefCell<Node>>, level: usize) -> MVPError {
    let borrowed = node.borrow();
    match &*borrowed {
        Node::Leaf(leaf) => {
            let _ = writeln!(stream, "LEAF{}  ({} points)", level, leaf.nbpoints);
            if let Some(sv1) = &leaf.sv1 {
                let _ = writeln!(stream, "    sv1: {}", sv1.id);
            }
            if let Some(sv2) = &leaf.sv2 {
                let _ = writeln!(stream, "    sv2: {}", sv2.id);
            }
            for (i, point) in leaf.points.iter().enumerate() {
                let _ = writeln!(stream, "        point[{}]: {}", i, point.id);
            }
            MVPError::Success
        }
        Node::Internal(internal) => {
            let _ = writeln!(stream, "INTERNAL{}", level);
            if let Some(sv1) = &internal.sv1 {
                let _ = writeln!(stream, "  sv1: {}", sv1.id);
            }
            if let Some(sv2) = &internal.sv2 {
                let _ = writeln!(stream, "  sv2: {}", sv2.id);
            }
            for (i, value) in internal.m1.iter().enumerate() {
                let _ = write!(stream, "  M1[{}] = {:.4};", i, value);
            }
            for (i, value) in internal.m2.iter().enumerate() {
                let _ = write!(stream, "  M2[{}] = {:.4};", i, value);
            }
            let _ = writeln!(stream);
            for child in &internal.child_nodes {
                let err = print_node(stream, tree, child, level + 2);
                if err != MVPError::Success {
                    return err;
                }
            }
            MVPError::Success
        }
    }
}
