use crate::Context;
use specification_core::Specification;

/// SelectedOptionConsistent — deterministic rule over the borrowed Choice context.
pub(crate) struct SelectedOptionConsistent;
impl<T> Specification<Context<'_, T>> for SelectedOptionConsistent {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        let Some(p) = c.prediction else {
            return false;
        };
        let Some(selected) = c.probability() else {
            return false;
        };
        let max = p
            .probabilities
            .iter()
            .map(|(_, p)| *p)
            .fold(f64::NEG_INFINITY, f64::max);
        max - selected <= c.policy.epsilon
    }
}
