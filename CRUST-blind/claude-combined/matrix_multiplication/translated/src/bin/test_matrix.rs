use matrix_multiplication::matrix;

#[test]
fn test_generate_matrix_4x4_non_random() {
    let m = matrix::generate_matrix(4, 4, false);
    assert_eq!(m.len(), 4);
    for row in &m {
        assert_eq!(row.len(), 4);
    }
    // Non-random formula: m[i][j] = (j + i*x) where x=4
    let expected: Vec<Vec<f32>> = vec![
        vec![0.0, 1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0, 7.0],
        vec![8.0, 9.0, 10.0, 11.0],
        vec![12.0, 13.0, 14.0, 15.0],
    ];
    assert_eq!(m, expected);
}

#[test]
fn test_generate_matrix_8x8_non_random() {
    let m = matrix::generate_matrix(8, 8, false);
    assert_eq!(m.len(), 8);
    for i in 0..8 {
        assert_eq!(m[i].len(), 8);
        for j in 0..8 {
            assert_eq!(m[i][j], (j + i * 8) as f32);
        }
    }
}

#[test]
fn test_generate_matrix_2x3_non_random() {
    // Non-random formula uses j + i*x where x=2 (the row count param).
    let m = matrix::generate_matrix(2, 3, false);
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].len(), 3);
    assert_eq!(m[1].len(), 3);
    let expected: Vec<Vec<f32>> = vec![
        vec![0.0, 1.0, 2.0],
        vec![2.0, 3.0, 4.0],
    ];
    assert_eq!(m, expected);
}

#[test]
fn test_generate_matrix_3x2_non_random() {
    let m = matrix::generate_matrix(3, 2, false);
    assert_eq!(m.len(), 3);
    let expected: Vec<Vec<f32>> = vec![
        vec![0.0, 1.0],
        vec![3.0, 4.0],
        vec![6.0, 7.0],
    ];
    assert_eq!(m, expected);
}

#[test]
fn test_generate_matrix_zero() {
    let m = matrix::generate_matrix(0, 0, false);
    assert_eq!(m.len(), 0);
}

#[test]
fn test_generate_matrix_random_dimensions() {
    let m = matrix::generate_matrix(3, 5, true);
    assert_eq!(m.len(), 3);
    for row in &m {
        assert_eq!(row.len(), 5);
    }
}

#[test]
fn test_free_matrix() {
    let mut m = matrix::generate_matrix(4, 4, false);
    assert_eq!(m.len(), 4);
    let rc = matrix::free_matrix(&mut m);
    assert_eq!(rc, 0);
    assert_eq!(m.len(), 0);
}

// Helper expected results (verified by running the C executable):
// 4x4 non-random multiplied by itself yields:
//   56 62 68 74
//  152 174 196 218
//  248 286 324 362
//  344 398 452 506
fn expected_4x4_result() -> Vec<Vec<f32>> {
    vec![
        vec![56.0, 62.0, 68.0, 74.0],
        vec![152.0, 174.0, 196.0, 218.0],
        vec![248.0, 286.0, 324.0, 362.0],
        vec![344.0, 398.0, 452.0, 506.0],
    ]
}

// 8x8 non-random multiplied by itself yields these values (from C):
fn expected_8x8_result() -> Vec<Vec<f32>> {
    vec![
        vec![1120.0, 1148.0, 1176.0, 1204.0, 1232.0, 1260.0, 1288.0, 1316.0],
        vec![2912.0, 3004.0, 3096.0, 3188.0, 3280.0, 3372.0, 3464.0, 3556.0],
        vec![4704.0, 4860.0, 5016.0, 5172.0, 5328.0, 5484.0, 5640.0, 5796.0],
        vec![6496.0, 6716.0, 6936.0, 7156.0, 7376.0, 7596.0, 7816.0, 8036.0],
        vec![8288.0, 8572.0, 8856.0, 9140.0, 9424.0, 9708.0, 9992.0, 10276.0],
        vec![10080.0, 10428.0, 10776.0, 11124.0, 11472.0, 11820.0, 12168.0, 12516.0],
        vec![11872.0, 12284.0, 12696.0, 13108.0, 13520.0, 13932.0, 14344.0, 14756.0],
        vec![13664.0, 14140.0, 14616.0, 15092.0, 15568.0, 16044.0, 16520.0, 16996.0],
    ]
}

