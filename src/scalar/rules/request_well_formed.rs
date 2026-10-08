use crate::scalar::{ScalarContext, ScalarRequest};
use specification_core::Specification;
pub(crate) struct ScalarRequestWellFormed;
impl Specification<ScalarContext<'_>> for ScalarRequestWellFormed {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        !c.request.question_id().trim().is_empty()
            && !c.request.instructions().trim().is_empty()
            && match c.request {
                ScalarRequest::Predicate(r) => [
                    r.true_description.as_deref(),
                    r.false_description.as_deref(),
                ]
                .into_iter()
                .flatten()
                .all(|s| !s.trim().is_empty()),
                ScalarRequest::Score(r) => {
                    !r.levels.is_empty() && r.levels.iter().all(|s| !s.trim().is_empty())
                }
            }
    }
}
