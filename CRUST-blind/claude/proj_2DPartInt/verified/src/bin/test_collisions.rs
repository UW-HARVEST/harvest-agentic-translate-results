use twoDPartInt::collisions;
use twoDPartInt::data;

#[test]
fn test_find_col_within_grid() {
    // x_squares=7, square_length=120 => grid spans x from -420 to 420
    assert_eq!(collisions::find_col(0.0, 7, 120.0), 3);
    assert_eq!(collisions::find_col(-420.0, 7, 120.0), 0);
    assert_eq!(collisions::find_col(-419.0, 7, 120.0), 0);
    assert_eq!(collisions::find_col(-300.0, 7, 120.0), 1);
    assert_eq!(collisions::find_col(120.0, 7, 120.0), 4);
}

#[test]
fn test_find_col_left_boundary() {
    assert_eq!(collisions::find_col(-501.0, 7, 120.0), -2);
    assert_eq!(collisions::find_col(-420.5, 7, 120.0), -2);
}

#[test]
fn test_find_col_right_boundary() {
    assert_eq!(collisions::find_col(420.0, 7, 120.0), -1);
}

#[test]
fn test_find_row_within_grid() {
    assert_eq!(collisions::find_row(0.0, 7, 120.0), 0);
    assert_eq!(collisions::find_row(50.0, 7, 120.0), 0);
    assert_eq!(collisions::find_row(120.0, 7, 120.0), 1);
    assert_eq!(collisions::find_row(839.0, 7, 120.0), 6);
}

#[test]
fn test_find_row_top_boundary() {
    // y >= y_squares*square_length should be -1
    assert_eq!(collisions::find_row(840.0, 7, 120.0), -1);
}

#[test]
fn test_find_row_below() {
    // y < 0 should be -2
    assert_eq!(collisions::find_row(-1.0, 7, 120.0), -2);
}

#[test]
fn test_find_square_within_grid() {
    // C says: find_square(0, 50) => row 0, col 3, idx = 0*7+3 = 3
    assert_eq!(collisions::find_square(0.0, 50.0, 7, 7, 120.0), 3);
    // (-420,50) => row 0, col 0
    assert_eq!(collisions::find_square(-420.0, 50.0, 7, 7, 120.0), 0);
    // (-300, 250) => row=2, col=1 => 2*7+1=15
    assert_eq!(collisions::find_square(-300.0, 250.0, 7, 7, 120.0), 15);
    // (0,0) => row 0, col 3 => 3
    assert_eq!(collisions::find_square(0.0, 0.0, 7, 7, 120.0), 3);
}

#[test]
fn test_find_square_outside() {
    // y<0 => -1
    assert_eq!(collisions::find_square(0.0, -1.0, 7, 7, 120.0), -1);
}

#[test]
fn test_compute_contacts_two_overlap() {
    // 3 particles in a 3x3 grid, square_length 200, radius 50.
    // Particle 0 at (0,100), Particle 1 at (50,100), Particle 2 at (-300,100).
    // Particles 0 and 1 overlap (50 apart, radii 50+50=100, overlap=50).
    // Particle 2 is far away.

    let p0 = data::Particle {
        x_coordinate: 0.0,
        y_coordinate: 100.0,
        radius: 50.0,
        next: None,
        idx: 0,
    };
    let p1 = data::Particle {
        x_coordinate: 50.0,
        y_coordinate: 100.0,
        radius: 50.0,
        next: None,
        idx: 1,
    };
    let p2 = data::Particle {
        x_coordinate: -300.0,
        y_coordinate: 100.0,
        radius: 50.0,
        next: None,
        idx: 2,
    };

    // Build the grid manually based on find_square logic:
    // 3x3 grid, square_length 200 => x spans -300 to 300, y from 0 to 600.
    // p0 at (0,100): row=0,col=find_col(0)=floor(|(-300-0)|/200)=1 => idx=0*3+1=1
    // p1 at (50,100): row=0,col=floor(|(-300-50)|/200)=1 => idx=1
    // p2 at (-300,100): row=0,col=floor(|(-300-(-300))|/200)=0 => idx=0

    let mut grid: Vec<Vec<&data::Particle>> = vec![Vec::new(); 9];
    grid[0].push(&p2);
    grid[1].push(&p0);
    grid[1].push(&p1);

    let grid_refs: Vec<&Vec<&data::Particle>> = grid.iter().collect();

    let mut contacts = vec![
        data::Contact { p1_idx: 0, p2_idx: 0, overlap: 0.0 };
        16
    ];
    let n = collisions::compute_contacts(&grid_refs, 3, 3, 200.0, &mut contacts);
    // C output: 2 contacts (0->1 and 1->0)
    assert_eq!(n, 2);

    // Verify the contacts contain (0,1) and (1,0) with overlap=50
    let mut seen_01 = false;
    let mut seen_10 = false;
    for i in 0..2 {
        let c = contacts[i];
        if c.p1_idx == 0 && c.p2_idx == 1 {
            seen_01 = true;
            assert!((c.overlap - 50.0).abs() < 0.0001);
        }
        if c.p1_idx == 1 && c.p2_idx == 0 {
            seen_10 = true;
            assert!((c.overlap - 50.0).abs() < 0.0001);
        }
    }
    assert!(seen_01, "contact 0->1 not found");
    assert!(seen_10, "contact 1->0 not found");
}

