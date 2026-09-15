import unittest
from compare import compare, parse

BASE = "P\t0\t4972,4472\nT\t0\t7,2\nF\t0\t\t\t66\t7,2\nE\t0\t\t\t66\t\t\t67\t0\t3,1\n"


class ComparatorTests(unittest.TestCase):
    def test_identity_and_zero_count_positive_cost(self):
        self.assertEqual(compare(parse(BASE), parse(BASE)), {"T": 1, "F": 1, "E": 1})

    def test_changed_counter_fails(self):
        with self.assertRaisesRegex(ValueError, "mismatched"):
            compare(parse(BASE), parse(BASE.replace("7,2", "8,2")))

    def test_lost_zero_count_edge_fails(self):
        with self.assertRaisesRegex(ValueError, "mismatched"):
            compare(parse(BASE), parse(BASE[:BASE.index("E\t")]))

    def test_reordered_events(self):
        other = BASE.replace("4972,4472", "4472,4972").replace("7,2", "2,7").replace("3,1", "1,3")
        self.assertEqual(compare(parse(BASE), parse(other))["E"], 1)

    def test_union_layout(self):
        a = "P\t0\t4972\nT\t0\t7\nP\t1\t4472,4972\nT\t1\t2,3\n"
        b = "P\t0\t4472,4972\nT\t0\t0,7\nP\t1\t4972,4472\nT\t1\t3,2\n"
        self.assertEqual(compare(parse(a), parse(b)), {"T": 2})
        with self.assertRaisesRegex(ValueError, "unmeasured"):
            compare(parse(a), parse(b.replace("0,7", "1,7")))

    def test_zero_only_materialization(self):
        other = BASE + "F\t0\t\t\t68\t0,0\nL\t0\t\t\t68\t\t1\t0,0\nE\t0\t\t\t68\t\t\t69\t0\t0,0\n"
        self.assertEqual(compare(parse(BASE), parse(other))["F"], 1)

    def test_inclusive_diagnostic_not_compared(self):
        self.assertEqual(compare(parse(BASE), parse(BASE + "I\t0\t\t\t66\t100,200\n"))["F"], 1)

    def test_duplicate_rejected_even_if_zero(self):
        with self.assertRaisesRegex(ValueError, "duplicate"):
            parse(BASE + "F\t0\t\t\t66\t0,0\n")

    def test_strict_wire_validation(self):
        bad = ["", BASE + "\n", BASE.replace("7,2", "-7,2"), BASE.replace("66", "xy"),
               BASE.replace("7,2", "7"), BASE.replace("7,2", "7.0,2"),
               BASE.replace("7,2", "7, 2"), BASE.replace("66", "f"),
               BASE.replace("66", "ff"), BASE + "T\t0\t0,0\n",
               BASE.replace("4972,4472", "4972,4972"), BASE.replace("T\t0\t7,2\n", ""),
               BASE.replace("\t0\t", "\t2\t")]
        for item in bad:
            with self.subTest(item=item), self.assertRaises(ValueError):
                parse(item)

    def test_integer_precision(self):
        huge = BASE.replace("7,2", "340282366920938463463374607431768211455,2")
        self.assertEqual(compare(parse(huge), parse(huge))["T"], 1)

    def test_missing_event_rejected(self):
        with self.assertRaisesRegex(ValueError, "event sets"):
            compare(parse(BASE), parse(BASE.replace("4472", "4477")))


if __name__ == "__main__":
    unittest.main()
