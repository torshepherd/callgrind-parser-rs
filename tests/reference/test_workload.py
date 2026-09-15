import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from check_annotate import check
from check_matrix import load_plan, validate
from run_workload import run


class WorkloadTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.out = self.root / "results"
        self.input = self.root / "input with spaces.txt"
        self.input.write_text("source text\n")
        self.plan = {
            "schema": 1, "name": "compiler-example", "configurations": ["ir-only", "balanced-cache"],
            "variants": [{"name": "compile", "argv": [sys.executable, "-c",
                "from pathlib import Path; import sys; Path('output.o').write_text(sys.argv[1]); print(sys.stdin.read(), end='')",
                "spaces; $(must-not-execute)"], "stdin": str(self.input)}],
        }
        self.plan_path = self.root / "plan.json"
        self.write_plan()
        self.profiler = self.executable("profiler", '''
import subprocess, sys
from pathlib import Path
args = sys.argv[1:]
profile = next(a.split('=', 1)[1] for a in args if a.startswith('--callgrind-out-file='))
target = next(i for i, a in enumerate(args) if not a.startswith('--'))
status = subprocess.run(args[target:]).returncode
Path(profile).write_text('version: 1\\nevents: Ir\\nsummary: 1\\nfl=f\\nfn=f\\n1 1\\n')
sys.exit(status)
''')
        self.reference = self.executable("reference", "print('Events shown: Ir\\n1 PROGRAM TOTALS\\nIr file:function\\n1 f:f')")

    def executable(self, name, code):
        path = self.root / name
        path.write_text(f"#!{sys.executable}\n" + code)
        path.chmod(0o755)
        return str(path)

    def write_plan(self):
        self.plan_path.write_text(json.dumps(self.plan))

    def test_non_sqlite_command_arguments_stdin_and_case_isolation(self):
        run(self.plan_path, self.out, self.profiler, self.reference)
        paths = validate(self.out, load_plan(self.plan_path))
        self.assertEqual(len(paths), 2)
        for path in paths:
            self.assertEqual((self.out / "work" / path.stem / "output.o").read_text(), "spaces; $(must-not-execute)")
            self.assertEqual((self.out / (path.stem + ".stdout.txt")).read_text(), "source text\n")
        self.assertEqual((self.out / "inputs/compile.stdin").read_text(), self.input.read_text())
        with self.assertRaisesRegex(ValueError, "empty"):
            run(self.plan_path, self.out, self.profiler, self.reference)

    def test_target_failure_retains_diagnostics_and_cannot_validate(self):
        self.plan["variants"][0]["argv"] = [sys.executable, "-c", "import sys; print('failed target', file=sys.stderr); sys.exit(7)"]
        self.write_plan()
        with self.assertRaises(subprocess.CalledProcessError):
            run(self.plan_path, self.out, self.profiler, self.reference)
        self.assertIn("failed target", next(self.out.glob("*.stderr.txt")).read_text())
        self.assertTrue(list(self.out.glob("*.command.json")))
        with self.assertRaises(ValueError):
            validate(self.out)

    def test_expected_plan_rejects_changed_or_missing_cases(self):
        run(self.plan_path, self.out, self.profiler, self.reference)
        expected = load_plan(self.plan_path)
        recorded = self.out / "PLAN.json"
        altered = copy.deepcopy(expected)
        altered["configurations"].pop()
        recorded.write_text(json.dumps(altered))
        with self.assertRaisesRegex(ValueError, "differs"):
            validate(self.out, expected)
        recorded.write_text(json.dumps(expected))
        next(self.out.glob("*.callgrind")).unlink()
        with self.assertRaisesRegex(ValueError, "case set"):
            validate(self.out, expected)

    def test_malformed_plans_are_rejected(self):
        changes = [{"schema": 2}, {"configurations": []}, {"configurations": ["unknown"]},
                   {"configurations": ["ir-only", "ir-only"]}, {"variants": []}, {"timeout_seconds": 0}]
        for change in changes:
            with self.subTest(change=change):
                self.plan_path.write_text(json.dumps(dict(self.plan, **change)))
                with self.assertRaises(ValueError):
                    load_plan(self.plan_path)
        for name in ("../escape", "ambiguous__name"):
            self.plan["variants"][0]["name"] = name
            self.write_plan()
            with self.assertRaisesRegex(ValueError, "safe and unique"):
                load_plan(self.plan_path)

    def test_duplicate_variants_are_rejected(self):
        self.plan["variants"] *= 2
        self.write_plan()
        with self.assertRaisesRegex(ValueError, "safe and unique"):
            load_plan(self.plan_path)

    def test_both_annotator_outputs_survive_nonzero_exit(self):
        broken = self.executable("broken", "import sys; print('partial report'); print('failure', file=sys.stderr); sys.exit(9)")
        artifacts = self.root / "reports"
        with self.assertRaises(subprocess.CalledProcessError):
            check(Path(broken), Path(self.reference), self.input, [], artifacts)
        self.assertEqual((artifacts / "rust.stdout.txt").read_text(), "partial report\n")
        self.assertEqual((artifacts / "rust.stderr.txt").read_text(), "failure\n")
        self.assertIn("PROGRAM TOTALS", (artifacts / "reference.stdout.txt").read_text())

    def test_semantic_mismatch_retains_both_reports(self):
        wrong = self.executable("wrong", "print('Events shown: Ir\\n2 PROGRAM TOTALS\\nIr file:function\\n2 f:f')")
        artifacts = self.root / "reports"
        with self.assertRaisesRegex(ValueError, "totals"):
            check(Path(wrong), Path(self.reference), self.input, [], artifacts)
        self.assertIn("2 PROGRAM TOTALS", (artifacts / "rust.stdout.txt").read_text())
        self.assertIn("1 PROGRAM TOTALS", (artifacts / "reference.stdout.txt").read_text())


if __name__ == "__main__":
    unittest.main()
