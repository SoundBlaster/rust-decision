mod request_well_formed;
pub(crate) use request_well_formed::ScalarRequestWellFormed;
mod policy_well_formed;
pub(crate) use policy_well_formed::ScalarPolicyWellFormed;
mod backend_supports_request;
pub(crate) use backend_supports_request::ScalarBackendSupportsRequest;
mod answer_matches_request;
pub(crate) use answer_matches_request::ScalarAnswerMatchesRequest;
mod probabilities_valid;
pub(crate) use probabilities_valid::ScalarProbabilitiesValid;
mod confidence_valid;
pub(crate) use confidence_valid::ScalarConfidenceValid;
mod value_consistent;
pub(crate) use value_consistent::ScalarValueConsistent;
mod evidence_present;
pub(crate) use evidence_present::ScalarEvidencePresent;
mod policy_satisfied;
pub(crate) use policy_satisfied::ScalarPolicySatisfied;
pub(crate) const IDS: [&str; 9] = [
    "ScalarRequestWellFormed",
    "ScalarPolicyWellFormed",
    "ScalarBackendSupportsRequest",
    "ScalarAnswerMatchesRequest",
    "ScalarProbabilitiesValid",
    "ScalarConfidenceValid",
    "ScalarValueConsistent",
    "ScalarEvidencePresent",
    "ScalarPolicySatisfied",
];
