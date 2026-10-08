use crate::Context;
use specification_core::Specification;

/// RequestWellFormed — deterministic rule over the borrowed Choice context.
pub(crate) struct RequestWellFormed;
impl<T> Specification<Context<'_, T>> for RequestWellFormed {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        !c.request.question_id.trim().is_empty()
            && !c.request.instructions.trim().is_empty()
            && !c.request.options.is_empty()
            && c.request
                .options
                .iter()
                .all(|o| !o.id.is_empty() && !o.description.trim().is_empty())
            && c.request
                .options
                .iter()
                .map(|o| &o.id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == c.request.options.len()
    }
}
