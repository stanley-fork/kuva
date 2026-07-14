#![cfg(all(feature = "cli", feature = "full", feature = "emit_code"))]
//! Verifies `--emit-code` output for every wired-up Tier-1 subcommand.
//!
//! Two layers: a structural smoke check (the snippet contains the expected
//! `use`/`Layout` scaffolding) and a real compilation check via `trybuild` —
//! each captured snippet is wrapped in `fn main() {}` and compiled against
//! the actual `kuva` crate, so a snippet that "looks right" but has a real
//! type error gets caught, not just eyeballed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run `kuva <args> --emit-code` and return its stdout.
fn run_emit_code(args: &[&str]) -> String {
    let exe = env!("CARGO_BIN_EXE_kuva");
    let output = Command::new(exe)
        .args(args)
        .arg("--emit-code")
        .output()
        .unwrap_or_else(|e| panic!("failed to run kuva {args:?}: {e}"));
    assert!(
        output.status.success(),
        "kuva {args:?} --emit-code failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("--emit-code output was not valid UTF-8")
}

/// The subcommands wired up for `--emit-code` so far, with a CLI invocation
/// (excluding the trailing `--emit-code`, added by `run_emit_code`) known to
/// produce output against the checked-in example data.
fn covered_subcommands() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("pie", vec!["pie", "examples/data/pie.tsv"]),
        (
            "bar",
            vec![
                "bar",
                "examples/data/pie.tsv",
                "--label-col",
                "feature",
                "--value-col",
                "percentage",
            ],
        ),
        (
            "scatter",
            vec![
                "scatter",
                "examples/data/scatter.tsv",
                "--x",
                "x",
                "--y",
                "y",
                "--color-by",
                "group",
                "--legend",
                "--trend",
                "--equation",
                "--correlation",
            ],
        ),
        (
            "box",
            vec![
                "box",
                "examples/data/measurements.tsv",
                "--group-col",
                "group",
                "--value-col",
                "value",
            ],
        ),
        ("venn", vec!["venn", "examples/data/venn.tsv"]),
        ("waffle", vec!["waffle", "examples/data/waffle.tsv"]),
        ("funnel", vec!["funnel", "examples/data/funnel.tsv"]),
        ("pyramid", vec!["pyramid", "examples/data/pyramid.tsv"]),
        ("lollipop", vec!["lollipop", "examples/data/lollipop.tsv"]),
        (
            "ecdf",
            vec!["ecdf", "examples/data/samples.tsv", "--value", "expression"],
        ),
        (
            "dot",
            vec![
                "dot",
                "examples/data/dot.tsv",
                "--x-col",
                "cell_type",
                "--y-col",
                "pathway",
                "--size-col",
                "pct_expressed",
                "--color-col",
                "mean_expr",
            ],
        ),
        (
            "mosaic",
            vec![
                "mosaic",
                "examples/data/mosaic.tsv",
                "--col-col",
                "region",
                "--row-col",
                "outcome",
                "--value-col",
                "count",
            ],
        ),
        (
            "pareto",
            vec![
                "pareto",
                "examples/data/pareto.tsv",
                "--label-col",
                "category",
                "--value-col",
                "count",
            ],
        ),
        (
            "qq",
            vec!["qq", "examples/data/samples.tsv", "--value", "expression"],
        ),
        (
            "line",
            vec![
                "line",
                "examples/data/measurements.tsv",
                "--x",
                "time",
                "--y",
                "value",
                "--color-by",
                "group",
                "--legend",
            ],
        ),
        (
            "histogram",
            vec![
                "histogram",
                "examples/data/measurements.tsv",
                "--y",
                "time,value",
                "--legend",
            ],
        ),
        (
            "violin",
            vec![
                "violin",
                "examples/data/samples.tsv",
                "--group-col",
                "group",
                "--value-col",
                "expression",
                "--overlay-swarm",
                "--group-colors",
                "steelblue,tomato,seagreen,goldenrod,mediumpurple",
            ],
        ),
        (
            "roc",
            vec![
                "roc",
                "examples/data/roc.tsv",
                "--score-col",
                "score",
                "--label-col",
                "label",
                "--ci",
                "--auc-label",
                "--legend",
                "Model",
            ],
        ),
        (
            "survival",
            vec![
                "survival",
                "examples/data/survival.tsv",
                "--time-col",
                "time",
                "--event-col",
                "event",
                "--group-col",
                "group",
                "--legend",
                "Group",
            ],
        ),
        (
            "radar",
            vec![
                "radar",
                "examples/data/radar.tsv",
                "--axes",
                "Sensitivity",
                "Specificity",
                "Precision",
                "F1",
                "AUC",
                "--label-col",
                "tool",
                "--legend",
            ],
        ),
        (
            "waterfall",
            vec![
                "waterfall",
                "examples/data/waterfall.tsv",
                "--label-col",
                "process",
                "--value-col",
                "log2fc",
                "--connectors",
                "--values",
            ],
        ),
        (
            "forest",
            vec![
                "forest",
                "examples/data/forest.tsv",
                "--label-col",
                "study",
                "--estimate-col",
                "estimate",
                "--ci-lower-col",
                "ci_lower",
                "--ci-upper-col",
                "ci_upper",
                "--weight-col",
                "weight",
            ],
        ),
        (
            "slope",
            vec![
                "slope",
                "examples/data/slope.tsv",
                "--label-col",
                "label",
                "--before-col",
                "before",
                "--after-col",
                "after",
                "--before-label",
                "2015",
                "--after-label",
                "2023",
                "--show-values",
            ],
        ),
        (
            "bump",
            vec![
                "bump",
                "examples/data/bump.tsv",
                "--series",
                "series",
                "--time",
                "time",
                "--rank",
                "rank",
                "--highlight",
                "Alpha",
                "--rank-labels",
            ],
        ),
    ]
}

