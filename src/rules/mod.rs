mod request_well_formed;
pub(crate) use request_well_formed::RequestWellFormed;
mod caller_policy_well_formed;
pub(crate) use caller_policy_well_formed::CallerPolicyWellFormed;
mod backend_supports_request;
pub(crate) use backend_supports_request::BackendSupportsRequest;
mod answer_matches_request;
pub(crate) use answer_matches_request::AnswerMatchesRequest;
mod probability_distribution_valid;
pub(crate) use probability_distribution_valid::ProbabilityDistributionValid;
mod advertised_confidence_valid;
pub(crate) use advertised_confidence_valid::AdvertisedConfidenceValid;
mod selected_option_consistent;
pub(crate) use selected_option_consistent::SelectedOptionConsistent;
mod acceptance_evidence_present;
pub(crate) use acceptance_evidence_present::AcceptanceEvidencePresent;
mod acceptance_policy_satisfied;
pub(crate) use acceptance_policy_satisfied::AcceptancePolicySatisfied;
pub(crate) const IDS: [&str; 9] = [
    "RequestWellFormed",
    "CallerPolicyWellFormed",
    "BackendSupportsRequest",
    "AnswerMatchesRequest",
    "ProbabilityDistributionValid",
    "AdvertisedConfidenceValid",
    "SelectedOptionConsistent",
    "AcceptanceEvidencePresent",
    "AcceptancePolicySatisfied",
];
