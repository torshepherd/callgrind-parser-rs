// Independent pprof readback and exact self-cost conservation against the
// production Callgrind parser's TSV export. Run inside the pinned pprof module.
package main

import (
	"bytes"
	"context"
	"encoding/hex"
	"flag"
	"fmt"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strconv"
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
func execute(binary string, args ...string) ([]byte, []byte) {
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, binary, args...)
	var stdout, stderr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	err := cmd.Run()
	if err != nil {
		panic(fmt.Sprintf("%s %v: %v\n%s", binary, args, err, stderr.String()))
	}
	return stdout.Bytes(), stderr.Bytes()
}
func numbers(raw string) []uint64 {
	out := []uint64{}
	for _, s := range strings.Split(raw, ",") {
		n, e := strconv.ParseUint(s, 10, 64)
		must(e)
		out = append(out, n)
	}
	return out
}
func add(rows map[string][]uint64, key string, values []uint64) {
	if _, ok := rows[key]; !ok {
		rows[key] = make([]uint64, len(values))
	}
	require(len(rows[key]) == len(values), "bad vector width")
	for i, v := range values {
		require(rows[key][i] <= math.MaxUint64-v, "sum overflow")
		rows[key][i] += v
	}
}
func hexName(s string) string {
	if s == "???" {
		s = ""
	}
	return hex.EncodeToString([]byte(s))
}
func original(s *profile.Sample, name string) string {
	values := s.Label["callgrind."+name]
	if len(values) == 0 {
		return ""
	}
	require(len(values) == 1 && strings.HasPrefix(values[0], "v:"), "bad original-string label")
	return strings.TrimPrefix(values[0], "v:")
}
func nonzero(rows map[string][]uint64) map[string][]uint64 {
	out := map[string][]uint64{}
	for k, v := range rows {
		for _, n := range v {
			if n != 0 {
				out[k] = v
				break
			}
		}
	}
	return out
}

type expected struct {
	events []string
	totals []uint64
	rows   map[string][]uint64
}

