use undercroft::{Pos, World};

/// Casts rays from `origin` to every point on a square of `radius`, marking
/// cells until a ray hits something solid (which is marked too).
pub(crate) fn compute(world: &World, origin: Pos, radius: i32, mut mark: impl FnMut(usize)) {
    let r2 = radius * radius;
    for i in -radius..=radius {
        let edges = [
            Pos {
                x: origin.x + i,
                y: origin.y - radius,
            },
            Pos {
                x: origin.x + i,
                y: origin.y + radius,
            },
            Pos {
                x: origin.x - radius,
                y: origin.y + i,
            },
            Pos {
                x: origin.x + radius,
                y: origin.y + i,
            },
        ];
        for target in edges {
            for p in line(origin, target) {
                let (dx, dy) = (p.x - origin.x, p.y - origin.y);
                if dx * dx + dy * dy > r2 {
                    break;
                }
                let Some(index) = world.terrain.index(p) else {
                    break;
                };
                mark(index);
                if !world.walkable(p) {
                    break;
                }
            }
        }
    }
}

/// Bresenham's line from `a` to `b`, inclusive.
fn line(a: Pos, b: Pos) -> Vec<Pos> {
    let (dx, dy) = ((b.x - a.x).abs(), -(b.y - a.y).abs());
    let (sx, sy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let mut err = dx + dy;
    let mut p = a;
    let mut points = vec![p];
    while p != b {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            p.x += sx;
        }
        if e2 <= dx {
            err += dx;
            p.y += sy;
        }
        points.push(p);
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_inclusive_and_connected() {
        let pts = line(Pos { x: 0, y: 0 }, Pos { x: 5, y: -3 });
        assert_eq!(pts.first(), Some(&Pos { x: 0, y: 0 }));
        assert_eq!(pts.last(), Some(&Pos { x: 5, y: -3 }));
        for pair in pts.windows(2) {
            assert!((pair[0].x - pair[1].x).abs() <= 1 && (pair[0].y - pair[1].y).abs() <= 1);
        }
    }
}
