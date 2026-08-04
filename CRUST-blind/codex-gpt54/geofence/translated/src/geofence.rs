use std::f64::consts::PI;
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
const EARTH_RADIUS: f64 = 6_378_137.0;
fn deg2rad(deg: f64) -> f64 {
    deg / (180.0 / PI)
}
fn lat2y_m(lat: f64) -> f64 {
    (deg2rad(lat) / 2.0 + PI / 4.0).tan().ln() * EARTH_RADIUS
}
fn lon2x_m(lon: f64) -> f64 {
    deg2rad(lon) * EARTH_RADIUS
}
fn is_point_in_polygon(p: &Point, vertices: &[Point]) -> bool {
    let mut number_of_intersections = 0i32;
    let x = p.x;
    let y = p.y;

    if vertices.len() < 3 {
        return false;
    }

    for (i, vertex) in vertices.iter().enumerate() {
        let x1 = vertex.x;
        let y1 = vertex.y;

        let next = vertices[(i + 1) % vertices.len()];
        let x2 = next.x;
        let y2 = next.y;

        if (y1 > y && y2 > y) || (y1 < y && y2 < y) {
            continue;
        }

        let s = x1 + (x2 - x1) * ((y - y1) / (y2 - y1));

        if s <= x {
            number_of_intersections += 1;
            if x == x1 && y == y1 {
                number_of_intersections -= 1;
            } else if s == x1 && y == y1 {
                number_of_intersections -= 1;
            }
        }
    }

    number_of_intersections % 2 == 1
}
pub fn is_position_in_geofence(p: &Point, vertices: &[Point]) -> bool {
    let xy: Vec<Point> = vertices
        .iter()
        .map(|vertex| Point {
            x: lon2x_m(vertex.x),
            y: lat2y_m(vertex.y),
        })
        .collect();

    let p_m = Point {
        x: lon2x_m(p.x),
        y: lat2y_m(p.y),
    };

    is_point_in_polygon(&p_m, &xy)
}
