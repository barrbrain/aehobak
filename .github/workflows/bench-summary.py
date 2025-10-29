import json
import sys

benches = json.load(sys.stdin)
print('# Benchmark Results')

default_stats = {
    "Callgrind": [
        "Ir",
        "L1hits",
        "LLhits",
        "RamHits",
        "TotalRW",
        "EstimatedCycles",
    ]
}

for bench in benches:
    name = bench['module_path']
    print(f"## `{name}`")
    for profile in bench['profiles']:
        summary = profile['summaries']['total']['summary']
        for tool in sorted(summary.keys()):
            print(f"### {tool}")
            details = summary[tool]
            print('| Counter | Diff | Factor |')
            print('| -- | --: | --: |')
            for stat in default_stats.get(tool, sorted(details.keys())):
                diff_pct = float(details[stat]['diffs']['diff_pct'])
                factor = float(details[stat]['diffs']['factor'])
                if diff_pct > 0:
                    print('| 🟠 %s | %+2.4f%% | %+2.4fx |' % (stat, diff_pct, factor))
                elif diff_pct < 0:
                    print('| 🟢 %s | %+2.4f%% | %+2.4fx |' % (stat, diff_pct, factor))
                else:
                    print(f"| ⚪ {stat} | No change |  |")
