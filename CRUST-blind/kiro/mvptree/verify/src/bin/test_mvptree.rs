use mvptree::mvptree::*;

// L1 distance matching C testmvp.c (BYTEARRAY version)
fn point_l1_distance(a: &MVPDatapoint, b: &MVPDatapoint) -> f32 {
    let mut sum: u32 = 0;
    for i in 0..a.datalen {
        let d1 = a.data[i] as i32;
        let d2 = b.data[i] as i32;
        sum += (d1 - d2).unsigned_abs();
    }
    sum as f32 / a.datalen as f32
}

// L2 distance matching C testmvp.c (BYTEARRAY version)
fn point_l2_distance(a: &MVPDatapoint, b: &MVPDatapoint) -> f32 {
    let mut sum: i32 = 0;
    for i in 0..a.datalen {
        let d1 = a.data[i] as i32;
        let d2 = b.data[i] as i32;
        let diff = d1 - d2;
        sum += diff * diff;
    }
    (sum as f32).sqrt() / a.datalen as f32
}

fn mkpt(id: &str, data: &[u8]) -> MVPDatapoint {
    MVPDatapoint::new(id.to_string(), data.to_vec(), MVPDataType::ByteArray)
}

// ---- error_to_string ----

#[test]
fn test_error_to_string_success() {
    assert_eq!(error_to_string(MVPError::Success), "no error");
}

#[test]
fn test_error_to_string_argerr() {
    assert_eq!(error_to_string(MVPError::ArgErr), "bad argument");
}

#[test]
fn test_error_to_string_emptytree() {
    assert_eq!(error_to_string(MVPError::EmptyTree), "empty tree");
}

#[test]
fn test_error_to_string_typemismatch() {
    assert_eq!(error_to_string(MVPError::TypeMismatch), "datatypes in conflict");
}

#[test]
fn test_error_to_string_munmap() {
    // C has "unmap eror" (typo preserved from original)
    assert_eq!(error_to_string(MVPError::Munmap), "unmap eror");
}

// ---- MVPDatapoint::new ----

#[test]
fn test_datapoint_new() {
    let dp = mkpt("test", &[1, 2, 3]);
    assert_eq!(dp.id, "test");
    assert_eq!(dp.data, vec![1, 2, 3]);
    assert_eq!(dp.datalen, 3);
    assert_eq!(dp.data_type, MVPDataType::ByteArray);
    assert!(dp.path.is_empty());
}

// ---- distance functions ----

#[test]
fn test_l1_distance() {
    let a = mkpt("a", &[10, 20, 30, 40, 50]);
    let b = mkpt("b", &[15, 25, 35, 45, 55]);
    let d = point_l1_distance(&a, &b);
    assert!((d - 5.0).abs() < 1e-6, "L1 expected 5.0, got {}", d);
}

#[test]
fn test_l2_distance() {
    let a = mkpt("a", &[10, 20, 30, 40, 50]);
    let b = mkpt("b", &[15, 25, 35, 45, 55]);
    let d = point_l2_distance(&a, &b);
    assert!((d - 2.236068).abs() < 1e-4, "L2 expected 2.236068, got {}", d);
}

#[test]
fn test_l1_same_point() {
    let a = mkpt("a", &[10, 20, 30, 40, 50]);
    assert!((point_l1_distance(&a, &a)).abs() < 1e-6);
}

#[test]
fn test_l2_same_point() {
    let a = mkpt("a", &[10, 20, 30, 40, 50]);
    assert!((point_l2_distance(&a, &a)).abs() < 1e-6);
}

#[test]
fn test_l2_3_4() {
    let a = mkpt("a", &[0, 0, 0, 0]);
    let b = mkpt("b", &[3, 4, 0, 0]);
    let d = point_l2_distance(&a, &b);
    assert!((d - 1.25).abs() < 1e-4, "L2 expected 1.25, got {}", d);
}

#[test]
fn test_l1_max_distance() {
    let a = mkpt("a", &[255, 255, 255, 255]);
    let b = mkpt("b", &[0, 0, 0, 0]);
    let d = point_l1_distance(&a, &b);
    assert!((d - 255.0).abs() < 1e-4, "L1 max expected 255.0, got {}", d);
}

#[test]
fn test_l2_max_distance() {
    let a = mkpt("a", &[255, 255, 255, 255]);
    let b = mkpt("b", &[0, 0, 0, 0]);
    let d = point_l2_distance(&a, &b);
    assert!((d - 127.5).abs() < 1e-4, "L2 max expected 127.5, got {}", d);
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
}

// ---- add 5 points and retrieve ----

fn build_5pt_tree() -> MVPTree {
    let mut tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let pts = vec![
        mkpt("A", &[0, 0, 0, 0]),
        mkpt("B", &[1, 1, 1, 1]),
        mkpt("C", &[2, 2, 2, 2]),
        mkpt("D", &[100, 100, 100, 100]),
        mkpt("E", &[0, 0, 0, 1]),
    ];
    let err = tree.add(pts);
    assert_eq!(err, MVPError::Success);
    tree
}

#[test]
fn test_add_5_points() {
    let tree = build_5pt_tree();
    assert!(tree.node.is_some());
}

#[test]
fn test_retrieve_radius_2() {
    let tree = build_5pt_tree();
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let results = tree.retrieve(&query, 10, 2.0).unwrap();
    assert_eq!(results.len(), 4, "expected 4 results, got {}", results.len());
    let mut ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["A", "B", "C", "E"]);
}

#[test]
fn test_retrieve_exact_match() {
    let tree = build_5pt_tree();
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let results = tree.retrieve(&query, 10, 0.0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "A");
}

