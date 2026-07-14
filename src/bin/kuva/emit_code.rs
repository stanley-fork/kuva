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

use kuva::plot::dotplot::DotPoint;
use kuva::plot::ecdf::EcdfGroup;
use kuva::plot::mosaic::MosaicCell;
use kuva::plot::qq::{QQGroup, QQMode};
use kuva::plot::{ColorMap, DotPlot, EcdfPlot, MosaicPlot, ParetoPlot, QQPlot};

use crate::layout_args::{AxisArgs, BaseArgs, LogArgs};

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
