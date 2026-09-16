import unittest

from compare import Export
from check_pprof import check


class PprofChecks(unittest.TestCase):
    def basic(self, tree=False):
        a = ("/audit/app", "audit.c", "main [pprof:f1:m1]" + (" [ctx:0]" if tree else ""))
        b = ("/audit/app", "audit.c", "foo [pprof:f2:m1]" + (" [ctx:1]" if tree else ""))
        return Export({0: ("E0",)}, {
            ("T", 0): (None, (100,)),
            ("F", 0, *a): (None, (10,)), ("F", 0, *b): (None, (90,)),
            ("E", 0, *a, *b): (0, (90,)),
        })

    def test_exact_graph_and_tree(self):
        check(self.basic(), "basic", "graph")
        check(self.basic(True), "basic", "tree")

    def test_changed_cost_and_count(self):
        for value in [(0, (89,)), (1, (90,))]:
            p = self.basic()
            p.rows[next(k for k in p.rows if k[0] == "E")] = value
            with self.assertRaises(ValueError):
                check(p, "basic", "graph")

    def test_flat_redistribution_cannot_hide_behind_totals(self):
        p = self.basic()
        for key in p.rows:
            if key[0] == "F":
                p.rows[key] = (None, (50,))
        with self.assertRaises(ValueError):
            check(p, "basic", "graph")

    def test_missing_tree_context(self):
        p = self.basic(True)
        del p.rows[next(k for k in p.rows if k[0] == "F")]
        with self.assertRaises(ValueError):
            check(p, "basic", "tree")

    def test_incorrect_event_and_total(self):
        p = self.basic()
        p.events[0] = ("Ir",)
        with self.assertRaises(ValueError):
            check(p, "basic", "graph")
        p = self.basic()
        p.rows[("T", 0)] = (None, (101,))
        with self.assertRaises(ValueError):
            check(p, "basic", "graph")
