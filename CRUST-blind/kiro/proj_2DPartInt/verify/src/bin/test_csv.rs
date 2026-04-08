use twoDPartInt::csv;
use twoDPartInt::data::Particle;

#[test]
fn test_ensure_output_folder() {
    let dir = std::env::temp_dir().join("test_ensure_output");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(csv::ensure_output_folder(dir.to_str().unwrap()), 0);
    assert!(dir.exists());
    // calling again on existing dir should still succeed
    assert_eq!(csv::ensure_output_folder(dir.to_str().unwrap()), 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_write_simulation_step() {
    let dir = std::env::temp_dir().join("test_write_sim");
    std::fs::create_dir_all(&dir).unwrap();
    let particles = [
        Particle { x_coordinate: 1.5, y_coordinate: 2.5, radius: 10.0, next: None, idx: 0 },
        Particle { x_coordinate: 3.0, y_coordinate: 4.0, radius: 20.0, next: None, idx: 1 },
    ];
    csv::write_simulation_step(2, &particles, dir.to_str().unwrap(), 42);
    let content = std::fs::read_to_string(dir.join("step_42.csv")).unwrap();
    assert!(content.starts_with("x,y,radius\n"));
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 3); // header + 2 particles
    assert!(lines[1].contains("1.5"));
    assert!(lines[2].contains("3"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_write_grid() {
    let dir = std::env::temp_dir().join("test_write_grid");
    std::fs::create_dir_all(&dir).unwrap();
    csv::write_grid(2, 2, 10.0, dir.to_str().unwrap());
    let content = std::fs::read_to_string(dir.join("grid.csv")).unwrap();
    assert!(content.starts_with("x,y\n"));
    // (x_squares+1) * (y_squares+1) = 3*3 = 9 data lines + header
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 10);
    std::fs::remove_dir_all(&dir).ok();
}

fn main() {}
