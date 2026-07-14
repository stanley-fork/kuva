//! `--emit-code`: print the equivalent Rust library code for a CLI-built plot
//! instead of rendering it. The emitted snippet bakes in the plot's resolved
//! data as literals — a copy-pasteable starting point, not a re-parse of the
//! input file — mirroring the style of the rustdoc examples in `src/plot/*.rs`.
//!
//! The `emit_base_args`/`emit_axis_args`/`emit_log_args` functions are direct
//! structural mirrors of `apply_base_args`/`apply_axis_args`/`apply_log_args`
//! in `layout_args.rs`: same `if let Some(x) = args.field` shape, but each
//! pushes a builder-call *string* instead of calling the real method. Keep
//! the two in sync when either changes.

use kuva::plot::boxplot::BoxGroup;
use kuva::plot::pie::{PieLabelPosition, PieSlice};
use kuva::plot::{
    BoxPlot, CellShape, FillOrder, FunnelColorMode, FunnelOrientation, FunnelPlot, FunnelStage,
    LollipopDomain, LollipopPlot, LollipopPoint, PiePlot, PopulationPyramid, PyramidMode,
    PyramidSeries, StripStyle, VennPlot, VennSet, WaffleCategory, WafflePlot,
};

use kuva::plot::bar::BarGroup;
use kuva::plot::dotplot::DotPoint;
use kuva::plot::ecdf::EcdfGroup;
use kuva::plot::mosaic::MosaicCell;
use kuva::plot::qq::{QQGroup, QQMode};
use kuva::plot::scatter::{ScatterPoint, TrendLine};
use kuva::plot::{
    BarPlot, ColorMap, DotPlot, EcdfPlot, MarkerShape, MosaicPlot, ParetoPlot, QQPlot, ScatterPlot,
};

use crate::layout_args::{AxisArgs, BaseArgs, LogArgs};

// Tier 2 additions (line/histogram/violin/roc/survival) — kept as a separate
// `use` block so concurrent edits to the Tier-1 import list above don't
// collide with this one.
use kuva::plot::{
    Histogram, KMGroup, LinePlot, LineStyle, RocGroup, RocPlot, SurvivalPlot, ViolinGroup,
    ViolinPlot,
};

// Tier 2 additions (strip/polar/rose/manhattan/volcano) — kept as its own
// `use` block for the same reason as above.
use kuva::plot::polar::PolarSeries;
use kuva::plot::strip::StripGroup;
use kuva::plot::volcano::LabelStyle;
use kuva::plot::{
    ManhattanPlot, ManhattanPoint, PolarMode, PolarPlot, RoseEncoding, RoseMode, RosePlot,
    StripPlot, VolcanoPlot,
};

// ── Literal helpers ───────────────────────────────────────────────────────────

/// Escape a string into a valid Rust string literal.
pub fn str_lit(s: &str) -> String {
    format!("{s:?}")
}

/// Format an `f64` so it unambiguously reads as `f64` in Rust source (always
/// has a decimal point, e.g. `42.0` not `42`).
pub fn f64_lit(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

/// Join a constructor expression with builder-call fragments into a
/// multi-line chained expression, e.g. `chain("BoxPlot::new()", vec![".with_color(\"red\")".into()])`
/// → `"BoxPlot::new()\n    .with_color(\"red\")"`.
pub fn chain(ctor: &str, frags: Vec<String>) -> String {
    let mut s = ctor.to_string();
    for f in frags {
        s.push_str("\n    ");
        s.push_str(&f);
    }
    s
}

// ── Layout-side emitters (mirror layout_args::apply_*_args) ─────────────────

fn theme_ctor(name: &str) -> &'static str {
    match name {
        "dark" => "Theme::dark()",
        "solarized" | "solar" => "Theme::solarized()",
        "minimal" => "Theme::minimal()",
        _ => "Theme::light()",
    }
}

fn palette_ctor(name: &str) -> Option<&'static str> {
    match name {
        "category10" => Some("Palette::category10()"),
        "wong" => Some("Palette::wong()"),
        "okabe-ito" | "okabe_ito" => Some("Palette::okabe_ito()"),
        "pastel" => Some("Palette::pastel()"),
        "bold" => Some("Palette::bold()"),
        "tol-bright" | "tol_bright" => Some("Palette::tol_bright()"),
        "tol-muted" | "tol_muted" => Some("Palette::tol_muted()"),
        "tol-light" | "tol_light" => Some("Palette::tol_light()"),
        "ibm" => Some("Palette::ibm()"),
        _ => None,
    }
}

fn cvd_palette_ctor(condition: &str) -> Option<&'static str> {
    match condition {
        "deuteranopia" | "deuter" => Some("Palette::deuteranopia()"),
        "protanopia" | "protan" => Some("Palette::protanopia()"),
        "tritanopia" | "tritan" => Some("Palette::tritanopia()"),
        _ => None,
    }
}

fn axis_line_ctor(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().replace('_', "-").as_str() {
        "open" | "left" | "primary" => Some("AxisLine::Open"),
        "box" | "frame" | "enclosed" => Some("AxisLine::Box"),
        _ => None,
    }
}

fn tick_align_ctor(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().replace('_', "-").as_str() {
        "inside" | "in" => Some("TickAlign::Inside"),
        "outside" | "out" => Some("TickAlign::Outside"),
        "center" | "centre" | "middle" => Some("TickAlign::Center"),
        _ => None,
    }
}

fn tick_pos_ctor(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().replace('_', "-").as_str() {
        "primary" | "left" | "bottom" | "lower" => Some("TickPos::Primary"),
        "both" | "mirror" | "mirrored" => Some("TickPos::Both"),
        _ => None,
    }
}

fn label_overlap_ctor(s: &str) -> Option<&'static str> {
    match s {
        "allow" => Some("AxisLabelOverlap::Allow"),
        "thin" => Some("AxisLabelOverlap::Thin"),
        "stagger" => Some("AxisLabelOverlap::Stagger"),
        _ => None,
    }
}

fn tick_format_ctor(s: &str) -> Option<String> {
    match s {
        "auto" => Some("TickFormat::Auto".to_string()),
        "int" => Some("TickFormat::Integer".to_string()),
        "sci" => Some("TickFormat::Sci".to_string()),
        "percent" => Some("TickFormat::Percent".to_string()),
        _ if s.starts_with("fixed:") => s["fixed:".len()..]
            .parse::<usize>()
            .ok()
            .map(|n| format!("TickFormat::Fixed({n})")),
        _ => None,
    }
}

/// Emit `.with_x(...)` fragments for `BaseArgs` — mirrors `apply_base_args`.
/// Terminal-only fields (`terminal`/`term_bg`/`term_width`/`term_height`) and
/// `embed_font` are intentionally skipped: the emitted snippet always targets
/// `SvgBackend`, and those flags only affect the CLI's own output dispatch.
pub fn emit_base_args(args: &BaseArgs) -> Vec<String> {
    let mut frags = Vec::new();
    if let Some(w) = args.width {
        frags.push(format!(".with_width({})", f64_lit(w)));
    }
    if let Some(h) = args.height {
        frags.push(format!(".with_height({})", f64_lit(h)));
    }
    if let Some(ref t) = args.title {
        frags.push(format!(".with_title({})", str_lit(t)));
    }
    if let Some(ref name) = args.theme {
        frags.push(format!(".with_theme({})", theme_ctor(name)));
    }
    if let Some(ref name) = args.palette {
        if let Some(ctor) = palette_ctor(name) {
            frags.push(format!(".with_palette({ctor})"));
        }
    }
    // --cvd-palette overrides --palette when both are provided.
    if let Some(ref condition) = args.cvd_palette {
        if let Some(ctor) = cvd_palette_ctor(condition) {
            frags.push(format!(".with_palette({ctor})"));
        }
    }
    if let Some(f) = args.scale {
        frags.push(format!(".with_scale({})", f64_lit(f)));
    }
    if args.interactive {
        frags.push(".with_interactive()".to_string());
    }
    if args.bw {
        frags.push(".with_bw_mode()".to_string());
    }
    if let Some(n) = args.wrap {
        frags.push(format!(".with_wrap({n})"));
    }
    if let Some(n) = args.title_wrap {
        frags.push(format!(".with_title_wrap({n})"));
    }
    if let Some(n) = args.x_label_wrap {
        frags.push(format!(".with_x_label_wrap({n})"));
    }
    if let Some(n) = args.y_label_wrap {
        frags.push(format!(".with_y_label_wrap({n})"));
    }
    if let Some(n) = args.y2_label_wrap {
        frags.push(format!(".with_y2_label_wrap({n})"));
    }
    if let Some(n) = args.legend_wrap {
        frags.push(format!(".with_legend_wrap({n})"));
    }
    frags
}

/// Emit `.with_x(...)` fragments for `AxisArgs` — mirrors `apply_axis_args`.
pub fn emit_axis_args(args: &AxisArgs) -> Vec<String> {
    let mut frags = Vec::new();
    if let Some(ref l) = args.x_label {
        frags.push(format!(".with_x_label({})", str_lit(l)));
    }
    if let Some(ref l) = args.y_label {
        frags.push(format!(".with_y_label({})", str_lit(l)));
    }
    if let Some(t) = args.ticks {
        frags.push(format!(".with_ticks({t})"));
    }
    if args.no_grid {
        frags.push(".with_show_grid(false)".to_string());
    }
    if let Some(ref line) = args.axis_line {
        if let Some(ctor) = axis_line_ctor(line) {
            frags.push(format!(".with_axis_line({ctor})"));
        }
    }
    if let Some(ref align) = args.tick_align {
        if let Some(ctor) = tick_align_ctor(align) {
            frags.push(format!(".with_tick_align({ctor})"));
        }
    }
    if let Some(ref pos) = args.tick_pos {
        if let Some(ctor) = tick_pos_ctor(pos) {
            frags.push(format!(".with_tick_pos({ctor})"));
        }
    }
    if let Some(v) = args.x_min {
        frags.push(format!(".with_x_axis_min({})", f64_lit(v)));
    }
    if let Some(v) = args.x_max {
        frags.push(format!(".with_x_axis_max({})", f64_lit(v)));
    }
    if let Some(v) = args.y_min {
        frags.push(format!(".with_y_axis_min({})", f64_lit(v)));
    }
    if let Some(v) = args.y_max {
        frags.push(format!(".with_y_axis_max({})", f64_lit(v)));
    }
    if let Some(s) = args.x_tick_step {
        frags.push(format!(".with_x_tick_step({})", f64_lit(s)));
    }
    if let Some(s) = args.y_tick_step {
        frags.push(format!(".with_y_tick_step({})", f64_lit(s)));
    }
    if let Some(n) = args.minor_ticks {
        frags.push(format!(".with_minor_ticks({n})"));
    }
    if args.minor_grid {
        frags.push(".with_show_minor_grid(true)".to_string());
    }
    if let Some(ref fmt) = args.x_tick_format {
        if let Some(ctor) = tick_format_ctor(fmt) {
            frags.push(format!(".with_x_tick_format({ctor})"));
        }
    }
    if let Some(ref fmt) = args.y_tick_format {
        if let Some(ctor) = tick_format_ctor(fmt) {
            frags.push(format!(".with_y_tick_format({ctor})"));
        }
    }
    if let Some(ref s) = args.x_label_overlap {
        if let Some(ctor) = label_overlap_ctor(s) {
            frags.push(format!(".with_x_label_overlap({ctor})"));
        }
    }
    frags
}

/// Emit `.with_x(...)` fragments for `LogArgs` — mirrors `apply_log_args`.
pub fn emit_log_args(args: &LogArgs) -> Vec<String> {
    let mut frags = Vec::new();
    if args.log_x {
        frags.push(".with_log_x()".to_string());
    }
    if args.log_y {
        frags.push(".with_log_y()".to_string());
    }
    frags
}

// ── Snippet assembly ──────────────────────────────────────────────────────────

/// Assemble the full printable snippet: imports, plot construction, layout
/// construction (via the emitted base/axis/log fragments), and render/write
/// boilerplate — styled like the rustdoc examples in `src/plot/*.rs`.
pub fn assemble(
    plot_uses: &[&str],
    variant: &str,
    plot_exprs: &[String],
    base: &BaseArgs,
    axis: Option<&AxisArgs>,
    log: Option<&LogArgs>,
) -> String {
    let mut out = String::new();
    out.push_str("use kuva::backend::svg::SvgBackend;\n");
    out.push_str("use kuva::render::render::render_multiple;\n");
    out.push_str("use kuva::render::layout::Layout;\n");
    out.push_str("use kuva::render::plots::Plot;\n");
    for u in plot_uses {
        out.push_str(&format!("use {u};\n"));
    }
    out.push('\n');

    let wrapped: Vec<String> = plot_exprs
        .iter()
        .map(|e| format!("Plot::{variant}({e})"))
        .collect();
    if wrapped.len() == 1 {
        out.push_str(&format!("let plots = vec![{}];\n\n", wrapped[0]));
    } else {
        out.push_str("let plots = vec![\n");
        for p in &wrapped {
            out.push_str(&format!("    {p},\n"));
        }
        out.push_str("];\n\n");
    }

    let mut frags = emit_base_args(base);
    if let Some(a) = axis {
        frags.extend(emit_axis_args(a));
    }
    if let Some(l) = log {
        frags.extend(emit_log_args(l));
    }

    out.push_str(&chain(
        "let layout = Layout::auto_from_plots(&plots)",
        frags,
    ));
    out.push_str(";\n\n");

    if let Some(ref bg) = base.background {
        out.push_str("let mut scene = render_multiple(plots, layout);\n");
        out.push_str(&format!(
            "scene.background_color = Some({}.to_string());\n",
            str_lit(bg)
        ));
        out.push_str("let svg = SvgBackend::new().render_scene(&scene);\n");
    } else {
        out.push_str(
            "let svg = SvgBackend::new().render_scene(&render_multiple(plots, layout));\n",
        );
    }
    out.push_str("std::fs::write(\"output.svg\", svg).unwrap();\n");
    out
}

