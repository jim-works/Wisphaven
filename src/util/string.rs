#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub struct Version {
    pub major: Option<u32>,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub cruft: Option<String>,
}

impl Version {
    pub fn game_version() -> Self {
        Self::from(env!("CARGO_PKG_VERSION"))
    }

    //game is compatible if the major/minor versions are equal
    pub fn game_compatible(&self, other: &Version) -> bool {
        return self.major.is_some()
            && self.major == other.major
            && self.minor.is_some()
            && self.minor == other.minor;
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.minor.partial_cmp(&other.minor) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.patch.partial_cmp(&other.patch) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.cruft.partial_cmp(&other.cruft)
    }
}

impl From<&str> for Version {
    fn from(value: &str) -> Self {
        let mut result = Self::default();
        let pieces = value.split(".").collect::<Vec<_>>();
        if let Some(major) = pieces.get(0) {
            result.major = major.parse::<u32>().ok();
        }
        if let Some(minor) = pieces.get(1) {
            result.minor = minor.parse::<u32>().ok();
        }
        if let Some(patch) = pieces.get(2) {
            result.patch = patch.parse::<u32>().ok();
        }
        if let Some(cruft) = pieces.get(3) {
            result.cruft = Some(cruft.to_string());
        }
        result
    }
}