// 16x16 non-random algorithm2 result (from C):
fn expected_16x16_result() -> Vec<Vec<f32>> {
    vec![
        vec![19840.0,19960.0,20080.0,20200.0,20320.0,20440.0,20560.0,20680.0,20800.0,20920.0,21040.0,21160.0,21280.0,21400.0,21520.0,21640.0],
        vec![50560.0,50936.0,51312.0,51688.0,52064.0,52440.0,52816.0,53192.0,53568.0,53944.0,54320.0,54696.0,55072.0,55448.0,55824.0,56200.0],
        vec![81280.0,81912.0,82544.0,83176.0,83808.0,84440.0,85072.0,85704.0,86336.0,86968.0,87600.0,88232.0,88864.0,89496.0,90128.0,90760.0],
        vec![112000.0,112888.0,113776.0,114664.0,115552.0,116440.0,117328.0,118216.0,119104.0,119992.0,120880.0,121768.0,122656.0,123544.0,124432.0,125320.0],
        vec![142720.0,143864.0,145008.0,146152.0,147296.0,148440.0,149584.0,150728.0,151872.0,153016.0,154160.0,155304.0,156448.0,157592.0,158736.0,159880.0],
        vec![173440.0,174840.0,176240.0,177640.0,179040.0,180440.0,181840.0,183240.0,184640.0,186040.0,187440.0,188840.0,190240.0,191640.0,193040.0,194440.0],
        vec![204160.0,205816.0,207472.0,209128.0,210784.0,212440.0,214096.0,215752.0,217408.0,219064.0,220720.0,222376.0,224032.0,225688.0,227344.0,229000.0],
        vec![234880.0,236792.0,238704.0,240616.0,242528.0,244440.0,246352.0,248264.0,250176.0,252088.0,254000.0,255912.0,257824.0,259736.0,261648.0,263560.0],
        vec![265600.0,267768.0,269936.0,272104.0,274272.0,276440.0,278608.0,280776.0,282944.0,285112.0,287280.0,289448.0,291616.0,293784.0,295952.0,298120.0],
        vec![296320.0,298744.0,301168.0,303592.0,306016.0,308440.0,310864.0,313288.0,315712.0,318136.0,320560.0,322984.0,325408.0,327832.0,330256.0,332680.0],
        vec![327040.0,329720.0,332400.0,335080.0,337760.0,340440.0,343120.0,345800.0,348480.0,351160.0,353840.0,356520.0,359200.0,361880.0,364560.0,367240.0],
        vec![357760.0,360696.0,363632.0,366568.0,369504.0,372440.0,375376.0,378312.0,381248.0,384184.0,387120.0,390056.0,392992.0,395928.0,398864.0,401800.0],
        vec![388480.0,391672.0,394864.0,398056.0,401248.0,404440.0,407632.0,410824.0,414016.0,417208.0,420400.0,423592.0,426784.0,429976.0,433168.0,436360.0],
        vec![419200.0,422648.0,426096.0,429544.0,432992.0,436440.0,439888.0,443336.0,446784.0,450232.0,453680.0,457128.0,460576.0,464024.0,467472.0,470920.0],
        vec![449920.0,453624.0,457328.0,461032.0,464736.0,468440.0,472144.0,475848.0,479552.0,483256.0,486960.0,490664.0,494368.0,498072.0,501776.0,505480.0],
        vec![480640.0,484600.0,488560.0,492520.0,496480.0,500440.0,504400.0,508360.0,512320.0,516280.0,520240.0,524200.0,528160.0,532120.0,536080.0,540040.0],
    ]
}

#[test]
fn test_algorithm1_4x4() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = vec![vec![0.0f32; 4]; 4];
    matrix::algorithm1(&m1, &m2, &mut r);
    assert_eq!(r, expected_4x4_result());
}

#[test]
fn test_algorithm3_4x4() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = vec![vec![0.0f32; 4]; 4];
    matrix::algorithm3(&m1, &m2, &mut r);
    assert_eq!(r, expected_4x4_result());
}

#[test]
fn test_algorithm1_8x8() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = vec![vec![0.0f32; 8]; 8];
    matrix::algorithm1(&m1, &m2, &mut r);
    assert_eq!(r, expected_8x8_result());
}

#[test]
fn test_algorithm3_8x8() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = vec![vec![0.0f32; 8]; 8];
    matrix::algorithm3(&m1, &m2, &mut r);
    assert_eq!(r, expected_8x8_result());
}

#[test]
fn test_algorithm2_8x8() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = vec![vec![0.0f32; 8]; 8];
    matrix::algorithm2(&m1, &m2, &mut r);
    assert_eq!(r, expected_8x8_result());
}

#[test]
fn test_algorithm2_16x16() {
    let m1 = matrix::generate_matrix(16, 16, false);
    let m2 = matrix::generate_matrix(16, 16, false);
    let mut r = vec![vec![0.0f32; 16]; 16];
    matrix::algorithm2(&m1, &m2, &mut r);
    assert_eq!(r, expected_16x16_result());
}

