use crate::Context;
use specification_core::Specification;

/// AnswerMatchesRequest — deterministic rule over the borrowed Choice context.
pub(crate) struct AnswerMatchesRequest;
impl<T> Specification<Context<'_, T>> for AnswerMatchesRequest {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        let Some(p) = c.prediction else {
            return false;
        };
        p.question_id == c.request.question_id
            && p.probabilities.len() == c.request.options.len()
            && p.probabilities
                .iter()
                .map(|(id, _)| id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == p.probabilities.len()
            && c.request
                .options
                .iter()
                .all(|o| p.probabilities.iter().any(|(id, _)| id == &o.id))
    }
}
