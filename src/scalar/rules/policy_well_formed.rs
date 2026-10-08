use crate::numeric::unit;
use crate::scalar::{ScalarContext, ScalarPolicy, ScalarRequest};
use crate::{Reason, scalar::ValueFallback};
use specification_core::Specification;
fn fallback_valid<T>(f: Option<&ValueFallback<T>>) -> bool {
    f.is_none_or(|f| {
        !f.triggers.contains(&Reason::ConflictingEvidence)
            && f.triggers
                .iter()
                .enumerate()
                .all(|(i, r)| !f.triggers[..i].contains(r))
    })
}
pub(crate) struct ScalarPolicyWellFormed;
impl Specification<ScalarContext<'_>> for ScalarPolicyWellFormed {
    fn is_satisfied_by(&self, c: &ScalarContext<'_>) -> bool {
        match (c.request, c.policy) {
            (ScalarRequest::Predicate(_), ScalarPolicy::Predicate(p)) => {
                unit(p.reject_at)
                    && unit(p.accept_at)
                    && p.reject_at < p.accept_at
                    && fallback_valid(p.fallback.as_ref())
            }
            (ScalarRequest::Score(_), ScalarPolicy::Score(p)) => {
                let range = |v: f64| v.is_finite() && (0.0..=c.max_score()).contains(&v);
                p.epsilon.is_finite()
                    && (0.0..=1e-3).contains(&p.epsilon)
                    && p.min_confidence.is_none_or(unit)
                    && p.min_score.is_none_or(range)
                    && p.max_score.is_none_or(range)
                    && p.min_score.zip(p.max_score).is_none_or(|(a, b)| a <= b)
                    && fallback_valid(p.fallback.as_ref())
                    && p.fallback.as_ref().is_none_or(|f| range(f.value))
            }
            _ => false,
        }
    }
}
