use std::marker::PhantomData;

use bevy::prelude::*;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, _app: &mut App) {
        
    }
}

//updates in fixed update, add implementation for each component you want to use on it
#[derive(Component, Copy, Clone)]
pub struct PIController<T> {
    pub kp: f32,  // Proportional gain
    pub ki: f32,  // Integral gain
    pub target_value: f32,
    integral: f32,
    _marker: PhantomData<T>
}

impl<T> PIController<T> {
    pub fn new(kp: f32, ki: f32, setpoint: f32) -> Self {
        PIController {
            kp,
            ki,
            target_value: setpoint,
            integral: 0.0,
            _marker: PhantomData
        }
    }

    pub fn update(&mut self, current_value: f32, delta_time: f32) -> f32 {
        let error = self.target_value - current_value;

        // Proportional term
        let p_term = self.kp * error;

        // Integral term
        self.integral += error * delta_time;
        let i_term = self.ki * self.integral;

        // Total control signal (acceleration)
        let acceleration = p_term + i_term;

        acceleration
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
    }
}