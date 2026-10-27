//! Clap definition for the `vectify` command line.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// How detailed the SVG output should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Detail {
    /// Every path, holes included, even tiny ones.
    Full,
    /// Drop paths smaller than a few pixels.
    Balanced,
    /// Keep only the largest shapes; ideal for logos.
    Bold,
}

#[derive(Debug, Parser)]
#[command(
    name = "vectify",
    version,
    about = "Convert raster images (PNG, JPG) into SVG vector paths",
    long_about = None,
    subcommand_negates_reqs = true,
    after_help = "Examples:\n  vectify logo.png logo.svg\n  vectify icon.png icon.svg --colors 4\n  vectify photo.jpg out.svg --threshold 140 --bezier-tolerance 1.0\n  vectify logo.png logo.svg --debug-dir ./trace\n  vectify info logo.png\n  vectify demo.png out.svg --detail bold\n\nVectify is a raster to vector tracer: pixels become regions, regions become\ncontours, contours become Bezier curves, curves become SVG paths."
)]
pub struct Cli {
    /// Input image path (PNG or JPEG); the eval subcommands also take a directory.
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Output SVG path; omit to print the SVG to stdout.
    #[arg(value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Number of colors in the palette; 1 or 2 selects black-and-white tracing.
    #[arg(
        short = 'c',
        long,
        value_name = "N",
        default_value_t = 8,
        global = true
    )]
    pub colors: usize,

    /// Luminance threshold (0-255) used in black-and-white mode.
    #[arg(
        short = 't',
        long,
        value_name = "L",
        default_value_t = 128,
        global = true
    )]
    pub threshold: u8,

    /// Douglas-Peucker tolerance in pixels; higher keeps fewer points.
    #[arg(long, value_name = "PX", default_value_t = 1.0, global = true)]
    pub simplify_tolerance: f64,

    /// Maximum Bezier deviation in pixels; higher yields fewer curves.
    #[arg(long, value_name = "PX", default_value_t = 0.5, global = true)]
    pub bezier_tolerance: f64,

    /// Ignore regions smaller than this many pixels.
    #[arg(long, value_name = "PX", default_value_t = 4, global = true)]
    pub min_area: usize,

    /// Backdrop color used under transparent pixels, as `#rrggbb`.
    #[arg(long, value_name = "COLOR", default_value = "#ffffff", global = true)]
    pub backdrop: String,

    /// Palette index treated as background and never traced.
    #[arg(long, value_name = "INDEX", default_value_t = 0, global = true)]
    pub background_index: usize,

    /// Decimal places kept on emitted coordinates.
    #[arg(long, value_name = "N", default_value_t = 2, global = true)]
    pub precision: u32,

    /// How aggressively small shapes are dropped.
    #[arg(short = 'd', long, value_enum, default_value_t = Detail::Balanced, global = true)]
    pub detail: Detail,

    /// Write per-stage debug PNGs into this directory.
    #[arg(long, value_name = "DIR", global = true)]
    pub debug_dir: Option<PathBuf>,

    /// Suppress the summary written to stderr.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Serve raster-to-SVG tracing over the Model Context Protocol on stdio.
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Print image and palette information without tracing.
    Info {
        /// Image to inspect (PNG or JPEG).
        #[arg(value_name = "INPUT")]
        input: PathBuf,
    },
    /// Serve the tracer over HTTP.
    Serve {
        /// Address to bind.
        #[arg(long, value_name = "ADDR", default_value = "127.0.0.1:8080")]
        bind: String,
    },
    /// Measure reconstruction error against the source image.
    ///
    /// Rasterizes the emitted SVG and reports pixel, perceptual, and geometric
    /// numbers. Takes a directory to measure a whole suite.
    Eval {
        /// Image or directory of images to measure.
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Weight of one geometric primitive in the combined loss.
        #[arg(long, value_name = "W", default_value_t = 0.01)]
        loss_complexity: f64,

        /// Weight of one kilobyte of SVG in the combined loss.
        #[arg(long, value_name = "W", default_value_t = 0.002)]
        loss_file_size: f64,
    },
    /// Record a baseline, or compare a run against one.
    Baseline {
        /// Image or directory of images to measure.
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Baseline JSON to read, or to write with `--record`.
        #[arg(
            long,
            value_name = "PATH",
            default_value = "tests/golden/baseline.json"
        )]
        baseline: PathBuf,

        /// Write the baseline instead of comparing against it.
        #[arg(long)]
        record: bool,

        /// Write per-stage PNGs for every image while comparing.
        #[arg(long, value_name = "DIR", conflicts_with = "record")]
        artifacts: Option<PathBuf>,
    },
    /// Write the synthetic golden suite as PNGs.
    Golden {
        /// Directory to write the suite into.
        #[arg(value_name = "DIR")]
        directory: PathBuf,
    },
    /// Explore one tracing parameter across the suite.
    Sweep {
        /// Image or directory of images to measure.
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Parameter to vary.
        #[arg(value_name = "PARAMETER")]
        parameter: String,

        /// Values to try; omit for a default ladder.
        #[arg(value_name = "VALUES", num_args = 1..)]
        values: Vec<f64>,
    },
    /// Emit a shell completion script.
    #[command(hide = true)]
    Completion {
        /// Target shell.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Start the MCP server using newline-delimited JSON-RPC on stdin/stdout.
    Serve,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_minimal_invocation() {
        let cli = Cli::try_parse_from(["vectify", "logo.png", "logo.svg"]).expect("valid");
        assert_eq!(cli.input.unwrap().to_str(), Some("logo.png"));
        assert_eq!(cli.output.unwrap().to_str(), Some("logo.svg"));
        assert_eq!(cli.colors, 8);
        assert_eq!(cli.detail, Detail::Balanced);
    }

    #[test]
    fn output_is_optional() {
        let cli = Cli::try_parse_from(["vectify", "logo.png"]).expect("valid");
        assert!(cli.output.is_none());
    }

    #[test]
    fn rejects_unknown_flag() {
        assert!(Cli::try_parse_from(["vectify", "logo.png", "--nope"]).is_err());
    }

    #[test]
    fn parses_tuning_flags() {
        let cli = Cli::try_parse_from([
            "vectify",
            "icon.png",
            "--colors",
            "4",
            "--bezier-tolerance",
            "1.5",
            "--detail",
            "bold",
        ])
        .expect("valid");
        assert_eq!(cli.colors, 4);
        assert_eq!(cli.bezier_tolerance, 1.5);
        assert_eq!(cli.detail, Detail::Bold);
    }

    #[test]
    fn detail_rejects_unknown_value() {
        assert!(Cli::try_parse_from(["vectify", "a.png", "--detail", "fancy"]).is_err());
    }

    #[test]
    fn info_takes_its_own_input_path() {
        let cli = Cli::try_parse_from(["vectify", "info", "logo.png"]).expect("valid");
        assert!(cli.input.is_none());
        let Some(Command::Info { input }) = cli.command else {
            panic!("expected the info subcommand");
        };
        assert_eq!(input.to_str(), Some("logo.png"));
    }

    #[test]
    fn parses_mcp_serve_command() {
        let cli = Cli::try_parse_from(["vectify", "mcp", "serve"]).expect("valid");
        assert!(matches!(
            cli.command,
            Some(Command::Mcp {
                command: McpCommand::Serve
            })
        ));
    }

    #[test]
    fn eval_takes_its_own_input_path_and_loss_weights() {
        let cli = Cli::try_parse_from(["vectify", "eval", "suite/", "--loss-complexity", "0.05"])
            .expect("valid");
        let Some(Command::Eval {
            input,
            loss_complexity,
            ..
        }) = cli.command
        else {
            panic!("expected the eval subcommand");
        };
        assert_eq!(input.to_str(), Some("suite/"));
        assert_eq!(loss_complexity, 0.05);
    }

    #[test]
    fn baseline_defaults_to_the_committed_suite() {
        let cli =
            Cli::try_parse_from(["vectify", "baseline", "tests/golden/images"]).expect("valid");
        let Some(Command::Baseline {
            baseline, record, ..
        }) = cli.command
        else {
            panic!("expected the baseline subcommand");
        };
        assert_eq!(
            baseline.to_str(),
            Some("tests/golden/baseline.json"),
            "the default baseline should be the committed one"
        );
        assert!(!record);
    }

    #[test]
    fn sweep_accepts_several_values() {
        let cli = Cli::try_parse_from(["vectify", "sweep", "suite/", "colors", "2", "4", "8"])
            .expect("valid");
        let Some(Command::Sweep {
            parameter, values, ..
        }) = cli.command
        else {
            panic!("expected the sweep subcommand");
        };
        assert_eq!(parameter, "colors");
        assert_eq!(values, vec![2.0, 4.0, 8.0]);
    }

    #[test]
    fn sweep_values_are_optional() {
        let cli = Cli::try_parse_from(["vectify", "sweep", "suite/", "threshold"]).expect("valid");
        let Some(Command::Sweep { values, .. }) = cli.command else {
            panic!("expected the sweep subcommand");
        };
        assert!(
            values.is_empty(),
            "an empty ladder falls back to the defaults"
        );
    }

    #[test]
    fn golden_writes_into_a_directory() {
        let cli = Cli::try_parse_from(["vectify", "golden", "tests/golden/images"]).expect("valid");
        let Some(Command::Golden { directory }) = cli.command else {
            panic!("expected the golden subcommand");
        };
        assert_eq!(directory.to_str(), Some("tests/golden/images"));
    }

    #[test]
    fn bare_invocation_leaves_input_to_the_binary_to_reject() {
        let cli = Cli::try_parse_from(["vectify"]).expect("valid");
        assert!(cli.input.is_none());
        assert!(cli.command.is_none());
    }
}
