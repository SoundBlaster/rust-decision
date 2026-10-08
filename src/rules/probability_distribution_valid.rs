use crate::Context;
use specification_core::Specification;

/// ProbabilityDistributionValid — deterministic rule over the borrowed Choice context.
pub(crate) struct ProbabilityDistributionValid;
impl<T> Specification<Context<'_, T>> for ProbabilityDistributionValid {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        let Some(p) = c.prediction else {
            return false;
        };
        let sum: f64 = p.probabilities.iter().map(|(_, p)| p).sum();
        p.probabilities
            .iter()
            .all(|(_, p)| p.is_finite() && (0.0..=1.0).contains(p))
            && sum.is_finite()
            && (sum - 1.0).abs() <= c.policy.epsilon
    }
}
