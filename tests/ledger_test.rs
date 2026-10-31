#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use tessel::{generate_plan, realize_tuple, save_ledger, verify_ledger, Spec};

#[test]
fn saves_and_verifies_ledger_zero_drift() {
    let tmp = std::env::temp_dir().join(format!(
        "tessel_test_{}",
        std::time::SystemTime::now().elapsed().unwrap().as_nanos()
    ));
    fs::create_dir_all(&tmp).unwrap();

    let spec_path = tmp.join("dimensions.toml");
    let toml = include_str!("../packs/softfactory.toml");
    fs::write(&spec_path, toml).unwrap();

    let spec = Spec::from_toml(toml, &spec_path).unwrap();
    let plan = generate_plan(&spec).unwrap();

    let mut cases = Vec::new();
    for t in plan.tuples.iter().take(5) {
        cases.push(realize_tuple(t, &spec, 357).unwrap());
    }

    let out_dir = tmp.join("out");
    save_ledger(&out_dir, &spec_path, &spec, &plan.tuples[..5], &cases).expect("save ledger");

    assert!(verify_ledger(&out_dir, &spec_path).is_ok());

    // Tamper with cases.jsonl
    let cases_file = out_dir.join("cases.jsonl");
    fs::write(&cases_file, "{\"tampered\": true}\n").unwrap();

    let verify_res = verify_ledger(&out_dir, &spec_path);
    assert!(
        verify_res.is_err(),
        "tampered file must cause verification failure"
    );

    let _ = fs::remove_dir_all(&tmp);
}
