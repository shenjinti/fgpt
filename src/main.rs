use clap::Parser;
use std::io::Write;

mod cli;
mod proxy;
mod rgpt;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[clap(long, short, help = "Via proxy server address")]
    proxy: Option<String>,

    #[cfg(feature = "cli")]
    #[clap(long, short, help = "Result as plain code")]
    code: bool,

    #[cfg(feature = "cli")]
    #[clap(long, short, help = "File to read from")]
    file: Option<String>,

    #[cfg(feature = "cli")]
    #[clap(long, help = "Interactive REPL mode")]
    repl: bool,

    #[cfg(feature = "proxy")]
    #[clap(long, short, help = "Serve the proxy at the given address")]
    serve: Option<String>,

    #[cfg(feature = "proxy")]
    #[clap(long, help = "The file to write the log to")]
    log_file: Option<String>,

    #[cfg(feature = "proxy")]
    #[clap(
        long,
        default_value = "info",
        help = "Log level: trace, debug, info, warn, error"
    )]
    log_level: String,

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
pub async fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();
    init_log(&args.log_level, false, &args.log_file);

    if args.serve.is_some() {
        proxy::serve(args).await
    } else {
        cli::run(args).await
    }
}
