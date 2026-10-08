use crate::Context;
use specification_core::Specification;

/// AdvertisedConfidenceValid — deterministic rule over the borrowed Choice context.
pub(crate) struct AdvertisedConfidenceValid;
impl<T> Specification<Context<'_, T>> for AdvertisedConfidenceValid {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        match c.prediction.map(|p| &p.confidence) {
            Some(crate::Confidence::Present(p)) => p.is_finite() && (0.0..=1.0).contains(p),
            Some(crate::Confidence::Unavailable) => true,
            _ => false,
        }
    }
}
