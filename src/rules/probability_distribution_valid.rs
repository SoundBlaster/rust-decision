use crate::Context;
use specification_core::Specification;

/// ProbabilityDistributionValid — deterministic rule over the borrowed Choice context.
pub(crate) struct ProbabilityDistributionValid;
impl<T> Specification<Context<'_, T>> for ProbabilityDistributionValid {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        let Some(p) = c.prediction else {
            return false;
        };
        if !p
            .probabilities
            .iter()
            .all(|(_, value)| value.is_finite() && (0.0..=1.0).contains(value))
        {
            return false;
        }
        let sum = crate::numeric::sum(p.probabilities.iter().map(|(_, value)| *value).collect());
        sum.is_finite() && (sum - 1.0).abs() <= c.policy.epsilon
    }
}