#[test]
fn test_retrieve_all_large_radius() {
    let tree = build_5pt_tree();
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let results = tree.retrieve(&query, 100, 1000.0).unwrap();
    assert_eq!(results.len(), 5);
    let mut ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["A", "B", "C", "D", "E"]);
}

// ---- retrieve from empty tree ----

#[test]
fn test_retrieve_empty_tree() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let err = tree.retrieve(&query, 10, 2.0);
    assert!(err.is_err());
    assert_eq!(err.unwrap_err(), MVPError::EmptyTree);
}

// ---- retrieve with bad args ----

#[test]
fn test_retrieve_zero_knearest() {
    let tree = build_5pt_tree();
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let err = tree.retrieve(&query, 0, 2.0);
    assert_eq!(err.unwrap_err(), MVPError::ArgErr);
}

#[test]
fn test_retrieve_negative_radius() {
    let tree = build_5pt_tree();
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let err = tree.retrieve(&query, 10, -1.0);
    assert_eq!(err.unwrap_err(), MVPError::ArgErr);
}

// ---- add single point to existing tree ----

#[test]
fn test_add_single_to_existing() {
    let mut tree = build_5pt_tree();
    let err = tree.add(vec![mkpt("F", &[3, 3, 3, 3])]);
    assert_eq!(err, MVPError::Success);
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let results = tree.retrieve(&query, 100, 1000.0).unwrap();
    assert_eq!(results.len(), 6);
}

// ---- type mismatch ----

#[test]
fn test_type_mismatch() {
    let mut tree = build_5pt_tree();
    let bad = MVPDatapoint::new("bad".to_string(), vec![0; 8], MVPDataType::UInt16Array);
    let err = tree.add(vec![bad]);
    assert_eq!(err, MVPError::TypeMismatch);
}

// ---- write and read roundtrip ----

#[test]
fn test_write_read_roundtrip() {
    let tree = build_5pt_tree();
    let err = tree.write("/tmp/test_rust_mvp.mvp", 0o755);
    assert_eq!(err, MVPError::Success);

    let tree2 = mvptree_read("/tmp/test_rust_mvp.mvp", point_l2_distance).unwrap();
    assert_eq!(tree2.branch_factor, 2);
    assert_eq!(tree2.path_length, 5);
    assert_eq!(tree2.leaf_capacity, 25);

    let query = mkpt("Q", &[0, 0, 0, 0]);
    let results = tree2.retrieve(&query, 10, 2.0).unwrap();
    assert_eq!(results.len(), 4);
    let mut ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["A", "B", "C", "E"]);
}

// ---- read nonexistent file ----

#[test]
fn test_read_nonexistent_file() {
    let result = mvptree_read("/tmp/nonexistent_file_xyz.mvp", point_l2_distance);
    assert!(result.is_err());
    match result {
        Err(e) => assert_eq!(e, MVPError::FileNotFound),
        Ok(_) => panic!("expected error"),
    }
}

// ---- write empty tree ----

#[test]
fn test_write_empty_tree() {
    let tree = MVPTree::new(2, 5, 25, MVPDataType::ByteArray, point_l2_distance);
    let err = tree.write("/tmp/test_empty.mvp", 0o755);
    assert_eq!(err, MVPError::ArgErr);
}

// ---- print tree ----

#[test]
fn test_print_tree() {
    let mut tree = build_5pt_tree();
    tree.add(vec![mkpt("F", &[3, 3, 3, 3])]);
    let mut buf = Vec::new();
    let err = tree.print(&mut buf);
    assert_eq!(err, MVPError::Success);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("LEAF0"), "output should contain LEAF0: {}", output);
    assert!(output.contains("sv1:"), "output should contain sv1:");
    assert!(output.contains("sv2:"), "output should contain sv2:");
}

// ---- ERROR_MSGS constants ----

#[test]
fn test_error_msgs_length() {
    assert_eq!(ERROR_MSGS.len(), 25);
}

#[test]
fn test_error_msgs_first_last() {
    assert_eq!(ERROR_MSGS[0], "no error");
    assert_eq!(ERROR_MSGS[24], "unrecognized node");
}

// ---- MVPDataType enum values ----

#[test]
fn test_datatype_values() {
    assert_eq!(MVPDataType::ByteArray as u32, 1);
    assert_eq!(MVPDataType::UInt16Array as u32, 2);
    assert_eq!(MVPDataType::UInt32Array as u32, 4);
    assert_eq!(MVPDataType::UInt64Array as u32, 8);
}

// ---- MVPError enum values ----

#[test]
fn test_error_enum_values() {
    assert_eq!(MVPError::Success as u32, 0);
    assert_eq!(MVPError::ArgErr as u32, 1);
    assert_eq!(MVPError::TypeMismatch as u32, 19);
    assert_eq!(MVPError::EmptyTree as u32, 21);
    assert_eq!(MVPError::FileNotFound as u32, 24);
}

// ---- TAG and VERSION constants ----

#[test]
fn test_constants() {
    assert_eq!(TAG, "phashmvp2010");
    assert_eq!(VERSION, 0x01000000);
    assert_eq!(HEADER_SIZE, 32);
}

// ---- retrieve data integrity ----

#[test]
fn test_retrieve_data_preserved() {
    let tree = build_5pt_tree();
    let query = mkpt("Q", &[0, 0, 0, 0]);
    let results = tree.retrieve(&query, 10, 0.0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "A");
    assert_eq!(results[0].data, vec![0, 0, 0, 0]);
    assert_eq!(results[0].data_type, MVPDataType::ByteArray);
    assert_eq!(results[0].datalen, 4);
}

fn main() {}
