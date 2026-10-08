use crate::Context;
use specification_core::Specification;

/// BackendSupportsRequest — deterministic rule over the borrowed Choice context.
pub(crate) struct BackendSupportsRequest;
impl<T> Specification<Context<'_, T>> for BackendSupportsRequest {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        c.capabilities.text_choice
            && c.request.options.len() <= c.capabilities.max_options
            && c.request
                .context
                .len()
                .checked_add(c.request.instructions.len())
                .and_then(|n| n.checked_add(c.request.question_id.len()))
                .and_then(|n| {
                    c.request.options.iter().try_fold(n, |n, o| {
                        n.checked_add(o.id.len())?.checked_add(o.description.len())
                    })
                })
                .is_some_and(|n| n <= c.capabilities.max_text_bytes)
    }
}
