use bevy::prelude::*;

//if there is no trajectory to target, will shoot straight in their direction
pub fn aim_projectile_straight_fallback(
    target_offset: Vec3,
    target_rel_v: Vec3,
    proj_speed: f32,
    gravity: Vec3,
) -> Vec3 {
    aim_projectile(target_offset, target_rel_v, proj_speed, gravity).unwrap_or(target_offset.normalize_or_zero()*proj_speed)
}

//returns fire velocity
pub fn aim_projectile(
    target_offset: Vec3,
    target_rel_v: Vec3,
    proj_speed: f32,
    gravity: Vec3,
) -> Option<Vec3> {
    //http://playtechs.blogspot.com/2007/04/aiming-at-moving-target.html
    //using gravity without velocity, but aiming at target_offset + time_to_impact*target_rel_v
    //so we solve for time to impact twice
    const VERTICAL_CORRECTION: Vec3 = Vec3::new(0.0,1.0,0.0); //not sure why but we always aim low by some amount proportional to time
    let t = calc_time_to_impact(target_offset, proj_speed, -gravity);
    t
        .and_then(|t| calc_time_to_impact(target_offset + target_rel_v * t, proj_speed, -gravity))
        .map(|t| {
            (target_offset + target_rel_v * t - 0.5 * t * t * gravity+VERTICAL_CORRECTION*t).normalize_or_zero()
                * proj_speed
        })
}

fn calc_time_to_impact(target: Vec3, proj_speed: f32, gravity: Vec3) -> Option<f32> {
    //http://playtechs.blogspot.com/2007/04/aiming-at-moving-target.html
    //using gravity without velocity, but aiming at target_offset + time_to_impact*target_rel_v
    if target == Vec3::ZERO {
        return Some(0.0);
    }
    let a = 0.25 * gravity.length_squared();
    let b = target.dot(gravity) - proj_speed * proj_speed;
    let c = target.length_squared();
    //solve quadratic a*(t^2)^2+b*(t^2)+c=0
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }
    //a is positive, so root1 <= root2
    let root1 = (-b - discriminant.sqrt()) / (2.0 * a);
    let root2 = (-b + discriminant.sqrt()) / (2.0 * a);
    if root1 >= 0.0 {
        return Some(root1);
    }
    if root2 >= 0.0 {
        return Some(root2);
    }
    None
}
