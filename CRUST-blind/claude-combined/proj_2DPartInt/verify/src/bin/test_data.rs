use twoDPartInt::data::{Contact, Particle, ParticleProperties, Vector};

#[test]
fn test_particle_construction() {
    let p = Particle {
        x_coordinate: 1.5,
        y_coordinate: -2.5,
        radius: 50.0,
        next: None,
        idx: 7,
    };
    assert_eq!(p.x_coordinate, 1.5);
    assert_eq!(p.y_coordinate, -2.5);
    assert_eq!(p.radius, 50.0);
    assert!(p.next.is_none());
    assert_eq!(p.idx, 7);
}

#[test]
fn test_particle_clone() {
    let p = Particle {
        x_coordinate: 3.0,
        y_coordinate: 4.0,
        radius: 10.0,
        next: None,
        idx: 2,
    };
    let q = p.clone();
    assert_eq!(q.x_coordinate, 3.0);
    assert_eq!(q.y_coordinate, 4.0);
    assert_eq!(q.radius, 10.0);
    assert_eq!(q.idx, 2);
}

#[test]
fn test_particle_with_next() {
    let head = Particle {
        x_coordinate: 0.0,
        y_coordinate: 0.0,
        radius: 5.0,
        next: Some(Box::new(Particle {
            x_coordinate: 10.0,
            y_coordinate: 20.0,
            radius: 5.0,
            next: None,
            idx: 1,
        })),
        idx: 0,
    };
    let nxt = head.next.as_ref().unwrap();
    assert_eq!(nxt.x_coordinate, 10.0);
    assert_eq!(nxt.y_coordinate, 20.0);
    assert_eq!(nxt.idx, 1);
}

#[test]
fn test_particle_properties() {
    let pp = ParticleProperties {
        mass: 0.5,
        kn: 1.0,
        ks: 2.0,
    };
    let pp_copy = pp;
    assert_eq!(pp.mass, 0.5);
    assert_eq!(pp.kn, 1.0);
    assert_eq!(pp.ks, 2.0);
    assert_eq!(pp_copy.mass, 0.5);
    assert_eq!(pp_copy.kn, 1.0);
    assert_eq!(pp_copy.ks, 2.0);
}

#[test]
fn test_vector() {
    let v = Vector { x_component: -1.25, y_component: 7.5 };
    assert_eq!(v.x_component, -1.25);
    assert_eq!(v.y_component, 7.5);
    let v2 = v;
    assert_eq!(v2.x_component, -1.25);
    assert_eq!(v2.y_component, 7.5);
}

#[test]
fn test_contact() {
    let c = Contact {
        p1_idx: 3,
        p2_idx: 5,
        overlap: 1.5,
    };
    let c2 = c;
    assert_eq!(c.p1_idx, 3);
    assert_eq!(c.p2_idx, 5);
    assert_eq!(c.overlap, 1.5);
    assert_eq!(c2.p1_idx, 3);
    assert_eq!(c2.p2_idx, 5);
    assert_eq!(c2.overlap, 1.5);
}

fn main() {}
