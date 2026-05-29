use twoDPartInt::data;

#[test]
fn test_particle_construction() {
    let p = data::Particle {
        x_coordinate: 1.5,
        y_coordinate: 2.5,
        radius: 50.0,
        next: None,
        idx: 7,
    };
    assert_eq!(p.x_coordinate, 1.5);
    assert_eq!(p.y_coordinate, 2.5);
    assert_eq!(p.radius, 50.0);
    assert!(p.next.is_none());
    assert_eq!(p.idx, 7);
}

#[test]
fn test_particle_clone() {
    let p = data::Particle {
        x_coordinate: 10.0,
        y_coordinate: 20.0,
        radius: 5.0,
        next: None,
        idx: 3,
    };
    let q = p.clone();
    assert_eq!(q.x_coordinate, 10.0);
    assert_eq!(q.y_coordinate, 20.0);
    assert_eq!(q.radius, 5.0);
    assert_eq!(q.idx, 3);
    assert!(q.next.is_none());
}

#[test]
fn test_particle_with_next() {
    let inner = data::Particle {
        x_coordinate: 100.0,
        y_coordinate: 200.0,
        radius: 1.0,
        next: None,
        idx: 99,
    };
    let outer = data::Particle {
        x_coordinate: 0.0,
        y_coordinate: 0.0,
        radius: 1.0,
        next: Some(Box::new(inner)),
        idx: 1,
    };
    assert!(outer.next.is_some());
    let n = outer.next.as_ref().unwrap();
    assert_eq!(n.x_coordinate, 100.0);
    assert_eq!(n.y_coordinate, 200.0);
    assert_eq!(n.radius, 1.0);
    assert_eq!(n.idx, 99);
}

#[test]
fn test_particle_properties() {
    let pp = data::ParticleProperties {
        mass: 0.049,
        kn: 247435.829652697,
        ks: 19033.5253578998,
    };
    assert_eq!(pp.mass, 0.049);
    assert_eq!(pp.kn, 247435.829652697);
    assert_eq!(pp.ks, 19033.5253578998);

    // Copy
    let pp2 = pp;
    assert_eq!(pp2.mass, 0.049);
    assert_eq!(pp2.kn, 247435.829652697);
    assert_eq!(pp2.ks, 19033.5253578998);
}

#[test]
fn test_vector() {
    let v = data::Vector {
        x_component: 3.0,
        y_component: -4.5,
    };
    assert_eq!(v.x_component, 3.0);
    assert_eq!(v.y_component, -4.5);

    let v2 = v;
    assert_eq!(v2.x_component, 3.0);
    assert_eq!(v2.y_component, -4.5);
}

#[test]
fn test_contact() {
    let c = data::Contact {
        p1_idx: 4,
        p2_idx: 7,
        overlap: 0.5,
    };
    assert_eq!(c.p1_idx, 4);
    assert_eq!(c.p2_idx, 7);
    assert_eq!(c.overlap, 0.5);

    let c2 = c;
    assert_eq!(c2.p1_idx, 4);
    assert_eq!(c2.p2_idx, 7);
    assert_eq!(c2.overlap, 0.5);
}

fn main() {}
