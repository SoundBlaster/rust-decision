#!/usr/bin/env python3
"""Offline finite Choice model. Standard library only; no production inference."""

from __future__ import annotations

import argparse
from collections import deque
from dataclasses import asdict, dataclass, replace
from datetime import datetime, timezone
import hashlib
from itertools import product
import json
from pathlib import Path
import platform
import subprocess
import sys

VERSION = "1.0.0"
RULES = (
    "RequestWellFormed", "CallerPolicyWellFormed", "BackendSupportsRequest",
    "AnswerMatchesRequest", "ProbabilityDistributionValid",
    "AdvertisedConfidenceValid", "SelectedOptionConsistent",
    "AcceptanceEvidencePresent", "AcceptancePolicySatisfied",
)
PHASES = (
    "Request", "Policy", "Capability", "Invoke", "Await", "Dispatch",
    "Output", "Evidence", "Thresholds", "Commit", "Done",
)
EVENTS = (
    "prediction", "refusal", "transport_error", "authentication_error",
    "malformed_response", "unsupported", "timed_out", "cancelled",
)
TRIGGERS = {"provider_refusal": 1, "missing_evidence": 2, "policy_rejected": 4}
CONFIDENCE = {
    "S": ("valid_present", "declared_absent"),
    "V": ("invalid_present",), "U": ("unsupported",), "C": ("contradictory",),
}
MUTATIONS = (
    "accept_invalid_confidence", "repeat_await", "fallback_authentication",
    "accept_missing_evidence", "rewrite_done", "nondeterministic_request",
)


@dataclass(frozen=True)
class Input:
    plan: tuple
    confidence: str
    requires_confidence: bool
    fallback: int  # -1 absent; 0..7 allowed trigger subsets


@dataclass(frozen=True)
class Outcome:
    kind: str
    reason: str = ""
    value: str = ""


@dataclass(frozen=True)
class State:
    inputs: Input
    phase: str = "Request"
    invocations: int = 0
    event: str = ""
    assessed: tuple = ("NE",) * 9
    log: tuple = ()
    cursor: int = 0
    cancelled: bool = False
    expired: bool = False
    candidate: Outcome | None = None
    published: Outcome | None = None


@dataclass(frozen=True)
class Action:
    cancel: bool = False
    expire: bool = False
    deliver: str = ""


def representatives(mode):
    """Enumerate all consistent plans, then quotient only dormant plan entries.

    A first structural failure makes later planned checks unobservable. If no
    structural check fails, a non-S evidence assessment hides only thresholds.
    All other input dimensions are retained. No executed assessment is erased.
    """
    classes = {}
    full_count = 0
    for plan in product("SVUC", repeat=9):
        if mode == "primary" and plan[7] == "S" and plan[8] not in "SV":
            continue
        failure = next((i for i in range(7) if plan[i] != "S"), None)
        observed = (
            ("structural_failure", failure, plan[failure]) if failure is not None
            else ("acceptance", plan[7], plan[8] if plan[7] == "S" else "NE")
        )
        for confidence in CONFIDENCE[plan[5]]:
            for required in (False, True):
                if (failure is None and confidence == "declared_absent"
                        and required and plan[7] not in "VU"):
                    continue
                full_count += 9
                for fallback in range(-1, 8):
                    key = (observed, confidence, required, fallback)
                    classes.setdefault(key, Input(plan, confidence, required, fallback))
    return tuple(classes.values()), full_count


def actions(state):
    deliveries = ("", *EVENTS) if state.phase == "Await" else ("",)
    return (Action(c, e, d) for c, e, d in product((False, True), (False, True), deliveries))


def route_reason(inputs, reason):
    bit = TRIGGERS.get(reason, 0)
    if bit and inputs.fallback >= 0 and inputs.fallback & bit:
        return Outcome("Fallback", reason, "fallback_value")
    return Outcome("Abstained", reason)


def propose(state, outcome):
    return replace(state, phase="Commit", candidate=outcome)


def evaluate(state, index):
    value = state.inputs.plan[index]
    assessed = list(state.assessed)
    assessed[index] = value
    return replace(state, assessed=tuple(assessed), log=state.log + ((index, value),)), value


