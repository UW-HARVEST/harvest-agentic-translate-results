use twoDPartInt::csv;
use twoDPartInt::data;

#[test]
fn test_ensure_output_folder_creates() {
    let dir = std::env::temp_dir().join("test_csv_ensure");
    let _ = std::fs::remove_dir_all(&dir);
    let result = csv::ensure_output_folder(dir.to_str().unwrap());
    assert_eq!(result, 0);
    assert!(dir.exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_ensure_output_folder_existing() {
    let dir = std::env::temp_dir().join("test_csv_ensure_exist");
    std::fs::create_dir_all(&dir).unwrap();
    let result = csv::ensure_output_folder(dir.to_str().unwrap());
    assert_eq!(result, 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_write_simulation_step() {
    let dir = std::env::temp_dir().join("test_csv_sim_step");
    std::fs::create_dir_all(&dir).unwrap();
    let particles = [
        data::Particle { x_coordinate: 1.5, y_coordinate: 2.5, radius: 3.0, next: None, idx: 0 },
        data::Particle { x_coordinate: 4.0, y_coordinate: 5.0, radius: 6.0, next: None, idx: 1 },
    ];
    csv::write_simulation_step(2, &particles, dir.to_str().unwrap(), 0);
    let content = std::fs::read_to_string(dir.join("step_0.csv")).unwrap();
    assert!(content.contains("x,y,radius"));
    assert!(content.contains("1.5,2.5,3"));
    assert!(content.contains("4,5,6"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_write_grid() {
    let dir = std::env::temp_dir().join("test_csv_grid");
    std::fs::create_dir_all(&dir).unwrap();
    csv::write_grid(2, 2, 100.0, dir.to_str().unwrap());
    let content = std::fs::read_to_string(dir.join("grid.csv")).unwrap();
    assert!(content.contains("x,y"));
    let lines: Vec<&str> = content.lines().collect();
    // header + (2+1)*(2+1) = 9 data lines + header = 10
    assert_eq!(lines.len(), 10);
    std::fs::remove_dir_all(&dir).ok();
}

fn main() {}
