use crate::scalar::{ScalarContext, ScalarRequest};
use specification_core::Specification;
pub(crate) struct ScalarBackendSupportsRequest;
impl Specification<ScalarContext<'_>> for ScalarBackendSupportsRequest {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        c.request
            .text_bytes()
            .is_some_and(|n| n <= c.capabilities.max_text_bytes)
            && match c.request {
                ScalarRequest::Predicate(_) => c.capabilities.predicate,
                ScalarRequest::Score(r) => {
                    c.capabilities.score && r.levels.len() <= c.capabilities.max_score_levels
                }
            }
    }
}
