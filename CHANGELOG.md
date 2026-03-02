# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

---

## [0.1.2] — 2026-03-02

### Added

- `Figure::with_figure_size(w, h)` — specify total figure dimensions and have cell sizes auto-computed to fit, accounting for padding, spacing, title height, and shared legend area

### Fixed

- Clippy warnings resolved: `type_complexity` in `TerminalBackend` (extracted `type Rgb = (u8, u8, u8)`), `manual_is_multiple_of` in `render_utils`, and `needless_range_loop` suppressed on intentional triangular matrix loops in chord rendering
- `test_missing_feature_error` / `test_missing_feature_pdf` marked `#[ignore]` — these tests check a compile-time feature gate and were producing false-positive failures when a stale binary built with `--features full` was present on disk
- CI Clippy step now runs with `-D warnings` — all warnings are errors

---

## [0.1.1] — 2026-03-01

### Added

- `kuva::prelude::*` — single-import module re-exporting all plot structs, `Plot`, `Layout`, `Figure`, `Theme`, `Palette`, render helpers, backends, annotations, and datetime utilities
- `Into<Plot>` for all 25 plot structs — write `plot.into()` instead of `Plot::Scatter(plot)`
- `render_to_svg(plots, layout) -> String` — full pipeline in one call
- `render_to_png(plots, layout, scale) -> Result<Vec<u8>, String>` — one-call PNG output (feature `png`)
- `render_to_pdf(plots, layout) -> Result<Vec<u8>, String>` — one-call PDF output (feature `pdf`)
- GitHub Actions workflow to deploy the mdBook documentation to GitHub Pages on every push to `main`

### Fixed

- Unresolved intra-doc links (`Rect`, `Text`, `Line`) in `backend::terminal` module doc

---

## [0.1.0] — 2026-02-28

Initial release of kuva.

### Added

**Plot types (25)**
- `ScatterPlot` — x/y scatter with optional trend line, Pearson correlation, error bars, confidence bands, bubble sizing, and colour-by grouping
- `LinePlot` — connected line plots with optional area fill, step mode, and line style (solid/dashed/dotted/dash-dot)
- `BarPlot` — vertical bar charts with optional grouping and stacking
- `Histogram` — single-variable frequency histogram with optional normalisation and log scale
- `Histogram2D` — 2D density histogram with configurable colourmap
- `BoxPlot` — box-and-whisker with optional strip/swarm overlay
- `ViolinPlot` — KDE violin with optional strip/swarm overlay and configurable bandwidth
- `PiePlot` — pie/donut chart with inside and outside label modes, percentages, and minimum label fraction threshold
- `SeriesPlot` — multi-series line chart sharing a common x axis
- `Heatmap` — matrix heatmap with configurable colourmap and optional value labels
- `BrickPlot` — per-read sequencing alignment visualisation with STRIGAR string support
- `BandPlot` — line with shaded confidence band
- `WaterfallPlot` — waterfall chart with delta/total bar kinds, connectors, value labels, and sign-based colouring
- `StripPlot` — strip/jitter plot with jitter, swarm, and centre modes
- `VolcanoPlot` — log2 fold-change vs −log10(p-value) with threshold lines, up/down/NS colouring, and gene labels
- `ManhattanPlot` — genome-wide association plot with per-chromosome colouring, gene labels, and hg19/hg38/T2T base-pair coordinate mode
- `DotPlot` — size + colour encoding on a categorical grid with stacked size legend and colour bar
- `UpSetPlot` — UpSet intersection diagram with bitmask input, sort modes, and set-size bars
- `StackedAreaPlot` — stacked area chart with absolute and 100%-normalised modes
- `CandlestickPlot` — OHLC candlestick chart with optional volume panel and datetime x axis
- `ContourPlot` — contour plot from scattered or grid data using marching squares and IDW interpolation; filled and line modes
- `ChordPlot` — chord diagram from an N×N flow matrix with per-node colours and Bézier ribbons
- `SankeyPlot` — Sankey diagram with auto column assignment, tapered Bézier ribbons, and source/gradient/per-link colour modes
- `PhyloTree` — phylogenetic tree from Newick string, edge list, distance matrix (UPGMA), or linkage matrix; rectangular/slanted/circular branch styles; Left/Right/Top/Bottom orientation; clade colouring; bootstrap support values
- `SyntenyPlot` — pairwise genomic synteny diagram with named sequences, forward/inverted blocks, Bézier ribbons, per-sequence or shared scale, and block colouring

**Rendering**
- SVG output via `SvgBackend` (always available; no system dependencies)
- PNG rasterisation via `PngBackend` (feature: `png`; uses `resvg`, pure Rust)
- Vector PDF output via `PdfBackend` (feature: `pdf`; uses `svg2pdf`, pure Rust)
- `Figure` for multi-plot grid layouts with merged cells, shared axes, panel labels (A/B/C, a/b/c, 1/2/3, or custom), and shared legends
- Secondary y axis (`render_twin_y`)
- Date/time x and y axes with automatic tick granularity (`DateTimeAxis`)
- Log-scale x and y axes with 1-2-5 tick generation
- Custom tick formatting (`TickFormat`: Auto, Fixed, Integer, Sci, Percent, Custom)
- Text annotations with optional arrow at data coordinates
- Reference lines (horizontal/vertical) with optional label and dash pattern
- Shaded regions (horizontal/vertical fills)
- Theme support: Default, Dark, Publication, and custom themes
- Named colour palettes with modulo-wrapping index access: `category10`, `wong`, `okabe_ito`, `tol_bright`, `tol_muted`, `tol_light`, `ibm`, `pastel`, `bold`, and `Palette::custom()`

**CLI binary (`kuva`)**
- 22 subcommands covering all plot types: `scatter`, `line`, `bar`, `histogram`, `box`, `violin`, `pie`, `strip`, `waterfall`, `stacked-area`, `volcano`, `manhattan`, `candlestick`, `heatmap`, `hist2d`, `contour`, `dot`, `upset`, `chord`, `sankey`, `phylo`, `synteny`
- Auto-detects TSV/CSV delimiter; optional `--no-header` and `-d/--delimiter`
- `--color-by` for palette-assigned group series on scatter, line, strip
- `--theme`, `--palette`, `--colourblind` for appearance control
- `--log-x` / `--log-y` on applicable subcommands
- PNG and PDF output when built with the corresponding feature flags
- Hidden `kuva man` subcommand generates a `man(1)` page via `clap_mangen`
- `--terminal` flag renders plots directly in the terminal using Unicode braille (U+2800–U+28FF), full-block (`█`) fills, and ANSI 24-bit colour; ideal for HPC and remote-server workflows with no display; auto-detects terminal dimensions, overrideable with `--term-width` / `--term-height`; supported by all subcommands except `upset`

### Known limitations

- `kuva brick` CLI subcommand is not yet implemented (pending integration with bladerunner)
- Terminal rendering is not yet supported for `upset` (the command prints a message and exits cleanly; use `-o file.svg` instead)
- No Python or other language bindings
