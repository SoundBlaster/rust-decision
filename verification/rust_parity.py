#!/usr/bin/env python3
"""Compare every reachable model transition against the compiled Rust engine."""
import argparse
from datetime import datetime, timezone
from collections import deque
from dataclasses import asdict
import hashlib
import json
from pathlib import Path
import subprocess
from choice_model import State, actions, representatives, successors


def check(binary):
    counts = {}
    with subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                          text=True, bufsize=1) as process:
        try:
            for mode in ("primary", "conservative_extension"):
                contexts, _ = representatives(mode)
                states = pairs = 0
                for context in contexts:
                    initial = State(context)
                    queue, seen = deque([initial]), {initial}
                    while queue:
                        state = queue.popleft()
                        offered = list(actions(state))
                        process.stdin.write(json.dumps({"state": asdict(state),
                            "actions": [asdict(a) for a in offered]}) + "\n")
                        process.stdin.flush()
                        line = process.stdout.readline()
                        if not line:
                            raise RuntimeError("Rust parity process stopped without a response")
                        observed = json.loads(line)
                        expected = []
                        for action in offered:
                            after, = successors(state, action)
                            expected.append(asdict(after))
                            if after not in seen:
                                seen.add(after)
                                queue.append(after)
                        # Roundtrip normalizes dataclass tuples to JSON arrays.
                        if observed != json.loads(json.dumps(expected)):
                            raise AssertionError(json.dumps({"mode": mode, "state": asdict(state),
                                "actions": [asdict(a) for a in offered],
                                "expected": expected, "observed": observed}))
                        states += 1
                        pairs += len(offered)
                counts[mode] = {"representatives": len(contexts), "states": states,
                                "state_action_pairs": pairs}
                print(f"{mode}: {states} states, {pairs} transitions match Rust", flush=True)
            process.stdin.close()
            if process.wait(timeout=30) != 0:
                raise RuntimeError("Rust parity process failed")
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
    return counts


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    files = [*sorted((root / "src").rglob("*.rs")), root / "Cargo.lock", root / "Cargo.toml",
             *sorted((root / "tests").glob("*.rs")),
             root / "examples/parity.rs", Path(__file__), root / "verification/choice_model.py"]
    report = {
        "schema_version": 1, "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "checkout_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "source_sha256": {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
        "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        "limits": "Representative finite transitions only; numerical rules use separate Rust tests. No live backend evidence.",
    }
    try:
        report.update(passed=True, modes=check(args.binary.resolve()))
    except Exception as error:
        report.update(passed=False, error=str(error))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    raise SystemExit(0 if report["passed"] else 1)
