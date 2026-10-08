"""Regression checks for the checker, quotient and reproducible witnesses."""

from collections import deque
from dataclasses import asdict, replace
import json
import unittest

from choice_model import (
    Action, Input, MUTATIONS, Outcome, State, actions, explore,
    successors, transition_property,
)


def inputs(plan=("S",) * 9, confidence="valid_present", required=False, fallback=-1):
    return Input(plan, confidence, required, fallback)


def from_json(data):
    data = dict(data)
    raw = data.pop("inputs")
    data["inputs"] = Input(tuple(raw["plan"]), raw["confidence"], raw["requires_confidence"], raw["fallback"])
    data["assessed"] = tuple(data["assessed"])
    data["log"] = tuple(tuple(item) for item in data["log"])
    for key in ("candidate", "published"):
        data[key] = Outcome(**data[key]) if data[key] is not None else None
    return State(**data)


def projected_graph(context):
    """Compare observable transition graphs, independently of quotient keys."""
    initial = State(context)
    queue, seen, edges = deque([initial]), {initial}, set()

    def projection(state):
        return replace(state, inputs=replace(state.inputs, plan=()))

    while queue:
        state = queue.popleft()
        for action in actions(state):
            after, = successors(state, action)
            edges.add((projection(state), action, projection(after)))
            if after not in seen:
                seen.add(after)
                queue.append(after)
    return edges


class CheckerTests(unittest.TestCase):
    def test_dormant_structural_suffix_has_identical_transition_graph(self):
        a = inputs(("S", "S", "S", "V", "S", "S", "S", "S", "S"))
        b = replace(a, plan=("S", "S", "S", "V", "C", "S", "U", "C", "V"))
        self.assertEqual(projected_graph(a), projected_graph(b))

    def test_dormant_threshold_has_identical_transition_graph(self):
        a = inputs(("S",) * 7 + ("V", "S"))
        b = replace(a, plan=("S",) * 7 + ("V", "C"))
        self.assertEqual(projected_graph(a), projected_graph(b))

    def test_backend_cancelled_wins_at_dispatch(self):
        state = State(inputs(), phase="Dispatch", invocations=1, event="cancelled")
        after, = successors(state, Action(expire=True))
        self.assertEqual(after.published, Outcome("Cancelled"))

    def test_committed_result_ignores_late_observations(self):
        state = State(inputs(), phase="Done", published=Outcome("Failed", "request_validation"))
        self.assertEqual(successors(state, Action(cancel=True, expire=True)), [state])

    def test_unknown_required_confidence_cannot_accept(self):
        context = inputs(("S",) * 7 + ("U", "S"), "declared_absent", True, 7)
        report = explore([context])
        self.assertTrue(report["passed"])
        self.assertFalse(any(kind == "Accepted" for kind, _ in report["outcomes"]))

    def test_all_control_counterexamples_survive_json_roundtrip_and_replay(self):
        cases = [
            inputs(),
            inputs(("S",) * 5 + ("V", "S", "S", "S"), "invalid_present"),
            inputs(("S",) * 7 + ("V", "S"), "declared_absent", True),
        ]
        expected = {
            "accept_invalid_confidence": "P8", "repeat_await": "P6",
            "fallback_authentication": "P7", "accept_missing_evidence": "P11",
            "rewrite_done": "P3", "nondeterministic_request": "P2",
        }
        for mutation in MUTATIONS:
            with self.subTest(mutation=mutation):
                result = explore(cases, mutation)
                self.assertFalse(result["passed"])
                witness = json.loads(json.dumps(result["counterexample"]))
                self.assertEqual(witness["property"], expected[mutation])
                state = from_json(witness["initial"])
                for entry in witness["prefix"]:
                    after, = successors(state, Action(**entry["action"]), mutation)
                    self.assertEqual(after, from_json(entry["after"]))
                    state = after
                self.assertEqual(state, from_json(witness["state"]))
                action = Action(**witness["action"])
                next_states = successors(state, action, mutation)
                observed_states = json.loads(json.dumps([asdict(s) for s in next_states]))
                self.assertEqual(observed_states, witness["successors"])
                observed = "P2" if len(next_states) != 1 else transition_property(state, action, next_states[0])
                self.assertEqual(observed, witness["property"])


if __name__ == "__main__":
    unittest.main()
