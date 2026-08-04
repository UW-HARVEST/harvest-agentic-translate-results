use mvptree::mvptree::*;

fn l1(a: &MVPDatapoint, b: &MVPDatapoint) -> f32 {
    let mut sum: u32 = 0;
    let n = a.datalen.min(a.data.len()).min(b.data.len());
    for i in 0..n {
        let x = a.data[i] as i32;
        let y = b.data[i] as i32;
        sum += (x - y).unsigned_abs();
    }
    if a.datalen == 0 {
        return 0.0;
    }
    sum as f32 / a.datalen as f32
}

fn l2(a: &MVPDatapoint, b: &MVPDatapoint) -> f32 {
    if a.datalen != b.datalen {
        return -2.0;
    }
    let mut sum: i32 = 0;
    let n = a.datalen.min(a.data.len()).min(b.data.len());
    for i in 0..n {
        let x = a.data[i] as i32;
        let y = b.data[i] as i32;
        let diff = x - y;
        sum += diff * diff;
    }
    (sum as f32).sqrt() / a.datalen as f32
}

fn make_dp(id: &str, bytes: &[u8]) -> MVPDatapoint {
    MVPDatapoint::new(id.to_string(), bytes.to_vec(), MVPDataType::ByteArray)
}

#[test]
fn test_constants() {
    assert_eq!(TAG, "phashmvp2010");
    assert_eq!(TAG.len(), 12);
    assert_eq!(VERSION, 0x01000000);
    assert_eq!(HEADER_SIZE, 32);
    assert_eq!(FILE_OFFSET_BITS, 64);
    assert_eq!(ERROR_MSGS.len(), 25);
}

#[test]
fn test_error_messages() {
    // Verify all error messages match C exactly
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
    assert_eq!(error_to_string(MVPError::Munmap), "unmap eror");
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
fn test_data_types() {
    assert_eq!(MVPDataType::ByteArray as u8, 1);
    assert_eq!(MVPDataType::UInt16Array as u8, 2);
    assert_eq!(MVPDataType::UInt32Array as u8, 4);
    assert_eq!(MVPDataType::UInt64Array as u8, 8);

    assert_eq!(MVPDataType::ByteArray.byte_width(), 1);
    assert_eq!(MVPDataType::UInt16Array.byte_width(), 2);
    assert_eq!(MVPDataType::UInt32Array.byte_width(), 4);
    assert_eq!(MVPDataType::UInt64Array.byte_width(), 8);

    assert_eq!(MVPDataType::from_u8(1), Some(MVPDataType::ByteArray));
    assert_eq!(MVPDataType::from_u8(2), Some(MVPDataType::UInt16Array));
    assert_eq!(MVPDataType::from_u8(4), Some(MVPDataType::UInt32Array));
    assert_eq!(MVPDataType::from_u8(8), Some(MVPDataType::UInt64Array));
    assert_eq!(MVPDataType::from_u8(3), None);
    assert_eq!(MVPDataType::from_u8(0), None);
    assert_eq!(MVPDataType::from_u8(5), None);
    assert_eq!(MVPDataType::from_u8(255), None);
}

#[test]
fn test_node_types() {
    assert_eq!(NodeType::InternalNode as u8, 1);
    assert_eq!(NodeType::LeafNode as u8, 2);
}

#[test]
fn test_l1_distance() {
    // Verified via C: L1([1,2,3,4,5], [2,4,6,8,10]) = 3.0
    let a = make_dp("a", &[1, 2, 3, 4, 5]);
    let b = make_dp("b", &[2, 4, 6, 8, 10]);
    assert_eq!(l1(&a, &b), 3.0);

    // L1(a, a) == 0
    let c = make_dp("c", &[1, 2, 3, 4, 5]);
    assert_eq!(l1(&a, &c), 0.0);
}

#[test]
fn test_datapoint_new() {
    let dp = MVPDatapoint::new("hello".to_string(), vec![1, 2, 3], MVPDataType::ByteArray);
    assert_eq!(dp.id, "hello");
    assert_eq!(dp.data, vec![1, 2, 3]);
    assert_eq!(dp.datalen, 3);
    assert_eq!(dp.data_type, MVPDataType::ByteArray);
    assert_eq!(dp.path.len(), 0);
}

#[test]
fn test_internal_node_new() {
    let n = InternalNode::new(2);
    assert_eq!(n.node_type, NodeType::InternalNode);
    assert!(n.sv1.is_none());
    assert!(n.sv2.is_none());
    assert_eq!(n.m1.len(), 1); // bf - 1 = 1
    assert_eq!(n.m2.len(), 2); // bf*(bf-1) = 2
    // bf*bf=4 capacity reserved for child_nodes
    assert_eq!(n.child_nodes.len(), 0);

    let n3 = InternalNode::new(3);
    assert_eq!(n3.m1.len(), 2);
    assert_eq!(n3.m2.len(), 6);
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
}

#[test]
fn test_mvptree_new() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    assert_eq!(tree.branch_factor, 2);
    assert_eq!(tree.path_length, 5);
    assert_eq!(tree.leaf_capacity, 25);
    assert_eq!(tree.datatype, MVPDataType::ByteArray);
    assert_eq!(tree.pos, 0);
    assert_eq!(tree.size, 0);
    assert_eq!(tree.pgsize, 4096);
    assert!(tree.buf.is_empty());
    assert!(tree.node.is_none());
}

