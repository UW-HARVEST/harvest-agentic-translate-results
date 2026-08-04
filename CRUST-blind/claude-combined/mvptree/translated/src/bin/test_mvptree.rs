use mvptree::mvptree::{
    error_to_string, InternalNode, LeafNode, MVPDataType, MVPDatapoint, MVPError, MVPTree, Node,
    NodeType, ERROR_MSGS, HEADER_SIZE, TAG, VERSION,
};

fn l1_distance(a: &MVPDatapoint, b: &MVPDatapoint) -> f32 {
    let mut sum: u32 = 0;
    let n = a.data.len();
    for i in 0..n {
        let d1 = a.data[i] as i32;
        let d2 = b.data[i] as i32;
        sum += (d1 - d2).unsigned_abs();
    }
    (sum as f32) / (a.data.len() as f32)
}

fn make_point(id: &str, data: Vec<u8>) -> MVPDatapoint {
    MVPDatapoint::new(id.to_string(), data, MVPDataType::ByteArray)
}

#[test]
fn test_constants() {
    assert_eq!(TAG, "phashmvp2010");
    assert_eq!(VERSION, 0x01000000);
    assert_eq!(HEADER_SIZE, 32);
    assert_eq!(ERROR_MSGS.len(), 25);
    assert_eq!(ERROR_MSGS[0], "no error");
    assert_eq!(ERROR_MSGS[1], "bad argument");
    assert_eq!(ERROR_MSGS[24], "unrecognized node");
}

#[test]
fn test_error_to_string_all() {
    assert_eq!(error_to_string(MVPError::Success), "no error");
    assert_eq!(error_to_string(MVPError::ArgErr), "bad argument");
    assert_eq!(error_to_string(MVPError::NoDistanceFunc), "no distance function found");
    assert_eq!(error_to_string(MVPError::MemAlloc), "mem alloc error");
    assert_eq!(error_to_string(MVPError::NoLeaf), "no leaf node created");
    assert_eq!(error_to_string(MVPError::NoInternal), "no internal node created");
    assert_eq!(error_to_string(MVPError::PathAlloc), "no path array alloc'd");
    assert_eq!(error_to_string(MVPError::VpNoSelect), "could not select vantage points");
    assert_eq!(error_to_string(MVPError::NoSv1Range), "could not calculate range from an sv1");
    assert_eq!(error_to_string(MVPError::NoSv2Range), "could not calculate range from an sv2");
    assert_eq!(error_to_string(MVPError::NoSpace), "points too compact");
    assert_eq!(error_to_string(MVPError::NoSort), "could not sort points");
    assert_eq!(error_to_string(MVPError::FileOpen), "could not open file");
    assert_eq!(error_to_string(MVPError::FileClose), "could not close file");
    assert_eq!(error_to_string(MVPError::MemMap), "mmap error");
    assert_eq!(error_to_string(MVPError::NoWrite), "no write");
    assert_eq!(error_to_string(MVPError::FileTruncate), "could not extend file");
    assert_eq!(error_to_string(MVPError::MremapFail), "could not remap file");
    assert_eq!(error_to_string(MVPError::TypeMismatch), "datatypes in conflict");
    assert_eq!(error_to_string(MVPError::KNearestCap), "no. retrieved exceeds k");
    assert_eq!(error_to_string(MVPError::EmptyTree), "empty tree");
    assert_eq!(error_to_string(MVPError::NoSplits), "distance value either NaN or less than zero");
    assert_eq!(error_to_string(MVPError::BadDistVal), "could not open file");
    assert_eq!(error_to_string(MVPError::FileNotFound), "unrecognized node");
}

#[test]
fn test_datatype_values() {
    assert_eq!(MVPDataType::ByteArray as u8, 1);
    assert_eq!(MVPDataType::UInt16Array as u8, 2);
    assert_eq!(MVPDataType::UInt32Array as u8, 4);
    assert_eq!(MVPDataType::UInt64Array as u8, 8);
}

#[test]
fn test_nodetype_values() {
    assert_eq!(NodeType::InternalNode as u8, 1);
    assert_eq!(NodeType::LeafNode as u8, 2);
}

#[test]
fn test_mvpdatapoint_new() {
    let dp = MVPDatapoint::new("hello".to_string(), vec![1, 2, 3, 4, 5], MVPDataType::ByteArray);
    assert_eq!(dp.id, "hello");
    assert_eq!(dp.data, vec![1, 2, 3, 4, 5]);
    assert_eq!(dp.datalen, 5);
    assert_eq!(dp.data_type, MVPDataType::ByteArray);
    assert_eq!(dp.path.len(), 0);
}

