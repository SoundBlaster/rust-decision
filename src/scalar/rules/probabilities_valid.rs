use crate::numeric::{sum, unit};
use crate::scalar::{ScalarContext, ScalarPolicy, ScalarPrediction, ScalarRequest};
use specification_core::Specification;
pub(crate) struct ScalarProbabilitiesValid;
impl Specification<ScalarContext<'_>> for ScalarProbabilitiesValid {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        match (c.request, c.prediction, c.policy) {
            (
                ScalarRequest::Predicate(_),
                Some(ScalarPrediction::Predicate {
                    probability_true, ..
                }),
                _,
            ) => unit(*probability_true),
            (
                ScalarRequest::Score(r),
                Some(ScalarPrediction::Score { probabilities, .. }),
                ScalarPolicy::Score(p),
            ) => {
                let mut ids: Vec<_> = probabilities.iter().map(|(id, _)| *id).collect();
                ids.sort_unstable();
                ids == (0..r.levels.len()).collect::<Vec<_>>()
                    && probabilities.iter().all(|(_, v)| unit(*v))
                    && (sum(probabilities.iter().map(|(_, v)| *v).collect()) - 1.0).abs()
                        <= p.epsilon
            }
            _ => false,
        }
    }
}