def successors(state, action, mutation=""):
    """Enabled transition rules for a fixed state and explicit environment action."""
    if state.phase == "Done":
        if mutation == "rewrite_done" and action.cancel:
            return [replace(state, published=Outcome("Cancelled"))]
        return [state]
    cancelled = state.cancelled or action.cancel or (
        state.phase == "Dispatch" and state.event == "cancelled"
    )
    updated = replace(state, cancelled=cancelled, expired=state.expired or action.expire)
    if updated.cancelled:
        return [replace(updated, phase="Done", published=Outcome("Cancelled"))]
    if updated.expired:
        return [replace(updated, phase="Done", published=Outcome("Failed", "timed_out"))]

    result = []
    if updated.phase in ("Request", "Policy", "Capability"):
        index = ("Request", "Policy", "Capability").index(updated.phase)
        assessed, value = evaluate(updated, index)
        if value == "S":
            result.append(replace(assessed, phase=("Policy", "Capability", "Invoke")[index]))
        else:
            reason = ("request_validation", "policy_validation", "capability_validation")[index]
            result.append(propose(assessed, Outcome("Failed", reason)))
    if updated.phase == "Invoke" and updated.invocations == 0 and updated.assessed[:3] == ("S",) * 3:
        result.append(replace(updated, phase="Await", invocations=1))
    if updated.phase == "Await":
        if mutation == "repeat_await" and not action.deliver:
            result.append(replace(updated, invocations=updated.invocations + 1))
        elif action.deliver:
            result.append(replace(updated, phase="Dispatch", event=action.deliver))
        else:
            result.append(updated)
    if updated.phase == "Dispatch":
        if updated.event == "prediction":
            result.append(replace(updated, phase="Output", cursor=0))
        elif updated.event == "refusal":
            result.append(propose(updated, route_reason(updated.inputs, "provider_refusal")))
        elif mutation == "fallback_authentication" and updated.event == "authentication_error":
            result.append(propose(updated, Outcome("Fallback", "authentication_error", "fallback_value")))
        elif updated.event in EVENTS:
            result.append(propose(updated, Outcome("Failed", updated.event)))
    if updated.phase == "Output":
        assessed, value = evaluate(updated, updated.cursor + 3)
        bypass = mutation == "accept_invalid_confidence" and updated.cursor == 2
        if value != "S" and not bypass:
            result.append(propose(assessed, Outcome("Failed", "output_validation")))
        elif updated.cursor == 3:
            result.append(replace(assessed, phase="Evidence"))
        else:
            result.append(replace(assessed, cursor=updated.cursor + 1))
    if updated.phase == "Evidence":
        assessed, value = evaluate(updated, 7)
        if value == "S":
            result.append(replace(assessed, phase="Thresholds"))
        elif mutation == "accept_missing_evidence":
            result.append(propose(assessed, Outcome("Accepted", value="selected_value")))
        else:
            reason = "conflicting_evidence" if value == "C" else "missing_evidence"
            result.append(propose(assessed, route_reason(updated.inputs, reason)))
    if updated.phase == "Thresholds":
        assessed, value = evaluate(updated, 8)
        if value == "S":
            outcome = Outcome("Accepted", value="selected_value")
        else:
            reason = {"V": "policy_rejected", "U": "missing_evidence", "C": "conflicting_evidence"}[value]
            outcome = route_reason(updated.inputs, reason)
        result.append(propose(assessed, outcome))
    if updated.phase == "Commit" and updated.candidate is not None:
        result.append(replace(updated, phase="Done", published=updated.candidate))
    if mutation == "nondeterministic_request" and updated.phase == "Request":
        result.append(propose(updated, Outcome("Failed", "internal_contract")))
    return result


def invariant(state):
    """Independent predicates over state, not a second call to the router."""
    if state.invocations not in (0, 1):
        return "P6"
    if (state.published is not None) != (state.phase == "Done"):
        return "P3"
    if len(state.log) > 9 or tuple(i for i, _ in state.log) != tuple(range(len(state.log))):
        return "P10"
    expected = tuple(state.inputs.plan[i] if i < len(state.log) else "NE" for i in range(9))
    if state.assessed != expected or any(value != state.inputs.plan[i] for i, value in state.log):
        return "P10"
    out = state.published
    if out is None:
        return None
    if out.kind == "Accepted":
        if state.inputs.confidence == "invalid_present":
            return "P8"
        if state.inputs.confidence == "declared_absent" and state.inputs.requires_confidence:
            return "P11"
        if state.assessed != ("S",) * 9 or out.value != "selected_value":
            return "P4"
        if state.event != "prediction" or state.invocations != 1:
            return "P5"
    if out.kind == "Fallback":
        bit = TRIGGERS.get(out.reason, 0)
        if (state.assessed[1] != "S" or state.inputs.fallback < 0 or not bit
                or not state.inputs.fallback & bit or out.value != "fallback_value"):
            return "P7"
    return None


def rank(state):
    return PHASES.index(state.phase) * 4 + (state.cursor if state.phase == "Output" else 0)