// ── Per-plot-type emitters (Tier 1) ───────────────────────────────────────────

fn emit_pie_slice(slice: &PieSlice) -> String {
    format!(
        ".with_slice({}, {}, {})",
        str_lit(&slice.label),
        f64_lit(slice.value),
        str_lit(&slice.color)
    )
}

pub fn emit_pie_plot(p: &PiePlot) -> String {
    let mut frags: Vec<String> = p.slices.iter().map(emit_pie_slice).collect();
    if p.inner_radius != 0.0 {
        frags.push(format!(".with_inner_radius({})", f64_lit(p.inner_radius)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    match p.label_position {
        PieLabelPosition::Inside => {
            frags.push(".with_label_position(PieLabelPosition::Inside)".to_string())
        }
        PieLabelPosition::Outside => {
            frags.push(".with_label_position(PieLabelPosition::Outside)".to_string())
        }
        PieLabelPosition::None => {
            frags.push(".with_label_position(PieLabelPosition::None)".to_string())
        }
        PieLabelPosition::Auto => {}
    }
    if p.show_percent {
        frags.push(".with_percent()".to_string());
    }
    if (p.min_label_fraction - 0.05).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_min_label_fraction({})",
            f64_lit(p.min_label_fraction)
        ));
    }
    chain("PiePlot::new()", frags)
}

fn emit_box_group(g: &BoxGroup) -> String {
    let values = g
        .values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    format!(".with_group({}, vec![{}])", str_lit(&g.label), values)
}

pub fn emit_boxplot(p: &BoxPlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(emit_box_group).collect();
    if p.color != "black" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors([{list}])"));
    }
    if (p.width - 0.8).abs() > f64::EPSILON {
        frags.push(format!(".with_width({})", f64_lit(p.width)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    match &p.overlay {
        Some(StripStyle::Strip { jitter }) => {
            frags.push(format!(".with_strip({})", f64_lit(*jitter)))
        }
        Some(StripStyle::Swarm) => frags.push(".with_swarm_overlay()".to_string()),
        // `Center` has no dedicated BoxPlot builder (not reachable from the CLI
        // today) and `None` needs no fragment.
        Some(StripStyle::Center) | None => {}
    }
    if p.horizontal {
        frags.push(".with_horizontal(true)".to_string());
    }
    if p.notch {
        frags.push(".with_notch(true)".to_string());
        if (p.notch_depth - 0.3).abs() > f64::EPSILON {
            frags.push(format!(".with_notch_depth({})", f64_lit(p.notch_depth)));
        }
        if (p.notch_width - 0.4).abs() > f64::EPSILON {
            frags.push(format!(".with_notch_width({})", f64_lit(p.notch_width)));
        }
    }
    chain("BoxPlot::new()", frags)
}

fn emit_venn_set(s: &VennSet) -> String {
    if let Some(ref elements) = s.elements {
        let els = elements
            .iter()
            .map(|e| str_lit(e))
            .collect::<Vec<_>>()
            .join(", ");
        format!(".with_set({}, vec![{}])", str_lit(&s.label), els)
    } else {
        format!(
            ".with_set_size({}, {})",
            str_lit(&s.label),
            s.size.unwrap_or(0)
        )
    }
}

pub fn emit_venn_plot(p: &VennPlot) -> String {
    // Note: `VennPlot::overlaps` is `pub(crate)` to the library and not
    // readable from this binary crate. That's fine here: the CLI's `venn.rs`
    // only ever builds sets via `with_set` (raw elements) — it never calls
    // `with_overlap` — so pre-computed overlaps are never present on a
    // CLI-built `VennPlot`.
    let mut frags: Vec<String> = p.sets.iter().map(emit_venn_set).collect();
    if !p.show_counts {
        frags.push(".with_counts(false)".to_string());
    }
    if p.show_percentages {
        frags.push(".with_percentages(true)".to_string());
    }
    if !p.show_set_labels {
        frags.push(".with_set_labels(false)".to_string());
    }
    if (p.fill_opacity - 0.25).abs() > f64::EPSILON {
        frags.push(format!(".with_fill_opacity({})", f64_lit(p.fill_opacity)));
    }
    if (p.stroke_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if p.proportional {
        frags.push(".with_proportional(true)".to_string());
    }
    if p.show_loss {
        frags.push(".with_loss(true)".to_string());
    }
    if let Some(ref colors) = p.colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_colors([{list}])"));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if p.leader_lines {
        frags.push(".with_leader_lines(true)".to_string());
    }
    if !p.show_set_indicators {
        frags.push(".with_set_indicators(false)".to_string());
    }
    chain("VennPlot::new()", frags)
}

fn fill_order_ctor(order: FillOrder) -> &'static str {
    match order {
        FillOrder::RowMajorTopLeft => "FillOrder::RowMajorTopLeft",
        FillOrder::RowMajorBottomLeft => "FillOrder::RowMajorBottomLeft",
        FillOrder::ColMajorTopLeft => "FillOrder::ColMajorTopLeft",
        FillOrder::ColMajorBottomLeft => "FillOrder::ColMajorBottomLeft",
    }
}

fn emit_waffle_category(c: &WaffleCategory) -> String {
    format!(
        ".with_category({}, {}, {})",
        str_lit(&c.label),
        f64_lit(c.value),
        str_lit(&c.color)
    )
}

pub fn emit_waffle_plot(p: &WafflePlot) -> String {
    let mut frags: Vec<String> = p.categories.iter().map(emit_waffle_category).collect();
    if p.rows != 10 || p.cols != 10 {
        frags.push(format!(".with_grid({}, {})", p.rows, p.cols));
    }
    if (p.gap - 0.1).abs() > f64::EPSILON {
        frags.push(format!(".with_gap({})", f64_lit(p.gap)));
    }
    if p.fill_order != FillOrder::RowMajorTopLeft {
        frags.push(format!(
            ".with_fill_order({})",
            fill_order_ctor(p.fill_order)
        ));
    }
    if p.shape == CellShape::Circle {
        frags.push(".with_shape(CellShape::Circle)".to_string());
    }
    if p.empty_color != "#e8e8e8" {
        frags.push(format!(".with_empty_color({})", str_lit(&p.empty_color)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if p.show_percents {
        frags.push(".with_show_percents()".to_string());
    }
    if p.show_counts {
        frags.push(".with_show_counts()".to_string());
    }
    if let Some(ref label) = p.unit_label {
        frags.push(format!(".with_unit_label({})", str_lit(label)));
    }
    chain("WafflePlot::new()", frags)
}

fn emit_funnel_stage(s: &FunnelStage) -> String {
    match s.color {
        Some(ref color) => format!(
            ".with_stage_color({}, {}, {})",
            str_lit(&s.label),
            f64_lit(s.value),
            str_lit(color)
        ),
        None => format!(".with_stage({}, {})", str_lit(&s.label), f64_lit(s.value)),
    }
}

fn funnel_color_mode_ctor(mode: &FunnelColorMode) -> &'static str {
    match mode {
        FunnelColorMode::Uniform => "FunnelColorMode::Uniform",
        FunnelColorMode::ByStage => "FunnelColorMode::ByStage",
        FunnelColorMode::Gradient => "FunnelColorMode::Gradient",
    }
}

pub fn emit_funnel_plot(p: &FunnelPlot) -> String {
    let mut frags: Vec<String> = p.stages.iter().map(emit_funnel_stage).collect();
    if !matches!(p.orientation, FunnelOrientation::Vertical) {
        frags.push(".with_orientation(FunnelOrientation::Horizontal)".to_string());
    }
    if !p.show_connectors {
        frags.push(".with_connectors(false)".to_string());
    }
    if (p.connector_opacity - 0.4).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_connector_opacity({})",
            f64_lit(p.connector_opacity)
        ));
    }
    if !p.show_values {
        frags.push(".with_show_values(false)".to_string());
    }
    if p.show_percents {
        frags.push(".with_show_percents(true)".to_string());
    }
    if !p.show_conversion {
        frags.push(".with_show_conversion(false)".to_string());
    }
    if !matches!(p.color_mode, FunnelColorMode::Uniform) {
        frags.push(format!(
            ".with_color_mode({})",
            funnel_color_mode_ctor(&p.color_mode)
        ));
    }
    if (p.stage_gap - 4.0).abs() > f64::EPSILON {
        frags.push(format!(".with_stage_gap({})", f64_lit(p.stage_gap)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if let Some(ref mirror) = p.mirror {
        let stages = mirror
            .iter()
            .map(|s| format!("({}, {})", str_lit(&s.label), f64_lit(s.value)))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_mirror_stages(vec![{stages}])"));
    }
    if let (Some(ref left), Some(ref right)) = (&p.left_label, &p.right_label) {
        frags.push(format!(
            ".with_mirror_labels({}, {})",
            str_lit(left),
            str_lit(right)
        ));
    }
    chain("FunnelPlot::new()", frags)
}

fn pyramid_mode_ctor(mode: &PyramidMode) -> &'static str {
    match mode {
        PyramidMode::Grouped => "PyramidMode::Grouped",
        PyramidMode::Overlap => "PyramidMode::Overlap",
    }
}

fn emit_pyramid_series(s: &PyramidSeries) -> Vec<String> {
    let groups = s
        .groups
        .iter()
        .map(|(age, left, right)| {
            format!(
                "({}, {}, {})",
                str_lit(age),
                f64_lit(*left),
                f64_lit(*right)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = vec![format!(
        ".with_series({}, vec![{}])",
        str_lit(&s.label),
        groups
    )];
    if let Some(ref color) = s.color {
        out.push(format!(
            ".with_series_color({}, {})",
            str_lit(&s.label),
            str_lit(color)
        ));
    }
    out
}

pub fn emit_pyramid_plot(p: &PopulationPyramid) -> String {
    let mut frags: Vec<String> = Vec::new();
    // Single anonymous series (the common `with_group` CLI path) collapses to
    // `.with_group(age, left, right)` calls; anything else (named series) uses
    // `.with_series(...)`.
    if p.series.len() == 1 && p.series[0].label.is_empty() {
        for (age, left, right) in &p.series[0].groups {
            frags.push(format!(
                ".with_group({}, {}, {})",
                str_lit(age),
                f64_lit(*left),
                f64_lit(*right)
            ));
        }
    } else {
        for s in &p.series {
            frags.extend(emit_pyramid_series(s));
        }
    }
    if p.left_label != "Left" {
        frags.push(format!(".with_left_label({})", str_lit(&p.left_label)));
    }
    if p.right_label != "Right" {
        frags.push(format!(".with_right_label({})", str_lit(&p.right_label)));
    }
    if p.left_color != "#4C72B0" {
        frags.push(format!(".with_left_color({})", str_lit(&p.left_color)));
    }
    if p.right_color != "#DD8452" {
        frags.push(format!(".with_right_color({})", str_lit(&p.right_color)));
    }
    if p.normalize {
        frags.push(".with_normalize(true)".to_string());
    }
    if p.show_values {
        frags.push(".with_show_values(true)".to_string());
    }
    if (p.group_gap - 0.15).abs() > f64::EPSILON {
        frags.push(format!(".with_group_gap({})", f64_lit(p.group_gap)));
    }
    if (p.bar_gap - 0.04).abs() > f64::EPSILON {
        frags.push(format!(".with_bar_gap({})", f64_lit(p.bar_gap)));
    }
    if !matches!(p.mode, PyramidMode::Grouped) {
        frags.push(format!(".with_mode({})", pyramid_mode_ctor(&p.mode)));
    }
    if p.show_legend {
        frags.push(".with_legend(true)".to_string());
    }
    chain("PopulationPyramid::new()", frags)
}

fn emit_lollipop_point(pt: &LollipopPoint) -> String {
    match (&pt.label, &pt.color) {
        (Some(label), Some(color)) => format!(
            ".with_labeled_colored_point({}, {}, {}, {})",
            f64_lit(pt.x),
            f64_lit(pt.y),
            str_lit(label),
            str_lit(color)
        ),
        (Some(label), None) => format!(
            ".with_labeled_point({}, {}, {})",
            f64_lit(pt.x),
            f64_lit(pt.y),
            str_lit(label)
        ),
        (None, Some(color)) => format!(
            ".with_colored_point({}, {}, {})",
            f64_lit(pt.x),
            f64_lit(pt.y),
            str_lit(color)
        ),
        (None, None) => format!(".with_point({}, {})", f64_lit(pt.x), f64_lit(pt.y)),
    }
}

fn emit_lollipop_domain(d: &LollipopDomain) -> String {
    let label = match d.label {
        Some(ref l) => format!("Some({})", str_lit(l)),
        None => "None".to_string(),
    };
    if (d.opacity - 0.35).abs() > f64::EPSILON {
        format!(
            ".with_domain_opacity({}, {}, {}, {}, {})",
            f64_lit(d.x_start),
            f64_lit(d.x_end),
            label,
            str_lit(&d.color),
            f64_lit(d.opacity)
        )
    } else {
        format!(
            ".with_domain({}, {}, {}, {})",
            f64_lit(d.x_start),
            f64_lit(d.x_end),
            label,
            str_lit(&d.color)
        )
    }
}

pub fn emit_lollipop_plot(p: &LollipopPlot) -> String {
    let mut frags: Vec<String> = p.points.iter().map(emit_lollipop_point).collect();
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if p.baseline != 0.0 {
        frags.push(format!(".with_baseline({})", f64_lit(p.baseline)));
    }
    if (p.stem_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stem_width({})", f64_lit(p.stem_width)));
    }
    if (p.dot_radius - 5.0).abs() > f64::EPSILON {
        frags.push(format!(".with_dot_radius({})", f64_lit(p.dot_radius)));
    }
    if let Some(ref stroke) = p.dot_stroke {
        frags.push(format!(".with_dot_stroke({})", str_lit(stroke)));
    }
    if (p.dot_stroke_width - 1.0).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_dot_stroke_width({})",
            f64_lit(p.dot_stroke_width)
        ));
    }
    if !p.show_baseline {
        frags.push(".with_show_baseline(false)".to_string());
    }
    if p.baseline_color != "#888888" {
        frags.push(format!(
            ".with_baseline_color({})",
            str_lit(&p.baseline_color)
        ));
    }
    if (p.baseline_width - 1.0).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_baseline_width({})",
            f64_lit(p.baseline_width)
        ));
    }
    if let Some(ref dash) = p.baseline_dash {
        frags.push(format!(".with_baseline_dash({})", str_lit(dash)));
    }
    frags.extend(p.domains.iter().map(emit_lollipop_domain));
    if (p.domain_height - 0.5).abs() > f64::EPSILON {
        frags.push(format!(".with_domain_height({})", f64_lit(p.domain_height)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("LollipopPlot::new()", frags)
}

fn emit_ecdf_group(g: &EcdfGroup) -> String {
    let values = g
        .data
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    match g.color {
        Some(ref color) => format!(
            ".with_data_colored({}, vec![{}], {})",
            str_lit(&g.label),
            values,
            str_lit(color)
        ),
        None => format!(".with_data({}, vec![{}])", str_lit(&g.label), values),
    }
}

pub fn emit_ecdf_plot(p: &EcdfPlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(emit_ecdf_group).collect();
    if p.complementary {
        frags.push(".with_complementary()".to_string());
    }
    if p.show_confidence_band {
        frags.push(".with_confidence_band()".to_string());
        if (p.band_alpha - 0.15).abs() > f64::EPSILON {
            frags.push(format!(".with_band_alpha({})", f64_lit(p.band_alpha)));
        }
    }
    if p.show_rug {
        frags.push(".with_rug()".to_string());
        if (p.rug_height - 6.0).abs() > f64::EPSILON {
            frags.push(format!(".with_rug_height({})", f64_lit(p.rug_height)));
        }
    }
    if !p.percentile_lines.is_empty() {
        let list = p
            .percentile_lines
            .iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_percentile_lines(vec![{list}])"));
    }
    if p.show_markers {
        frags.push(".with_markers()".to_string());
        if (p.marker_size - 3.0).abs() > f64::EPSILON {
            frags.push(format!(".with_marker_size({})", f64_lit(p.marker_size)));
        }
    }
    if p.smooth {
        frags.push(".with_smooth()".to_string());
        if p.smooth_samples != 200 {
            frags.push(format!(".with_smooth_samples({})", p.smooth_samples));
        }
    }
    if (p.stroke_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if let Some(ref dash) = p.line_dash {
        frags.push(format!(".with_line_dash({})", str_lit(dash)));
    }
    chain("EcdfPlot::new()", frags)
}

fn emit_dot_point(pt: &DotPoint) -> String {
    format!(
        "({}, {}, {}, {})",
        str_lit(&pt.x_cat),
        str_lit(&pt.y_cat),
        f64_lit(pt.size),
        f64_lit(pt.color)
    )
}

fn color_map_ctor(map: &ColorMap) -> String {
    // `ColorMap` derives a `Debug` impl that already prints `ColorMap::Variant`
    // (or `ColorMap::Custom(<fn>)`, unreachable from the CLI — no flag builds one).
    format!("{map:?}")
}

pub fn emit_dot_plot(p: &DotPlot) -> String {
    let mut frags: Vec<String> = Vec::new();
    if !p.points.is_empty() {
        let tuples = p
            .points
            .iter()
            .map(emit_dot_point)
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_data(vec![{tuples}])"));
    }
    if !matches!(p.color_map, ColorMap::Viridis) {
        frags.push(format!(".with_color_map({})", color_map_ctor(&p.color_map)));
    }
    if (p.max_radius - 12.0).abs() > f64::EPSILON {
        frags.push(format!(".with_max_radius({})", f64_lit(p.max_radius)));
    }
    if (p.min_radius - 1.0).abs() > f64::EPSILON {
        frags.push(format!(".with_min_radius({})", f64_lit(p.min_radius)));
    }
    if let Some((min, max)) = p.size_range {
        frags.push(format!(
            ".with_size_range({}, {})",
            f64_lit(min),
            f64_lit(max)
        ));
    }
    if let Some((min, max)) = p.color_range {
        frags.push(format!(
            ".with_color_range({}, {})",
            f64_lit(min),
            f64_lit(max)
        ));
    }
    if let Some(ref label) = p.size_label {
        frags.push(format!(".with_size_legend({})", str_lit(label)));
    }
    if let Some(ref label) = p.color_legend_label {
        frags.push(format!(".with_colorbar({})", str_lit(label)));
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    chain("DotPlot::new()", frags)
}

fn emit_mosaic_cell(c: &MosaicCell) -> String {
    format!(
        ".with_cell({}, {}, {})",
        str_lit(&c.col),
        str_lit(&c.row),
        f64_lit(c.value)
    )
}

pub fn emit_mosaic_plot(p: &MosaicPlot) -> String {
    let mut frags: Vec<String> = p.cells.iter().map(emit_mosaic_cell).collect();
    if !p.col_order.is_empty() {
        let list = p
            .col_order
            .iter()
            .map(|s| str_lit(s))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_col_order(vec![{list}])"));
    }
    if !p.row_order.is_empty() {
        let list = p
            .row_order
            .iter()
            .map(|s| str_lit(s))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_row_order(vec![{list}])"));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors(vec![{list}])"));
    }
    if (p.gap - 2.0).abs() > f64::EPSILON {
        frags.push(format!(".with_gap({})", f64_lit(p.gap)));
    }
    if !p.show_percents {
        frags.push(".with_percents(false)".to_string());
    }
    if p.show_values {
        frags.push(".with_values(true)".to_string());
    }
    if (p.min_label_height - 18.0).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_min_label_height({})",
            f64_lit(p.min_label_height)
        ));
    }
    if !p.normalize {
        frags.push(".with_normalize(false)".to_string());
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("MosaicPlot::new()", frags)
}

