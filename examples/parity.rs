//! Verification-only line protocol; not a provider or public production API.
use rust_decision::routing::{Action, State};
use std::io::{self, BufRead};
#[derive(serde::Deserialize)]
struct Case {
    state: State,
    actions: Vec<Action>,
}
fn main() {
    for line in io::stdin().lock().lines() {
        let case: Case = serde_json::from_str(&line.expect("input")).expect("case");
        assert_eq!(case.state.inputs.plan.len(), 9);
        let out: Vec<_> = case
            .actions
            .iter()
            .map(|action| {
                case.state
                    .step(action, |i| case.state.inputs.plan[i].clone())
            })
            .collect();
        println!("{}", serde_json::to_string(&out).expect("state"));
    }
}
