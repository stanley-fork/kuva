# kuva pareto

Pareto chart — bars sorted descending by value, with a cumulative-percentage line on a secondary axis.

**Input:** one row per category with a label column and a value column.

| Flag | Default | Description |
|---|---|---|
| `--label-col <COL>` | `0` | Category label column |
| `--value-col <COL>` | `1` | Category value column |
| `--color <CSS>` | `steelblue` | Bar fill color |
| `--line-color <CSS>` | `firebrick` | Cumulative-line color |
| `--bar-width <FRAC>` | `0.8` | Bar width as a fraction of the category slot |
| `--no-sort` | off | Preserve input row order instead of sorting descending by value |
| `--threshold <PCT>` | `80.0` | Reference-line value (cumulative %) |
| `--no-threshold` | off | Hide the dashed reference line |
| `--cumulative-labels` | off | Show a `%` label above (or beside, in `--horizontal` mode) each cumulative-line point |
| `--legend <BAR,LINE>` | `Value,Cumulative %` | Legend labels for the bars and the cumulative line, comma-separated |
| `--no-legend` | off | Hide the legend (shown by default) |
| `--max-categories <N>` | — (no bucketing) | Collapse categories beyond the top `N - 1` into one stacked "Other" bar |
| `--other-label <STR>` | `Other` | Label for the bucketed bar; no effect without `--max-categories` |
| `--horizontal` | off | Categories on Y-axis, values on X-axis; cumulative line on a secondary X-axis drawn on top |

```bash
kuva pareto data.tsv --label-col category --value-col count

kuva pareto data.tsv --label-col category --value-col count \
    --color seagreen --line-color darkorange --threshold 90 \
    --cumulative-labels --legend "Count,Cumulative %"

kuva pareto data.tsv --label-col category --value-col count \
    --horizontal --max-categories 5 --other-label "Misc"
```

---

*See also: [Shared flags](./index.md#shared-flags) — output, appearance, axes, log scale.*
