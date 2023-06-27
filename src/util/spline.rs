use bevy::prelude::Vec2;

pub struct Spline<const S: usize> {
    control_points: [Vec2; S],
}

impl<const S: usize> Spline<S> {
    pub const fn new(control_points: [Vec2; S]) -> Spline<S> {
        Spline { control_points }
    }
    pub fn map(&self, x: f32) -> f32 {
        match self
            .control_points
            .iter()
            .enumerate()
            .filter(|(_, point)| x < point.x)
            .next()
        {
            Some((i, point)) => {
                if i == 0 {
                    point.y
                } else {
                    let t = (x - self.control_points[i - 1].x)
                        / (self.control_points[i].x - self.control_points[i - 1].x);
                    super::lerp(self.control_points[i - 1].y, self.control_points[i].y, t)
                }
            }
            None => match self.control_points.last() {
                Some(p) => p.y,
                _ => x,
            },
        }
    }
}