#[test]
fn test_leaf_node_new() {
    let leaf = LeafNode::new(25);
    assert_eq!(leaf.node_type, NodeType::LeafNode);
    assert!(leaf.sv1.is_none());
    assert!(leaf.sv2.is_none());
    assert_eq!(leaf.points.len(), 0);
    assert_eq!(leaf.d1.len(), 25);
    assert_eq!(leaf.d2.len(), 25);
    assert_eq!(leaf.nbpoints, 0);
    for v in &leaf.d1 {
        assert_eq!(*v, 0.0);
    }
    for v in &leaf.d2 {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_internal_node_new() {
    let internal = InternalNode::new(2);
    assert_eq!(internal.node_type, NodeType::InternalNode);
    assert!(internal.sv1.is_none());
    assert!(internal.sv2.is_none());
    assert_eq!(internal.m1.len(), 1); // bf-1 = 1
    assert_eq!(internal.m2.len(), 2); // (bf-1)*bf = 2
    assert_eq!(internal.child_nodes.len(), 0);
    for v in &internal.m1 {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_internal_node_bf3() {
    let internal = InternalNode::new(3);
    assert_eq!(internal.m1.len(), 2);
    assert_eq!(internal.m2.len(), 6);
}

#[test]
fn test_distance_function_l1() {
    let a = make_point("A", vec![0, 0, 0, 0, 0]);
    let b = make_point("B", vec![1, 1, 1, 1, 1]);
    let d = l1_distance(&a, &b);
    // (5 / 5) = 1.0, matches C
    assert_eq!(d, 1.0);
}

#[test]
fn test_mvptree_new() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    assert_eq!(tree.branch_factor, 2);
    assert_eq!(tree.path_length, 5);
    assert_eq!(tree.leaf_capacity, 25);
    assert_eq!(tree.datatype, MVPDataType::ByteArray);
    assert_eq!(tree.pos, 0);
    assert_eq!(tree.size, 0);
    assert!(tree.node.is_none());
    assert_eq!(tree.buf.len(), 0);
}

#[test]
fn test_mvptree_add_empty() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let err = tree.add(vec![]);
    assert_eq!(err, MVPError::Success);
    assert!(tree.node.is_none());
}

#[test]
fn test_mvptree_add_small() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..5 {
        let data = vec![i as u8; 5];
        pts.push(make_point(&format!("pt{}", i), data));
    }
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);
    assert!(tree.node.is_some());
}

#[test]
fn test_mvptree_retrieve_empty_tree() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let target = make_point("Q", vec![0, 0, 0, 0, 0]);
    let r = tree.retrieve(&target, 5, 1.0);
    assert!(r.is_err());
    assert_eq!(r.err().unwrap(), MVPError::EmptyTree);
}

#[test]
fn test_mvptree_retrieve_argerr_zero_k() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let target = make_point("Q", vec![0, 0, 0, 0, 0]);
    let r = tree.retrieve(&target, 0, 1.0);
    assert!(r.is_err());
    assert_eq!(r.err().unwrap(), MVPError::ArgErr);
}

#[test]
fn test_mvptree_retrieve_argerr_negative_radius() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let target = make_point("Q", vec![0, 0, 0, 0, 0]);
    let r = tree.retrieve(&target, 5, -1.0);
    assert!(r.is_err());
    assert_eq!(r.err().unwrap(), MVPError::ArgErr);
}

#[test]
fn test_mvptree_retrieve_radius_1() {
    // C reference: pt0..pt4 with data[i]=i,i,i,i,i, query Q={2,2,2,2,2}, radius=1.0
    // Expected (from C): pt1, pt2, pt3 (3 results)
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..5 {
        let data = vec![i as u8; 5];
        pts.push(make_point(&format!("pt{}", i), data));
    }
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);

    let target = make_point("Q", vec![2u8; 5]);
    let res = tree.retrieve(&target, 10, 1.0).unwrap();
    let ids: Vec<String> = res.iter().map(|d| d.id.clone()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(res.len(), 3);
    assert_eq!(sorted, vec!["pt1".to_string(), "pt2".to_string(), "pt3".to_string()]);
}

#[test]
fn test_mvptree_retrieve_radius_0_exact() {
    // C reference: query exactly pt2, radius=0 => 1 result (pt2)
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..5 {
        let data = vec![i as u8; 5];
        pts.push(make_point(&format!("pt{}", i), data));
    }
    tree.add(pts);

    let target = make_point("Q", vec![2u8; 5]);
    let res = tree.retrieve(&target, 10, 0.0).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "pt2");
}