#[test]
fn emit_code_snippets_are_structurally_sound() {
    for (name, args) in covered_subcommands() {
        let snippet = run_emit_code(&args);
        assert!(
            snippet.contains("use kuva::backend::svg::SvgBackend;"),
            "{name}: missing SvgBackend import"
        );
        assert!(
            snippet.contains("Layout::auto_from_plots(&plots)"),
            "{name}: missing Layout::auto_from_plots"
        );
        assert!(
            snippet.contains("render_multiple(plots, layout)"),
            "{name}: missing render_multiple call"
        );
        assert!(
            !snippet.to_lowercase().contains("todo!") && !snippet.contains("unimplemented!"),
            "{name}: emitted a placeholder, not real code"
        );
    }
}

/// Split a captured snippet into its leading `use ...;` lines (kept at module
/// scope) and the remaining statements (wrapped in `fn main() { ... }`).
fn wrap_as_program(snippet: &str) -> String {
    let mut uses = String::new();
    let mut body = String::new();
    let mut past_uses = false;
    for line in snippet.lines() {
        if !past_uses && (line.starts_with("use ") || line.is_empty()) {
            uses.push_str(line);
            uses.push('\n');
        } else {
            past_uses = true;
            body.push_str(line);
            body.push('\n');
        }
    }
    format!("{uses}\nfn main() {{\n{body}}}\n")
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/emit_code_fixtures")
}

#[test]
fn emit_code_snippets_compile() {
    let dir = fixtures_dir();
    fs::create_dir_all(&dir).expect("failed to create fixtures dir");

    let cases = trybuild::TestCases::new();
    for (name, args) in covered_subcommands() {
        let snippet = run_emit_code(&args);
        let program = wrap_as_program(&snippet);
        let path = dir.join(format!("{name}.rs"));
        fs::write(&path, program).unwrap_or_else(|e| panic!("failed to write {path:?}: {e}"));
        cases.pass(path);
    }
}
