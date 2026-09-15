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

    def test_distinct_objects_project_to_exact_legacy_sums(self):
        split = TEXT.replace("100 (10.00%) 10 (100.0%) a:g [app]", "70 7 a:g [libA]\n30 3 a:g [libB]")
        self.assertEqual(table(split), table(TEXT))
        self.assertNotEqual(table(split.replace("30 3", "31 3")), table(TEXT))
        with self.assertRaisesRegex(ValueError, "sort order"):
            table(split.replace("70 7 a:g [libA]\n30 3 a:g [libB]", "30 3 a:g [libB]\n70 7 a:g [libA]"))

    def test_projected_tree_sums_keep_both_endpoints(self):
        head = "Events shown: Ir\n30 PROGRAM TOTALS\nIr file:function\n"
        split = head + "20 * a:f [A]\n10 > a:g (2x)\n10 * a:f [B]\n5 > a:g (1x)\n"
        merged = head + "30 * a:f [B]\n15 > a:g (3x)\n"
        self.assertEqual(table(split), table(merged))
        self.assertNotEqual(table(split.replace("(1x)", "(2x)")), table(merged))
        parents = head + "20 * a:f\n10 > a:h (1x)\n10 * a:g\n5 > a:h (1x)\n"
        swapped = parents.replace("10 > a:h", "5 > a:h").replace("10 * a:g\n5 > a:h", "10 * a:g\n10 > a:h")
        self.assertNotEqual(table(parents), table(swapped))

    def test_caller_rows_attach_to_the_following_function(self):
        head = "Events shown: Ir\n30 PROGRAM TOTALS\nIr file:function\n"
        split = head + "10 < a:main (2x)\n20 * a:f [A]\n5 < a:main (1x)\n10 * a:f [B]\n"
        merged = head + "15 < a:main (3x)\n30 * a:f [B]\n"
        self.assertEqual(table(split), table(merged))


if __name__ == "__main__":
    unittest.main()
