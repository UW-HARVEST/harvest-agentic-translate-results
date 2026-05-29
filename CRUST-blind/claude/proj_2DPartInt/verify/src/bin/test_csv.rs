use twoDPartInt::csv;
use twoDPartInt::data;
use std::fs;
use std::path::Path;

fn unique_dir(name: &str) -> String {
    format!(
        "/tmp/twoDPartInt_csv_test_{}_{}",
        name,
        std::process::id()
    )
}

#[test]
fn test_ensure_output_folder_creates() {
    let dir = unique_dir("create_dir");
    let _ = fs::remove_dir_all(&dir);
    assert!(!Path::new(&dir).exists());
    let result = csv::ensure_output_folder(&dir);
    assert_eq!(result, 0);
    assert!(Path::new(&dir).exists() && Path::new(&dir).is_dir());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_ensure_output_folder_existing() {
    let dir = unique_dir("existing_dir");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let result = csv::ensure_output_folder(&dir);
    assert_eq!(result, 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_ensure_output_folder_file_exists() {
    let dir = unique_dir("file_exists");
    let _ = fs::remove_file(&dir);
    fs::write(&dir, b"hello").unwrap();
    let result = csv::ensure_output_folder(&dir);
    assert_eq!(result, -1);
    let _ = fs::remove_file(&dir);
}

#[test]
fn test_write_simulation_step() {
    let dir = unique_dir("simstep");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let particles = vec![
        data::Particle { x_coordinate: 1.0, y_coordinate: 2.0, radius: 0.5, next: None, idx: 0 },
        data::Particle { x_coordinate: -1.5, y_coordinate: 3.5, radius: 1.0, next: None, idx: 1 },
    ];
    csv::write_simulation_step(2, &particles, &dir, 7);
    let path = format!("{}/step_7.csv", dir);
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "x_coordinate,y_coordinate,z_coordinate,radius,idx");
    assert_eq!(lines[1], "1,2,0,0.5,0");
    assert_eq!(lines[2], "-1.5,3.5,0,1,1");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_grid() {
    let dir = unique_dir("grid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    csv::write_grid(2, 2, 100.0, &dir);
    let path = format!("{}/grid.csv", dir);
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    // 1 header + 4 cells = 5 lines
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], "row,col,x_left,y_bottom,x_right,y_top");
    // x_left_limit = -100 (=2*100/2 = 100, then negated)
    // row 0, col 0: x_left=-100, y_bottom=0, x_right=0, y_top=100
    assert_eq!(lines[1], "0,0,-100,0,0,100");
    // row 0, col 1: x_left=0, y_bottom=0, x_right=100, y_top=100
    assert_eq!(lines[2], "0,1,0,0,100,100");
    // row 1, col 0: x_left=-100, y_bottom=100, x_right=0, y_top=200
    assert_eq!(lines[3], "1,0,-100,100,0,200");
    // row 1, col 1: x_left=0, y_bottom=100, x_right=100, y_top=200
    assert_eq!(lines[4], "1,1,0,100,100,200");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_particles_from_grid() {
    let dir = unique_dir("partgrid");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let mut p0 = data::Particle { x_coordinate: 1.0, y_coordinate: 2.0, radius: 0.5, next: None, idx: 0 };
    let mut p1 = data::Particle { x_coordinate: -1.5, y_coordinate: 3.5, radius: 1.0, next: None, idx: 1 };

    let mut cell0: Vec<&mut data::Particle> = vec![&mut p0];
    let mut cell1: Vec<&mut data::Particle> = vec![];
    let mut cell2: Vec<&mut data::Particle> = vec![&mut p1];
    let mut cell3: Vec<&mut data::Particle> = vec![];

    let grid: Vec<&mut Vec<&mut data::Particle>> = vec![&mut cell0, &mut cell1, &mut cell2, &mut cell3];

    csv::write_particles_from_grid(2, 2, &dir, &grid, 5);
    let path = format!("{}/grid_particles_5.csv", dir);
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "square_idx,x_coordinate,y_coordinate,radius,idx");
    assert_eq!(lines[1], "0,1,2,0.5,0");
    assert_eq!(lines[2], "2,-1.5,3.5,1,1");
    let _ = fs::remove_dir_all(&dir);
}

fn main() {}
