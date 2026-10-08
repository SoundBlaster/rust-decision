use crate::Context;
use specification_core::Specification;

/// AcceptanceEvidencePresent — deterministic rule over the borrowed Choice context.
pub(crate) struct AcceptanceEvidencePresent;
impl<T> Specification<Context<'_, T>> for AcceptanceEvidencePresent {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        c.policy
            .min_probability
            .is_none_or(|_| c.probability().is_some())
            && c.policy.min_confidence.is_none_or(|_| {
                matches!(
                    c.prediction.map(|p| &p.confidence),
                    Some(crate::Confidence::Present(_))
                )
            })
    }
}
