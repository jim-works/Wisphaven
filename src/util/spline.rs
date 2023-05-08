use bevy::prelude::Vec2;

pub struct Spline {
    control_points: Vec<Vec2>
}

impl Spline {
    pub fn new(control_points: &[Vec2]) -> Spline {
        Spline {
            control_points: Vec::from(control_points)
        }
    }
    pub fn map(&self, x: f32) -> f32 {
        for i in 0..self.control_points.len()
        {
            if x < self.control_points[i].x
            {
                if i == 0 { return self.control_points[i].y }
                let t = (x-self.control_points[i-1].x)/(self.control_points[i].x-self.control_points[i-1].x);
                return super::lerp(self.control_points[i-1].y, self.control_points[i].y, t);
            }
        }
        match self.control_points.last() {
            Some(p) => p.y,
            _ => x
        }
    }
}