func parseExport(raw []byte) []expected {
	var out []expected
	for _, line := range strings.Split(strings.TrimSpace(string(raw)), "\n") {
		f := strings.Split(line, "\t")
		require(len(f) >= 3, "bad TSV")
		part, e := strconv.Atoi(f[1])
		must(e)
		if f[0] == "P" {
			require(part == len(out), "noncontiguous part")
			p := expected{rows: map[string][]uint64{}}
			for _, s := range strings.Split(f[2], ",") {
				b, e := hex.DecodeString(s)
				must(e)
				p.events = append(p.events, string(b))
			}
			out = append(out, p)
			continue
		}
		require(part >= 0 && part < len(out), "unknown part")
		switch f[0] {
		case "T":
			out[part].totals = numbers(f[2])
		case "F", "L":
			add(out[part].rows, strings.Join(append([]string{f[0]}, f[2:len(f)-1]...), "\t"), numbers(f[len(f)-1]))
		}
	}
	return out
}
func validate(p *profile.Profile, want expected) {
	must(p.CheckValid())
	require(len(p.SampleType) == len(want.events), "event width changed")
	require(len(p.Mapping) == 0 && p.TimeNanos == 0 && p.DurationNanos == 0 && p.Period == 0, "invented mapping/timing")
	for i, t := range p.SampleType {
		require(t.Type == want.events[i] && t.Unit != "", "event order/name/unit changed")
	}
	require(p.DefaultSampleType == want.events[0], "wrong default column")
	totals := make([]uint64, len(want.events))
	actual := map[string][]uint64{}
	for _, s := range p.Sample {
		require(len(s.Location) == 1, "invented stack")
		l := s.Location[0]
		require(l.Mapping == nil && l.Address == 0 && len(l.Line) == 1, "invented runtime location")
		line := l.Line[0]
		require(line.Function != nil && line.Column == 0 && !l.IsFolded, "bad source location")
		f := line.Function
		require(f.Filename == original(s, "source_file") && f.StartLine == 0 && f.SystemName == "", "invented function metadata")
		name := original(s, "function")
		if name == "" {
			name = "<unknown>"
		}
		require(f.Name == fmt.Sprintf("%s [callgrind:f%d]", name, f.ID), "function identity changed")
		key := strings.Join([]string{hexName(original(s, "object")), hexName(original(s, "defining_file")), hexName(original(s, "function"))}, "\t")
		values := make([]uint64, len(s.Value))
		for i, v := range s.Value {
			require(v >= 0, "negative sample")
			values[i] = uint64(v)
			require(totals[i] <= math.MaxInt64-uint64(v), "signed total overflow")
			totals[i] += uint64(v)
		}
		add(actual, "F\t"+key, values)
		if line.Line > 0 {
			add(actual, fmt.Sprintf("L\t%s\t%s\t%d", key, hexName(original(s, "source_file")), line.Line), values)
		}
		for _, key := range []string{"instruction", "basic_block", "line"} {
			if v, ok := s.Label["callgrind."+key]; ok {
				require(len(v) == 1, "duplicate position label")
				n, e := strconv.ParseUint(v[0], 10, 64)
				must(e)
				if key == "line" {
					require(n == uint64(line.Line), "line label mismatch")
				}
			}
		}
	}
	require(reflect.DeepEqual(totals, want.totals), fmt.Sprintf("totals differ: %v != %v", totals, want.totals))
	require(reflect.DeepEqual(nonzero(actual), nonzero(want.rows)), "exclusive function/source costs or identities changed")
}
func main() {
	converter := flag.String("converter", "", "Rust callgrind2pprof")
	exporter := flag.String("exporter", "", "production Callgrind TSV exporter")
	pprof := flag.String("pprof", "", "pinned upstream CLI")
	out := flag.String("out", "", "new empty output directory")
	sqlite := flag.String("sqlite", "", "directory containing exactly 12 workload .callgrind files")
	fixtures := flag.String("fixtures", "", "reference fixtures directory")
	generated := flag.String("generated", "", "forward-converter output directory")
	flag.Parse()
	must(os.MkdirAll(*out, 0755))
	entries, e := os.ReadDir(*out)
	must(e)
	require(len(entries) == 0, "output must be empty")
	var inputs []string
	for _, name := range []string{"annotate-basic", "basename-collision", "combined-aliases", "cycle", "event-remapping", "inline-source", "object-collision"} {
		inputs = append(inputs, filepath.Join(*fixtures, name+".callgrind"))
	}
	if *generated != "" {
		paths, e := filepath.Glob(filepath.Join(*generated, "*.rust-*.callgrind"))
		must(e)
		require(len(paths) == 22, "expected 22 forward outputs")
		inputs = append(inputs, paths...)
	}
	if *sqlite != "" {
		paths, e := filepath.Glob(filepath.Join(*sqlite, "*.callgrind"))
		must(e)
		require(len(paths) == 12, "expected 12 SQLite profiles")
		inputs = append(inputs, paths...)
	}
	parts := 0
	for _, input := range inputs {
		raw, _ := execute(*exporter, input)
		expected := parseExport(raw)
		must(os.WriteFile(filepath.Join(*out, filepath.Base(input)+".input.tsv"), raw, 0644))
		for part, want := range expected {
			name := fmt.Sprintf("%s.part%d", filepath.Base(input), part)
			filename := filepath.Join(*out, name+".pb.gz")
			_, stderr := execute(*converter, input, "--part", strconv.Itoa(part), "-o", filename)
			must(os.WriteFile(filepath.Join(*out, name+".stderr"), stderr, 0644))
			data, e := os.ReadFile(filename)
			must(e)
			p, e := profile.ParseData(data)
			must(e)
			validate(p, want)
			report, diagnostics := execute(*pprof, "-symbolize=none", "-top", "-sample_index=0", "-nodecount=0", "-nodefraction=0", filename)
			must(os.WriteFile(filepath.Join(*out, name+".top.txt"), report, 0644))
			must(os.WriteFile(filepath.Join(*out, name+".top.stderr"), diagnostics, 0644))
			require(len(report) > 0, "empty pprof report")
			fmt.Printf("validated %s: %d events, %d exclusive locations; upstream pprof report passed\n", name, len(want.events), len(p.Sample))
			parts++
		}
	}
	fmt.Printf("callgrind2pprof: %d files / %d parts independently validated\n", len(inputs), parts)
}
