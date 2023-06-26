use bevy::prelude::*;

use super::FacetValue;

#[derive(Component, Debug, Clone)]
pub struct PhysicalAttributes {
    pub strength: FacetValue,
    pub agility: FacetValue,
    pub disease_resistence: FacetValue,
    pub fortitude: FacetValue,
}

#[derive(Component, Debug, Clone)]
pub struct MentalAttributes {
    pub willpower: FacetValue,
    pub creativity: FacetValue,
    pub memory: FacetValue,
    pub patience: FacetValue,
    pub empathy: FacetValue,
    pub persistence: FacetValue,
}

#[derive(Component, Debug, Clone)]
pub struct PersonalityValues {
    pub family: FacetValue,
    pub power: FacetValue,
    pub tradition: FacetValue,
    pub wealth: FacetValue,
    pub status: FacetValue,
}

#[derive(Bundle)]
pub struct PersonalityBundle {
    pub personality: PersonalityValues,
    pub mental_attributes: MentalAttributes,
    pub physical_attributes: PhysicalAttributes,
    pub tasks: TaskSet
}

#[derive(Debug, Clone)]
pub struct Task {
    pub category: TaskCategory,
    pub attributes: TaskAttributes,
}

//NPCs will only look to the next level when evaluating tasks
//E.g. Will mowing the lawn (short) help me achieve improving my standing with my mom (long)
//      not will it help me rule the world (dream)
#[derive(Debug, Clone, Component)]
pub struct TaskSet {
    //Lifelong ambition. Most NPCs should only have one or two over the course of their lives
    pub dream: Option<Task>,
    //What step am I currently taking to fulfill that needs. Typically one or two per month
    pub long_term: Option<Task>,
    //What am I doing right now. Typically several per day.
    //Note: not the same as an action. Fighting would be a short term task; but dodging left, swinging my sword, are not.
    pub short_term: Option<Task>
}

#[derive(Debug, Copy, Clone)]
pub enum TaskCategory {
    Fighting,
    Exploring,
    Digging,
    Socializing
}

//levels of what the NPC will have to go through to complete the task
#[derive(Debug, Clone)]
pub struct TaskAttributes {
    //high: play chess, low: play anarchy chess
    pub mental_difficulty: f32,
    //high: lift this boulder, low: lift this pencil
    pub strength_difficulty: f32,
    //high: american ninja warrior, low: eating a sandwich
    pub coordination_difficulty: f32,
    //high: thread of rejection/reputation tarnishment, low: no one cares
    pub social_danger: f32,
    //high: threat of death, low: safe
    pub physical_danger: f32,
    //high: stealing, low: it's legal
    pub legal_danger: f32,
    //high: rollercoaster, low: watch paint dry
    pub thrill: f32,
    //physical pain. high: femur breaker, low: none
    pub pain: f32,
    //creativity required. high: design a new novel weapon, low: plow land
    pub ingenuity: f32,
    //how delayed the gratification is. high: training for your next performance in a year, low: playing rocket league
    pub deepness: f32,
}

//what the NPC is expecting to get out of this task
#[derive(Debug, Clone)]
pub struct TaskOutcomes {
    //material gains: money, items, etc
    pub wealth: f32,
    //broad social status: prestige, power, etc
    pub status: f32,
    //expected change in hp, sickness, etc
    pub health: f32,
    //expected violence caused
    pub violence: f32,
    //how novel the task is
    pub adventure: f32,
    //gain social approval, typically from a specific NPC
    pub approval: f32,
    //bonuses for completing the task
    pub mental_improvement: Option<MentalAttributes>,
    pub physical_improvement: Option<PhysicalAttributes> 
}