use mvptree::mvptree::*;

// L2 distance matching the C testmvp.c byte-array version
fn point_l2_distance(a: &MVPDatapoint, b: &MVPDatapoint) -> f32 {
    let mut sum: u32 = 0;
    for i in 0..a.datalen.min(b.datalen) {
        let d1 = a.data[i] as i32;
        let d2 = b.data[i] as i32;
        let diff = d1 - d2;
        sum += (diff * diff) as u32;
    }
    (sum as f32).sqrt() / a.datalen as f32
}

fn make_point(id: &str, data: &[u8]) -> MVPDatapoint {
    MVPDatapoint::new(id.to_string(), data.to_vec(), MVPDataType::ByteArray)
}

// ---- error_to_string tests ----

#[test]
fn test_error_to_string_success() {
    assert_eq!(error_to_string(MVPError::Success), "no error");
}

#[test]
fn test_error_to_string_argerr() {
    assert_eq!(error_to_string(MVPError::ArgErr), "bad argument");
}

#[test]
fn test_error_to_string_no_distance_func() {
    assert_eq!(error_to_string(MVPError::NoDistanceFunc), "no distance function found");
}

#[test]
fn test_error_to_string_type_mismatch() {
    assert_eq!(error_to_string(MVPError::TypeMismatch), "datatypes in conflict");
}

#[test]
fn test_error_to_string_empty_tree() {
    assert_eq!(error_to_string(MVPError::EmptyTree), "empty tree");
}

#[test]
fn test_error_to_string_knearest_cap() {
    assert_eq!(error_to_string(MVPError::KNearestCap), "no. retrieved exceeds k");
}

#[test]
fn test_error_to_string_file_not_found() {
    // C has a bug: MVP_FILENOTFOUND=24 maps to error_msgs[24]="unrecognized node"
    // The Rust code replicates this behavior
    assert_eq!(error_to_string(MVPError::FileNotFound), "unrecognized node");
}

// ---- constants ----

#[test]
fn test_tag() {
    assert_eq!(TAG, "phashmvp2010");
}

#[test]
fn test_version() {
    assert_eq!(VERSION, 0x01000000);
}

#[test]
fn test_header_size() {
    assert_eq!(HEADER_SIZE, 32);
}

#[test]
fn test_error_msgs_length() {
    assert_eq!(ERROR_MSGS.len(), 25);
}

// ---- MVPDataType ----

#[test]
fn test_datatype_values() {
    assert_eq!(MVPDataType::ByteArray as u32, 1);
    assert_eq!(MVPDataType::UInt16Array as u32, 2);
    assert_eq!(MVPDataType::UInt32Array as u32, 4);
    assert_eq!(MVPDataType::UInt64Array as u32, 8);
}

// ---- NodeType ----

#[test]
fn test_nodetype_values() {
    assert_eq!(NodeType::InternalNode as u8, 1);
    assert_eq!(NodeType::LeafNode as u8, 2);
}

// ---- MVPDatapoint::new ----

#[test]
fn test_datapoint_new() {
    let dp = MVPDatapoint::new("test".to_string(), vec![1, 2, 3], MVPDataType::ByteArray);
    assert_eq!(dp.id, "test");
    assert_eq!(dp.data, vec![1, 2, 3]);
    assert_eq!(dp.datalen, 3);
    assert_eq!(dp.data_type, MVPDataType::ByteArray);
    assert!(dp.path.is_empty());
}

// ---- MVPTree::new ----

#[test]
fn test_tree_new() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    assert_eq!(tree.branch_factor, 2);
    assert_eq!(tree.path_length, 5);
    assert_eq!(tree.leaf_capacity, 25);
    assert!(tree.node.is_none());
}

// ---- add empty ----

#[test]
fn test_add_empty() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let err = tree.add(vec![]);
    assert_eq!(err, MVPError::Success);
    assert!(tree.node.is_none());
}

// ---- add single point ----

#[test]
fn test_add_single_point() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let p = make_point("p1", &[1, 2, 3, 4]);
    let err = tree.add(vec![p]);
    assert_eq!(err, MVPError::Success);
    assert!(tree.node.is_some());
}

// ---- type mismatch ----

#[test]
fn test_type_mismatch() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let p1 = make_point("p1", &[1, 2, 3, 4]);
    tree.add(vec![p1]);

    let p2 = MVPDatapoint::new("p2".to_string(), vec![0; 8], MVPDataType::UInt16Array);
    let err = tree.add(vec![p2]);
    assert_eq!(err, MVPError::TypeMismatch);
}

