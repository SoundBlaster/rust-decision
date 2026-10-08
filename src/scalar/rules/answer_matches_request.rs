use crate::scalar::{ScalarContext, ScalarPrediction, ScalarRequest};
use specification_core::Specification;
pub(crate) struct ScalarAnswerMatchesRequest;
impl Specification<ScalarContext<'_>> for ScalarAnswerMatchesRequest {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        match (c.request, c.prediction) {
            (
                ScalarRequest::Predicate(r),
                Some(ScalarPrediction::Predicate { question_id, .. }),
            ) => &r.question_id == question_id,
            (ScalarRequest::Score(r), Some(ScalarPrediction::Score { question_id, .. })) => {
                &r.question_id == question_id
            }
            _ => false,
        }
    }
}
