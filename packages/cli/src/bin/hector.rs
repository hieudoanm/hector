//! Thin CLI shell: parse flags, call the library, report.

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap::Parser;
use hector::cli::{Cli, Command};
use hector::pipeline::{trace_verbose, TraceStats, Traced};
use hector::preprocess;
use hector::{router, to_svg, DebugWriter, Error, Raster, ServerConfig, SvgOptions, TracingArgs};
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("baseline check found a regression");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// Dispatch the parsed command line.
///
/// The measurement subcommands return `Ok(false)` when a baseline check found a
/// regression, which the caller turns into a non-zero exit code.
fn run(cli: &Cli) -> Result<bool> {
    let clean = match &cli.command {
        Some(Command::Info { input }) => {
            info(input, TracingArgs::from_cli(cli))?;
            true
        }
        Some(Command::Serve { bind }) => {
            serve(bind, TracingArgs::from_cli(cli))?;
            true
        }
        Some(Command::Mcp {
            command: hector::cli::McpCommand::Serve,
        }) => {
            hector::mcp::serve_stdio()?;
            true
        }
        Some(Command::Completion { shell }) => {
            completion(*shell)?;
            true
        }
        None => {
            convert(cli)?;
            true
        }
        #[cfg(feature = "eval")]
        Some(Command::Eval {
            input,
            loss_complexity,
            loss_file_size,
        }) => {
            run_eval(
                input,
                TracingArgs::from_cli(cli),
                *loss_complexity,
                *loss_file_size,
                cli.debug_dir.as_deref(),
            )?;
            true
        }
        #[cfg(feature = "eval")]
        Some(Command::Baseline {
            input,
            baseline,
            record,
            artifacts,
        }) => run_baseline(input, baseline, *record, artifacts.as_deref(), cli)?,
        #[cfg(feature = "eval")]
        Some(Command::Golden { directory }) => {
            emit_golden(directory)?;
            true
        }
        #[cfg(feature = "eval")]
        Some(Command::Sweep {
            input,
            parameter,
            values,
        }) => {
            run_sweep(input, parameter, values, TracingArgs::from_cli(cli))?;
            true
        }
        #[cfg(not(feature = "eval"))]
        Some(
            Command::Eval { .. }
            | Command::Baseline { .. }
            | Command::Golden { .. }
            | Command::Sweep { .. },
        ) => {
            return Err(Error::Config(
                "this build omits the measurement tools; rebuild with `--features eval`".into(),
            )
            .into())
        }
    };
    Ok(clean)
}

/// Measure one image or a whole suite and print the scores.
#[cfg(feature = "eval")]
fn run_eval(
    input: &Path,
    args: TracingArgs<'_>,
    loss_complexity: f64,
    loss_file_size: f64,
    debug_dir: Option<&Path>,
) -> Result<()> {
    let precision = args.precision;
    hector::cli::measure::run(
        input,
        args,
        precision,
        hector::eval::LossWeights {
            complexity: loss_complexity,
            file_size: loss_file_size,
        },
        debug_dir,
    )
}

/// Record a baseline, or compare a run against one and report regressions.
#[cfg(feature = "eval")]
fn run_baseline(
    input: &Path,
    baseline: &Path,
    record: bool,
    artifacts: Option<&Path>,
    cli: &Cli,
) -> Result<bool> {
    let config = TracingArgs::from_cli(cli).to_config()?;
    let svg_options = SvgOptions {
        precision: cli.precision,
        ..SvgOptions::default()
    };
    if record {
        hector::cli::baseline::save(input, baseline, &config, &svg_options)?;
        return Ok(true);
    }
    hector::cli::baseline::compare(
        baseline,
        input,
        TracingArgs::from_cli(cli),
        cli.precision,
        hector::eval::LossWeights::default(),
        artifacts,
    )
}

/// Write the golden suite PNGs.
#[cfg(feature = "eval")]
fn emit_golden(directory: &Path) -> Result<()> {
    hector::cli::baseline::emit_golden(directory)
}

/// Sweep one parameter across a suite and print the trade-off.
#[cfg(feature = "eval")]
fn run_sweep(input: &Path, parameter: &str, values: &[f64], args: TracingArgs<'_>) -> Result<()> {
    let parameter = hector::eval::Parameter::parse(parameter)?;
    let precision = args.precision;
    let values = (!values.is_empty()).then(|| values.to_vec());
    hector::cli::sweep::run(
        parameter,
        values,
        input,
        args,
        precision,
        hector::eval::LossWeights::default(),
    )
}