#[test]
fn test_multiply_method1_4x4() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = vec![vec![0.0f32; 4]; 4];
    let rc = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(rc, 0);
    assert_eq!(r, expected_4x4_result());
}

#[test]
fn test_multiply_method3_4x4() {
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(4, 4, false);
    let mut r = vec![vec![0.0f32; 4]; 4];
    let rc = matrix::multiply(&m1, &m2, &mut r, 3);
    assert_eq!(rc, 0);
    assert_eq!(r, expected_4x4_result());
}

#[test]
fn test_multiply_method2_8x8() {
    let m1 = matrix::generate_matrix(8, 8, false);
    let m2 = matrix::generate_matrix(8, 8, false);
    let mut r = vec![vec![0.0f32; 8]; 8];
    let rc = matrix::multiply(&m1, &m2, &mut r, 2);
    assert_eq!(rc, 0);
    assert_eq!(r, expected_8x8_result());
}

#[test]
fn test_multiply_dim_mismatch() {
    // x1=4 y1=4 m1, x2=1 y2=4 m2 -> y1 != x2 -> -1
    let m1 = matrix::generate_matrix(4, 4, false);
    let m2 = matrix::generate_matrix(1, 4, false);
    let mut r = vec![vec![0.0f32; 4]; 4];
    let rc = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(rc, -1);
}

#[test]
fn test_multiply_invalid_method() {
    let m1 = matrix::generate_matrix(1, 2, false);
    let m2 = matrix::generate_matrix(2, 1, false);
    let mut r = vec![vec![0.0f32; 1]; 1];
    // Method 0 is invalid -> -1
    let rc = matrix::multiply(&m1, &m2, &mut r, 0);
    assert_eq!(rc, -1);
}

#[test]
fn test_multiply_1x1() {
    // x1=y1=x2=y2=1, non-random -> m1=[[0]], m2=[[0]] -> r=[[0]]
    let m1 = matrix::generate_matrix(1, 1, false);
    let m2 = matrix::generate_matrix(1, 1, false);
    let mut r = vec![vec![1.0f32; 1]; 1];
    let rc = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(rc, 0);
    assert_eq!(r, vec![vec![0.0f32]]);

    let mut r2 = vec![vec![1.0f32; 1]; 1];
    let rc2 = matrix::multiply(&m1, &m2, &mut r2, 3);
    assert_eq!(rc2, 0);
    assert_eq!(r2, vec![vec![0.0f32]]);
}

#[test]
fn test_multiply_1x2_2x1() {
    // m1 = generate_matrix(1, 2, false): x=1, j+i*x = j -> [[0,1]]
    // m2 = generate_matrix(2, 1, false): x=2, j+i*x = i*2 -> [[0],[2]]
    // product (1x1): 0*0 + 1*2 = 2
    let m1 = matrix::generate_matrix(1, 2, false);
    let m2 = matrix::generate_matrix(2, 1, false);
    assert_eq!(m1, vec![vec![0.0f32, 1.0]]);
    assert_eq!(m2, vec![vec![0.0f32], vec![2.0]]);

    let mut r = vec![vec![0.0f32; 1]; 1];
    let rc = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(rc, 0);
    assert_eq!(r, vec![vec![2.0f32]]);

    let mut r2 = vec![vec![0.0f32; 1]; 1];
    let rc2 = matrix::multiply(&m1, &m2, &mut r2, 3);
    assert_eq!(rc2, 0);
    assert_eq!(r2, vec![vec![2.0f32]]);
}

#[test]
fn test_multiply_2x3_3x2() {
    // m1 = generate_matrix(2, 3, false): x=2 -> [[0,1,2],[2,3,4]]
    // m2 = generate_matrix(3, 2, false): x=3 -> [[0,1],[3,4],[6,7]]
    // product (verified via running C code):
    // [0*0+1*3+2*6, 0*1+1*4+2*7] = [15, 18]
    // [2*0+3*3+4*6, 2*1+3*4+4*7] = [33, 42]
    let m1 = matrix::generate_matrix(2, 3, false);
    let m2 = matrix::generate_matrix(3, 2, false);
    let mut r = vec![vec![0.0f32; 2]; 2];
    let rc = matrix::multiply(&m1, &m2, &mut r, 1);
    assert_eq!(rc, 0);
    assert_eq!(r, vec![vec![15.0f32, 18.0], vec![33.0, 42.0]]);

    let mut r2 = vec![vec![0.0f32; 2]; 2];
    let rc2 = matrix::multiply(&m1, &m2, &mut r2, 3);
    assert_eq!(rc2, 0);
    assert_eq!(r2, vec![vec![15.0f32, 18.0], vec![33.0, 42.0]]);
}

fn main() {}