#[test]
fn test_compute_contacts_no_overlap() {
    // 4 particles, none overlap.
    // Particles in a 3x3 grid, square_length=200.
    // p0=(-100,100), p1=(100,100), p2=(-100,300), p3=(100,300)
    // distance between any two same-y is 200, sum of radii is 100 -> no overlap.
    let p0 = data::Particle { x_coordinate: -100.0, y_coordinate: 100.0, radius: 50.0, next: None, idx: 0 };
    let p1 = data::Particle { x_coordinate: 100.0, y_coordinate: 100.0, radius: 50.0, next: None, idx: 1 };
    let p2 = data::Particle { x_coordinate: -100.0, y_coordinate: 300.0, radius: 50.0, next: None, idx: 2 };
    let p3 = data::Particle { x_coordinate: 100.0, y_coordinate: 300.0, radius: 50.0, next: None, idx: 3 };

    // Grid placements according to C output:
    // p0 -> grid[1], p1 -> grid[2], p2 -> grid[4], p3 -> grid[5]
    let mut grid: Vec<Vec<&data::Particle>> = vec![Vec::new(); 9];
    grid[1].push(&p0);
    grid[2].push(&p1);
    grid[4].push(&p2);
    grid[5].push(&p3);

    let grid_refs: Vec<&Vec<&data::Particle>> = grid.iter().collect();
    let mut contacts = vec![
        data::Contact { p1_idx: 0, p2_idx: 0, overlap: 0.0 };
        16
    ];
    let n = collisions::compute_contacts(&grid_refs, 3, 3, 200.0, &mut contacts);
    assert_eq!(n, 0);
}

#[test]
fn test_compute_contacts_empty_grid() {
    let grid: Vec<Vec<&data::Particle>> = vec![Vec::new(); 9];
    let grid_refs: Vec<&Vec<&data::Particle>> = grid.iter().collect();
    let mut contacts = vec![data::Contact { p1_idx: 0, p2_idx: 0, overlap: 0.0 }; 4];
    let n = collisions::compute_contacts(&grid_refs, 3, 3, 200.0, &mut contacts);
    assert_eq!(n, 0);
}

#[test]
fn test_compute_contacts_same_square() {
    // Two particles in the same square that overlap.
    let p0 = data::Particle { x_coordinate: 100.0, y_coordinate: 100.0, radius: 50.0, next: None, idx: 0 };
    let p1 = data::Particle { x_coordinate: 130.0, y_coordinate: 100.0, radius: 50.0, next: None, idx: 1 };

    // 3x3 grid, square_length=200; both in cell row=0, col=2 => idx=2
    let mut grid: Vec<Vec<&data::Particle>> = vec![Vec::new(); 9];
    grid[2].push(&p0);
    grid[2].push(&p1);
    let grid_refs: Vec<&Vec<&data::Particle>> = grid.iter().collect();
    let mut contacts = vec![data::Contact { p1_idx: 0, p2_idx: 0, overlap: 0.0 }; 16];
    let n = collisions::compute_contacts(&grid_refs, 3, 3, 200.0, &mut contacts);
    // Each particle finds the other in same-square loop => 2 contacts
    assert_eq!(n, 2);
    let mut seen_01 = false;
    let mut seen_10 = false;
    for i in 0..2 {
        let c = contacts[i];
        if c.p1_idx == 0 && c.p2_idx == 1 {
            seen_01 = true;
            // overlap = 50+50 - 30 = 70
            assert!((c.overlap - 70.0).abs() < 0.0001);
        }
        if c.p1_idx == 1 && c.p2_idx == 0 {
            seen_10 = true;
            assert!((c.overlap - 70.0).abs() < 0.0001);
        }
    }
    assert!(seen_01);
    assert!(seen_10);
}

fn main() {}