def transition_property(before, action, after):
    if before.inputs != after.inputs:
        return "immutable_input"
    if before.phase == "Done" and before != after:
        return "P3"
    if before.phase == "Await" and before.invocations != after.invocations:
        return "P6"
    if before.event and before.event != after.event:
        return "latched_event"
    if before.cancelled and not after.cancelled or before.expired and not after.expired:
        return "sticky_observations"
    if after.invocations > before.invocations and (
        before.phase != "Invoke" or before.assessed[:3] != ("S",) * 3
    ):
        return "P6"
    if before.phase != "Done":
        cancelled = before.cancelled or action.cancel or (before.phase == "Dispatch" and before.event == "cancelled")
        if cancelled and after.published != Outcome("Cancelled"):
            return "P9"
        if not cancelled and (before.expired or action.expire) and after.published != Outcome("Failed", "timed_out"):
            return "deadline_priority"
        if cancelled or before.expired or action.expire:
            if before.log != after.log or before.invocations != after.invocations:
                return "P10"
        elif before.phase == "Commit" and after.published != before.candidate:
            return "P3"
        if after == before and before.phase != "Await":
            return "P12"
        if after != before and rank(after) <= rank(before):
            return "P12"
    return invariant(after)


def witness(state, action, next_states, property_id, parents):
    path = []
    cursor = state
    while parents[cursor] is not None:
        previous, previous_action = parents[cursor]
        path.append({"action": asdict(previous_action), "after": asdict(cursor)})
        cursor = previous
    path.reverse()
    return {
        "property": property_id, "initial": asdict(cursor), "prefix": path,
        "state": asdict(state), "action": asdict(action),
        "successors": [asdict(s) for s in next_states],
    }


def explore(inputs, mutation=""):
    counts = {"states": 0, "state_action_pairs": 0}
    outcomes = set()
    for initial_input in inputs:
        initial = State(initial_input)
        queue = deque([initial])
        parents = {initial: None}
        while queue:
            state = queue.popleft()
            counts["states"] += 1
            if state.published:
                outcomes.add((state.published.kind, state.published.reason))
            for action in actions(state):
                counts["state_action_pairs"] += 1
                next_states = successors(state, action, mutation)
                prop = "P1" if not next_states else "P2" if len(next_states) != 1 else None
                if prop is None:
                    prop = transition_property(state, action, next_states[0])
                if prop:
                    return {**counts, "passed": False, "counterexample": witness(state, action, next_states, prop, parents)}
                after = next_states[0]
                if after not in parents:
                    parents[after] = (state, action)
                    queue.append(after)
    return {**counts, "passed": True, "outcomes": sorted(outcomes), "counterexample": None}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify():
    results = {}
    primary_inputs = None
    for mode in ("primary", "conservative_extension"):
        inputs, full_count = representatives(mode)
        if mode == "primary":
            primary_inputs = inputs
        result = explore(inputs)
        results[mode] = {
            "consistent_full_inputs": full_count, "representative_inputs": len(inputs),
            **result,
            "properties": {f"P{i}": "passed" if result["passed"] else "not_established" for i in range(1, 13)},
            "P12_scope": "Rank increases on non-stutter transitions; only Await and Done stutter. Termination conditional on eventual backend event/observable cancellation/expiry and fair core progression.",
        }
    controls = {}
    for mutation in MUTATIONS:
        result = explore(primary_inputs, mutation)
        controls[mutation] = {"detected": not result["passed"], **result}
    root = Path(__file__).resolve().parents[1]
    try:
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        revision = None
    passed = all(r["passed"] for r in results.values()) and all(c["detected"] for c in controls.values())
    return {
        "schema_version": 1, "checker_version": VERSION, "passed": passed,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "python_version": platform.python_version(), "checkout_revision": revision,
        "source_sha256": {str(p.relative_to(root)): digest(p) for p in (
            Path(__file__).resolve(), root / "docs/choice-finite-model.md", root / "docs/choice-routing-contract.md",
            root / "verification/README.md", root / "verification/test_choice_model.py",
        )},
        "command": [sys.executable, *sys.argv],
        "models": results, "mutation_controls": controls,
        "limits": [
            "Finite routing abstraction with dormant-plan quotient; not production Rust verification.",
            "Validator correctness, floating point, mapping, model accuracy and real cancellation are not proved.",
            "Termination is conditional; permanently waiting backend is allowed in safety exploration.",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = verify()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for mode, result in report["models"].items():
        print(f"{mode}: passed={result['passed']}; {result['consistent_full_inputs']} inputs, "
              f"{result['representative_inputs']} representatives, {result['states']} states")
    print(f"Mutation controls detected: {sum(c['detected'] for c in report['mutation_controls'].values())}/{len(MUTATIONS)}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
