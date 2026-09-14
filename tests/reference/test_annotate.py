import unittest
from check_annotate import table

TEXT = "Events shown: Ir Dr\n1,000 (100.0%) 10 (100.0%) PROGRAM TOTALS\nIr Dr file:function\n900 (90.00%) . a:f [app]\n100 (10.00%) 10 (100.0%) a:g [app]\n"


class AnnotatorComparisonTests(unittest.TestCase):
    def test_exact_counter_extraction(self):
        events, totals, rows, _ = table(TEXT)
        self.assertEqual(events, ["Ir", "Dr"])
        self.assertEqual(totals, (1000, 10))
        self.assertEqual(rows, [("a:f", (900, 0)), ("a:g", (100, 10))])

    def test_changed_counter_and_order_are_detected(self):
        self.assertNotEqual(table(TEXT), table(TEXT.replace("900", "901")))
        lines = TEXT.splitlines()
        self.assertNotEqual(table(TEXT), table("\n".join(lines[:3] + lines[3:][::-1])))

    def test_invalid_report_rejected(self):
        for report in ["", TEXT.replace("PROGRAM TOTALS", "bogus"), TEXT + "1 1 a:f [app]\n"]:
            with self.assertRaises(ValueError):
                table(report)

    def test_tree_empty_object_format_normalization(self):
        self.assertEqual(table(TEXT + "1 2 > a:g (1x) []\n"), table(TEXT + "1 2 >   a:g (1x)\n"))


if __name__ == "__main__":
    unittest.main()
