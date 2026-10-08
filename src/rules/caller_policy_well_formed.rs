use crate::Context;
use specification_core::Specification;

/// CallerPolicyWellFormed — deterministic rule over the borrowed Choice context.
pub(crate) struct CallerPolicyWellFormed;
impl<T> Specification<Context<'_, T>> for CallerPolicyWellFormed {
    fn is_satisfied_by(&self, c: &Context<'_, T>) -> bool {
        let unit = |p: f64| p.is_finite() && (0.0..=1.0).contains(&p);
        c.policy.min_probability.is_none_or(unit)
            && c.policy.min_confidence.is_none_or(unit)
            && c.policy.epsilon.is_finite()
            && (0.0..=1e-3).contains(&c.policy.epsilon)
            && c.policy.fallback.as_ref().is_none_or(|f| {
                c.request.options.iter().any(|o| o.id == f.option_id)
                    && !f.triggers.contains(&crate::Reason::ConflictingEvidence)
                    && f.triggers
                        .iter()
                        .enumerate()
                        .all(|(i, r)| !f.triggers[..i].contains(r))
            })
    }
}
