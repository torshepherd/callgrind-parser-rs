import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "dispatch", Path(__file__).resolve().parents[2] / "scripts/dispatch-binaries.py"
)
dispatch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dispatch)


class DispatchTests(unittest.TestCase):
    def test_three_clis_produce_one_conventional_tag(self):
        releases = [{"package_name": name, "version": "0.1.0"} for name in dispatch.BINARIES]
        self.assertEqual(dispatch.release_tag(releases, "0.1.0"), "v0.1.0")

    def test_library_only_release_does_not_dispatch(self):
        self.assertIsNone(dispatch.release_tag(
            [{"package_name": "callgrind-parser", "version": "0.1.0"}], "0.1.0"))

    def test_mixed_versions_are_rejected(self):
        with self.assertRaises(ValueError):
            dispatch.release_tag([{"package_name": "pprof2callgrind", "version": "0.2.0"}],
                                 "0.1.0")

    def test_existing_tag_at_another_commit_is_preserved(self):
        result = subprocess.CompletedProcess([], 0, '{"object":{"type":"commit","sha":"old"}}', '')
        with patch.object(dispatch.subprocess, "run", return_value=result) as run:
            with self.assertRaisesRegex(ValueError, "Refusing to move"):
                dispatch.ensure_tag("owner/repo", "v0.1.0", "new")
            self.assertEqual(run.call_count, 1)

    def test_api_permission_failure_does_not_attempt_tag_creation(self):
        result = subprocess.CompletedProcess([], 1, '', 'HTTP 403')
        with patch.object(dispatch.subprocess, "run", return_value=result) as run:
            with self.assertRaises(RuntimeError):
                dispatch.ensure_tag("owner/repo", "v0.1.0", "new")
            self.assertEqual(run.call_count, 1)


if __name__ == "__main__":
    unittest.main()
