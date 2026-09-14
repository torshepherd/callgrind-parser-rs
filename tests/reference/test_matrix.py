import csv
from pathlib import Path
import tempfile
import unittest
from check_matrix import EXPECTED, validate


class MatrixTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.rows = []
        for case, pages in sorted(EXPECTED):
            stem = f"sqlite-cache-{pages}__{case}"
            (self.root / (stem + ".callgrind")).write_text("fixture")
            (self.root / (stem + ".annotated.txt")).write_text("1 PROGRAM TOTALS\n")
            self.rows.append([case, pages, "Ir", "1", "7"])
        self.manifest()

    def manifest(self):
        with (self.root / "SUMMARY.tsv").open("w", newline="") as stream:
            writer = csv.writer(stream, delimiter="\t")
            writer.writerow(["case", "sqlite_cache_pages", "events", "summary", "profile_bytes"])
            writer.writerows(self.rows)

    def test_exact_matrix(self):
        self.assertEqual(len(validate(self.root)), 12)

    def test_duplicate_manifest_pair(self):
        self.rows[-1] = self.rows[0]
        self.manifest()
        with self.assertRaisesRegex(ValueError, "12 unique"):
            validate(self.root)

    def test_extra_profile(self):
        (self.root / "unexpected.callgrind").write_text("x")
        with self.assertRaisesRegex(ValueError, "case set"):
            validate(self.root)

    def test_missing_annotation(self):
        next(self.root.glob("*.annotated.txt")).unlink()
        with self.assertRaisesRegex(ValueError, "case set"):
            validate(self.root)

    def test_corrupt_annotation(self):
        next(self.root.glob("*.annotated.txt")).write_text("wrong")
        with self.assertRaisesRegex(ValueError, "PROGRAM TOTALS"):
            validate(self.root)

    def test_incorrect_size(self):
        self.rows[0][-1] = "0"
        self.manifest()
        with self.assertRaisesRegex(ValueError, "byte count"):
            validate(self.root)


if __name__ == "__main__":
    unittest.main()
