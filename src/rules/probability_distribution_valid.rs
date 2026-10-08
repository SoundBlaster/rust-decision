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
        // Canonical numerical order makes the result independent of request,
        // response and opaque ID ordering. Sort a copy, preserving caller data.
        let mut values: Vec<_> = p.probabilities.iter().map(|(_, value)| *value).collect();
        values.sort_unstable_by(f64::total_cmp);
        // Neumaier compensation recovers rounding lost by sequential addition.
        // Canonical ordering is still required: compensation alone does not
        // guarantee bitwise reproducibility under arbitrary permutations.
        let (mut sum, mut correction) = (0.0_f64, 0.0_f64);
        for value in values {
            let next = sum + value;
            correction += if sum.abs() >= value.abs() {
                (sum - next) + value
            } else {
                (value - next) + sum
            };
            sum = next;
        }
        let sum = sum + correction;
        sum.is_finite() && (sum - 1.0).abs() <= c.policy.epsilon
    }
}
