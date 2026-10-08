use crate::Confidence;
use crate::numeric::unit;
use crate::scalar::{ScalarContext, ScalarPrediction};
use specification_core::Specification;
pub(crate) struct ScalarConfidenceValid;
impl Specification<ScalarContext<'_>> for ScalarConfidenceValid {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        match c.prediction {
            Some(ScalarPrediction::Predicate { .. }) => true,
            Some(ScalarPrediction::Score {
                confidence: Confidence::Present(v),
                ..
            }) => unit(*v),
            Some(ScalarPrediction::Score {
                confidence: Confidence::Unavailable,
                ..
            }) => true,
            _ => false,
        }
    }
}
