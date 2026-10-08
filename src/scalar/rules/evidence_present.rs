use crate::Confidence;
use crate::scalar::{ScalarContext, ScalarPolicy, ScalarPrediction};
use specification_core::Specification;
pub(crate) struct ScalarEvidencePresent;
impl Specification<ScalarContext<'_>> for ScalarEvidencePresent {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        match c.policy {
            ScalarPolicy::Predicate(_) => {
                matches!(c.prediction, Some(ScalarPrediction::Predicate { .. }))
            }
            ScalarPolicy::Score(p) => {
                p.min_confidence.is_none() || matches!(c.confidence(), Some(Confidence::Present(_)))
            }
        }
    }
}
