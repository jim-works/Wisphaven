pub struct CreateLevelEvent {
    pub name: &'static str,
    pub seed: u64,
}

pub struct OpenLevelEvent {
    pub name: &'static str,
}