pub fn emit_pareto_plot(p: &ParetoPlot) -> String {
    let mut frags: Vec<String> = Vec::new();
    if !p.categories.is_empty() {
        let cats = p
            .categories
            .iter()
            .map(|c| format!("({}, {})", str_lit(&c.label), f64_lit(c.value)))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_categories(vec![{cats}])"));
    }
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if p.line_color != "firebrick" {
        frags.push(format!(".with_line_color({})", str_lit(&p.line_color)));
    }
    if (p.width - 0.8).abs() > f64::EPSILON {
        frags.push(format!(".with_width({})", f64_lit(p.width)));
    }
    if !p.sorted {
        frags.push(".with_sorted(false)".to_string());
    }
    if p.show_cumulative_labels {
        frags.push(".with_cumulative_labels(true)".to_string());
    }
    if (p.threshold - 80.0).abs() > f64::EPSILON {
        frags.push(format!(".with_threshold({})", f64_lit(p.threshold)));
    }
    if !p.show_threshold {
        frags.push(".with_show_threshold(false)".to_string());
    }
    let default_bar_label = p.bar_legend_label.as_deref() == Some("Value");
    let default_line_label = p.line_legend_label.as_deref() == Some("Cumulative %");
    if !default_bar_label || !default_line_label {
        frags.push(format!(
            ".with_legend({}, {})",
            str_lit(p.bar_legend_label.as_deref().unwrap_or("")),
            str_lit(p.line_legend_label.as_deref().unwrap_or(""))
        ));
    }
    if !p.show_legend {
        frags.push(".with_show_legend(false)".to_string());
    }
    if let Some(max) = p.max_categories {
        frags.push(format!(".with_max_categories({max})"));
    }
    if p.other_label != "Other" {
        frags.push(format!(".with_other_label({})", str_lit(&p.other_label)));
    }
    if p.horizontal {
        frags.push(".with_horizontal(true)".to_string());
    }
    chain("ParetoPlot::new()", frags)
}

fn emit_qq_group(g: &QQGroup, mode: &QQMode) -> String {
    let values = g
        .data
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    match (mode, &g.color) {
        (QQMode::Genomic, Some(color)) => format!(
            ".with_pvalues_colored({}, vec![{}], {})",
            str_lit(&g.label),
            values,
            str_lit(color)
        ),
        (QQMode::Genomic, None) => {
            format!(".with_pvalues({}, vec![{}])", str_lit(&g.label), values)
        }
        (QQMode::Normal, Some(color)) => format!(
            ".with_data_colored({}, vec![{}], {})",
            str_lit(&g.label),
            values,
            str_lit(color)
        ),
        (QQMode::Normal, None) => format!(".with_data({}, vec![{}])", str_lit(&g.label), values),
    }
}

