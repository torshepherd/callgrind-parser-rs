import contextlib
import importlib.util
import io
from pathlib import Path
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "release_ci", Path(__file__).resolve().parents[2] / "scripts/check-release-ci.py"
)
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class ReleaseGateTests(unittest.TestCase):
    def check_gate(self, *, conclusion="success", branch="master", jobs=None, head="abc"):
        run = {"id": 42, "head_branch": branch, "status": "completed",
               "conclusion": conclusion, "html_url": "https://example.test/ci/42"}
        if jobs is None:
            jobs = [{"name": name, "status": "completed", "conclusion": "success"}
                    for name in ["Rust and Python tests", "Callgrind integration (sqlite)",
                                 "Pprof and KCachegrind integration"]]
        responses = [{"object": {"sha": head}}, {"workflow_runs": [run]}, {"jobs": jobs}]
        with patch("sys.argv", ["check-release-ci.py", "HEAD", "--require-head"]), \
             patch.object(gate.subprocess, "check_output", return_value="abc\n"), \
             patch.object(gate, "api", side_effect=responses), \
             contextlib.redirect_stdout(io.StringIO()):
            gate.main()

    def test_complete_native_and_rust_run_passes(self):
        self.check_gate()

    def test_failed_run_cannot_publish(self):
        with self.assertRaisesRegex(SystemExit, "requires successful CI"):
            self.check_gate(conclusion="failure")

    def test_another_branch_does_not_authorize_publication(self):
        with self.assertRaisesRegex(SystemExit, "No push CI run on master"):
            self.check_gate(branch="release-preview")

    def test_green_rust_alone_is_insufficient(self):
        with self.assertRaisesRegex(SystemExit, "Missing successful jobs"):
            self.check_gate(jobs=[{"name": "Rust and Python tests", "status": "completed",
                                   "conclusion": "success"}])

    def test_stale_commit_cannot_publish(self):
        with self.assertRaisesRegex(SystemExit, "no longer the master head"):
            self.check_gate(head="def")


if __name__ == "__main__":
    unittest.main()
