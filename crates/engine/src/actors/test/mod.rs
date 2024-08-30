use crate::actors::personality::scoring::score_task;

use super::personality::components::*;

#[test]
fn zero_personality_scoring() {
    let values = PersonalityValues::default();
    let mental = MentalAttributes::default();
    let physical = PhysicalAttributes::default();
    let tasks = TaskSet::default();
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks::default(),
        outcomes: TaskOutcomes::default(),
    };
    assert_eq!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall(),
        0.0
    );
}

#[test]
fn default_social_danger_reduces_score() {
    let values = PersonalityValues::default();
    let mental = MentalAttributes::default();
    let physical = PhysicalAttributes::default();
    let tasks = TaskSet::default();
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            social_danger: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
}

#[test]
fn default_physical_danger_reduces_score() {
    let values = PersonalityValues::default();
    let mental = MentalAttributes::default();
    let physical = PhysicalAttributes::default();
    let tasks = TaskSet::default();
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            physical_danger: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
}

#[test]
fn default_legal_danger_reduces_score() {
    let values = PersonalityValues::default();
    let mental = MentalAttributes::default();
    let physical = PhysicalAttributes::default();
    let tasks = TaskSet::default();
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            legal_danger: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
}

#[test]
fn default_legal_risks_reduce_score() {
    let values = PersonalityValues::default();
    let mental = MentalAttributes::default();
    let physical = PhysicalAttributes::default();
    let tasks = TaskSet::default();
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            mental_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            strength_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(score.overall() < 0.0, "score: {:?}", score);
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            coordination_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            thrill: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            pain: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            monotony: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            shallowness: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    assert!(
        score_task(&mut to_score, &physical, &mental, &values, &tasks)
            .0
            .overall()
            < 0.0
    );
}

#[test]
fn physical_attrs_reduce_risks() {
    let values = PersonalityValues::default();
    let mental = MentalAttributes::default();
    let default_physical = PhysicalAttributes::default();
    let tasks = TaskSet::default();
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            strength_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &default_physical, &mental, &values, &tasks).0;
    let physical = PhysicalAttributes {
        strength: FacetValue::new(1.0, 1.0).unwrap(),
        ..PhysicalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() < updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            coordination_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &default_physical, &mental, &values, &tasks).0;
    let physical = PhysicalAttributes {
        agility: FacetValue::new(1.0, 1.0).unwrap(),
        ..PhysicalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() < updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            physical_danger: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &default_physical, &mental, &values, &tasks).0;
    let physical = PhysicalAttributes {
        fortitude: FacetValue::new(1.0, 1.0).unwrap(),
        ..PhysicalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() < updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    //disease resistance does nothing, reminder to update the test when i implement it
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            coordination_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &default_physical, &mental, &values, &tasks).0;
    let physical = PhysicalAttributes {
        disease_resistence: FacetValue::new(1.0, 1.0).unwrap(),
        ..PhysicalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() == updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );
}

#[test]
fn mental_attrs_reduce_risks() {
    let values = PersonalityValues::default();
    let default_mental = MentalAttributes::default();
    let physical = PhysicalAttributes::default();
    let tasks = TaskSet::default();
    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            mental_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &physical, &default_mental, &values, &tasks).0;
    let mental = MentalAttributes {
        intelligence: FacetValue::new(1.0, 1.0).unwrap(),
        ..MentalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() < updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            pain: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &physical, &default_mental, &values, &tasks).0;
    let mental = MentalAttributes {
        willpower: FacetValue::new(1.0, 1.0).unwrap(),
        ..MentalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() < updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            monotony: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &physical, &default_mental, &values, &tasks).0;
    let mental = MentalAttributes {
        creativity: FacetValue::new(1.0, 1.0).unwrap(),
        ..MentalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        //should increase effect of monotony
        default_score.overall() > updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            mental_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &physical, &default_mental, &values, &tasks).0;
    let mental = MentalAttributes {
        creativity: FacetValue::new(1.0, 1.0).unwrap(),
        ..MentalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        //and reduce the effect of mental_difficulty
        default_score.overall() < updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            mental_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &physical, &default_mental, &values, &tasks).0;
    let mental = MentalAttributes {
        memory: FacetValue::new(1.0, 1.0).unwrap(),
        ..MentalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        //does nothing, serves as a reminder
        default_score.overall() == updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            social_danger: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &physical, &default_mental, &values, &tasks).0;
    let mental = MentalAttributes {
        social_awareness: FacetValue::new(1.0, 1.0).unwrap(),
        ..MentalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() > updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );

    let mut to_score = Task {
        category: TaskCategory::Idle,
        risks: TaskRisks {
            strength_difficulty: 1.0,
            ..TaskRisks::default()
        },
        outcomes: TaskOutcomes::default(),
    };
    let default_score = score_task(&mut to_score, &physical, &default_mental, &values, &tasks).0;
    let mental = MentalAttributes {
        persistence: FacetValue::new(1.0, 1.0).unwrap(),
        ..MentalAttributes::default()
    };
    let updated_score = score_task(&mut to_score, &physical, &mental, &values, &tasks).0;
    assert!(
        default_score.overall() < updated_score.overall(),
        "default score: {:?}, updated score: {:?}",
        default_score,
        updated_score
    );
}