#[test]
fn test_add_empty() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let err = tree.add(vec![]);
    assert_eq!(err, MVPError::Success);
    assert!(tree.node.is_none());
}

#[test]
fn test_add_single_point() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp = make_dp("only", &[10, 20, 30, 40, 50]);
    let err = tree.add(vec![dp]);
    assert_eq!(err, MVPError::Success);
    assert!(tree.node.is_some());
}

#[test]
fn test_retrieve_empty_tree() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let target = make_dp("t", &[1, 2, 3, 4, 5]);
    let res = tree.retrieve(&target, 10, 1.0);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), MVPError::EmptyTree);
}

#[test]
fn test_retrieve_zero_knearest() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp = make_dp("a", &[1, 2, 3, 4, 5]);
    tree.add(vec![dp]);
    let target = make_dp("t", &[1, 2, 3, 4, 5]);
    let res = tree.retrieve(&target, 0, 1.0);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), MVPError::ArgErr);
}

#[test]
fn test_retrieve_negative_radius() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp = make_dp("a", &[1, 2, 3, 4, 5]);
    tree.add(vec![dp]);
    let target = make_dp("t", &[1, 2, 3, 4, 5]);
    let res = tree.retrieve(&target, 10, -1.0);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), MVPError::ArgErr);
}

#[test]
fn test_retrieve_single_point_match() {
    // Tree with single point. Retrieve same data should find it.
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp = make_dp("only", &[10, 20, 30, 40, 50]);
    tree.add(vec![dp]);
    let target = make_dp("q", &[10, 20, 30, 40, 50]);
    let res = tree.retrieve(&target, 5, 1.0).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "only");
}

#[test]
fn test_retrieve_single_point_mismatch() {
    // Distant target should produce no results.
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp = make_dp("only", &[10, 20, 30, 40, 50]);
    tree.add(vec![dp]);
    let target = make_dp("q", &[200, 200, 200, 200, 200]);
    let res = tree.retrieve(&target, 5, 1.0).unwrap();
    assert_eq!(res.len(), 0);
}

#[test]
fn test_type_mismatch() {
    // Add a byte-array first, then try uint16 - should fail
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp1 = make_dp("p1", &[1, 2, 3, 4, 5]);
    assert_eq!(tree.add(vec![dp1]), MVPError::Success);

    let dp2 = MVPDatapoint::new("p2".to_string(), vec![0; 4], MVPDataType::UInt16Array);
    let err = tree.add(vec![dp2]);
    assert_eq!(err, MVPError::TypeMismatch);
}

#[test]
fn test_add_and_retrieve_many() {
    // Build a tree and verify retrievals match
    let mut tree = MVPTree::new(2, 5, 5, MVPDataType::ByteArray, l1);
    let mut pts = Vec::new();
    for k in 0..20u8 {
        let buf = [k.wrapping_mul(5), k.wrapping_mul(7), k];
        pts.push(make_dp(&format!("p{:02}", k), &buf));
    }
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);

    // C ground truth:
    // r=1.0 query at p10's data finds: p10 (1 result)
    // r=10.0 finds: p10 p08 p09 p11 p12 (5 results)
    let q = make_dp("q", &[50, 70, 10]); // p10 data
    let res1 = tree.retrieve(&q, 30, 1.0).unwrap();
    assert_eq!(res1.len(), 1);
    let ids: Vec<&str> = res1.iter().map(|d| d.id.as_str()).collect();
    assert!(ids.contains(&"p10"));

    let res5 = tree.retrieve(&q, 30, 10.0).unwrap();
    assert_eq!(res5.len(), 5);
    let ids: Vec<String> = res5.iter().map(|d| d.id.clone()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(sorted, vec!["p08", "p09", "p10", "p11", "p12"]);

    // Find all
    let res_all = tree.retrieve(&q, 30, 10000.0).unwrap();
    assert_eq!(res_all.len(), 20);

    // knearest cap: k=3 with huge radius -> 3 results
    let res_k3 = tree.retrieve(&q, 3, 10000.0).unwrap();
    assert_eq!(res_k3.len(), 3);
}

#[test]
fn test_write_read_roundtrip() {
    let mut tree = MVPTree::new(2, 5, 5, MVPDataType::ByteArray, l1);
    let mut pts = Vec::new();
    for k in 0..15u8 {
        let buf = [k.wrapping_mul(5), k.wrapping_mul(7), k];
        pts.push(make_dp(&format!("p{:02}", k), &buf));
    }
    tree.add(pts);

    let path = "/tmp/test_mvptree_rt.mvp";
    let _ = std::fs::remove_file(path);
    let werr = tree.write(path, 0o644);
    assert_eq!(werr, MVPError::Success);

    let tree2 = mvptree_read(path, l1).unwrap();
    assert_eq!(tree2.branch_factor, 2);
    assert_eq!(tree2.path_length, 5);
    assert_eq!(tree2.leaf_capacity, 5);
    assert_eq!(tree2.datatype, MVPDataType::ByteArray);

    // Retrieve from re-read tree should give same results
    let q = make_dp("q", &[50, 70, 10]); // p10
    let res = tree2.retrieve(&q, 30, 1.0).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "p10");
}

