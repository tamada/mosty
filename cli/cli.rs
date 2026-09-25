use clap::{CommandFactory, Parser, ValueEnum, error::ErrorKind};
use mosty::{Error, Result};
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(
    version,
    about = "mosty: MOu Seiseki Teisei ha Yadayo\nDetects misaligned cross-sheet references in grade workbooks.",
    after_help = "Run `mosty init` to write the config files, review them, then run `mosty check`."
)]
pub struct MostyApp {
    #[clap(short, long, value_name = "LEVEL", value_enum, default_value_t = Level::Warn, global = true, display_order = 100,
        help = "Specify the log level", ignore_case = true)]
    level: Level,

    #[clap(subcommand)]
    pub commands: MostyCommand,
}

#[derive(Debug, Parser)]
pub enum MostyCommand {
    #[command(
        name = "init",
        about = "Analyze the Excel files and write the config files (pass 1)"
    )]
    Init(InitOpts),

    #[command(
        name = "check",
        visible_alias = "verify",
        about = "Verify the cross-sheet references with the config files (pass 2)"
    )]
    Check(CheckOpts),
}

#[derive(Debug, Parser)]
pub struct InitOpts {
    #[clap(
        short = 'p',
        long = "id-pattern",
        value_name = "REGEX",
        default_value_t = String::from("^[0-9]{7}$"),
        help = "Specify the pattern of student ids. It must match the whole cell.\n\
                A named group `id' (e.g., `^[A-Za-z]*(?<id>[0-9]{7})$') is used to compare the ids."
    )]
    pub pattern: String,

    #[clap(
        short = 'C',
        long = "max-column",
        value_name = "INDEX",
        default_value_t = 2,
        help = "Specify the last column index (0-origin) to search student ids and names.\n\
                Columns 0..=INDEX are searched (2 means A to C)."
    )]
    pub max_column: u32,

    #[clap(
        short,
        long,
        value_name = "FILE",
        help = "Specify the destination of the config file. `-' means stdout.\n\
                Available only for a single Excel file. [default: mosty.<EXCEL_FILE>.json5]"
    )]
    pub output: Option<PathBuf>,

    #[clap(
        short,
        long,
        default_value_t = false,
        help = "Overwrite the existing config files"
    )]
    pub force: bool,

    #[clap(
        value_name = "EXCEL_FILEs",
        required = true,
        help = "The target Excel files (.xlsx, .xlsm)"
    )]
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct CheckOpts {
    #[clap(
        short,
        long = "config",
        value_name = "FILE",
        help = "Specify the config file. Available only for a single Excel file.\n\
                [default: mosty.<EXCEL_FILE>.json5 in the same directory as the Excel file]"
    )]
    pub config_file: Option<PathBuf>,

    #[clap(
        short = 'p',
        long = "id-pattern",
        value_name = "REGEX",
        help = "Specify the pattern of student ids. It overrides `id_pattern' in the config file."
    )]
    pub pattern: Option<String>,

    #[clap(
        long = "no-init",
        default_value_t = false,
        help = "Exit with an error when the config file is missing, instead of running `init'"
    )]
    pub no_init: bool,

    #[clap(
        short,
        long,
        value_name = "FILE",
        help = "Specify the destination of the results. `-' means stdout. [default: -]"
    )]
    pub output: Option<PathBuf>,

    #[clap(
        short = 'F',
        long,
        value_name = "FORMAT",
        value_enum,
        default_value_t = Format::Default,
        ignore_case = true,
        help = "Specify the output format of the results"
    )]
    pub format: Format,

    #[clap(
        value_name = "EXCEL_FILEs",
        required = true,
        help = "The target Excel files (.xlsx, .xlsm)"
    )]
    pub files: Vec<PathBuf>,
}

#[derive(Debug, ValueEnum, Parser, Clone)]
pub enum Format {
    Default,
    Json,
    Markdown,
}

impl MostyApp {
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

fn arg_error(message: &str) -> Error {
    Error::Clap(MostyApp::command().error(ErrorKind::ArgumentConflict, message))
}

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

#[derive(Parser, Debug, ValueEnum, Clone)]
enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Off,
}
