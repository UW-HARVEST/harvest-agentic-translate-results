use twoDPartInt::data;

#[test]
fn test_particle_fields() {
    let p = data::Particle { x_coordinate: 1.5, y_coordinate: 2.5, radius: 3.0, next: None, idx: 7 };
    assert_eq!(p.x_coordinate, 1.5);
    assert_eq!(p.y_coordinate, 2.5);
    assert_eq!(p.radius, 3.0);
    assert!(p.next.is_none());
    assert_eq!(p.idx, 7);
}

#[test]
fn test_particle_properties_fields() {
    let pp = data::ParticleProperties { mass: 0.049, kn: 247435.829652697, ks: 19033.5253578998 };
    assert_eq!(pp.mass, 0.049);
    assert_eq!(pp.kn, 247435.829652697);
    assert_eq!(pp.ks, 19033.5253578998);
}

#[test]
fn test_vector_fields() {
    let v = data::Vector { x_component: -3.14, y_component: 2.71 };
    assert_eq!(v.x_component, -3.14);
    assert_eq!(v.y_component, 2.71);
}

#[test]
fn test_contact_fields() {
    let c = data::Contact { p1_idx: 0, p2_idx: 1, overlap: 42.5 };
    assert_eq!(c.p1_idx, 0);
    assert_eq!(c.p2_idx, 1);
    assert_eq!(c.overlap, 42.5);
}

#[test]
fn test_particle_clone() {
    let p = data::Particle { x_coordinate: 1.0, y_coordinate: 2.0, radius: 3.0, next: None, idx: 0 };
    let p2 = p.clone();
    assert_eq!(p2.x_coordinate, 1.0);
    assert_eq!(p2.y_coordinate, 2.0);
}

#[test]
fn test_vector_copy() {
    let v = data::Vector { x_component: 1.0, y_component: 2.0 };
    let v2 = v;
    assert_eq!(v.x_component, v2.x_component);
    assert_eq!(v.y_component, v2.y_component);
}

#[test]
fn test_contact_copy() {
    let c = data::Contact { p1_idx: 3, p2_idx: 5, overlap: 10.0 };
    let c2 = c;
    assert_eq!(c.p1_idx, c2.p1_idx);
    assert_eq!(c.p2_idx, c2.p2_idx);
    assert_eq!(c.overlap, c2.overlap);
}

fn main() {}
