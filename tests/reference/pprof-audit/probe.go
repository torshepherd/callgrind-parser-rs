// Source-audit fixture generator and CLI probes, not a Callgrind converter.
// Run inside the pinned google/pprof module; see README.md in this directory.
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"time"

	"github.com/google/pprof/profile"
)

func must(err error) {
	if err != nil {
		panic(err)
	}
}

func require(ok bool, message string) {
	if !ok {
		panic(message)
	}
}

func makeProfile(names ...string) *profile.Profile {
	p := &profile.Profile{SampleType: []*profile.ValueType{{Type: "work", Unit: "count"}}}
	m := &profile.Mapping{ID: 1, Start: 0x1000, Limit: 0x10000, File: "/audit/app", HasFunctions: true, HasFilenames: true, HasLineNumbers: true}
	p.Mapping = []*profile.Mapping{m}
	for i, name := range names {
		f := &profile.Function{ID: uint64(i + 1), Name: name, SystemName: name, Filename: "audit.c", StartLine: int64(10 * (i + 1))}
		l := &profile.Location{ID: uint64(i + 1), Mapping: m, Address: uint64(0x1000 + 0x100*i), Line: []profile.Line{{Function: f, Line: f.StartLine}}}
		p.Function = append(p.Function, f)
		p.Location = append(p.Location, l)
	}
	return p
}

// Stack indices are leaf first, as required by profile.proto.
func sample(p *profile.Profile, value int64, stack ...int) {
	s := &profile.Sample{Value: []int64{value}}
	for _, i := range stack {
		s.Location = append(s.Location, p.Location[i])
	}
	p.Sample = append(p.Sample, s)
}