// ---- retrieve on empty tree ----

#[test]
fn test_retrieve_empty_tree() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let target = make_point("q", &[1, 2, 3, 4]);
    let result = tree.retrieve(&target, 10, 5.0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), MVPError::EmptyTree);
}

// ---- retrieve with knearest=0 ----

#[test]
fn test_retrieve_knearest_zero() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let target = make_point("q", &[1, 2, 3, 4]);
    let result = tree.retrieve(&target, 0, 5.0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), MVPError::ArgErr);
}

// ---- retrieve with negative radius ----

#[test]
fn test_retrieve_negative_radius() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let target = make_point("q", &[1, 2, 3, 4]);
    let result = tree.retrieve(&target, 10, -1.0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), MVPError::ArgErr);
}

// ---- small tree: add and retrieve ----

#[test]
fn test_small_tree_retrieve() {
    // Matches C test_basic.c behavior
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("p1", &[1, 2, 3, 4]),
        make_point("p2", &[2, 3, 4, 5]),
        make_point("p3", &[1, 2, 3, 5]),
        make_point("p4", &[100, 200, 150, 250]),
    ];
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);

    // Retrieve near p1 with radius 2.0 — C returns p1, p2, p3
    let query = make_point("query", &[1, 2, 3, 4]);
    let results = tree.retrieve(&query, 10, 2.0).unwrap();
    assert_eq!(results.len(), 3);
    let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"p1"));
    assert!(ids.contains(&"p2"));
    assert!(ids.contains(&"p3"));
}

#[test]
fn test_retrieve_radius_zero() {
    // C: retrieve r=0 returns only exact match "p1"
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("p1", &[1, 2, 3, 4]),
        make_point("p2", &[2, 3, 4, 5]),
        make_point("p3", &[1, 2, 3, 5]),
        make_point("p4", &[100, 200, 150, 250]),
    ];
    tree.add(pts);

    let query = make_point("query", &[1, 2, 3, 4]);
    let results = tree.retrieve(&query, 10, 0.0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "p1");
}

#[test]
fn test_retrieve_large_radius() {
    // C: retrieve r=1000 returns all 4 points
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("p1", &[1, 2, 3, 4]),
        make_point("p2", &[2, 3, 4, 5]),
        make_point("p3", &[1, 2, 3, 5]),
        make_point("p4", &[100, 200, 150, 250]),
    ];
    tree.add(pts);

    let query = make_point("query", &[1, 2, 3, 4]);
    let results = tree.retrieve(&query, 10, 1000.0).unwrap();
    assert_eq!(results.len(), 4);
}

// ---- knearest cap ----

#[test]
fn test_knearest_cap() {
    // C: knearest=2, radius=1000 returns 2 results with KNearestCap
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("pt0", &[10, 20, 30, 40]),
        make_point("pt1", &[11, 21, 31, 41]),
        make_point("pt2", &[50, 60, 70, 80]),
        make_point("pt3", &[51, 61, 71, 81]),
        make_point("pt4", &[100, 110, 120, 130]),
    ];
    tree.add(pts);

    let query = make_point("q", &[10, 20, 30, 40]);
    // KNearestCap is treated as success by retrieve(), results are capped
    let results = tree.retrieve(&query, 2, 1000.0).unwrap();
    assert_eq!(results.len(), 2);
}

// ---- distance function ----

#[test]
fn test_l2_distance() {
    let a = make_point("a", &[0, 0, 0, 0]);
    let b = make_point("b", &[3, 4, 0, 0]);
    let d = point_l2_distance(&a, &b);
    // sqrt(9+16)/4 = 5/4 = 1.25
    assert!((d - 1.25).abs() < 1e-5);
}

#[test]
fn test_l2_distance_identical() {
    let a = make_point("a", &[5, 10, 15, 20]);
    let d = point_l2_distance(&a, &a);
    assert_eq!(d, 0.0);
}

// ---- print ----

