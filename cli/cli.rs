//! The command line arguments (spec 9).
//!
//! The doc comments of the items below are also the help messages of clap.
//! The first paragraph is shown by `-h`, and all paragraphs are shown by `--help`.

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum, error::ErrorKind};
use mosty::{Error, Result};
use std::path::PathBuf;

/// Detects misaligned cross-sheet references in grade workbooks.
///
/// mosty: MOu Seiseki Teisei ha Yadayo.
#[derive(Debug, Parser)]
#[command(
    version,
    after_help = "Run `mosty init` to write the config files, review them, then run `mosty check`."
)]
pub struct MostyApp {
    /// Specify the log level
    #[arg(short, long, value_name = "LEVEL", value_enum, default_value_t = Level::Warn,
        global = true, display_order = 100, ignore_case = true)]
    level: Level,

    /// The subcommand to run.
    #[command(subcommand)]
    pub commands: MostyCommand,
}

/// The subcommands.
#[derive(Debug, Subcommand)]
pub enum MostyCommand {
    /// Analyze the Excel files and write the config files (pass 1)
    ///
    /// The config files describe which rows represent which students in each sheet.
    /// Review them, and fix them if they are wrong.
    #[command(name = "init")]
    Init(InitOpts),

    /// Verify the cross-sheet references with the config files (pass 2)
    ///
    /// If the config file is missing, `init` runs first unless `--no-init` is given.
    #[command(name = "check", visible_alias = "verify")]
    Check(CheckOpts),
}

/// The options of `mosty init`.
#[derive(Debug, Args)]
pub struct InitOpts {
    /// Specify the pattern of student ids
    ///
    /// It must match the whole cell. A named group `id` (e.g., `^[A-Za-z]*(?<id>[0-9]{7})$`)
    /// is used to compare the ids.
    #[arg(short = 'p', long = "id-pattern", value_name = "REGEX",
        default_value_t = String::from("^[0-9]{7}$"))]
    pub pattern: String,

    /// Specify the last column index (0-origin) to search student ids and names
    ///
    /// Columns 0..=INDEX are searched (2 means A to C).
    #[arg(
        short = 'C',
        long = "max-column",
        value_name = "INDEX",
        default_value_t = 2
    )]
    pub max_column: u32,

    /// Specify the destination of the config file (`-` means stdout)
    ///
    /// Available only for a single Excel file. [default: mosty.<EXCEL_FILE>.json5]
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Overwrite the existing config files
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// The target Excel files (.xlsx, .xlsm)
    #[arg(value_name = "EXCEL_FILEs", required = true)]
    pub files: Vec<PathBuf>,
}

/// The options of `mosty check`.
#[derive(Debug, Args)]
pub struct CheckOpts {
    /// Specify the config file
    ///
    /// Available only for a single Excel file.
    /// [default: mosty.<EXCEL_FILE>.json5 in the same directory as the Excel file]
    #[arg(short, long = "config", value_name = "FILE")]
    pub config_file: Option<PathBuf>,

    /// Specify the pattern of student ids
    ///
    /// It overrides `id_pattern` in the config file.
    #[arg(short = 'p', long = "id-pattern", value_name = "REGEX")]
    pub pattern: Option<String>,

    /// Exit with an error when the config file is missing, instead of running `init`
    #[arg(long = "no-init", default_value_t = false)]
    pub no_init: bool,

    /// Specify the destination of the results (`-` means stdout) [default: -]
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Specify the output format of the results
    #[arg(short = 'F', long, value_name = "FORMAT", value_enum,
        default_value_t = Format::Default, ignore_case = true)]
    pub format: Format,

    /// The target Excel files (.xlsx, .xlsm)
    #[arg(value_name = "EXCEL_FILEs", required = true)]
    pub files: Vec<PathBuf>,
}

/// The output formats of the results. It is converted into [`mosty::output::Format`].
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    /// For reading in terminals
    Default,
    /// For processing with other tools
    Json,
    /// For pasting into reports and notes
    Markdown,
}

impl From<Format> for mosty::output::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::Default => mosty::output::Format::Default,
            Format::Json => mosty::output::Format::Json,
            Format::Markdown => mosty::output::Format::Markdown,
        }
    }
}

impl MostyApp {
    /// Validates the arguments, and initializes the logger with the log level.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Clap`] if the arguments violate the constraints (see `validate`).
    pub fn init(&self) -> Result<()> {
        self.validate()?;
        let filter = level_to_log_filter(&self.level);
        env_logger::Builder::new().filter_level(filter).init();
        Ok(())
    }

    /// Checks the constraints between arguments that clap cannot express.
    fn validate(&self) -> Result<()> {
        match &self.commands {
            MostyCommand::Init(opts) if opts.output.is_some() && opts.files.len() > 1 => Err(
                arg_error("--output is available only for a single Excel file"),
            ),
            MostyCommand::Check(opts) if opts.config_file.is_some() && opts.files.len() > 1 => Err(
                arg_error("--config is available only for a single Excel file"),
            ),
            _ => Ok(()),
        }
    }
}

/// Creates an error of conflicting arguments, which exits with the status 3.
fn arg_error(message: &str) -> Error {
    Error::Clap(MostyApp::command().error(ErrorKind::ArgumentConflict, message))
}

/// Converts the log level into the filter of the logger.
fn level_to_log_filter(level: &Level) -> log::LevelFilter {
    match level {
        Level::Error => log::LevelFilter::Error,
        Level::Warn => log::LevelFilter::Warn,
        Level::Info => log::LevelFilter::Info,
        Level::Debug => log::LevelFilter::Debug,
        Level::Trace => log::LevelFilter::Trace,
        Level::Off => log::LevelFilter::Off,
    }
}

/// The log levels of the diagnostic messages (not of the results).
#[derive(Debug, Clone, ValueEnum)]
enum Level {
    /// Errors only
    Error,
    /// Warnings and errors
    Warn,
    /// Information, such as skipped references
    Info,
    /// Messages for debugging
    Debug,
    /// All messages
    Trace,
    /// No messages
    Off,
}
