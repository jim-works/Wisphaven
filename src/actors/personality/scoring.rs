use bevy::prelude::Vec2;

use crate::util::{MeanExt, Spline};

use super::components::*;

#[derive(Clone, Copy, Default, Debug)]
pub struct TaskScore {
    pub ease: f32,
    pub enjoyment: f32,
    pub safety: f32,
    pub loot: f32,
    pub goals: f32,
}

#[derive(Clone, Copy, Default, Debug)]
struct RawTaskScore(TaskScore);
#[derive(Clone, Copy, Default, Debug)]
pub struct AdjustedTaskScore(pub TaskScore);

impl RawTaskScore {
    //adjusts scores according to the mean of the attributes in MentalAttributes
    pub fn scale(&self, mental: &MentalAttributes) -> AdjustedTaskScore {
        const WILLPOWER_ENJOYMENT_REDUCTION: Spline<3> = Spline::new([
            Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
        ]);
        const PATIENCE_GOALS_INCREMENT: Spline<3> = Spline::new([
            Vec2::new(-MAX_ATTRIBUTE_VALUE, 0.1),
            Vec2::new(0.0, 1.0),
            Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
        ]);
        const PERSISTENCE_DIFFICULTY_REDUCTION: Spline<3> = Spline::new([
            Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
        ]);

        let mut scaled = AdjustedTaskScore(self.0);
        scaled.0.enjoyment *= WILLPOWER_ENJOYMENT_REDUCTION.map(mental.willpower.mean());
        scaled.0.goals *= PATIENCE_GOALS_INCREMENT.map(mental.patience.mean());
        scaled.0.ease *= PERSISTENCE_DIFFICULTY_REDUCTION.map(mental.persistence.mean());
        scaled
    }
}

impl TaskScore {
    //returns the average of all scores, rescaled to be 0..1
    pub fn overall(&self) -> f32 {
        (self.safety+self.ease + self.enjoyment + self.goals+self.loot) / (5.0*5.0)
    }
}

pub fn score_task(
    to_score: &mut Task,
    physical: &PhysicalAttributes,
    mental: &MentalAttributes,
    values: &PersonalityValues,
    _tasks: &TaskSet,
) -> AdjustedTaskScore {
    attribute_adjustment(to_score, physical, mental);
    let score = personality_score(to_score, values);
    score.scale(mental)
}

fn attribute_adjustment(
    to_score: &mut Task,
    physical: &PhysicalAttributes,
    mental: &MentalAttributes,
) {
    physical_attribute_adjustment(to_score, physical);
    mental_attribute_adjustment(to_score, mental);
}

fn physical_attribute_adjustment(to_adjust: &mut Task, physical: &PhysicalAttributes) {
    //splines to adjust scores based on each attribute
    const STRENGTH_DIFFICULTY_REDUCTION: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
    ]);
    const AGILITY_DIFFICULTY_REDUCTION: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
    ]);
    const FORTITUDE_DANGER_REDUCTION: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
    ]);

    //calculation
    to_adjust.risks.strength_difficulty *=
        STRENGTH_DIFFICULTY_REDUCTION.map(physical.strength.mean());
    to_adjust.risks.coordination_difficulty *=
        AGILITY_DIFFICULTY_REDUCTION.map(physical.agility.mean());
    to_adjust.risks.physical_danger *=
        FORTITUDE_DANGER_REDUCTION.map(physical.fortitude.mean());
}

fn mental_attribute_adjustment(to_adjust: &mut Task, mental: &MentalAttributes) {
    //splines to adjust scores based on each attribute
    const INTELLIGENCE_DIFFICULTY_REDUCTION: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
    ]);
    const WILLPOWER_PAIN_REDUCTION: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
    ]);
    const CREATIVITY_MONOTONY_INCREMENT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, -5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);
    const CREATIVITY_MENTAL_DIFFICULTY_REDUCTION: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 0.1),
    ]);
    const SOCIAL_AWARENESS_SOCIAL_DANGER_INCREMENT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 0.1),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);
    const SOCIAL_AWARENESS_STATUS_INCREMENT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 0.1),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);
    const SOCIAL_AWARENESS_APPROVAL_INCREMENT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 0.1),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);
    const EMPATHY_VIOLENCE_INCREMENT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);

    //calculation
    to_adjust.risks.mental_difficulty *=
        [CREATIVITY_MENTAL_DIFFICULTY_REDUCTION.map(mental.creativity.mean()), INTELLIGENCE_DIFFICULTY_REDUCTION.map(mental.intelligence.mean())].iter().mean::<f32>();
    to_adjust.risks.pain *= WILLPOWER_PAIN_REDUCTION.map(mental.willpower.mean());
    to_adjust.risks.monotony *= CREATIVITY_MONOTONY_INCREMENT.map(mental.creativity.mean());
    to_adjust.risks.social_danger *=
        SOCIAL_AWARENESS_SOCIAL_DANGER_INCREMENT.map(mental.social_awareness.mean());
    to_adjust.outcomes.status *=
        SOCIAL_AWARENESS_STATUS_INCREMENT.map(mental.social_awareness.mean());
    to_adjust.outcomes.approval *=
        SOCIAL_AWARENESS_APPROVAL_INCREMENT.map(mental.social_awareness.mean());
    to_adjust.outcomes.violence *= EMPATHY_VIOLENCE_INCREMENT.map(mental.empathy.mean());
}