#[test]
fn test_print_small_tree() {
    // C output for 5 points: LEAF0 with sv1=pt0, sv2=pt4, 3 points
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("pt0", &[10, 20, 30, 40]),
        make_point("pt1", &[11, 21, 31, 41]),
        make_point("pt2", &[50, 60, 70, 80]),
        make_point("pt3", &[51, 61, 71, 81]),
        make_point("pt4", &[100, 110, 120, 130]),
    ];
    tree.add(pts);

    let mut buf = Vec::new();
    let err = tree.print(&mut buf);
    assert_eq!(err, MVPError::Success);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("LEAF0"));
    assert!(output.contains("pt0"));
    assert!(output.contains("pt4"));
}

#[test]
fn test_print_null_tree() {
    // C: mvptree_print(stdout, NULL) returns MVP_ARGERR
    // In Rust, print on tree with no node should handle gracefully
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let mut buf = Vec::new();
    let err = tree.print(&mut buf);
    // With no node, it prints "NULL0"
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("NULL0"));
}

// ---- write and read ----

#[test]
fn test_write_and_read() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("pt0", &[10, 20, 30, 40]),
        make_point("pt1", &[11, 21, 31, 41]),
        make_point("pt2", &[50, 60, 70, 80]),
        make_point("pt3", &[51, 61, 71, 81]),
        make_point("pt4", &[100, 110, 120, 130]),
    ];
    tree.add(pts);

    let path = "/tmp/test_mvptree_rw.mvp";
    let err = tree.write(path, 0o755);
    assert_eq!(err, MVPError::Success);

    let tree2 = mvptree_read(path, point_l2_distance);
    assert!(tree2.is_ok());
    let tree2 = tree2.unwrap();
    assert_eq!(tree2.branch_factor, 2);
    assert_eq!(tree2.path_length, 5);
    assert_eq!(tree2.leaf_capacity, 25);

    // Verify same retrieval behavior after read
    let query = make_point("q", &[10, 20, 30, 40]);
    let results = tree2.retrieve(&query, 10, 1000.0).unwrap();
    assert_eq!(results.len(), 5);

    // Clean up
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_read_nonexistent() {
    let result = mvptree_read("/tmp/nonexistent_mvp_file.mvp", point_l2_distance);
    assert!(result.is_err());
    match result {
        Err(e) => assert_eq!(e, MVPError::FileNotFound),
        Ok(_) => panic!("expected error"),
    }
}

// ---- write on empty tree ----

#[test]
fn test_write_empty_tree() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let err = tree.write("/tmp/test_empty.mvp", 0o755);
    assert_eq!(err, MVPError::ArgErr);
}

// ---- add incrementally ----

#[test]
fn test_add_incremental() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);

    // Add first batch
    let pts = vec![
        make_point("a1", &[10, 20, 30, 40]),
        make_point("a2", &[11, 21, 31, 41]),
        make_point("a3", &[50, 60, 70, 80]),
    ];
    assert_eq!(tree.add(pts), MVPError::Success);

    // Add second batch
    let pts2 = vec![
        make_point("b1", &[12, 22, 32, 42]),
        make_point("b2", &[100, 110, 120, 130]),
    ];
    assert_eq!(tree.add(pts2), MVPError::Success);

    // All 5 should be retrievable
    let query = make_point("q", &[10, 20, 30, 40]);
    let results = tree.retrieve(&query, 10, 1000.0).unwrap();
    assert_eq!(results.len(), 5);
}

// ---- add one at a time ----

#[test]
fn test_add_one_at_a_time() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);

    for i in 0..5u8 {
        let p = make_point(&format!("p{}", i), &[i * 10, i * 20, i * 30, i * 40]);
        assert_eq!(tree.add(vec![p]), MVPError::Success);
    }

    let query = make_point("q", &[0, 0, 0, 0]);
    let results = tree.retrieve(&query, 10, 1000.0).unwrap();
    assert_eq!(results.len(), 5);
}

// ---- larger tree that forces internal nodes ----

#[test]
fn test_large_tree_internal_nodes() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);

    // Add enough points to force internal node creation (> leafcap + 2 = 27)
    let mut pts = Vec::new();
    for i in 0..30u8 {
        let data = [i.wrapping_mul(7), i.wrapping_mul(13), i.wrapping_mul(17), i.wrapping_mul(23)];
        pts.push(make_point(&format!("p{}", i), &data));
    }
    assert_eq!(tree.add(pts), MVPError::Success);

    // All 30 should be retrievable with large radius
    let query = make_point("q", &[0, 0, 0, 0]);
    let results = tree.retrieve(&query, 100, 10000.0).unwrap();
    assert_eq!(results.len(), 30);
}