pub fn emit_qq_plot(p: &QQPlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(|g| emit_qq_group(g, &p.mode)).collect();
    // `with_pvalues`/`with_pvalues_colored` already switch the plot to genomic
    // mode as a side effect, so a group loop above is enough in the common
    // case. Only an empty-groups genomic plot (unreachable from the CLI, which
    // always has at least one row of data) needs an explicit call here.
    if p.groups.is_empty() && matches!(p.mode, QQMode::Genomic) {
        frags.push(".with_genomic()".to_string());
    }
    if !p.show_reference_line {
        frags.push(".without_reference_line()".to_string());
    }
    if p.show_ci_band {
        frags.push(".with_ci_band()".to_string());
        if (p.ci_alpha - 0.15).abs() > f64::EPSILON {
            frags.push(format!(".with_ci_alpha({})", f64_lit(p.ci_alpha)));
        }
    }
    if !p.show_lambda {
        frags.push(".without_lambda()".to_string());
    }
    if (p.marker_size - 3.0).abs() > f64::EPSILON {
        frags.push(format!(".with_marker_size({})", f64_lit(p.marker_size)));
    }
    if (p.stroke_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(op) = p.fill_opacity {
        frags.push(format!(".with_fill_opacity({})", f64_lit(op)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("QQPlot::new()", frags)
}

fn emit_bar_group_pairs(groups: &[BarGroup]) -> String {
    groups
        .iter()
        .map(|g| format!("({}, {})", str_lit(&g.label), f64_lit(g.bars[0].value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn emit_bar_colored_pairs(groups: &[BarGroup]) -> String {
    groups
        .iter()
        .map(|g| {
            format!(
                "({}, {}, {})",
                str_lit(&g.label),
                f64_lit(g.bars[0].value),
                str_lit(&g.bars[0].color)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn emit_bar_group_multi(g: &BarGroup) -> String {
    let values = g
        .bars
        .iter()
        .map(|b| format!("({}, {})", f64_lit(b.value), str_lit(&b.color)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(".with_group({}, vec![{}])", str_lit(&g.label), values)
}

/// Serialize a `BarPlot`'s *final* resolved state — works uniformly whether it
/// came from the simple, wide-`--y`, or `--color-by` CLI branch, since all
/// three converge on the same `BarPlot` struct.
pub fn emit_bar_plot(p: &BarPlot) -> String {
    let mut frags: Vec<String> = Vec::new();

    let all_single = !p.groups.is_empty() && p.groups.iter().all(|g| g.bars.len() == 1);
    if all_single {
        let colors: Vec<&str> = p.groups.iter().map(|g| g.bars[0].color.as_str()).collect();
        let uniform_color = colors.iter().all(|c| *c == colors[0]);
        if uniform_color {
            frags.push(format!(
                ".with_bars(vec![{}])",
                emit_bar_group_pairs(&p.groups)
            ));
            if colors[0] != "steelblue" {
                frags.push(format!(".with_color({})", str_lit(colors[0])));
            }
        } else {
            frags.push(format!(
                ".with_colored_bars(vec![{}])",
                emit_bar_colored_pairs(&p.groups)
            ));
        }
    } else {
        for g in &p.groups {
            frags.push(emit_bar_group_multi(g));
        }
        if let Some(ref labels) = p.legend_label {
            let list = labels
                .iter()
                .map(|l| str_lit(l))
                .collect::<Vec<_>>()
                .join(", ");
            frags.push(format!(".with_legend(vec![{list}])"));
        }
    }

    if p.stacked {
        frags.push(".with_stacked()".to_string());
    }
    if (p.width - 0.8).abs() > f64::EPSILON {
        frags.push(format!(".with_width({})", f64_lit(p.width)));
    }
    if p.horizontal {
        frags.push(".with_horizontal(true)".to_string());
    }
    if let Some(ref errors) = p.errors {
        let symmetric = errors.iter().all(|(lo, hi)| (lo - hi).abs() < f64::EPSILON);
        if symmetric {
            let list = errors
                .iter()
                .map(|(lo, _)| f64_lit(*lo))
                .collect::<Vec<_>>()
                .join(", ");
            frags.push(format!(".with_error(vec![{list}])"));
        } else {
            let list = errors
                .iter()
                .map(|(lo, hi)| format!("({}, {})", f64_lit(*lo), f64_lit(*hi)))
                .collect::<Vec<_>>()
                .join(", ");
            frags.push(format!(".with_asymmetric_error(vec![{list}])"));
        }
    }
    if let Some(ref c) = p.error_color {
        frags.push(format!(".with_error_color({})", str_lit(c)));
    }
    if (p.error_cap_width - 0.2).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_error_cap_width({})",
            f64_lit(p.error_cap_width)
        ));
    }

    chain("BarPlot::new()", frags)
}

fn emit_scatter_data(data: &[ScatterPoint]) -> String {
    data.iter()
        .map(|p| format!("({}, {})", f64_lit(p.x), f64_lit(p.y)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn marker_shape_ctor(m: MarkerShape) -> &'static str {
    match m {
        MarkerShape::Circle => "MarkerShape::Circle",
        MarkerShape::Square => "MarkerShape::Square",
        MarkerShape::Triangle => "MarkerShape::Triangle",
        MarkerShape::Diamond => "MarkerShape::Diamond",
        MarkerShape::Cross => "MarkerShape::Cross",
        MarkerShape::Plus => "MarkerShape::Plus",
    }
}

/// Serialize a single `ScatterPlot` series. Called once per element of the
/// CLI's `Vec<ScatterPlot>` (one series for the simple case, N for
/// `--color-by`/multi-`--y` mode) — `assemble()` wraps each result in
/// `Plot::Scatter(...)` and joins them into one `vec![...]`.
pub fn emit_scatter_plot(p: &ScatterPlot) -> String {
    let mut frags = vec![format!(".with_data(vec![{}])", emit_scatter_data(&p.data))];
    if p.color != "black" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if (p.size - 3.0).abs() > f64::EPSILON {
        frags.push(format!(".with_size({})", f64_lit(p.size)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if let Some(ref t) = p.trend {
        let ctor = match t {
            TrendLine::Linear => "TrendLine::Linear",
        };
        frags.push(format!(".with_trend({ctor})"));
    }
    if p.show_equation {
        frags.push(".with_equation()".to_string());
    }
    if p.show_correlation {
        frags.push(".with_correlation()".to_string());
    }
    if p.marker != MarkerShape::Circle {
        frags.push(format!(".with_marker({})", marker_shape_ctor(p.marker)));
    }
    chain("ScatterPlot::new()", frags)
}

// ── Per-plot-type emitters (Tier 2) ───────────────────────────────────────────

use kuva::plot::bump::{BumpSeries, BumpTieBreak, CurveStyle};
use kuva::plot::radar::{RadarReference, RadarSeries};
use kuva::plot::waterfall::{WaterfallBar, WaterfallKind};
use kuva::plot::{BumpPlot, ForestPlot, ForestRow, RadarPlot};
use kuva::plot::{SlopePlot, SlopePoint, SlopeValueFormat, WaterfallPlot};

fn emit_radar_series(s: &RadarSeries) -> Vec<String> {
    let values = s
        .values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags = match (&s.label, &s.color) {
        (Some(label), Some(color)) => vec![format!(
            ".with_series_color(vec![{values}], {}, {})",
            str_lit(label),
            str_lit(color)
        )],
        (Some(label), None) => vec![format!(
            ".with_series_labeled(vec![{values}], {})",
            str_lit(label)
        )],
        (None, _) => vec![format!(".with_series(vec![{values}])")],
    };
    if let Some(ref errors) = s.errors {
        let list = errors
            .iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_series_errors(vec![{list}])"));
    }
    if let Some(ref dash) = s.dasharray {
        frags.push(format!(".with_series_dasharray({})", str_lit(dash)));
    }
    frags
}

fn emit_radar_reference(r: &RadarReference) -> String {
    let values = r
        .values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    // `with_reference`/`with_reference_color` are the only public
    // constructors and both always set a label, so `label: None` is
    // unreachable from any CLI/library-builder path. Handled defensively by
    // falling back to an empty label rather than matching on label presence.
    match &r.color {
        Some(color) => format!(
            ".with_reference_color(vec![{values}], {}, {})",
            str_lit(r.label.as_deref().unwrap_or("")),
            str_lit(color)
        ),
        None => format!(
            ".with_reference(vec![{values}], {})",
            str_lit(r.label.as_deref().unwrap_or(""))
        ),
    }
}

/// Serialize a `RadarPlot`'s resolved state. Handles both the per-row and
/// `--color-by` (grouped-mean) CLI construction paths uniformly, since both
/// converge on the same `series`/`references` fields.
pub fn emit_radar_plot(p: &RadarPlot) -> String {
    let axes = p
        .axes
        .iter()
        .map(|a| str_lit(a))
        .collect::<Vec<_>>()
        .join(", ");
    let ctor = format!("RadarPlot::new(vec![{axes}])");

    let mut frags: Vec<String> = Vec::new();
    for s in &p.series {
        frags.extend(emit_radar_series(s));
    }
    frags.extend(p.references.iter().map(emit_radar_reference));

    if p.filled {
        frags.push(".with_filled(true)".to_string());
        if (p.opacity - 0.25).abs() > f64::EPSILON {
            frags.push(format!(".with_opacity({})", f64_lit(p.opacity)));
        }
    }
    if let Some((min, max)) = p.range {
        frags.push(format!(".with_range({}, {})", f64_lit(min), f64_lit(max)));
    }
    for (i, r) in p.axis_ranges.iter().enumerate() {
        if let Some((min, max)) = r {
            frags.push(format!(
                ".with_axis_range({i}, {}, {})",
                f64_lit(*min),
                f64_lit(*max)
            ));
        }
    }
    if p.inverted_axes.iter().any(|b| *b) {
        let idxs = p
            .inverted_axes
            .iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .map(|(i, _)| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_inverted_axes(vec![{idxs}])"));
    }
    if p.grid_lines != 5 {
        frags.push(format!(".with_grid_lines({})", p.grid_lines));
    }
    if !p.show_grid {
        frags.push(".with_grid(false)".to_string());
    }
    if p.circular_grid {
        frags.push(".with_circular_grid(true)".to_string());
    }
    if p.show_legend {
        frags.push(".with_legend(true)".to_string());
    }
    if let Some(size) = p.dot_size {
        frags.push(format!(".with_dot_size({})", f64_lit(size)));
    }
    if (p.stroke_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if p.normalize {
        frags.push(".with_normalize(true)".to_string());
    }
    if p.vertex_labels {
        frags.push(".with_vertex_labels(true)".to_string());
    }
    if (p.start_angle_deg - (-90.0)).abs() > f64::EPSILON {
        frags.push(format!(".with_start_angle({})", f64_lit(p.start_angle_deg)));
    }
    if p.axis_ticks {
        frags.push(".with_axis_ticks(true)".to_string());
    }

    chain(&ctor, frags)
}

fn emit_waterfall_bar(b: &WaterfallBar) -> String {
    match &b.kind {
        WaterfallKind::Delta => {
            format!(".with_delta({}, {})", str_lit(&b.label), f64_lit(b.value))
        }
        WaterfallKind::Total => format!(".with_total({})", str_lit(&b.label)),
        WaterfallKind::Difference { from, to } => format!(
            ".with_difference({}, {}, {})",
            str_lit(&b.label),
            f64_lit(*from),
            f64_lit(*to)
        ),
    }
}

pub fn emit_waterfall_plot(p: &WaterfallPlot) -> String {
    let mut frags: Vec<String> = p.bars.iter().map(emit_waterfall_bar).collect();
    if (p.bar_width - 0.6).abs() > f64::EPSILON {
        frags.push(format!(".with_bar_width({})", f64_lit(p.bar_width)));
    }
    if p.color_positive != "rgb(68,170,68)" {
        frags.push(format!(
            ".with_color_positive({})",
            str_lit(&p.color_positive)
        ));
    }
    if p.color_negative != "rgb(204,68,68)" {
        frags.push(format!(
            ".with_color_negative({})",
            str_lit(&p.color_negative)
        ));
    }
    if p.color_total != "steelblue" {
        frags.push(format!(".with_color_total({})", str_lit(&p.color_total)));
    }
    if p.show_connectors {
        frags.push(".with_connectors()".to_string());
    }
    if p.show_values {
        frags.push(".with_values()".to_string());
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    chain("WaterfallPlot::new()", frags)
}

fn emit_forest_row(r: &ForestRow) -> String {
    match (&r.weight, &r.color) {
        (Some(w), Some(c)) => format!(
            ".with_weighted_colored_row({}, {}, {}, {}, {}, {})",
            str_lit(&r.label),
            f64_lit(r.estimate),
            f64_lit(r.ci_lower),
            f64_lit(r.ci_upper),
            f64_lit(*w),
            str_lit(c)
        ),
        (Some(w), None) => format!(
            ".with_weighted_row({}, {}, {}, {}, {})",
            str_lit(&r.label),
            f64_lit(r.estimate),
            f64_lit(r.ci_lower),
            f64_lit(r.ci_upper),
            f64_lit(*w)
        ),
        (None, Some(c)) => format!(
            ".with_colored_row({}, {}, {}, {}, {})",
            str_lit(&r.label),
            f64_lit(r.estimate),
            f64_lit(r.ci_lower),
            f64_lit(r.ci_upper),
            str_lit(c)
        ),
        (None, None) => format!(
            ".with_row({}, {}, {}, {})",
            str_lit(&r.label),
            f64_lit(r.estimate),
            f64_lit(r.ci_lower),
            f64_lit(r.ci_upper)
        ),
    }
}

pub fn emit_forest_plot(p: &ForestPlot) -> String {
    let mut frags: Vec<String> = p.rows.iter().map(emit_forest_row).collect();
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if (p.marker_size - 6.0).abs() > f64::EPSILON {
        frags.push(format!(".with_marker_size({})", f64_lit(p.marker_size)));
    }
    if (p.whisker_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_whisker_width({})", f64_lit(p.whisker_width)));
    }
    if let Some(v) = p.null_value {
        if v != 0.0 {
            frags.push(format!(".with_null_value({})", f64_lit(v)));
        }
    }
    if !p.show_null_line {
        frags.push(".with_show_null_line(false)".to_string());
    }
    if p.cap_size != 0.0 {
        frags.push(format!(".with_cap_size({})", f64_lit(p.cap_size)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("ForestPlot::new()", frags)
}

fn slope_value_format_ctor(fmt: &SlopeValueFormat) -> String {
    match fmt {
        SlopeValueFormat::Auto => "SlopeValueFormat::Auto".to_string(),
        SlopeValueFormat::Fixed(n) => format!("SlopeValueFormat::Fixed({n})"),
        SlopeValueFormat::Integer => "SlopeValueFormat::Integer".to_string(),
    }
}

pub fn emit_slope_plot(p: &SlopePlot) -> String {
    let points = p
        .points
        .iter()
        .map(|pt: &SlopePoint| {
            format!(
                "({}, {}, {})",
                str_lit(&pt.label),
                f64_lit(pt.before),
                f64_lit(pt.after)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags: Vec<String> = vec![format!(".with_points(vec![{points}])")];

    if let Some(ref l) = p.before_label {
        frags.push(format!(".with_before_label({})", str_lit(l)));
    }
    if let Some(ref l) = p.after_label {
        frags.push(format!(".with_after_label({})", str_lit(l)));
    }
    if p.color_up != "#2ca02c" {
        frags.push(format!(".with_color_up({})", str_lit(&p.color_up)));
    }
    if p.color_down != "#d62728" {
        frags.push(format!(".with_color_down({})", str_lit(&p.color_down)));
    }
    if p.color_flat != "#aaaaaa" {
        frags.push(format!(".with_color_flat({})", str_lit(&p.color_flat)));
    }
    if !p.color_by_direction {
        frags.push(".with_direction_colors(false)".to_string());
    }
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors(vec![{list}])"));
    }
    if (p.dot_radius - 6.0).abs() > f64::EPSILON {
        frags.push(format!(".with_dot_radius({})", f64_lit(p.dot_radius)));
    }
    if (p.line_width - 2.5).abs() > f64::EPSILON {
        frags.push(format!(".with_line_width({})", f64_lit(p.line_width)));
    }
    if (p.dot_opacity - 1.0).abs() > f64::EPSILON {
        frags.push(format!(".with_dot_opacity({})", f64_lit(p.dot_opacity)));
    }
    if (p.line_opacity - 0.7).abs() > f64::EPSILON {
        frags.push(format!(".with_line_opacity({})", f64_lit(p.line_opacity)));
    }
    if p.show_values {
        frags.push(".with_values(true)".to_string());
    }
    if !matches!(p.value_format, SlopeValueFormat::Auto) {
        frags.push(format!(
            ".with_value_format({})",
            slope_value_format_ctor(&p.value_format)
        ));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("SlopePlot::new()", frags)
}

fn curve_style_ctor(style: &CurveStyle) -> &'static str {
    match style {
        CurveStyle::Sigmoid => "CurveStyle::Sigmoid",
        CurveStyle::Straight => "CurveStyle::Straight",
    }
}

fn emit_opt_f64_vec(values: &[Option<f64>]) -> String {
    values
        .iter()
        .map(|r| match r {
            Some(v) => format!("Some({})", f64_lit(*v)),
            None => "None".to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn emit_bump_series(s: &BumpSeries) -> String {
    // `BumpSeries::color` has no public setter reachable from `with_series`/
    // `with_ranked_series` (always `None`), so it's never emitted here.
    format!(
        ".with_ranked_series({}, vec![{}])",
        str_lit(&s.name),
        emit_opt_f64_vec(&s.ranks)
    )
}

fn tie_break_ctor(mode: &BumpTieBreak) -> &'static str {
    match mode {
        BumpTieBreak::Average => "BumpTieBreak::Average",
        BumpTieBreak::Min => "BumpTieBreak::Min",
        BumpTieBreak::Max => "BumpTieBreak::Max",
        BumpTieBreak::Stable => "BumpTieBreak::Stable",
    }
}

fn emit_bump_raw_series(
    (name, values, _color): &(String, Vec<Option<f64>>, Option<String>),
) -> String {
    // `color` (the tuple's 3rd field) has no public setter reachable from
    // `with_raw_series`/`with_raw_series_opt` (always `None`), so it's never
    // emitted here — same reasoning as `BumpSeries::color` above.
    format!(
        ".with_raw_series_opt({}, vec![{}])",
        str_lit(name),
        emit_opt_f64_vec(values)
    )
}

/// Serialize a `BumpPlot`'s resolved series — both the pre-ranked path
/// (`series`) and the raw-value auto-ranking path (`raw_values`) are read
/// directly off the struct, so this works uniformly regardless of which
/// `bump.rs` CLI branch (`--raw-value` or not) built the plot.
pub fn emit_bump_plot(p: &BumpPlot) -> String {
    let mut frags: Vec<String> = p.series.iter().map(emit_bump_series).collect();
    frags.extend(p.raw_values.iter().map(emit_bump_raw_series));
    // `rank_ascending`/`tie_break` only affect raw-value resolution — no-ops
    // when `raw_values` is empty, so only emit them alongside raw series.
    if !p.raw_values.is_empty() {
        if p.rank_ascending {
            frags.push(".with_rank_ascending(true)".to_string());
        }
        if !matches!(p.tie_break, BumpTieBreak::Average) {
            frags.push(format!(".with_tie_break({})", tie_break_ctor(&p.tie_break)));
        }
    }
    if !p.x_labels.is_empty() {
        let list = p
            .x_labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_x_labels(vec![{list}])"));
    }
    if !matches!(p.curve_style, CurveStyle::Sigmoid) {
        frags.push(format!(
            ".with_curve_style({})",
            curve_style_ctor(&p.curve_style)
        ));
    }
    if p.show_rank_labels {
        frags.push(".with_show_rank_labels(true)".to_string());
    }
    if !p.show_series_labels {
        frags.push(".with_show_series_labels(false)".to_string());
    }
    if (p.dot_radius - 6.0).abs() > f64::EPSILON {
        frags.push(format!(".with_dot_radius({})", f64_lit(p.dot_radius)));
    }
    if (p.stroke_width - 2.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if let Some(ref hl) = p.highlight {
        frags.push(format!(".with_highlight({})", str_lit(hl)));
    }
    if !p.legend {
        frags.push(".with_legend(false)".to_string());
    }
    chain("BumpPlot::new()", frags)
}

// ── Per-plot-type emitters (Tier 2) ───────────────────────────────────────────

/// Serialize a single `LinePlot` series. Called once per element of the
/// CLI's `Vec<LinePlot>` (one series for the simple case, N for
/// `--color-by`/multi-`--y` mode) — `assemble()` wraps each result in
/// `Plot::Line(...)` and joins them into one `vec![...]`.
///
/// `LinePlot::data`'s per-point `x_err`/`y_err` fields and the nested `band`
/// (a whole separate `BandPlot`) are never set by any branch in `line.rs`, so
/// they're intentionally not serialized here — the same simplification
/// `emit_scatter_data` already makes for `ScatterPlot`'s identically-shaped
/// error-bar fields.
pub fn emit_line_plot(p: &LinePlot) -> String {
    let data = p
        .data
        .iter()
        .map(|pt| format!("({}, {})", f64_lit(pt.x), f64_lit(pt.y)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags = vec![format!(".with_data(vec![{data}])")];
    if p.color != "black" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if (p.stroke_width - 2.0).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    match &p.line_style {
        LineStyle::Solid => {}
        LineStyle::Dashed => frags.push(".with_dashed()".to_string()),
        LineStyle::Dotted => frags.push(".with_dotted()".to_string()),
        LineStyle::DashDot => frags.push(".with_dashdot()".to_string()),
        LineStyle::Custom(s) => frags.push(format!(
            ".with_line_style(LineStyle::Custom({}.to_string()))",
            str_lit(s)
        )),
    }
    if p.step {
        frags.push(".with_step()".to_string());
    }
    if p.fill {
        frags.push(".with_fill()".to_string());
        if (p.fill_opacity - 0.3).abs() > f64::EPSILON {
            frags.push(format!(".with_fill_opacity({})", f64_lit(p.fill_opacity)));
        }
    }
    chain("LinePlot::new()", frags)
}

/// Serialize a single `Histogram`. Called once per element of the CLI's
/// `Vec<Histogram>` (one histogram for the single-column case, N for
/// multi-`--y` overlay mode) — `assemble()` wraps each result in
/// `Plot::Histogram(...)` and joins them into one `vec![...]`.
///
/// Handles both the raw-samples path (`with_data`/`with_bins`/`with_range`,
/// the only path `histogram.rs` ever takes) and the precomputed-bins path
/// (`with_precomputed`) for completeness, since `Histogram::precomputed` is a
/// plain `(Vec<f64>, Vec<f64>)` — no dedicated serializer needed.
pub fn emit_histogram_plot(p: &Histogram) -> String {
    let mut frags: Vec<String> = Vec::new();
    if let Some((ref edges, ref counts)) = p.precomputed {
        let e = edges
            .iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
        let c = counts
            .iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_precomputed(vec![{e}], vec![{c}])"));
    } else {
        let data = p
            .data
            .iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_data(vec![{data}])"));
        if p.bins != 10 {
            frags.push(format!(".with_bins({})", p.bins));
        }
        if let Some((min, max)) = p.range {
            frags.push(format!(".with_range(({}, {}))", f64_lit(min), f64_lit(max)));
        }
    }
    if p.color != "black" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if p.normalize {
        frags.push(".with_normalize()".to_string());
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    if p.show_kde {
        frags.push(".with_kde(true)".to_string());
        if let Some(ref color) = p.kde_color {
            frags.push(format!(".with_kde_color({})", str_lit(color)));
        }
        if let Some(bw) = p.kde_bandwidth {
            frags.push(format!(".with_kde_bandwidth({})", f64_lit(bw)));
        }
        if p.kde_samples != 200 {
            frags.push(format!(".with_kde_samples({})", p.kde_samples));
        }
    }
    chain("Histogram::new()", frags)
}

fn emit_violin_group(g: &ViolinGroup) -> String {
    let values = g
        .values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    format!(".with_group({}, vec![{}])", str_lit(&g.label), values)
}

/// Serialize a `ViolinPlot`'s *final* resolved state — works uniformly
/// whether it came from the `--group-col` or multi-`--y` CLI branch in
/// `violin.rs`, since both converge on the same `ViolinPlot` struct.
///
/// Split-violin fields (`split`/`split_groups`/`split_color`/
/// `split_group_colors`/`split_legend_label`, added this session) are never
/// touched here: `violin.rs` exposes no `--split` flag, so `split` is always
/// `false` and `split_groups` always empty for any CLI-built `ViolinPlot`.
pub fn emit_violin_plot(p: &ViolinPlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(emit_violin_group).collect();
    if p.color != "black" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors([{list}])"));
    }
    if (p.width - 0.8).abs() > f64::EPSILON {
        frags.push(format!(".with_width({})", f64_lit(p.width)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if let Some(h) = p.bandwidth {
        frags.push(format!(".with_bandwidth({})", f64_lit(h)));
    }
    if p.kde_samples != 200 {
        frags.push(format!(".with_kde_samples({})", p.kde_samples));
    }
    match &p.overlay {
        Some(StripStyle::Strip { jitter }) => {
            frags.push(format!(".with_strip({})", f64_lit(*jitter)))
        }
        Some(StripStyle::Swarm) => frags.push(".with_swarm_overlay()".to_string()),
        // `Center` has no dedicated `ViolinPlot` builder (not reachable from
        // any CLI path today) and `None` needs no fragment.
        Some(StripStyle::Center) | None => {}
    }
    if p.overlay_color != "rgba(0,0,0,0.45)" {
        frags.push(format!(
            ".with_overlay_color({})",
            str_lit(&p.overlay_color)
        ));
    }
    if (p.overlay_size - 3.0).abs() > f64::EPSILON {
        frags.push(format!(".with_overlay_size({})", f64_lit(p.overlay_size)));
    }
    if p.horizontal {
        frags.push(".with_horizontal(true)".to_string());
    }
    chain("ViolinPlot::new()", frags)
}

fn emit_roc_group(g: &RocGroup) -> String {
    let mut inner: Vec<String> = Vec::new();
    if let Some(ref raw) = g.raw_predictions {
        let pairs = raw
            .iter()
            .map(|&(s, l)| format!("({}, {l})", f64_lit(s)))
            .collect::<Vec<_>>()
            .join(", ");
        inner.push(format!(".with_raw(vec![{pairs}])"));
    } else if let Some(ref pts) = g.precomputed_points {
        let pairs = pts
            .iter()
            .map(|&(f, t)| format!("({}, {})", f64_lit(f), f64_lit(t)))
            .collect::<Vec<_>>()
            .join(", ");
        inner.push(format!(".with_points(vec![{pairs}])"));
    }
    if let Some(ref color) = g.color {
        inner.push(format!(".with_color({})", str_lit(color)));
    }
    if g.show_ci {
        inner.push(".with_ci(true)".to_string());
        if (g.ci_alpha - 0.15).abs() > f64::EPSILON {
            inner.push(format!(".with_ci_alpha({})", f64_lit(g.ci_alpha)));
        }
    }
    if let Some((lo, hi)) = g.pauc_range {
        inner.push(format!(".with_pauc({}, {})", f64_lit(lo), f64_lit(hi)));
    }
    if g.show_optimal_point {
        inner.push(".with_optimal_point()".to_string());
    }
    if !g.show_auc_label {
        inner.push(".with_auc_label(false)".to_string());
    }
    if (g.line_width - 2.0).abs() > f64::EPSILON {
        inner.push(format!(".with_line_width({})", f64_lit(g.line_width)));
    }
    if let Some(ref d) = g.dasharray {
        inner.push(format!(".with_dasharray({})", str_lit(d)));
    }
    chain(&format!("RocGroup::new({})", str_lit(&g.label)), inner)
}

/// Serialize a `RocPlot`'s *final* resolved state — one `RocGroup` per curve,
/// however many the CLI's `--color-by` loop (or the single-classifier
/// fallback) built. `RocPlot::diagonal_color`/`diagonal_dasharray` have no
/// dedicated builder methods (fixed defaults, not customizable), so they're
/// never emitted. `RocGroup::precomputed_points` is likewise unreachable from
/// `roc.rs` (only `with_raw` is ever called) but is still handled here for
/// correctness against the real struct shape.
pub fn emit_roc_plot(p: &RocPlot) -> String {
    let mut frags: Vec<String> = p
        .groups
        .iter()
        .map(|g| format!(".with_group({})", emit_roc_group(g)))
        .collect();
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if !p.show_diagonal {
        frags.push(".with_diagonal(false)".to_string());
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("RocPlot::new()", frags)
}

fn emit_km_group(g: &KMGroup) -> String {
    let times = g
        .times
        .iter()
        .map(|t| f64_lit(*t))
        .collect::<Vec<_>>()
        .join(", ");
    let events = g
        .events
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    match &g.color {
        Some(color) => format!(
            ".with_colored_group({}, vec![{}], vec![{}], {})",
            str_lit(&g.label),
            times,
            events,
            str_lit(color)
        ),
        None => format!(
            ".with_group({}, vec![{}], vec![{}])",
            str_lit(&g.label),
            times,
            events
        ),
    }
}

/// Serialize a `SurvivalPlot`'s *final* resolved state — one `KMGroup` per
/// curve, whether `survival.rs` built it via the `--group-col` loop or the
/// single ungrouped "All" fallback (both always call `with_colored_group`,
/// so `color` is always `Some` in practice, but the `None` arm in
/// `emit_km_group` is kept for correctness against the real struct shape).
pub fn emit_survival_plot(p: &SurvivalPlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(emit_km_group).collect();
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors([{list}])"));
    }
    if (p.line_width - 2.0).abs() > f64::EPSILON {
        frags.push(format!(".with_line_width({})", f64_lit(p.line_width)));
    }
    if p.show_ci {
        frags.push(".with_ci(true)".to_string());
        if (p.ci_alpha - 0.2).abs() > f64::EPSILON {
            frags.push(format!(".with_ci_alpha({})", f64_lit(p.ci_alpha)));
        }
    }
    if !p.show_censoring {
        frags.push(".with_censoring(false)".to_string());
    }
    if (p.censoring_size - 4.0).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_censoring_size({})",
            f64_lit(p.censoring_size)
        ));
    }
    if let Some(ref text) = p.pvalue_text {
        frags.push(format!(".with_pvalue_text({})", str_lit(text)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("SurvivalPlot::new()", frags)
}

// ── Per-plot-type emitters (Tier 2 continued: calendar/ternary/hexbin/density/upset) ──

use kuva::plot::{
    CalendarAgg, CalendarPlot, DensityPlot, HexbinPlot, TernaryPlot, TernaryPoint, UpSetPlot,
    UpSetSort, WeekStart, ZReduce,
};

fn calendar_agg_ctor(agg: &CalendarAgg) -> &'static str {
    match agg {
        CalendarAgg::Count => "CalendarAgg::Count",
        CalendarAgg::Sum => "CalendarAgg::Sum",
        CalendarAgg::Mean => "CalendarAgg::Mean",
        CalendarAgg::Max => "CalendarAgg::Max",
    }
}

fn week_start_ctor(ws: &WeekStart) -> &'static str {
    match ws {
        WeekStart::Monday => "WeekStart::Monday",
        WeekStart::Sunday => "WeekStart::Sunday",
    }
}

/// Serialize a `CalendarPlot`'s resolved state. `color_map` is never emitted:
/// `calendar.rs` has no flag that changes it, and the default itself is a
/// `ColorMap::Custom` closure (the GitHub-style green gradient) that can't be
/// represented as a Rust literal regardless.
pub fn emit_calendar_plot(p: &CalendarPlot) -> String {
    let data = p
        .data
        .iter()
        .map(|(d, v)| format!("({}, {})", str_lit(d), f64_lit(*v)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags = vec![format!(".with_data(vec![{data}])")];

    if !matches!(p.aggregation, CalendarAgg::Count) {
        frags.push(format!(
            ".with_aggregation({})",
            calendar_agg_ctor(&p.aggregation)
        ));
    }
    if p.missing_color != "#ebedf0" {
        frags.push(format!(
            ".with_missing_color({})",
            str_lit(&p.missing_color)
        ));
    }
    if let Some(ref c) = p.zero_color {
        frags.push(format!(".with_zero_color({})", str_lit(c)));
    }
    if !matches!(p.week_start, WeekStart::Monday) {
        frags.push(format!(
            ".with_week_start({})",
            week_start_ctor(&p.week_start)
        ));
    }
    if !p.show_month_labels {
        frags.push(".with_month_labels(false)".to_string());
    }
    if !p.show_day_labels {
        frags.push(".with_day_labels(false)".to_string());
    }
    if (p.cell_size - 13.0).abs() > f64::EPSILON {
        frags.push(format!(".with_cell_size({})", f64_lit(p.cell_size)));
    }
    if (p.cell_gap - 2.0).abs() > f64::EPSILON {
        frags.push(format!(".with_cell_gap({})", f64_lit(p.cell_gap)));
    }
    // `periods` (set by `with_date_range`/`with_period`/`with_periods`) takes
    // priority over `years` when resolving display rows, so mirror that here:
    // only fall back to `years` when no explicit periods are set.
    if let Some(ref periods) = p.periods {
        let list = periods
            .iter()
            .map(|per| {
                format!(
                    "({}, {}, {})",
                    str_lit(&per.label),
                    str_lit(&per.start),
                    str_lit(&per.end)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_periods(vec![{list}])"));
    } else if let Some(ref years) = p.years {
        if years.len() == 1 {
            frags.push(format!(".with_year({})", years[0]));
        } else {
            let list = years
                .iter()
                .map(|y| y.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            frags.push(format!(".with_years(vec![{list}])"));
        }
    }
    if !p.show_legend {
        frags.push(".with_legend(false)".to_string());
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend_label({})", str_lit(label)));
    }
    if let Some((min, max)) = p.value_range {
        frags.push(format!(
            ".with_value_range({}, {})",
            f64_lit(min),
            f64_lit(max)
        ));
    }
    chain("CalendarPlot::new()", frags)
}

fn emit_ternary_point(pt: &TernaryPoint) -> String {
    match &pt.group {
        Some(g) => format!(
            ".with_point_group({}, {}, {}, {})",
            f64_lit(pt.a),
            f64_lit(pt.b),
            f64_lit(pt.c),
            str_lit(g)
        ),
        None => format!(
            ".with_point({}, {}, {})",
            f64_lit(pt.a),
            f64_lit(pt.b),
            f64_lit(pt.c)
        ),
    }
}

/// Serialize a `TernaryPlot`'s resolved state — handles both the plain
/// (`--color-by` absent) and grouped (`--color-by` present) construction
/// paths in `ternary.rs` uniformly, since both just push `TernaryPoint`s with
/// or without a `group` label.
pub fn emit_ternary_plot(p: &TernaryPlot) -> String {
    let mut frags: Vec<String> = p.points.iter().map(emit_ternary_point).collect();
    let default_labels = ["A".to_string(), "B".to_string(), "C".to_string()];
    if p.corner_labels != default_labels {
        frags.push(format!(
            ".with_corner_labels({}, {}, {})",
            str_lit(&p.corner_labels[0]),
            str_lit(&p.corner_labels[1]),
            str_lit(&p.corner_labels[2])
        ));
    }
    if p.normalize {
        frags.push(".with_normalize(true)".to_string());
    }
    if (p.marker_size - 5.0).abs() > f64::EPSILON {
        frags.push(format!(".with_marker_size({})", f64_lit(p.marker_size)));
    }
    if p.grid_lines != 5 {
        frags.push(format!(".with_grid_lines({})", p.grid_lines));
    }
    if !p.show_grid {
        frags.push(".with_grid(false)".to_string());
    }
    if p.show_legend {
        frags.push(".with_legend(true)".to_string());
    }
    if !p.show_percentages {
        frags.push(".with_percentages(false)".to_string());
    }
    if let Some(op) = p.marker_opacity {
        frags.push(format!(".with_marker_opacity({})", f64_lit(op)));
    }
    if let Some(w) = p.marker_stroke_width {
        frags.push(format!(".with_marker_stroke_width({})", f64_lit(w)));
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    chain("TernaryPlot::new()", frags)
}

fn z_reduce_ctor(r: &ZReduce) -> &'static str {
    match r {
        ZReduce::Count => "ZReduce::Count",
        ZReduce::Mean => "ZReduce::Mean",
        ZReduce::Sum => "ZReduce::Sum",
        ZReduce::Median => "ZReduce::Median",
        ZReduce::Min => "ZReduce::Min",
        ZReduce::Max => "ZReduce::Max",
    }
}

/// Serialize a `HexbinPlot`'s resolved state. Data is raw (x, y) scatter
/// points fed straight into `with_data` — `hexbin.rs` never pre-bins, so
/// there's no matrix-shaped state to worry about here.
pub fn emit_hexbin_plot(p: &HexbinPlot) -> String {
    let x =
        p.x.iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
    let y =
        p.y.iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
    let mut frags = vec![format!(".with_data(vec![{x}], vec![{y}])")];

    if let Some(ref z) = p.z {
        let zs = z.iter().map(|v| f64_lit(*v)).collect::<Vec<_>>().join(", ");
        frags.push(format!(
            ".with_z(vec![{zs}], {})",
            z_reduce_ctor(&p.z_reduce)
        ));
    }
    if p.n_bins != 20 {
        frags.push(format!(".with_n_bins({})", p.n_bins));
    }
    if let Some(s) = p.bin_size {
        frags.push(format!(".with_bin_size({})", f64_lit(s)));
    }
    if !matches!(p.color_map, ColorMap::Viridis) {
        frags.push(format!(".with_color_map({})", color_map_ctor(&p.color_map)));
    }
    if p.log_color {
        frags.push(".with_log_color(true)".to_string());
    }
    if p.min_count != 1 {
        frags.push(format!(".with_min_count({})", p.min_count));
    }
    if p.normalize {
        frags.push(".with_normalize(true)".to_string());
    }
    if !p.show_colorbar {
        frags.push(".with_colorbar(false)".to_string());
    }
    if let Some(ref label) = p.colorbar_label {
        frags.push(format!(".with_colorbar_label({})", str_lit(label)));
    }
    if let Some(ref stroke) = p.stroke_color {
        frags.push(format!(".with_stroke({})", str_lit(stroke)));
    }
    if (p.stroke_width - 0.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if p.flat_top {
        frags.push(".with_flat_top(true)".to_string());
    }
    if let Some((lo, hi)) = p.x_range {
        frags.push(format!(".with_x_range({}, {})", f64_lit(lo), f64_lit(hi)));
    }
    if let Some((lo, hi)) = p.y_range {
        frags.push(format!(".with_y_range({}, {})", f64_lit(lo), f64_lit(hi)));
    }
    if let Some((lo, hi)) = p.color_range {
        frags.push(format!(
            ".with_color_range({}, {})",
            f64_lit(lo),
            f64_lit(hi)
        ));
    }
    chain("HexbinPlot::new()", frags)
}

/// Serialize a single `DensityPlot`. Called once per element of the CLI's
/// `Vec<Plot>` (one curve for the simple case, N for `--color-by`/multi-`--y`
/// overlay mode in `density.rs`) — the integration sites filter each `Plot`
/// down to its `DensityPlot` and pass the resulting `Vec<String>` to
/// `assemble()`.
///
/// `x_lo`/`x_hi` are read independently off the final struct rather than
/// re-deriving which of the two CLI branches set them — `density.rs`'s
/// single-curve branch only calls `with_x_range` when *both* `--x-min` and
/// `--x-max` are given, while the multi-series branches call `with_x_lo`/
/// `with_x_hi` independently, but both converge on the same two `Option<f64>`
/// fields.
pub fn emit_density_plot(p: &DensityPlot) -> String {
    let mut frags: Vec<String> = Vec::new();
    let ctor = if let Some((ref x, ref y)) = p.precomputed {
        let xs = x.iter().map(|v| f64_lit(*v)).collect::<Vec<_>>().join(", ");
        let ys = y.iter().map(|v| f64_lit(*v)).collect::<Vec<_>>().join(", ");
        format!("DensityPlot::from_curve(vec![{xs}], vec![{ys}])")
    } else {
        let data = p
            .data
            .iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_data(vec![{data}])"));
        "DensityPlot::new()".to_string()
    };

    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if p.filled {
        frags.push(".with_filled(true)".to_string());
        if (p.opacity - 0.2).abs() > f64::EPSILON {
            frags.push(format!(".with_opacity({})", f64_lit(p.opacity)));
        }
    }
    if let Some(bw) = p.bandwidth {
        frags.push(format!(".with_bandwidth({})", f64_lit(bw)));
    }
    if p.kde_samples != 200 {
        frags.push(format!(".with_kde_samples({})", p.kde_samples));
    }
    if (p.stroke_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if let Some(ref dash) = p.line_dash {
        frags.push(format!(".with_line_dash({})", str_lit(dash)));
    }
    match (p.x_lo, p.x_hi) {
        (Some(lo), Some(hi)) => {
            frags.push(format!(".with_x_range({}, {})", f64_lit(lo), f64_lit(hi)))
        }
        (Some(lo), None) => frags.push(format!(".with_x_lo({})", f64_lit(lo))),
        (None, Some(hi)) => frags.push(format!(".with_x_hi({})", f64_lit(hi))),
        (None, None) => {}
    }
    if p.fit_y {
        frags.push(".with_fit()".to_string());
    }
    chain(&ctor, frags)
}

fn upset_sort_ctor(s: &UpSetSort) -> &'static str {
    match s {
        UpSetSort::ByFrequency => "UpSetSort::ByFrequency",
        UpSetSort::ByDegree => "UpSetSort::ByDegree",
        UpSetSort::Natural => "UpSetSort::Natural",
    }
}

/// Serialize an `UpSetPlot`'s resolved state. `upset.rs` always calls
/// `with_data` (it computes intersection masks itself from binary
/// set-membership columns; it never calls `with_sets` with raw elements), so
/// the precomputed form is the only one ever reachable from the CLI —
/// emitted here unconditionally. `show_counts`/`dot_empty_color` have no
/// public setter reachable from any builder, so a CLI-built plot can never
/// differ from their defaults; they're intentionally not serialized.
pub fn emit_upset_plot(p: &UpSetPlot) -> String {
    let names = p
        .set_names
        .iter()
        .map(|s| str_lit(s))
        .collect::<Vec<_>>()
        .join(", ");
    let sizes = p
        .set_sizes
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let inter = p
        .intersections
        .iter()
        .map(|i| format!("({}, {})", i.mask, i.count))
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags = vec![format!(
        ".with_data(vec![{names}], vec![{sizes}], vec![{inter}])"
    )];

    if !matches!(p.sort, UpSetSort::ByFrequency) {
        frags.push(format!(".with_sort({})", upset_sort_ctor(&p.sort)));
    }
    if let Some(n) = p.max_visible {
        frags.push(format!(".with_max_visible({n})"));
    }
    if !p.show_set_sizes {
        frags.push(".without_set_sizes()".to_string());
    }
    if p.bar_color != "#333333" {
        frags.push(format!(".with_bar_color({})", str_lit(&p.bar_color)));
    }
    if p.dot_color != "#333333" {
        frags.push(format!(".with_dot_color({})", str_lit(&p.dot_color)));
    }
    chain("UpSetPlot::new()", frags)
}

// ── Per-plot-type emitters (Tier 2: parallel/ridgeline/stacked_area/streamgraph/raincloud) ──

use kuva::plot::streamgraph::{StreamBaseline, StreamOrder};
use kuva::plot::{
    ParallelPlot, ParallelRow, RaincloudGroup, RaincloudPlot, RidgelineGroup, RidgelinePlot,
    StackedAreaPlot, StreamgraphPlot,
};

fn emit_parallel_row(r: &ParallelRow) -> String {
    let values = r
        .values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    match &r.group {
        Some(g) => format!(".with_row_group({}, vec![{}])", str_lit(g), values),
        None => format!(".with_row(vec![{values}])"),
    }
}

/// Serialize a `ParallelPlot`'s resolved state — works uniformly whether
/// `parallel.rs` built it via the ungrouped (`with_row`) or `--group-col`
/// (`with_row_group`) branch, since both converge on the same `rows` field.
pub fn emit_parallel_plot(p: &ParallelPlot) -> String {
    let axis_names = p
        .axis_names
        .iter()
        .map(|n| str_lit(n))
        .collect::<Vec<_>>()
        .join(", ");
    let ctor = format!("ParallelPlot::new().with_axis_names(vec![{axis_names}])");

    let mut frags: Vec<String> = p.rows.iter().map(emit_parallel_row).collect();

    if !p.normalize {
        frags.push(".with_normalize(false)".to_string());
    }
    if p.curved {
        frags.push(".with_curved(true)".to_string());
    }
    if (p.stroke_width - 1.2).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if (p.opacity - 0.6).abs() > f64::EPSILON {
        frags.push(format!(".with_opacity({})", f64_lit(p.opacity)));
    }
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors(vec![{list}])"));
    }
    if !p.show_axis_ticks {
        frags.push(".with_axis_ticks(false)".to_string());
    }
    if p.axis_ticks != 5 {
        frags.push(format!(".with_tick_count({})", p.axis_ticks));
    }
    if p.show_mean {
        frags.push(".with_mean(true)".to_string());
    }
    if (p.mean_stroke_width - 3.0).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_mean_stroke_width({})",
            f64_lit(p.mean_stroke_width)
        ));
    }
    if p.inverted_axes.iter().any(|b| *b) {
        let idxs = p
            .inverted_axes
            .iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .map(|(i, _)| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_inverted_axes(vec![{idxs}])"));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if p.show_axis_bands {
        frags.push(".with_axis_bands(true)".to_string());
    }
    chain(&ctor, frags)
}

fn emit_ridgeline_group(g: &RidgelineGroup) -> String {
    let values = g
        .values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    match &g.color {
        Some(c) => format!(
            ".with_group_color({}, vec![{}], {})",
            str_lit(&g.label),
            values,
            str_lit(c)
        ),
        None => format!(".with_group({}, vec![{}])", str_lit(&g.label), values),
    }
}

/// Serialize a `RidgelinePlot`'s resolved state — works uniformly across
/// `ridgeline.rs`'s three CLI branches (`--group-by`, multi-`--y`, and the
/// single-column fallback that uses group label `"data"`), since all three
/// converge on the same `groups` field.
pub fn emit_ridgeline_plot(p: &RidgelinePlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(emit_ridgeline_group).collect();
    if !p.filled {
        frags.push(".with_filled(false)".to_string());
    }
    if (p.opacity - 0.7).abs() > f64::EPSILON {
        frags.push(format!(".with_opacity({})", f64_lit(p.opacity)));
    }
    if let Some(bw) = p.bandwidth {
        frags.push(format!(".with_bandwidth({})", f64_lit(bw)));
    }
    if p.kde_samples != 200 {
        frags.push(format!(".with_kde_samples({})", p.kde_samples));
    }
    if (p.stroke_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if (p.overlap - 0.5).abs() > f64::EPSILON {
        frags.push(format!(".with_overlap({})", f64_lit(p.overlap)));
    }
    if p.normalize {
        frags.push(".with_normalize(true)".to_string());
    }
    if p.show_legend {
        frags.push(".with_legend(true)".to_string());
    }
    if let Some(ref dash) = p.line_dash {
        frags.push(format!(".with_line_dash({})", str_lit(dash)));
    }
    if !p.show_baseline {
        frags.push(".with_baseline(false)".to_string());
    }
    chain("RidgelinePlot::new()", frags)
}

/// Serialize one series of a `StackedAreaPlot`/`StreamgraphPlot`-shaped
/// builder: `.with_series(...)` followed immediately by the optional
/// `.with_color(...)`/label call that mutate the just-pushed entry — mirrors
/// the "call `with_series` then chain `with_color`/`with_legend`" pattern
/// both plot types share.
fn emit_area_series(
    values: &[f64],
    color: &Option<String>,
    label: &Option<String>,
    label_method: &str,
) -> Vec<String> {
    let vals = values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags = vec![format!(".with_series(vec![{vals}])")];
    if let Some(c) = color {
        frags.push(format!(".with_color({})", str_lit(c)));
    }
    if let Some(l) = label {
        frags.push(format!("{label_method}({})", str_lit(l)));
    }
    frags
}

/// Serialize a `StackedAreaPlot`'s resolved state — `stacked_area.rs` only
/// ever builds it via one `--group-col` loop (`with_x` once, then
/// `with_series`/`with_color`/`with_legend` per group), but this reads the
/// struct's own parallel `series`/`colors`/`labels` vectors directly so it
/// stays correct even if another construction path is added later.
pub fn emit_stacked_area_plot(p: &StackedAreaPlot) -> String {
    let xs =
        p.x.iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
    let mut frags: Vec<String> = vec![format!(".with_x(vec![{xs}])")];
    for (i, s) in p.series.iter().enumerate() {
        let color = p.colors.get(i).cloned().flatten();
        let label = p.labels.get(i).cloned().flatten();
        frags.extend(emit_area_series(s, &color, &label, ".with_legend"));
    }
    if (p.fill_opacity - 0.7).abs() > f64::EPSILON {
        frags.push(format!(".with_fill_opacity({})", f64_lit(p.fill_opacity)));
    }
    if (p.stroke_width - 1.5).abs() > f64::EPSILON {
        frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
    }
    if !p.show_strokes {
        frags.push(".with_strokes(false)".to_string());
    }
    if p.normalized {
        frags.push(".with_normalized()".to_string());
    }
    // `legend_position` has no CLI flag in `stacked_area.rs` (always the
    // struct default `OutsideRightTop`) and `LegendPosition` has no `Debug`
    // impl, so it's intentionally never emitted here.
    chain("StackedAreaPlot::new()", frags)
}

fn stream_baseline_ctor(b: &StreamBaseline) -> &'static str {
    match b {
        StreamBaseline::Wiggle => "StreamBaseline::Wiggle",
        StreamBaseline::Symmetric => "StreamBaseline::Symmetric",
        StreamBaseline::Zero => "StreamBaseline::Zero",
    }
}

fn stream_order_ctor(o: &StreamOrder) -> &'static str {
    match o {
        StreamOrder::InsideOut => "StreamOrder::InsideOut",
        StreamOrder::ByTotal => "StreamOrder::ByTotal",
        StreamOrder::Original => "StreamOrder::Original",
    }
}

/// Serialize a `StreamgraphPlot`'s resolved state — one `--group-col` loop
/// in `streamgraph.rs` builds every series via `with_x` once plus
/// `with_series`/`with_color`/`with_label` per group; read generically off
/// the struct's own parallel vectors, same approach as `emit_stacked_area_plot`.
pub fn emit_streamgraph_plot(p: &StreamgraphPlot) -> String {
    let xs =
        p.x.iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
    let mut frags: Vec<String> = vec![format!(".with_x(vec![{xs}])")];
    for (i, s) in p.series.iter().enumerate() {
        let color = p.colors.get(i).cloned().flatten();
        let label = p.labels.get(i).cloned().flatten();
        frags.extend(emit_area_series(s, &color, &label, ".with_label"));
    }
    if !matches!(p.baseline, StreamBaseline::Wiggle) {
        frags.push(format!(
            ".with_baseline({})",
            stream_baseline_ctor(&p.baseline)
        ));
    }
    if !matches!(p.order, StreamOrder::InsideOut) {
        frags.push(format!(".with_order({})", stream_order_ctor(&p.order)));
    }
    if !p.smooth {
        frags.push(".with_linear()".to_string());
    }
    if (p.fill_opacity - 0.85).abs() > f64::EPSILON {
        frags.push(format!(".with_fill_opacity({})", f64_lit(p.fill_opacity)));
    }
    if p.stroke_between {
        frags.push(".with_stroke()".to_string());
        if (p.stroke_width - 0.8).abs() > f64::EPSILON {
            frags.push(format!(".with_stroke_width({})", f64_lit(p.stroke_width)));
        }
    }
    if !p.show_labels {
        frags.push(".with_stream_labels(false)".to_string());
    }
    if (p.min_label_height - 14.0).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_min_label_height({})",
            f64_lit(p.min_label_height)
        ));
    }
    if p.normalized {
        frags.push(".with_normalized()".to_string());
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    // `legend_position` has no CLI flag in `streamgraph.rs` either — same
    // reasoning as `emit_stacked_area_plot`.
    chain("StreamgraphPlot::new()", frags)
}

fn emit_raincloud_group(g: &RaincloudGroup) -> String {
    let values = g
        .values
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    format!(".with_group({}, vec![{}])", str_lit(&g.label), values)
}

/// Serialize a `RaincloudPlot`'s resolved state. `raincloud.rs` always calls
/// `with_group_colors` (even for a single group), so `group_colors` is
/// effectively always `Some` for a CLI-built plot, but the `None` case is
/// still handled for correctness against the real struct shape.
pub fn emit_raincloud_plot(p: &RaincloudPlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(emit_raincloud_group).collect();
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors(vec![{list}])"));
    }
    if (p.cloud_width - 30.0).abs() > f64::EPSILON {
        frags.push(format!(".with_cloud_width({})", f64_lit(p.cloud_width)));
    }
    if let Some(bw) = p.bandwidth {
        frags.push(format!(".with_bandwidth({})", f64_lit(bw)));
    }
    if (p.bandwidth_scale - 1.0).abs() > f64::EPSILON {
        frags.push(format!(
            ".with_bandwidth_scale({})",
            f64_lit(p.bandwidth_scale)
        ));
    }
    if p.kde_samples != 200 {
        frags.push(format!(".with_kde_samples({})", p.kde_samples));
    }
    if (p.cloud_alpha - 0.7).abs() > f64::EPSILON {
        frags.push(format!(".with_cloud_alpha({})", f64_lit(p.cloud_alpha)));
    }
    if !p.show_cloud {
        frags.push(".with_cloud(false)".to_string());
    }
    if (p.box_width - 0.08).abs() > f64::EPSILON {
        frags.push(format!(".with_box_width({})", f64_lit(p.box_width)));
    }
    if !p.show_box {
        frags.push(".with_box(false)".to_string());
    }
    if (p.rain_size - 3.0).abs() > f64::EPSILON {
        frags.push(format!(".with_rain_size({})", f64_lit(p.rain_size)));
    }
    if (p.rain_jitter - 0.05).abs() > f64::EPSILON {
        frags.push(format!(".with_rain_jitter({})", f64_lit(p.rain_jitter)));
    }
    if (p.rain_alpha - 0.7).abs() > f64::EPSILON {
        frags.push(format!(".with_rain_alpha({})", f64_lit(p.rain_alpha)));
    }
    if !p.show_rain {
        frags.push(".with_rain(false)".to_string());
    }
    if p.flip {
        frags.push(".with_flip(true)".to_string());
    }
    if p.horizontal {
        frags.push(".with_horizontal(true)".to_string());
    }
    if (p.rain_offset - 0.20).abs() > f64::EPSILON {
        frags.push(format!(".with_rain_offset({})", f64_lit(p.rain_offset)));
    }
    if (p.cloud_offset - 0.15).abs() > f64::EPSILON {
        frags.push(format!(".with_cloud_offset({})", f64_lit(p.cloud_offset)));
    }
    if p.seed != 42 {
        frags.push(format!(".with_seed({})", p.seed));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("RaincloudPlot::new()", frags)
}

// ── Per-plot-type emitters (Tier 2: strip/polar/rose/manhattan/volcano) ──────

/// Serialize one `StripGroup`. `strip.rs` never builds per-point colored or
/// shaped groups (only plain `.with_group`), but this handles all four
/// `StripGroup` shapes generically since it just inspects the struct's own
/// `point_colors`/`point_shapes` fields — matching the design principle of
/// reading resolved state rather than re-deriving which CLI branch ran.
fn emit_strip_group(g: &StripGroup) -> String {
    match (&g.point_colors, &g.point_shapes) {
        (Some(colors), Some(shapes)) => {
            let triples = g
                .values
                .iter()
                .zip(colors.iter())
                .zip(shapes.iter())
                .map(|((v, c), s)| {
                    format!(
                        "({}, {}, {})",
                        f64_lit(*v),
                        str_lit(c),
                        marker_shape_ctor(*s)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(".with_styled_group({}, vec![{triples}])", str_lit(&g.label))
        }
        (Some(colors), None) => {
            let pairs = g
                .values
                .iter()
                .zip(colors.iter())
                .map(|(v, c)| format!("({}, {})", f64_lit(*v), str_lit(c)))
                .collect::<Vec<_>>()
                .join(", ");
            format!(".with_colored_group({}, vec![{pairs}])", str_lit(&g.label))
        }
        (None, Some(shapes)) => {
            let pairs = g
                .values
                .iter()
                .zip(shapes.iter())
                .map(|(v, s)| format!("({}, {})", f64_lit(*v), marker_shape_ctor(*s)))
                .collect::<Vec<_>>()
                .join(", ");
            format!(".with_shaped_group({}, vec![{pairs}])", str_lit(&g.label))
        }
        (None, None) => {
            let values = g
                .values
                .iter()
                .map(|v| f64_lit(*v))
                .collect::<Vec<_>>()
                .join(", ");
            format!(".with_group({}, vec![{values}])", str_lit(&g.label))
        }
    }
}

/// Serialize a `StripPlot`'s resolved state — works uniformly whether it came
/// from the simple `--group-col` or the wide `--y` CLI branch in `strip.rs`,
/// since both converge on the same `StripPlot` struct.
pub fn emit_strip_plot(p: &StripPlot) -> String {
    let mut frags: Vec<String> = p.groups.iter().map(emit_strip_group).collect();
    if p.color != "steelblue" {
        frags.push(format!(".with_color({})", str_lit(&p.color)));
    }
    if (p.point_size - 4.0).abs() > f64::EPSILON {
        frags.push(format!(".with_point_size({})", f64_lit(p.point_size)));
    }
    match &p.style {
        StripStyle::Strip { jitter } if (*jitter - 0.3).abs() > f64::EPSILON => {
            frags.push(format!(".with_jitter({})", f64_lit(*jitter)));
        }
        StripStyle::Strip { .. } => {}
        StripStyle::Swarm => frags.push(".with_swarm()".to_string()),
        StripStyle::Center => frags.push(".with_center()".to_string()),
    }
    if p.seed != 42 {
        frags.push(format!(".with_seed({})", p.seed));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if let Some(ref colors) = p.group_colors {
        let list = colors
            .iter()
            .map(|c| str_lit(c))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_group_colors([{list}])"));
    }
    if let Some(op) = p.marker_opacity {
        frags.push(format!(".with_marker_opacity({})", f64_lit(op)));
    }
    if let Some(w) = p.marker_stroke_width {
        frags.push(format!(".with_marker_stroke_width({})", f64_lit(w)));
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    chain("StripPlot::new()", frags)
}

fn polar_mode_ctor(mode: &PolarMode) -> &'static str {
    match mode {
        PolarMode::Scatter => "PolarMode::Scatter",
        PolarMode::Line => "PolarMode::Line",
    }
}

/// Serialize one `PolarSeries`. `PolarSeries::marker_size`/`stroke_width`/
/// `line_dash` have no public setter reachable from any `PolarPlot` builder
/// method (only `color`/`marker_opacity`/`marker_stroke_width` can be set,
/// each acting on the last-added series), so they're always default and
/// never emitted here.
fn emit_polar_series(s: &PolarSeries) -> Vec<String> {
    let r =
        s.r.iter()
            .map(|v| f64_lit(*v))
            .collect::<Vec<_>>()
            .join(", ");
    let theta = s
        .theta
        .iter()
        .map(|v| f64_lit(*v))
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags = match &s.label {
        Some(label) => vec![format!(
            ".with_series_labeled(vec![{r}], vec![{theta}], {}, {})",
            str_lit(label),
            polar_mode_ctor(&s.mode)
        )],
        None => match s.mode {
            PolarMode::Scatter => vec![format!(".with_series(vec![{r}], vec![{theta}])")],
            PolarMode::Line => vec![format!(".with_series_line(vec![{r}], vec![{theta}])")],
        },
    };
    if let Some(ref color) = s.color {
        frags.push(format!(".with_color({})", str_lit(color)));
    }
    if let Some(op) = s.marker_opacity {
        frags.push(format!(".with_marker_opacity({})", f64_lit(op)));
    }
    if let Some(w) = s.marker_stroke_width {
        frags.push(format!(".with_marker_stroke_width({})", f64_lit(w)));
    }
    frags
}

/// Serialize a `PolarPlot`'s resolved state. Handles both the simple and
/// `--color-by` (per-group series) CLI construction paths in `polar.rs`
/// uniformly, since both converge on the same `series` field.
pub fn emit_polar_plot(p: &PolarPlot) -> String {
    let mut frags: Vec<String> = Vec::new();
    for s in &p.series {
        frags.extend(emit_polar_series(s));
    }
    if let Some(v) = p.r_max {
        frags.push(format!(".with_r_max({})", f64_lit(v)));
    }
    if let Some(v) = p.r_min {
        frags.push(format!(".with_r_min({})", f64_lit(v)));
    }
    if p.theta_start != 0.0 {
        frags.push(format!(".with_theta_start({})", f64_lit(p.theta_start)));
    }
    if !p.clockwise {
        frags.push(".with_clockwise(false)".to_string());
    }
    if let Some(n) = p.r_grid_lines {
        frags.push(format!(".with_r_grid_lines({n})"));
    }
    if p.theta_divisions != 12 {
        frags.push(format!(".with_theta_divisions({})", p.theta_divisions));
    }
    if !p.show_grid {
        frags.push(".with_grid(false)".to_string());
    }
    if !p.show_r_labels {
        frags.push(".with_r_labels(false)".to_string());
    }
    if p.show_legend {
        frags.push(".with_legend(true)".to_string());
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    chain("PolarPlot::new()", frags)
}

/// Serialize a `RosePlot`'s resolved state. `rose.rs`'s single-series CLI
/// path (one `.with_slice(label, value)` call per row) always leaves the
/// series named `"Values"` — used here as the anonymous-series heuristic,
/// mirroring `emit_pyramid_plot`'s treatment of its own single-anonymous-
/// series case. The `--compass` flag mutates `p.labels` in place before this
/// runs, so the compass-transformed strings are already what gets baked in
/// as literals — no need to re-call `with_compass_labels` here.
pub fn emit_rose_plot(p: &RosePlot) -> String {
    let mut frags: Vec<String> = Vec::new();
    if p.series.len() == 1 && p.series[0].name == "Values" {
        let slices = p
            .labels
            .iter()
            .zip(p.series[0].values.iter())
            .map(|(l, v)| format!("({}, {})", str_lit(l), f64_lit(*v)))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_slices(vec![{slices}])"));
    } else {
        if !p.labels.is_empty() {
            let list = p
                .labels
                .iter()
                .map(|l| str_lit(l))
                .collect::<Vec<_>>()
                .join(", ");
            frags.push(format!(".with_x_labels(vec![{list}])"));
        }
        // `RoseSeries::color` has no public setter reachable from any
        // `RosePlot` builder method, so it's always `None` and never
        // emitted here.
        for s in &p.series {
            let values = s
                .values
                .iter()
                .map(|v| f64_lit(*v))
                .collect::<Vec<_>>()
                .join(", ");
            let ctor = match p.mode {
                RoseMode::Grouped => "with_group",
                RoseMode::Stacked => "with_stack",
            };
            frags.push(format!(".{ctor}({}, vec![{values}])", str_lit(&s.name)));
        }
    }
    if !matches!(p.encoding, RoseEncoding::Area) {
        frags.push(".with_encoding(RoseEncoding::Radius)".to_string());
    }
    if p.start_angle != 0.0 {
        frags.push(format!(".with_start_angle({})", f64_lit(p.start_angle)));
    }
    if !p.clockwise {
        frags.push(".with_clockwise(false)".to_string());
    }
    if p.inner_radius != 0.0 {
        frags.push(format!(".with_inner_radius({})", f64_lit(p.inner_radius)));
    }
    if (p.gap - 1.0).abs() > f64::EPSILON {
        frags.push(format!(".with_gap({})", f64_lit(p.gap)));
    }
    if !p.show_grid {
        frags.push(".with_grid(false)".to_string());
    }
    if p.grid_lines != 4 {
        frags.push(format!(".with_grid_lines({})", p.grid_lines));
    }
    if !p.show_spokes {
        frags.push(".with_spokes(false)".to_string());
    }
    if !p.show_labels {
        frags.push(".with_show_labels(false)".to_string());
    }
    if p.show_values {
        frags.push(".with_show_values(true)".to_string());
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    chain("RosePlot::new()", frags)
}

/// Shared by [`emit_manhattan_plot`] and [`emit_volcano_plot`] — both plot
/// types use the same `LabelStyle` enum for gene-label placement.
fn label_style_ctor(style: &LabelStyle) -> String {
    match style {
        LabelStyle::Exact => "LabelStyle::Exact".to_string(),
        LabelStyle::Nudge => "LabelStyle::Nudge".to_string(),
        LabelStyle::Arrow { offset_x, offset_y } => format!(
            "LabelStyle::Arrow {{ offset_x: {}, offset_y: {} }}",
            f64_lit(*offset_x),
            f64_lit(*offset_y)
        ),
    }
}

/// Serialize a `ManhattanPlot`'s resolved state via `with_data_x`, which
/// reconstructs the exact `(chromosome, x, pvalue)` triples regardless of
/// whether `manhattan.rs` built the plot via `with_data` (sequential mode) or
/// `with_data_bp` (genome-build mode) — `ManhattanPlot` doesn't retain which
/// `GenomeBuild` was used, so this is the only generically faithful
/// reconstruction available from the struct alone.
///
/// One caveat: `with_data_bp` also produces empty chromosome-band spans for
/// chromosomes with zero data points (so they still appear on the x-axis);
/// those are lost here since `with_data_x` derives spans purely from the
/// data actually present. `manhattan.rs` never calls `with_point_labels`
/// (only `with_label_top`), but any baked-in point labels are still
/// re-attached generically below in case a future CLI path sets them.
pub fn emit_manhattan_plot(p: &ManhattanPlot) -> String {
    let data = p
        .points
        .iter()
        .map(|pt| {
            format!(
                "({}, {}, {})",
                str_lit(&pt.chromosome),
                f64_lit(pt.x),
                f64_lit(pt.pvalue)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags: Vec<String> = vec![format!(".with_data_x(vec![{data}])")];

    let labeled: Vec<String> = p
        .points
        .iter()
        .filter(|pt: &&ManhattanPoint| pt.label.is_some())
        .map(|pt| {
            format!(
                "({}, {}, {})",
                str_lit(&pt.chromosome),
                f64_lit(pt.x),
                str_lit(pt.label.as_deref().unwrap_or(""))
            )
        })
        .collect();
    if !labeled.is_empty() {
        frags.push(format!(".with_point_labels(vec![{}])", labeled.join(", ")));
    }

    if (p.genome_wide - (-5e-8_f64.log10())).abs() > 1e-9 {
        frags.push(format!(".with_genome_wide({})", f64_lit(p.genome_wide)));
    }
    if (p.suggestive - 5.0).abs() > f64::EPSILON {
        frags.push(format!(".with_suggestive({})", f64_lit(p.suggestive)));
    }
    if p.color_a != "steelblue" {
        frags.push(format!(".with_color_a({})", str_lit(&p.color_a)));
    }
    if p.color_b != "#5aadcb" {
        frags.push(format!(".with_color_b({})", str_lit(&p.color_b)));
    }
    if let Some(ref pal) = p.palette {
        match palette_ctor(pal.name) {
            Some(ctor) => frags.push(format!(".with_palette({ctor})")),
            None => {
                let colors = pal
                    .colors()
                    .iter()
                    .map(|c| str_lit(c))
                    .collect::<Vec<_>>()
                    .join(", ");
                frags.push(format!(
                    ".with_palette(Palette::custom({}, vec![{colors}]))",
                    str_lit(pal.name)
                ));
            }
        }
    }
    if (p.point_size - 2.5).abs() > f64::EPSILON {
        frags.push(format!(".with_point_size({})", f64_lit(p.point_size)));
    }
    if p.label_top != 0 {
        frags.push(format!(".with_label_top({})", p.label_top));
    }
    if !matches!(p.label_style, LabelStyle::Nudge) {
        frags.push(format!(
            ".with_label_style({})",
            label_style_ctor(&p.label_style)
        ));
    }
    if let Some(f) = p.pvalue_floor {
        frags.push(format!(".with_pvalue_floor({})", f64_lit(f)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    chain("ManhattanPlot::new()", frags)
}

/// Serialize a `VolcanoPlot`'s resolved state — `volcano.rs` only ever calls
/// `with_points`, but this reads the struct's own `points` field generically.
pub fn emit_volcano_plot(p: &VolcanoPlot) -> String {
    let points = p
        .points
        .iter()
        .map(|pt| {
            format!(
                "({}, {}, {})",
                str_lit(&pt.name),
                f64_lit(pt.log2fc),
                f64_lit(pt.pvalue)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut frags: Vec<String> = vec![format!(".with_points(vec![{points}])")];

    if (p.fc_cutoff - 1.0).abs() > f64::EPSILON {
        frags.push(format!(".with_fc_cutoff({})", f64_lit(p.fc_cutoff)));
    }
    if (p.p_cutoff - 0.05).abs() > f64::EPSILON {
        frags.push(format!(".with_p_cutoff({})", f64_lit(p.p_cutoff)));
    }
    if p.color_up != "firebrick" {
        frags.push(format!(".with_color_up({})", str_lit(&p.color_up)));
    }
    if p.color_down != "steelblue" {
        frags.push(format!(".with_color_down({})", str_lit(&p.color_down)));
    }
    if p.color_ns != "#aaaaaa" {
        frags.push(format!(".with_color_ns({})", str_lit(&p.color_ns)));
    }
    if (p.point_size - 3.0).abs() > f64::EPSILON {
        frags.push(format!(".with_point_size({})", f64_lit(p.point_size)));
    }
    if p.label_top != 0 {
        frags.push(format!(".with_label_top({})", p.label_top));
    }
    if !matches!(p.label_style, LabelStyle::Nudge) {
        frags.push(format!(
            ".with_label_style({})",
            label_style_ctor(&p.label_style)
        ));
    }
    if let Some(f) = p.pvalue_floor {
        frags.push(format!(".with_pvalue_floor({})", f64_lit(f)));
    }
    if let Some(ref label) = p.legend_label {
        frags.push(format!(".with_legend({})", str_lit(label)));
    }
    if p.show_tooltips {
        frags.push(".with_tooltips()".to_string());
    }
    if let Some(ref labels) = p.tooltip_labels {
        let list = labels
            .iter()
            .map(|l| str_lit(l))
            .collect::<Vec<_>>()
            .join(", ");
        frags.push(format!(".with_tooltip_labels(vec![{list}])"));
    }
    chain("VolcanoPlot::new()", frags)
}
