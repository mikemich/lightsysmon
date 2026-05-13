mod cli;
mod config;
mod display;
mod interactive;
mod metrics;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let config = config::load_config(cli.config.as_deref());
    let command = cli.command.or_else(|| interactive::prompt_command(&config));

    match command {
        Some(Commands::Watch(args)) => {
            if args.tui {
                display::tui::run_tui(&args, &config);
            } else {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build tokio runtime");
                runtime.block_on(display::plain::run_watch(&args, &config));
            }
        }
        Some(Commands::Top(args)) => {
            display::plain::run_top(&args, &config);
        }
        Some(Commands::Info) => {
            display::plain::print_info();
        }
        Some(Commands::Snapshot(args)) => {
            display::plain::run_snapshot(&args, &config);
        }
        Some(Commands::Completions(args)) => {
            use clap::CommandFactory;
            use clap_complete::generate;
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "lightsysmon", &mut std::io::stdout());
        }
        None => {
            println!("No action selected.");
        }
    }
}