fn personality_score(task: &Task, values: &PersonalityValues) -> RawTaskScore {
    const POWER_STATUS_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, -3.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 3.0),
    ]);
    const POWER_IMPROVEMENT_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, -3.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 3.0),
    ]);
    const POWER_HEALTH_MULT: Spline<2> =
        Spline::new([Vec2::new(0.0, 1.0), Vec2::new(MAX_ATTRIBUTE_VALUE, 2.0)]);
    const TRADIATION_ADVENTURE_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 2.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, -0.25),
    ]);
    const TRADIATION_THRILL_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 1.5),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, -0.5),
    ]);
    const WEALTH_WEALTH_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, -5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);
    const WEALTH_HEALTH_MULT: Spline<2> =
        Spline::new([Vec2::new(0.0, 1.0), Vec2::new(MAX_ATTRIBUTE_VALUE, 1.5)]);
    const STATUS_STATUS_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, -5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);
    const STATUS_SOCIAL_DANGER_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, -5.0),
    ]);
    const HEDONISM_PAIN_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, -5.0),
    ]);
    const HEDONISM_DEEPNESS_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, -5.0),
    ]);
    const EXCITEMENT_ADVENTURE_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, -2.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 2.0),
    ]);
    const EXCITEMENT_THRILL_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, -5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, 5.0),
    ]);
    const PACIFISM_VIOLENCE_MULT: Spline<3> = Spline::new([
        Vec2::new(-MAX_ATTRIBUTE_VALUE, 5.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(MAX_ATTRIBUTE_VALUE, -5.0),
    ]);
    //scale accoring to the value maps, and negate to make risks a detriment
    let riskscore = TaskRiskScores {
        mental_difficulty: -task.risks.mental_difficulty,
        strength_difficulty: -task.risks.strength_difficulty,
        coordination_difficulty: -task.risks.coordination_difficulty,
        social_danger: -task.risks.social_danger*STATUS_SOCIAL_DANGER_MULT.map(values.status.mean()),
        physical_danger: -task.risks.physical_danger,
        legal_danger: -task.risks.legal_danger,
        thrill: -task.risks.thrill*[TRADIATION_THRILL_MULT.map(values.tradition.mean()),EXCITEMENT_THRILL_MULT.map(values.excitement.mean())].iter().mean::<f32>(),
        pain: -task.risks.pain*HEDONISM_PAIN_MULT.map(values.hedonism.mean()),
        monotony: -task.risks.monotony,
        shallowness: -task.risks.shallowness*HEDONISM_DEEPNESS_MULT.map(values.hedonism.mean()),
    };
    let outscore = TaskOutcomeScores {
        wealth: task.outcomes.wealth*WEALTH_WEALTH_MULT.map(values.wealth.mean()),
        status: task.outcomes.status*[POWER_STATUS_MULT.map(values.power.mean()),STATUS_STATUS_MULT.map(values.status.mean())].iter().mean::<f32>(),
        health: task.outcomes.health*[POWER_HEALTH_MULT.map(values.power.mean()),WEALTH_HEALTH_MULT.map(values.wealth.mean())].iter().mean::<f32>(),
        violence: task.outcomes.violence*PACIFISM_VIOLENCE_MULT.map(values.pacifism.mean()),
        adventure: task.outcomes.adventure*[TRADIATION_ADVENTURE_MULT.map(values.tradition.mean()),EXCITEMENT_ADVENTURE_MULT.map(values.excitement.mean())].iter().mean::<f32>(),
        approval: task.outcomes.approval
    };
    //average across each category, but set negative weight on detrimental categories (i.e. those from riskscore)
    RawTaskScore(TaskScore {
        ease: [
            riskscore.mental_difficulty,
            riskscore.strength_difficulty,
            riskscore.coordination_difficulty,
        ]
        .iter()
        .mean(),
        enjoyment: [
            riskscore.thrill,
            riskscore.pain,
            riskscore.monotony,
            riskscore.shallowness,
            outscore.violence,
            outscore.adventure
        ]
        .iter()
        .mean(),
        safety: [
            riskscore.social_danger,
            riskscore.physical_danger,
            riskscore.legal_danger,
        ]
        .iter()
        .mean(),
        loot: [
            outscore.wealth,
            outscore.status,
            outscore.health,
            outscore.approval,
        ]
        .iter()
        .mean(),
        goals: 0.0,
    })
}
