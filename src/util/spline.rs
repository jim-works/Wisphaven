use bevy::prelude::Vec2;


pub struct Spline<const S: usize> {
    control_points: [Vec2; S],
}

impl<const S: usize> Spline<S>{
    pub const fn new(control_points: [Vec2; S]) -> Spline<S> {
        Spline { control_points }
    }
    //will extrapolate 
    pub fn map(&self, x: f32) -> f32 {
        let i  = match self
            .control_points
            .iter()
            .enumerate().find(|(_, point)| x < point.x)
        {
            Some((i, _)) => i.max(1),
            None => 1.max(self.control_points.len()-1),
        };
        let t = (x - self.control_points[i - 1].x)
                        / (self.control_points[i].x - self.control_points[i - 1].x);
                    super::lerp(self.control_points[i - 1].y, self.control_points[i].y, t)
    }
}

pub struct ClampedSpline<const S: usize> {
    control_points: [Vec2; S],
}

impl<const S: usize> ClampedSpline<S>{
    pub const fn new(control_points: [Vec2; S]) -> ClampedSpline<S> {
        ClampedSpline { control_points }
    }
    //will clamp value at max and min control points 
    pub fn map(&self, x: f32) -> f32 {
        let i  = match self
            .control_points
            .iter()
            .enumerate().find(|(_, point)| x < point.x)
        {
            Some((i, val)) => {
                if i == 0 {
                    return val.y;
                }
                i
            },
            None => {
                return self.control_points.last().unwrap().y;
            },
        };
        let t = (x - self.control_points[i - 1].x)
                        / (self.control_points[i].x - self.control_points[i - 1].x);
                    super::lerp(self.control_points[i - 1].y, self.control_points[i].y, t)
    }
}
