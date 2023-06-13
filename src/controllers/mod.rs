mod player_controller;
use leafwing_input_manager::prelude::*;
pub use player_controller::*;

mod input;
pub use input::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{actors::MoveSpeed, physics::PhysicsObjectBundle};

pub struct ControllersPlugin;

impl Plugin for ControllersPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<Action>::default())
        //player
        .add_systems((rotate_mouse,jump_player,move_player,follow_local_player,player_punch,player_use,player_scroll_inventory).in_base_set(CoreSet::Update))
        //common
        .add_system(do_planar_movement.in_base_set(CoreSet::PostUpdate))
        ;
    }
}

//vector is the desired proportion of the movespeed to use, it's not normalized, but if the magnitude is greater than 1 it will be.
//reset every frame in do_planar_movement
#[derive(Component)]
pub struct FrameMovement(Vec3);

#[derive(Bundle)]
pub struct ControllableBundle {
    pub physics: PhysicsObjectBundle,
    pub frame_movement: FrameMovement,
    pub move_speed: MoveSpeed,   
}

impl Default for ControllableBundle {
    fn default() -> Self {
        ControllableBundle { physics: PhysicsObjectBundle::default(), frame_movement: FrameMovement(Vec3::default()), move_speed: MoveSpeed::default() }
    }
}

fn do_planar_movement(
    mut query: Query<(&mut FrameMovement, &mut ExternalImpulse, &Transform, &Velocity, &MoveSpeed)>,
    time: Res<Time>
 ) {
    const EPSILON: f32 = 1e-3;
    for (mut fm, mut impulse, tf, v, ms) in query.iter_mut() {
        let local_movement = fm.0;
        let local_speed = local_movement.length();
        //don't actively resist sliding if no input is provided (also smooths out jittering)
        if local_speed < EPSILON {fm.0 = Vec3::ZERO; continue;}
        //global space
        let mut v_desired = if local_speed > 1.0 {
            tf.rotation*(local_movement*(ms.max_speed/local_speed))
        } else {
            tf.rotation*local_movement*ms.max_speed
        };
        v_desired.y = 0.0;

        //create impulse that pushes us in the desired direction
        //this impulse will be pushing back into the circle of radius ms.max, so no need to normalize
        let mut dv = v_desired-v.linvel;
        dv.y = 0.0;
        let dv_len = dv.length();
        //don't overcorrect
        if dv_len > EPSILON { impulse.impulse += dv*(ms.current_accel*time.delta_seconds()/dv_len); }
        fm.0 = Vec3::ZERO;
    }
}