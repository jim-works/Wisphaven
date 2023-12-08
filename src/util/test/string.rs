use crate::util::string::Version;

#[test]
pub fn create_version() {
    let version = "1.2.3.asdfasdf";
    assert_eq!(Version::from(version), Version { major: Some(1), minor: Some(2), patch: Some(3), cruft: Some("asdfasdf".to_string()) });
}

#[test]
pub fn test_compatability() {
    assert!(Version::from("1.1").game_compatible(&"1.1.2.stuff".into()));
    assert!(!Version::from("1.1").game_compatible(&"1.2.2.stuff".into()));
}

#[test]
pub fn test_ord_eq() {
    assert!(Version::from("1.1") < Version::from("1.1.2.stuff"));
    assert!(Version::from("1.2.0.a") > Version::from("1.1.2.stuff"));
    assert!(Version::from("1.2.3.stuff") == Version::from("1.2.3.stuff"));
}