#[test]
fn test_read_missing_file() {
    let res = mvptree_read("/tmp/no_such_file_for_mvptree_xyzabc.mvp", l1);
    assert!(res.is_err());
    match res {
        Err(e) => assert_eq!(e, MVPError::FileNotFound),
        Ok(_) => panic!("expected error"),
    }
}

#[test]
fn test_clear() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp = make_dp("a", &[1, 2, 3, 4, 5]);
    tree.add(vec![dp]);
    assert!(tree.node.is_some());
    let mut empty: Option<Box<Node>> = None;
    tree.clear(&mut empty);
    assert!(tree.node.is_none());
}

#[test]
fn test_print_basic() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let dp = make_dp("only", &[10, 20, 30, 40, 50]);
    tree.add(vec![dp]);
    let mut out: Vec<u8> = Vec::new();
    let err = tree.print(&mut out);
    assert_eq!(err, MVPError::Success);
    let s = String::from_utf8_lossy(&out);
    // Should contain LEAF marker and the id
    assert!(s.contains("LEAF"), "expected LEAF marker, got: {}", s);
    assert!(s.contains("only"), "expected id 'only' in output, got: {}", s);
}

#[test]
fn test_extend_mvpfile() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    tree.pgsize = 4096;
    let r = tree.extend_mvpfile();
    assert_eq!(r, 0);
    assert_eq!(tree.size, 4096);
    assert_eq!(tree.buf.len(), 4096);
}

#[test]
fn test_dp_select_vantage_points_zero() {
    let mut dp = make_dp("a", &[1, 2, 3]);
    let r = dp.select_vantage_points(0, 0, -1, l1);
    assert_eq!(r, -1);
}

#[test]
fn test_dp_select_vantage_points_nonzero() {
    let mut dp = make_dp("a", &[1, 2, 3]);
    let r = dp.select_vantage_points(5, 0, -1, l1);
    assert_eq!(r, 0);
}

#[test]
fn test_dp_find_distance_range_for_vp_zero() {
    let mut dp = make_dp("a", &[1, 2, 3]);
    let vp = make_dp("v", &[1, 2, 3]);
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let r = dp.find_distance_range_for_vp(0, &vp, &tree, 0);
    assert_eq!(r, -1);
    let r2 = dp.find_distance_range_for_vp(1, &vp, &tree, -1);
    assert_eq!(r2, -1);
}

#[test]
fn test_dp_find_distance_range_for_vp_writes_path() {
    let mut dp = make_dp("a", &[1, 2, 3]);
    let vp = make_dp("v", &[2, 4, 6]); // L1 distance = (1+2+3)/3 = 2.0
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let r = dp.find_distance_range_for_vp(1, &vp, &tree, 2);
    assert_eq!(r, 0);
    assert_eq!(dp.path.len(), 5);
    assert_eq!(dp.path[2], 2.0);
}

#[test]
fn test_dp_find_splits_zero() {
    let mut dp = make_dp("a", &[1, 2, 3]);
    let vp = make_dp("v", &[2, 4, 6]);
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let r = dp.find_splits(0, &vp, &tree, 1);
    assert_eq!(r, -1.0);
}

#[test]
fn test_dp_find_splits_nonzero() {
    let mut dp = make_dp("a", &[1, 2, 3]);
    let vp = make_dp("v", &[2, 4, 6]);
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    let r = dp.find_splits(1, &vp, &tree, 1);
    // L1(dp, vp) = (1+2+3)/3 = 2.0
    assert_eq!(r, 2.0);
}

#[test]
fn test_dp_write_returns_bytelength() {
    let dp = make_dp("hello", &[1, 2, 3, 4, 5]); // id len 5, datalen 5, type=1
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, l1);
    // bytelength = 1 (idlen) + 5 (id) + 4 (datalen) + 5*1 (data) + 5*4 (path) = 35
    let r = dp.write(&tree);
    assert_eq!(r, 35);
}

#[test]
fn test_dp_sort_points_creates_bins() {
    let mut dp = make_dp("a", &[1, 2, 3]);
    let vp = make_dp("v", &[2, 4, 6]);
    let tree = MVPTree::new(3, 5, 25, MVPDataType::ByteArray, l1);
    let mut counts: Vec<Vec<i32>> = Vec::new();
    let bins = dp.sort_points(0, -1, -1, &vp, &tree, &mut counts, vec![]);
    // bf=3, so 3x3 bins
    assert_eq!(bins.len(), 3);
    for row in &bins {
        assert_eq!(row.len(), 3);
        for inner in row {
            assert_eq!(inner.len(), 0);
        }
    }
    // counts gets set to 3x3 zero matrix
    assert_eq!(counts.len(), 3);
    for row in &counts {
        assert_eq!(row.len(), 3);
        assert!(row.iter().all(|&x| x == 0));
    }
}

fn main() {}
