# Funnel Chart

A funnel chart shows how values **attrit through ordered stages** — each bar represents one stage, and widths are proportional to stage values.  Trapezoidal connectors between bars make the drop-off visually explicit.  A diverging (back-to-back) mode supports side-by-side comparisons such as treatment vs. control arms.

## Basic usage

```rust,no_run
use kuva::plot::funnel::FunnelPlot;
use kuva::render::{plots::Plot, layout::Layout, render::render_multiple};
use kuva::backend::svg::SvgBackend;

let plot = FunnelPlot::new()
    .with_stage("Screened",   1200)
    .with_stage("Eligible",    800)
    .with_stage("Enrolled",    600)
    .with_stage("Completed",   540);

let plots = vec![Plot::Funnel(plot)];
let layout = Layout::auto_from_plots(&plots).with_title("CONSORT Flow");
let svg = SvgBackend.render_scene(&render_multiple(plots, layout));
std::fs::write("funnel.svg", svg).unwrap();
```

## Bulk stages

```rust,no_run
use kuva::plot::funnel::FunnelPlot;

let plot = FunnelPlot::new().with_stages([
    ("Awareness", 5000.0),
    ("Interest",  3000.0),
    ("Desire",    2000.0),
    ("Action",    1200.0),
]);
```

## Color modes

```rust,no_run
use kuva::plot::funnel::{FunnelPlot, FunnelColorMode};

// Each stage gets a distinct category10 color
let plot = FunnelPlot::new()
    .with_stages([("A", 1000.0), ("B", 700.0), ("C", 400.0)])
    .with_color_mode(FunnelColorMode::ByStage);

// Bars progressively darken from top to bottom
let plot = FunnelPlot::new()
    .with_stages([("A", 1000.0), ("B", 700.0), ("C", 400.0)])
    .with_color_mode(FunnelColorMode::Gradient);

// Per-stage explicit colors
let plot = FunnelPlot::new()
    .with_stage_color("Screened", 1200.0, "#2980b9")
    .with_stage_color("Eligible",  800.0, "#27ae60")
    .with_stage_color("Enrolled",  600.0, "#e67e22");
```

## Horizontal orientation

```rust,no_run
use kuva::plot::funnel::{FunnelPlot, FunnelOrientation};

let plot = FunnelPlot::new()
    .with_stages([("Q1", 1200.0), ("Q2", 950.0), ("Q3", 720.0), ("Q4", 580.0)])
    .with_orientation(FunnelOrientation::Horizontal);
```

## Diverging (mirror) mode

Show two parallel funnels side-by-side — useful for treatment vs. control arms in clinical trials.

```rust,no_run
use kuva::plot::funnel::FunnelPlot;

let plot = FunnelPlot::new()
    .with_stage("Screened",   1200)
    .with_stage("Eligible",    840)
    .with_stage("Enrolled",    720)
    .with_stage("Completed",   648)
    .with_mirror_stages([
        ("Screened",  1150.0),
        ("Eligible",   810.0),
        ("Enrolled",   690.0),
        ("Completed",  620.0),
    ])
    .with_mirror_labels("Treatment", "Control");
```

## Label options

```rust,no_run
let plot = FunnelPlot::new()
    .with_stages([("A", 1000.0), ("B", 700.0), ("C", 400.0)])
    .with_show_percents(true)     // show "700 (70.0%)" alongside value
    .with_show_conversion(true)   // show "70.0%" step-to-step rate in connectors
    .with_show_values(true);      // show absolute values (default: true)
```

## Builder reference

| Method | Default | Description |
|--------|---------|-------------|
| `.with_stage(label, value)` | — | Append one stage. |
| `.with_stage_color(label, value, css)` | — | Stage with explicit CSS fill color. |
| `.with_stages(iter)` | — | Append multiple `(label, value)` stages at once. |
| `.with_mirror(stages)` | `None` | Enable diverging mode with `Vec<FunnelStage>`. |
| `.with_mirror_stages(iter)` | `None` | Enable diverging mode from `(label, value)` iterator. |
| `.with_mirror_labels(left, right)` | — | Side labels for diverging mode. |
| `.with_orientation(o)` | `Vertical` | `Vertical` or `Horizontal`. |
| `.with_connectors(bool)` | `true` | Draw trapezoidal connectors between bars. |
| `.with_connector_opacity(f64)` | `0.4` | Connector fill opacity 0–1. Mirrors `SankeyPlot::with_link_opacity`. |
| `.with_show_values(bool)` | `true` | Absolute value label on each bar. |
| `.with_show_percents(bool)` | `false` | Show percentage-of-first-stage alongside value. |
| `.with_show_conversion(bool)` | `true` | Step-to-step conversion rate in connector areas. |
| `.with_color_mode(mode)` | `Uniform` | `Uniform`, `ByStage`, or `Gradient`. |
| `.with_stage_gap(f64)` | `4.0` | Pixel gap between adjacent bars. Mirrors `SankeyPlot::with_node_gap`. |
| `.with_legend(label)` | `None` | Enable legend with given label. Mirrors `SankeyPlot::with_legend`. |

**See also:** [Bar Chart](./bar.md) for a simpler categorical comparison, [Pie Chart](./pie.md) for a static (non-sequential) share view.
