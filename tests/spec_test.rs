#![allow(clippy::unwrap_used, clippy::expect_used)]

use tessel::Spec;

#[test]
fn loads_default_softfactory_pack() {
    let toml = include_str!("../packs/softfactory.toml");
    let spec =
        Spec::from_toml(toml, "packs/softfactory.toml").expect("must parse softfactory pack");
    assert_eq!(spec.spec.id, "softfactory-core");
    assert_eq!(spec.dimensions.len(), 5);
}

#[test]
fn rejects_duplicate_dimension_name() {
    let toml = r#"
[spec]
id = "test"
version = "1.0.0"

[[dimension]]
name = "persona"
values = [{ id = "dev", weight = 1 }]

[[dimension]]
name = "persona"
values = [{ id = "admin", weight = 1 }]
"#;
    let res = Spec::from_toml(toml, "test.toml");
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("duplicate dimension name"));
}

#[test]
fn rejects_invalid_id_characters() {
    let toml = r#"
[spec]
id = "TEST!"
version = "1.0.0"

[[dimension]]
name = "valid"
values = [{ id = "v1", weight = 1 }]
"#;
    let res = Spec::from_toml(toml, "test.toml");
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("spec.id `TEST!` is invalid"));
}

#[test]
fn rejects_zero_weight() {
    let toml = r#"
[spec]
id = "test"
version = "1.0.0"

[[dimension]]
name = "persona"
values = [{ id = "dev", weight = 0 }]
"#;
    let res = Spec::from_toml(toml, "test.toml");
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("has weight 0"));
}