/// Load the image and print its palette, without tracing geometry.
fn info(path: &Path, args: TracingArgs<'_>) -> Result<()> {
    let raster = load(path)?;
    let config = args.to_config()?;
    let map = preprocess::label(&raster, config.color_mode(), config.backdrop);
    println!("{}x{} pixels", raster.width(), raster.height());
    println!("palette ({} colors):", map.palette.len());
    for swatch in &map.palette {
        println!("  {}  {:>9} px", swatch.color.to_hex(), swatch.population);
    }
    Ok(())
}

/// Run the HTTP API until the process is asked to stop.
fn serve(bind: &str, args: TracingArgs<'_>) -> Result<()> {
    let config = ServerConfig {
        defaults: args.to_config()?,
        precision: args.precision,
    };
    runtime()?.block_on(async move {
        let listener = tokio::net::TcpListener::bind(bind)
            .await
            .with_context(|| format!("binding {bind}"))?;
        eprintln!("vectify serving on http://{bind} (POST /vectify, GET /health, GET /info)");
        axum::serve(listener, router(config))
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("server stopped")
    })
}

/// A multi-threaded runtime, created lazily so tracing-only runs never pay for it.
fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the tokio runtime")
}

/// Resolve on Ctrl+C so in-flight requests finish before the process exits.
async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        eprintln!("cannot listen for shutdown: {error}");
    }
}

/// Write a shell completion script to stdout.
fn completion(shell: clap_complete::Shell) -> Result<()> {
    let mut command = Cli::command();
    clap_complete::generate(shell, &mut command, "vectify", &mut std::io::stdout());
    Ok(())
}

/// Trace the input and write SVG to the output path or stdout.
fn convert(cli: &Cli) -> Result<()> {
    let config = TracingArgs::from_cli(cli).to_config()?;
    let raster = load(input_of(cli)?)?;
    let traced = trace_verbose(&raster, &config).context("tracing failed")?;
    let svg = to_svg(
        &traced.image,
        &SvgOptions {
            precision: cli.precision,
            ..SvgOptions::default()
        },
    );
    write_svg(cli, &svg)?;
    write_debug(cli, &traced, &svg)?;
    report(cli, &traced.stats, &svg);
    Ok(())
}

/// Path to trace; `subcommand_negates_reqs` means it is absent only with a subcommand.
fn input_of(cli: &Cli) -> Result<&Path> {
    cli.input
        .as_deref()
        .ok_or_else(|| Error::Config("no input image given; try `vectify --help`".into()).into())
}

/// Decode the input image with the path in every error message.
fn load(path: &Path) -> Result<Raster> {
    Raster::load(path).with_context(|| format!("reading `{}`", path.display()))
}

/// Write the SVG to a file, or to stdout when no output path was given.
fn write_svg(cli: &Cli, svg: &str) -> Result<()> {
    match &cli.output {
        Some(path) => {
            std::fs::write(path, svg).with_context(|| format!("writing `{}`", path.display()))
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(svg.as_bytes())
                .context("writing SVG to stdout")?;
            stdout.flush().context("flushing stdout")
        }
    }
}

/// Write per-stage debug artifacts when `--debug-dir` was given.
fn write_debug(cli: &Cli, traced: &Traced, svg: &str) -> Result<()> {
    let Some(directory) = &cli.debug_dir else {
        return Ok(());
    };
    let writer = DebugWriter::new(directory);
    writer.write_quantized(&traced.labels)?;
    writer.write_regions(
        &traced.labels,
        &traced.region_of_pixel,
        traced.traced_regions,
    )?;
    writer.write_curves(&traced.image)?;
    writer.write_curve_svg(svg)?;
    if !cli.quiet {
        eprintln!("debug stages written to {}", writer.directory().display());
    }
    Ok(())
}

/// Print a one-line summary to stderr, keeping stdout free for the SVG.
fn report(cli: &Cli, stats: &TraceStats, svg: &str) {
    if cli.quiet || !std::io::stderr().is_terminal() {
        return;
    }
    eprintln!(
        "{} colors, {} regions, {} paths, {} curves, {} -> {} bytes",
        stats.colors,
        stats.regions,
        stats.contours - stats.holes,
        stats.curves,
        stats.points_after_simplify,
        svg.len()
    );
}
