use libbeaufort::tableau;

#[test]
fn test_tableau_default_alpha_size() {
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = tableau::beaufort_tableau(alpha);
    assert_eq!(mat.len(), 62, "tableau should have 62 rows for default alphabet");
    for row in &mat {
        assert_eq!(row.len(), 62, "each row should have 62 columns");
    }
}

#[test]
fn test_tableau_default_alpha_first_row() {
    // Per C: mat[0][x] = alpha[(size + 0) % size] when x=0 (since j=size at start),
    // then alpha[(size-1+0)%size] when x=1, ...
    // So row 0 starts with alpha[0], then walks alpha backwards from end.
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = tableau::beaufort_tableau(alpha);

    // Row 0: x=0 -> (size + 0) % size = 0 -> '0'
    // x=1 -> (size-1) % size = 61 -> 'z'
    // x=2 -> (size-2) % size = 60 -> 'y'
    assert_eq!(mat[0][0], b'0');
    assert_eq!(mat[0][1], b'z');
    assert_eq!(mat[0][2], b'y');
    assert_eq!(mat[0][61], b'1');
}

#[test]
fn test_tableau_small_alpha_ab() {
    // alpha="AB", size=2
    // Row 0: x=0 -> (2+0)%2 = 0 -> 'A'; x=1 -> (1+0)%2=1 -> 'B' => "AB"
    // Row 1: x=0 -> (2+1)%2 = 1 -> 'B'; x=1 -> (1+1)%2=0 -> 'A' => "BA"
    let mat = tableau::beaufort_tableau("AB");
    assert_eq!(mat.len(), 2);
    assert_eq!(mat[0], b"AB");
    assert_eq!(mat[1], b"BA");
}

#[test]
fn test_tableau_small_alpha_abcde() {
    // From the C reference run:
    // AEDCB
    // BAEDC
    // CBAED
    // DCBAE
    // EDCBA
    let mat = tableau::beaufort_tableau("ABCDE");
    assert_eq!(mat.len(), 5);
    assert_eq!(mat[0], b"AEDCB");
    assert_eq!(mat[1], b"BAEDC");
    assert_eq!(mat[2], b"CBAED");
    assert_eq!(mat[3], b"DCBAE");
    assert_eq!(mat[4], b"EDCBA");
}

#[test]
fn test_tableau_small_alpha_single() {
    // alpha="X", size=1, single row "X"
    let mat = tableau::beaufort_tableau("X");
    assert_eq!(mat.len(), 1);
    assert_eq!(mat[0], b"X");
}

#[test]
fn test_tableau_alpha_abcdefghij() {
    // From C reference output:
    let mat = tableau::beaufort_tableau("ABCDEFGHIJ");
    assert_eq!(mat.len(), 10);
    assert_eq!(mat[0], b"AJIHGFEDCB");
    assert_eq!(mat[1], b"BAJIHGFEDC");
    assert_eq!(mat[9], b"JIHGFEDCBA");
}

#[test]
fn test_tableau_first_column_is_alphabet() {
    // For each row y, mat[y][0] = alpha[(size + y) % size] = alpha[y]
    let alpha = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mat = tableau::beaufort_tableau(alpha);
    for (y, row) in mat.iter().enumerate() {
        assert_eq!(row[0], alpha.as_bytes()[y]);
    }
}

fn main() {}
