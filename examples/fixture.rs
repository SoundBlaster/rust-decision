//! Local, deterministic Choice example; no provider or credentials.
use rust_decision::*;
struct Fixture;
impl Backend for Fixture {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            text_choice: true,
            max_text_bytes: 4096,
            max_options: 8,
        }
    }
    fn invoke(&mut self, request: &BackendRequest) -> BackendEvent {
        BackendEvent::Prediction(Prediction {
            question_id: request.question_id.clone(),
            selected_id: "policy".into(),
            probabilities: vec![("policy".into(), 0.85), ("mechanics".into(), 0.15)],
            confidence: Confidence::Unavailable,
        })
    }
}
#[derive(Clone, Debug)]
enum Category {
    Policy,
    Mechanics,
}
fn main() {
    let request = Request {
        question_id: "category".into(),
        context: "An illustrative reviewed rule".into(),
        instructions: "Choose the category".into(),
        options: vec![
            Choice {
                id: "policy".into(),
                description: "Domain policy".into(),
                value: Category::Policy,
            },
            Choice {
                id: "mechanics".into(),
                description: "Implementation mechanics".into(),
                value: Category::Mechanics,
            },
        ],
    };
    let policy = Policy {
        min_probability: Some(0.8),
        ..Default::default()
    };
    let report = decide(&request, &policy, &mut Fixture, Observation::default);
    println!(
        "{:?}; {} evaluated rules",
        report.decision,
        report.rules.len()
    );
}
