use std::fs;
use std::path::PathBuf;
use twoDPartInt::csv;
use twoDPartInt::data::Particle;

fn temp_dir(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(name);
    if p.exists() {
        let _ = fs::remove_dir_all(&p);
    }
    p
}

#[test]
fn test_ensure_output_folder_creates() {
    let dir = temp_dir("test_csv_dir_create");
    assert!(!dir.exists());
    let rc = csv::ensure_output_folder(dir.to_str().unwrap());
    assert_eq!(rc, 0);
    assert!(dir.exists());
    assert!(dir.is_dir());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_ensure_output_folder_existing() {
    let dir = temp_dir("test_csv_dir_existing");
    fs::create_dir_all(&dir).unwrap();
    assert!(dir.exists());
    let rc = csv::ensure_output_folder(dir.to_str().unwrap());
    assert_eq!(rc, 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_simulation_step_writes_file() {
    let dir = temp_dir("test_csv_sim_step");
    fs::create_dir_all(&dir).unwrap();
    let particles = vec![
        Particle { x_coordinate: 1.5, y_coordinate: 2.5, radius: 5.0, next: None, idx: 0 },
        Particle { x_coordinate: -3.5, y_coordinate: 7.5, radius: 5.0, next: None, idx: 1 },
    ];
    csv::write_simulation_step(2, &particles, dir.to_str().unwrap(), 7);
    let mut file_path = dir.clone();
    file_path.push("step_7.csv");
    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).unwrap();
    // Expected: header then two lines of values.
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[1].contains("1.5"));
    assert!(lines[1].contains("2.5"));
    assert!(lines[2].contains("-3.5"));
    assert!(lines[2].contains("7.5"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_grid_writes_file() {
    let dir = temp_dir("test_csv_grid");
    fs::create_dir_all(&dir).unwrap();
    csv::write_grid(2, 2, 100.0, dir.to_str().unwrap());
    let mut file_path = dir.clone();
    file_path.push("grid.csv");
    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    // header + 4 squares.
    assert_eq!(lines.len(), 5);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_write_particles_from_grid_writes_file() {
    let dir = temp_dir("test_csv_grid_particles");
    fs::create_dir_all(&dir).unwrap();

    // The function takes &Vec<&mut Vec<&mut Particle>>; create owners and re-borrow.
    let mut p0 = Particle { x_coordinate: 1.0, y_coordinate: 2.0, radius: 3.0, next: None, idx: 0 };
    let mut p1 = Particle { x_coordinate: 4.0, y_coordinate: 5.0, radius: 6.0, next: None, idx: 1 };

    let mut sq0: Vec<&mut Particle> = vec![&mut p0];
    let mut sq1: Vec<&mut Particle> = vec![&mut p1];
    let mut sq2: Vec<&mut Particle> = vec![];
    let mut sq3: Vec<&mut Particle> = vec![];
    let grid: Vec<&mut Vec<&mut Particle>> = vec![&mut sq0, &mut sq1, &mut sq2, &mut sq3];
    csv::write_particles_from_grid(2, 2, dir.to_str().unwrap(), &grid, 5);

    let mut file_path = dir.clone();
    file_path.push("grid_step_5.csv");
    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    // header + 2 particles.
    assert_eq!(lines.len(), 3);
    let _ = fs::remove_dir_all(&dir);
}

fn main() {}
