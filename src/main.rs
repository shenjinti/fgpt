use clap::Parser;
use std::{io::Write, sync::Arc};
#[cfg(feature = "cli")]
mod cli;
mod fgpt;
#[cfg(feature = "proxy")]
mod proxy;

#[derive(Clone)]
pub(crate) struct AppState {
    pub device_id: String,
    pub code: bool,
    pub model: String,
    pub lang: String,
    pub proxy: Option<String>,
    pub qusetion: Option<String>,
    pub input_file: Option<String>,
    pub repl: bool,
    pub dump_stats: bool,
}

impl AppState {
    pub fn new(args: &Args) -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            code: args.code,
            qusetion: args.question.clone(),
            input_file: args.file.clone(),
            repl: args.repl,
            dump_stats: args.stats,
            proxy: args.proxy.clone(),
            lang: args.lang.as_ref().unwrap_or(&"en-US".to_string()).clone(),
            model: args
                .model
                .as_ref()
                .unwrap_or(&"text-davinci-002-render-sha".to_string())
                .clone(),
        }
    }
}
type StateRef = Arc<AppState>;

#[derive(Parser, Debug)]
#[command(version)]
pub(crate) struct Args {
    #[clap(help = "Your help message")]
    question: Option<String>,
    #[clap(
        long,
        default_value = "text-davinci-002-render-sha",
        help = "Default model"
    )]
    model: Option<String>,

    #[clap(long, default_value = "en-US", help = "Language")]
    lang: Option<String>,

    #[clap(long, short, help = "Via proxy server address")]
    proxy: Option<String>,

    #[clap(long, help = "The file to write the log to")]
    log_file: Option<String>,

    #[clap(
        long,
        default_value = "info",
        help = "Log level: trace, debug, info, warn, error"
    )]
    log_level: String,

    #[cfg(feature = "cli")]
    #[clap(long, short, help = "Result as plain code")]
    code: bool,

    #[cfg(feature = "cli")]
    #[clap(long, short, help = "File to read from")]
    file: Option<String>,

    #[cfg(feature = "cli")]
    #[clap(long, help = "Interactive REPL mode")]
    repl: bool,

    #[cfg(feature = "cli")]
    #[clap(long, help = "Dump stats to stdout")]
    stats: bool,

    #[cfg(feature = "proxy")]
    #[clap(long, short, help = "Serve the proxy at the given address")]
    serve: Option<String>,

    #[cfg(feature = "proxy")]
    #[clap(long, default_value = "/v1")]
    prefix: Option<String>,

    #[cfg(feature = "proxy")]
    #[clap(long, default_value = "false", help = "Disable CORS access control")]
    disable_cors: bool,
}

fn init_log(level: &String, is_test: bool, log_file_name: &Option<String>) {
    let target = match log_file_name
        .as_ref()
        .map(|log_file| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file)
                .ok()
        })
        .unwrap_or_default()
    {
        Some(log_file) => Box::new(log_file),
        None => Box::new(std::io::stdout()) as Box<dyn std::io::Write + Send>,
    };

    let _ = env_logger::builder()
        .is_test(is_test)
        .format(|buf, record| {
            let short_file_name = record
                .file()
                .unwrap_or("unknown")
                .split('/')
                .last()
                .unwrap_or("unknown");

            writeln!(
                buf,
                "{} [{}] {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                short_file_name,
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(target))
        .format_timestamp(None)
        .filter_level(level.parse().unwrap())
        .try_init();
}

#[tokio::main]
pub async fn main() -> Result<(), crate::fgpt::Error> {
    let args = Args::parse();
    init_log(&args.log_level, false, &args.log_file);

    let state = Arc::new(AppState::new(&args));

    #[cfg(feature = "proxy")]
    if args.serve.is_some() {
        return proxy::serve(state).await;
    }

    #[cfg(feature = "cli")]
    cli::run(state).await
}
