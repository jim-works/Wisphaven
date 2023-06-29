use bevy::prelude::*;
use rand_distr::Normal;
use std::ops::{Deref, DerefMut};

pub const MAX_ATTRIBUTE_VALUE: f32 = 100.0;

//stats
#[derive(Component, Debug, Clone, Default)]
pub struct PhysicalAttributes {
    //physical strength
    //task scoring: reduces effect of strength_difficulty
    pub strength: FacetValue,
    //task scoring: reduces effect of coordination_difficulty
    pub agility: FacetValue,
    pub disease_resistence: FacetValue,
    //how well they can take a hit
    //task scoring: reduces effect of physical_danger
    pub fortitude: FacetValue,
}

//stats - affect a
#[derive(Component, Debug, Clone, Default)]
pub struct MentalAttributes {
    //task scoring: reduces the effect of mental_difficulty
    pub intelligence: FacetValue,
    //reduces the importance of enjoyment in task scaling
    //task scoring: reduces the effect of pain
    pub willpower: FacetValue,
    //task scoring: reduces the efect of mental_difficulty, increases the effect of monotony
    pub creativity: FacetValue,
    pub memory: FacetValue,
    //increases the importance of goals in task scaling
    pub patience: FacetValue,
    //task scoring: increases the effect of social_danger, status, and approval
    pub social_awareness: FacetValue,
    //task scoring: incrases the effect of violence
    pub empathy: FacetValue,
    //reduces the importance of difficulty in task scaling
    pub persistence: FacetValue,
}

//values the NPC holds. positive values are very important, 0 values are neutral, and negative values are disliked
pub type PersonalityValues = GenericPersonality<FacetValue>;
pub type PersonalityScores = GenericPersonality<f32>;

#[derive(Component, Debug, Clone, Default)]
pub struct GenericPersonality<T: core::fmt::Debug+Clone> {
    //task scoring: likes approval from family, dislikes violence towards them (TODO: Implement)
    pub family: T,
    //task scoring: likes status, improvement, health (if positive)
    pub power: T,
    //task scoring: dislikes adventure and slightly dislikes thrill (TODO: Change)
    pub tradition: T,
    //task scoring: likes wealth and slightly likes health (if positive)
    pub wealth: T,
    //task scoring: likes status, dislikes social_danger
    pub status: T,
    //task scoring: dislikes pain and deepness
    pub hedonism: T,
    //task scoring: slightly likes adventure and likes thrill
    pub excitement: T,
    //task scoring: dislikes violence
    pub pacifism: T,
}

#[derive(Bundle, Default)]
pub struct PersonalityBundle {
    pub personality: PersonalityValues,
    pub mental_attributes: MentalAttributes,
    pub physical_attributes: PhysicalAttributes,
    pub tasks: TaskSet
}

#[derive(Debug, Clone)]
pub struct Task {
    pub category: TaskCategory,
    pub risks: TaskRisks,
    pub outcomes: TaskOutcomes
}

//NPCs will only look to the next level when evaluating tasks
//E.g. Will mowing the lawn (short) help me achieve improving my standing with my mom (long)
//      not will it help me rule the world (dream)
#[derive(Debug, Clone, Component, Default)]
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
    Socializing,
    Idle
}

pub type TaskRisks = GenericTaskRisks<f32>;
pub type TaskRiskScores = GenericTaskRisks<f32>;
//levels of what the NPC will have to go through to complete the task
//positive values are generally detriments to doing the task 
#[derive(Debug, Clone, Default)]
pub struct GenericTaskRisks<T: core::fmt::Debug+Clone+Default> {
    //high: play chess, low: play anarchy chess
    pub mental_difficulty: T,
    //high: lift this boulder, low: lift this pencil
    pub strength_difficulty: T,
    //high: american ninja warrior, low: eating a sandwich
    pub coordination_difficulty: T,
    //high: thread of rejection/reputation tarnishment, low: no one cares
    pub social_danger: T,
    //high: threat of death, low: safe
    pub physical_danger: T,
    //high: stealing, low: it's legal
    pub legal_danger: T,
    //high: rollercoaster, low: watch paint dry
    pub thrill: T,
    //physical pain. high: femur breaker, low: none
    pub pain: T,
    //creativity required. high: design a new novel weapon, low: plow land
    pub monotony: T,
    //how delayed the gratification is. high: training for your next performance in a year, low: playing rocket league
    pub shallowness: T,
}

pub type TaskOutcomes = GenericTaskOutcomes<f32>;
pub type TaskOutcomeScores = GenericTaskOutcomes<f32>;
//what the NPC is expecting to get out of this task
//positive values are generally encouragements to do the task
#[derive(Debug, Clone, Default)]
pub struct GenericTaskOutcomes<T: core::fmt::Debug+Clone+Default> {
    //material gains: money, items, etc
    pub wealth: T,
    //broad social status: prestige, power, etc
    pub status: T,
    //expected change in hp, sickness, etc
    pub health: T,
    //expected violence caused
    pub violence: T,
    //how novel the task is
    pub adventure: T,
    //gain social approval, typically from a specific NPC
    pub approval: T,
    // //bonuses for completing the task (TODO)
    // pub mental_improvement: MentalAttributes,
    // pub physical_improvement: PhysicalAttributes 
}

//treated as a normal distribution with mean value and variance
#[derive(Debug, Clone, Copy)]
pub struct FacetValue(Normal<f32>);

impl Deref for FacetValue {
    type Target = Normal<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FacetValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for FacetValue {
    fn default() -> Self {
        Self(Normal::new(0.0, 1.0).unwrap())
    }
}

impl FacetValue {
    pub fn new(value: f32, std_dev: f32) -> Result<Self, rand_distr::NormalError> {
        let dist = Normal::new(value,std_dev)?;
        Ok(Self(dist))
    }
}