// ---- write/read roundtrip with internal nodes ----

#[test]
fn test_write_read_internal_nodes() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);

    let mut pts = Vec::new();
    for i in 0..30u8 {
        let data = [i.wrapping_mul(7), i.wrapping_mul(13), i.wrapping_mul(17), i.wrapping_mul(23)];
        pts.push(make_point(&format!("p{}", i), &data));
    }
    tree.add(pts);

    let path = "/tmp/test_mvptree_internal_rw.mvp";
    assert_eq!(tree.write(path, 0o755), MVPError::Success);

    let tree2 = mvptree_read(path, point_l2_distance).unwrap();
    let query = make_point("q", &[0, 0, 0, 0]);
    let results = tree2.retrieve(&query, 100, 10000.0).unwrap();
    assert_eq!(results.len(), 30);

    let _ = std::fs::remove_file(path);
}

// ---- add to existing tree after read ----

#[test]
fn test_add_after_read() {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("a", &[10, 20, 30, 40]),
        make_point("b", &[50, 60, 70, 80]),
        make_point("c", &[90, 100, 110, 120]),
    ];
    tree.add(pts);

    let path = "/tmp/test_mvptree_add_after_read.mvp";
    tree.write(path, 0o755);

    let mut tree2 = mvptree_read(path, point_l2_distance).unwrap();
    let new_pts = vec![make_point("d", &[15, 25, 35, 45])];
    assert_eq!(tree2.add(new_pts), MVPError::Success);

    let query = make_point("q", &[10, 20, 30, 40]);
    let results = tree2.retrieve(&query, 10, 1000.0).unwrap();
    assert_eq!(results.len(), 4);

    let _ = std::fs::remove_file(path);
}

// ---- MVPError enum values ----

#[test]
fn test_error_enum_distinct() {
    // Verify key error variants are distinct
    assert_ne!(MVPError::Success, MVPError::ArgErr);
    assert_ne!(MVPError::ArgErr, MVPError::EmptyTree);
    assert_ne!(MVPError::TypeMismatch, MVPError::KNearestCap);
}

// ---- ERROR_MSGS content ----

#[test]
fn test_error_msgs_content() {
    assert_eq!(ERROR_MSGS[0], "no error");
    assert_eq!(ERROR_MSGS[1], "bad argument");
    assert_eq!(ERROR_MSGS[11], "could not sort points");
    assert_eq!(ERROR_MSGS[19], "datatypes in conflict");
    assert_eq!(ERROR_MSGS[20], "no. retrieved exceeds k");
    assert_eq!(ERROR_MSGS[21], "empty tree");
    assert_eq!(ERROR_MSGS[24], "unrecognized node");
}

// ---- LeafNode / InternalNode constructors ----

#[test]
fn test_leaf_node_new() {
    let leaf = LeafNode::new(2);
    assert_eq!(leaf.node_type, NodeType::LeafNode);
    assert!(leaf.sv1.is_none());
    assert!(leaf.sv2.is_none());
    assert_eq!(leaf.nbpoints, 0);
    assert!(leaf.points.is_empty());
}

#[test]
fn test_internal_node_new() {
    let internal = InternalNode::new(2);
    assert_eq!(internal.node_type, NodeType::InternalNode);
    assert!(internal.sv1.is_none());
    assert!(internal.sv2.is_none());
    assert_eq!(internal.m1.len(), 1); // bf-1
    assert_eq!(internal.m2.len(), 2); // (bf-1)*bf
    assert!(internal.child_nodes.is_empty());
}

#[test]
fn test_internal_node_new_bf3() {
    let internal = InternalNode::new(3);
    assert_eq!(internal.m1.len(), 2); // bf-1 = 2
    assert_eq!(internal.m2.len(), 6); // (bf-1)*bf = 6
}

// ---- two identical points ----

#[test]
fn test_two_identical_points() {
    // C: with 2 identical points, select_vantage_points finds sv2_pos=-1 (no distinct pair)
    // Only sv1 is checked during retrieve, so only 1 result returned
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        make_point("x", &[5, 5, 5, 5]),
        make_point("y", &[5, 5, 5, 5]),
    ];
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);

    let query = make_point("q", &[5, 5, 5, 5]);
    let results = tree.retrieve(&query, 10, 0.0).unwrap();
    // C returns only 1 because sv2 is NULL when all points are identical
    assert_eq!(results.len(), 1);
}

fn main() {}
