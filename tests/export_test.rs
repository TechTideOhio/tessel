#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use tessel::{export_reval_cases, generate_plan, realize_tuple, Spec};

#[test]
fn exports_cases_to_reval_folder_structure() {
    let tmp = std::env::temp_dir().join(format!(
        "tessel_export_test_{}",
        std::time::SystemTime::now().elapsed().unwrap().as_nanos()
    ));
    fs::create_dir_all(&tmp).unwrap();

    let toml = include_str!("../packs/softfactory.toml");
    let spec = Spec::from_toml(toml, "pack.toml").unwrap();
    let plan = generate_plan(&spec).unwrap();

    let mut cases = Vec::new();
    for t in plan.tuples.iter().take(3) {
        cases.push(realize_tuple(t, &spec, 357).unwrap());
    }

    let n = export_reval_cases(&cases, &tmp).expect("export to reval format");
    assert_eq!(n, 3);

    for c in &cases {
        let case_toml = tmp.join("cases").join(&c.id).join("case.toml");
        let prompt_md = tmp.join("cases").join(&c.id).join("prompt.md");
        assert!(case_toml.is_file(), "case.toml missing for {}", c.id);
        assert!(prompt_md.is_file(), "prompt.md missing for {}", c.id);

        let content = fs::read_to_string(&case_toml).unwrap();
        assert!(content.contains(&c.id));
        assert!(content.contains(&c.stratum));
    }

    let _ = fs::remove_dir_all(&tmp);
}