func main() {
	pprof := flag.String("pprof", "", "absolute path to the pinned pprof executable")
	out := flag.String("out", "", "new, empty output directory")
	roundtrip := flag.String("roundtrip", "", "optional Rust pprof roundtrip executable")
	converter := flag.String("converter", "", "optional Rust pprof2callgrind executable")
	flag.Parse()
	require(filepath.IsAbs(*pprof) && filepath.IsAbs(*out), "--pprof and --out must be absolute")
	must(os.MkdirAll(*out, 0755))
	entries, err := os.ReadDir(*out)
	must(err)
	require(len(entries) == 0, "output directory must be empty")

	profiles := make(map[string]*profile.Profile)
	basic := makeProfile("main", "foo")
	sample(basic, 10, 0)
	sample(basic, 90, 1, 0)
	profiles["basic"] = basic
	flat := makeProfile("main", "foo")
	sample(flat, 10, 0)
	sample(flat, 90, 1)
	profiles["flat"] = flat
	for _, swap := range []bool{false, true} {
		p := makeProfile("main", "A", "B", "M", "X", "Y")
		a, b, name := 1, 2, "ambiguous-a"
		if swap {
			a, b, name = b, a, "ambiguous-b"
		}
		sample(p, 4, 4, 3, a, 0)
		sample(p, 4, 5, 3, b, 0)
		profiles[name] = p
	}
	recursive := makeProfile("main", "recur")
	sample(recursive, 2, 0)
	sample(recursive, 7, 1, 1, 0)
	profiles["recursive"] = recursive
	mutual := makeProfile("main", "A", "B")
	sample(mutual, 7, 1, 2, 1, 2, 1, 0)
	profiles["mutual"] = mutual
	objects := makeProfile("main", "foo")
	m := &profile.Mapping{ID: 2, Start: 0x10000, Limit: 0x20000, File: "/audit/lib.so", HasFunctions: true, HasFilenames: true, HasLineNumbers: true}
	objects.Mapping = append(objects.Mapping, m)
	objects.Location[1].Mapping = m
	objects.Location[1].Address = 0x11000
	sample(objects, 90, 1, 0)
	profiles["objects"] = objects
	inlined := makeProfile("main", "outer", "inner")
	inlined.Location[1].Line = []profile.Line{{Function: inlined.Function[2], Line: 30}, {Function: inlined.Function[1], Line: 20}}
	sample(inlined, 9, 1, 0)
	profiles["inline"] = inlined
	units := makeProfile("a", "b")
	units.SampleType = []*profile.ValueType{{Type: "cpu", Unit: "nanoseconds"}}
	sample(units, 1500, 0)
	sample(units, 2500, 1)
	profiles["units"] = units
	large := makeProfile("large")
	sample(large, 1<<53+1, 0)
	profiles["large"] = large
	negative := makeProfile("negative")
	sample(negative, -5, 0)
	profiles["negative"] = negative
	multi := makeProfile("main", "foo")
	multi.SampleType = []*profile.ValueType{{Type: "samples", Unit: "count"}, {Type: "cpu", Unit: "nanoseconds"}}
	sample(multi, 2, 1, 0)
	multi.Sample[0].Value = []int64{2, 100}
	multi.Sample[0].Label = map[string][]string{"thread": {"worker"}}
	multi.PeriodType = &profile.ValueType{Type: "cpu", Unit: "nanoseconds"}
	multi.Period = 10
	multi.Comments = []string{"audit metadata"}
	profiles["multi"] = multi

	for name, p := range profiles {
		must(p.CheckValid())
		var buf bytes.Buffer
		must(p.Write(&buf))
		// Exercise the independent profile reader as well as its writer.
		decoded, err := profile.ParseData(buf.Bytes())
		must(err)
		must(decoded.CheckValid())
		must(os.WriteFile(filepath.Join(*out, name+".pb.gz"), buf.Bytes(), 0644))
		must(os.WriteFile(filepath.Join(*out, name+".profile.txt"), []byte(decoded.String()), 0644))
		if *roundtrip != "" {
			cmd := exec.Command(*roundtrip)
			cmd.Stdin = bytes.NewReader(buf.Bytes())
			raw, err := cmd.Output()
			must(err)
			reread, err := profile.ParseData(raw)
			must(err)
			must(reread.CheckValid())
			require(reflect.DeepEqual(decoded, reread), "Rust protobuf roundtrip changed "+name)
			must(os.WriteFile(filepath.Join(*out, name+".rust.pb.gz"), raw, 0644))
		}
		if *converter != "" {
			for _, mode := range []string{"graph", "tree"} {
				ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
				cmd := exec.CommandContext(ctx, *converter, "--mode", mode, filepath.Join(*out, name+".pb.gz"))
				var stdout, stderr bytes.Buffer
				cmd.Stdout, cmd.Stderr = &stdout, &stderr
				err := cmd.Run()
				cancel()
				must(os.WriteFile(filepath.Join(*out, name+".rust-"+mode+".stderr"), stderr.Bytes(), 0644))
				if name == "negative" {
					require(err != nil && stdout.Len() == 0 && strings.Contains(stderr.String(), "negative sample"), "negative conversion must fail without output")
				} else {
					must(err)
					must(os.WriteFile(filepath.Join(*out, name+".rust-"+mode+".callgrind"), stdout.Bytes(), 0644))
				}
			}
		}
	}

	run := func(name, input string, options ...string) []byte {
		args := append([]string{"-symbolize=none"}, options...)
		args = append(args, filepath.Join(*out, input+".pb.gz"))
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()
		cmd := exec.CommandContext(ctx, *pprof, args...)
		cmd.Dir = *out
		var stdout, stderr bytes.Buffer
		cmd.Stdout, cmd.Stderr = &stdout, &stderr
		err := cmd.Run()
		must(os.WriteFile(filepath.Join(*out, name+".stdout"), stdout.Bytes(), 0644))
		must(os.WriteFile(filepath.Join(*out, name+".stderr"), stderr.Bytes(), 0644))
		command, jsonErr := json.MarshalIndent(struct {
			Argv []string `json:"argv"`
			Cwd  string   `json:"cwd"`
		}{cmd.Args, cmd.Dir}, "", "  ")
		must(jsonErr)
		must(os.WriteFile(filepath.Join(*out, name+".command.json"), command, 0644))
		must(err)
		return stdout.Bytes()
	}

	a := run("ambiguous-a", "ambiguous-a", "-callgrind", "-unit=count")
	b := run("ambiguous-b", "ambiguous-b", "-callgrind", "-unit=count")
	require(bytes.Equal(a, b), "different stack populations should collapse to identical ordinary Callgrind output")
	at := run("ambiguous-a-traces", "ambiguous-a", "-traces", "-unit=count")
	bt := run("ambiguous-b-traces", "ambiguous-b", "-traces", "-unit=count")
	require(!bytes.Equal(at, bt), "the original pprof stack populations must differ")

	for _, name := range []string{"basic", "flat", "recursive", "mutual", "objects", "inline", "large", "negative"} {
		run(name, name, "-callgrind", "-unit=count")
		run(name+"-top", name, "-top", "-unit=count", "-nodecount=0", "-nodefraction=0")
	}
	for _, name := range []string{"ambiguous-a", "ambiguous-b", "recursive"} {
		run(name+"-tree", name, "-callgrind", "-call_tree", "-unit=count")
	}
	largeRaw := run("large-raw", "large", "-raw")
	require(bytes.Contains(largeRaw, []byte("9007199254740993")), "protobuf and raw reader must preserve 2^53+1")
	u := run("units-auto", "units", "-callgrind")
	require(bytes.Contains(u, []byte("events: cpu(us)")), "expected automatic microseconds")
	run("units-ns", "units", "-callgrind", "-unit=ns")
	base := run("multi-ns", "multi", "-callgrind", "-unit=ns")
	div := run("multi-divide", "multi", "-callgrind", "-unit=ns", "-divide_by=2")
	require(bytes.Equal(base, div), "pinned Callgrind formatter ignores divide_by at explicit units")
	run("multi-count", "multi", "-callgrind", "-sample_index=samples", "-unit=count")
	run("multi-mean", "multi", "-callgrind", "-mean", "-unit=ns")
	run("hidden", "basic", "-callgrind", "-hide=foo", "-unit=count")
	run("inline-noinlines", "inline", "-callgrind", "-noinlines", "-unit=count")
	// Pin selected wire-level observations without implementing another parser.
	for name, snippets := range map[string][]string{
		"basic":          {"-256 10 10\n", "calls=0 * 20\n* * 90\n"},
		"recursive":      {"0x1100 20 7\n", "* * 7\n"},
		"objects":        {"/audit/lib.so", "/audit/app", "calls=0 * 20\n* * 90\n"},
		"recursive-tree": {"fn=(1) recur\n", "recur [1/2]", "recur [2/2]"},
		"units-auto":     {"0x1100 20 2\n", "-256 10 1\n"},
		"units-ns":       {"0x1100 20 2500\n", "-256 10 1500\n"},
		"negative":       {"0x1000 10 -5\n"},
		"multi-mean":     {"events: mean_cpu(ns)", "0x1100 20 50\n", "* * 100\n"},
		"multi-count":    {"events: samples(count)", "0x1100 20 2\n"},
		"hidden":         {"0x1000 10 100\n"},
	} {
		data, err := os.ReadFile(filepath.Join(*out, name+".stdout"))
		must(err)
		for _, snippet := range snippets {
			require(bytes.Contains(data, []byte(snippet)), name+": missing expected wire fragment "+snippet)
		}
	}
	for name, calls := range map[string]int{"flat": 0, "basic": 1, "recursive": 1, "mutual": 3, "objects": 1, "inline": 2, "inline-noinlines": 1} {
		data, err := os.ReadFile(filepath.Join(*out, name+".stdout"))
		must(err)
		require(bytes.Count(data, []byte("calls=")) == calls, name+": unexpected edge count")
		if name == "objects" {
			require(!bytes.Contains(data, []byte("cob=")), "unexpected callee object field")
		}
	}

	// This is an observed loss in the formatter, not the protobuf representation.
	largeOutput, err := os.ReadFile(filepath.Join(*out, "large.stdout"))
	must(err)
	require(strings.Contains(string(largeOutput), "9007199254740992"), "expected float64 rounding of 2^53+1")
	fmt.Println("pprof audit probes passed: distinct stacks collapse; unit, integer and option losses reproduced")
}
