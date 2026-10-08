use crate::Context;
use specification_core::Specification;

/// AcceptancePolicySatisfied — deterministic rule over the borrowed Choice context.
pub(crate) struct AcceptancePolicySatisfied;
impl<T> Specification<Context<'_, T>> for AcceptancePolicySatisfied {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        c.policy.min_probability.is_none_or(|t| c.probability().is_some_and(|p| p >= t))
            && c.policy.min_confidence.is_none_or(|t| matches!(c.prediction.map(|p| &p.confidence), Some(crate::Confidence::Present(p)) if *p >= t))
    }
}
