//! Every case of the golden table, recorded from `hx` 25.07.1, replayed in
//! the simulator. The table is regenerated, and its drift from a live `hx`
//! checked, by `examples/hx_differential.rs`.

use crate::helix::goldens::GoldenTable;
use std::collections::HashSet;

const TABLE: &str = include_str!("hx_goldens.toml");

fn table() -> GoldenTable {
    GoldenTable::parse(TABLE).expect("hx_goldens.toml is a valid golden table")
}

#[test]
fn case_ids_are_unique() {
    let table = table();
    let ids: HashSet<&str> = table.cases.iter().map(|case| case.id.as_str()).collect();
    assert_eq!(ids.len(), table.cases.len());
}

#[test]
fn every_case_has_a_recorded_result() {
    for case in table().cases {
        assert!(
            !case.expected.is_empty(),
            "{} has no expected text",
            case.id
        );
    }
}

#[test]
fn the_simulator_matches_every_recorded_case() {
    let mismatches: Vec<String> = table()
        .cases
        .iter()
        .filter_map(|case| match case.simulate() {
            Ok(actual) if actual == case.expected => None,
            Ok(actual) => Some(format!(
                "{}\n  hx:        {:?}\n  simulator: {actual:?}",
                case.id, case.expected
            )),
            Err(error) => Some(format!("{}: {error}", case.id)),
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
