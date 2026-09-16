package main

import (
	"github.com/google/pprof/profile"
	"testing"
)

func TestReverseOracle(t *testing.T) {
	f := &profile.Function{ID: 1, Name: "f [callgrind:f1]", Filename: "a.c"}
	l := &profile.Location{ID: 1, Line: []profile.Line{{Function: f, Line: 2}}}
	p := &profile.Profile{
		SampleType: []*profile.ValueType{{Type: "Ir", Unit: "count"}}, DefaultSampleType: "Ir",
		Function: []*profile.Function{f}, Location: []*profile.Location{l},
		Sample: []*profile.Sample{{Location: []*profile.Location{l}, Value: []int64{7}, Label: map[string][]string{
			"callgrind.object": {"v:app"}, "callgrind.function": {"v:f"}, "callgrind.defining_file": {"v:a.c"}, "callgrind.source_file": {"v:a.c"}, "callgrind.line": {"2"},
		}}},
	}
	want := expected{events: []string{"Ir"}, totals: []uint64{7}, rows: map[string][]uint64{
		"F\t617070\t612e63\t66": {7}, "L\t617070\t612e63\t66\t612e63\t2": {7},
	}}
	validate(p, want)
	mutations := map[string]func(*profile.Profile){
		"cost":     func(p *profile.Profile) { p.Sample[0].Value[0]++ },
		"identity": func(p *profile.Profile) { p.Sample[0].Label["callgrind.object"] = []string{"v:other"} },
		"source":   func(p *profile.Profile) { p.Function[0].Filename = "wrong.c" },
		"stack":    func(p *profile.Profile) { p.Sample[0].Location = append(p.Sample[0].Location, p.Location[0]) },
		"address":  func(p *profile.Profile) { p.Location[0].Address = 4096 },
		"event":    func(p *profile.Profile) { p.SampleType[0].Type = "Dr" },
		"line":     func(p *profile.Profile) { p.Location[0].Line[0].Line = 3 },
	}
	for name, mutate := range mutations {
		t.Run(name, func(t *testing.T) {
			q := p.Copy()
			mutate(q)
			defer func() {
				if recover() == nil {
					t.Fatal("accepted corrupted output")
				}
			}()
			validate(q, want)
		})
	}
}