#[test]
fn test_mvptree_retrieve_radius_5_all() {
    // C reference: radius=5.0 => 5 results (all pts)
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..5 {
        let data = vec![i as u8; 5];
        pts.push(make_point(&format!("pt{}", i), data));
    }
    tree.add(pts);

    let target = make_point("Q", vec![2u8; 5]);
    let res = tree.retrieve(&target, 10, 5.0).unwrap();
    let mut ids: Vec<String> = res.iter().map(|d| d.id.clone()).collect();
    ids.sort();
    assert_eq!(res.len(), 5);
    assert_eq!(
        ids,
        vec![
            "pt0".to_string(),
            "pt1".to_string(),
            "pt2".to_string(),
            "pt3".to_string(),
            "pt4".to_string()
        ]
    );
}

#[test]
fn test_mvptree_retrieve_exact_pt3_r0() {
    // C reference: query={3,3,3,3,3}, radius=0 => 1 result pt3
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..5 {
        let data = vec![i as u8; 5];
        pts.push(make_point(&format!("pt{}", i), data));
    }
    tree.add(pts);

    let target = make_point("Q3", vec![3u8; 5]);
    let res = tree.retrieve(&target, 10, 0.0).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "pt3");
}

#[test]
fn test_mvptree_typemismatch() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    // Add mismatched type
    let bad = MVPDatapoint::new("X".to_string(), vec![0, 0, 0, 0, 0], MVPDataType::UInt32Array);
    let err = tree.add(vec![bad]);
    assert_eq!(err, MVPError::TypeMismatch);
}

#[test]
fn test_mvptree_internal_node_tree() {
    // Larger tree (lc=5, M=20) to trigger internal nodes.
    // C reference: retrieve query {25,15,35,5,10} radius=100 => 20 results.
    let mut tree = MVPTree::new(2, 5, 5, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..20 {
        let data = vec![
            (i * 5) as u8,
            (i * 3) as u8,
            (i * 7) as u8,
            i as u8,
            (i * 2) as u8,
        ];
        pts.push(make_point(&format!("p{}", i), data));
    }
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);

    let target = make_point("Q", vec![25, 15, 35, 5, 10]);
    let res = tree.retrieve(&target, 100, 100.0).unwrap();
    assert_eq!(res.len(), 20);
}

#[test]
fn test_extend_mvpfile() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    tree.pgsize = 16;
    tree.size = 0;
    let r = tree.extend_mvpfile();
    assert_eq!(r, 0);
    assert_eq!(tree.size, 16);
    assert_eq!(tree.buf.len(), 16);
}

#[test]
fn test_print_node() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..3 {
        let data = vec![i as u8; 5];
        pts.push(make_point(&format!("pt{}", i), data));
    }
    tree.add(pts);
    let mut buf: Vec<u8> = Vec::new();
    let err = tree.print(&mut buf);
    assert_eq!(err, MVPError::Success);
    let s = String::from_utf8(buf).unwrap();
    // Must contain the LEAF identifier
    assert!(s.contains("LEAF"));
}

#[test]
fn test_dp_path_after_add() {
    // After adding, the points' path field must be allocated to path_length
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let mut pts = Vec::new();
    for i in 0..3 {
        let data = vec![i as u8; 5];
        pts.push(make_point(&format!("pt{}", i), data));
    }
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);
    // Check that there's a node and points have their path
    let node = tree.node.as_ref().unwrap();
    let nref = node.borrow();
    if let Node::Leaf(l) = &*nref {
        if let Some(sv1) = &l.sv1 {
            assert_eq!(sv1.path.len(), 5);
        }
    }
}

#[test]
fn test_select_vantage_points_method() {
    let mut dp = make_point("X", vec![1, 2, 3, 4, 5]);
    let r = dp.select_vantage_points(1, 0, -1, l1_distance);
    assert_eq!(r, 0);
}

#[test]
fn test_find_distance_range_method() {
    let mut dp = make_point("X", vec![1, 2, 3, 4, 5]);
    dp.path = vec![0.0; 5];
    let vp = make_point("V", vec![0, 0, 0, 0, 0]);
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let r = dp.find_distance_range_for_vp(1, &vp, &tree, 0);
    assert_eq!(r, 0);
    // L1((1,2,3,4,5),(0,0,0,0,0)) = (1+2+3+4+5)/5 = 3.0
    assert_eq!(dp.path[0], 3.0);
}

#[test]
fn test_find_splits_method() {
    let mut dp = make_point("X", vec![1, 2, 3, 4, 5]);
    let vp = make_point("V", vec![0, 0, 0, 0, 0]);
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let r = dp.find_splits(1, &vp, &tree, 1);
    assert_eq!(r, 3.0);
}

#[test]
fn test_write_method() {
    let dp = make_point("hi", vec![1, 2, 3]);
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1_distance);
    let n = dp.write(&tree);
    // 1 + 4 + 1 + 2 + 4 + 3 + 5*4 = 35
    assert_eq!(n, 35);
}

fn main() {}
