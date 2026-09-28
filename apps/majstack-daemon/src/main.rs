use clap::Parser;
use majstack_config::MajstackPaths;
use majstack_core::Result;
use majstack_orchestration::{Engine, RunOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "majstackd",
    version,
    about = "Majstack daemon: processes queued goals and long-running work."
)]
struct Cli {
    #[arg(long, default_value = ".")]
    path: PathBuf,
    #[arg(long, default_value_t = 5)]
    interval: u64,
    #[arg(long, help = "Process the queue once and exit")]
    once: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = MajstackPaths::discover(&cli.path);
    paths.ensure()?;
    let queue = paths.dir.join("queue");
    let done = queue.join("done");
    std::fs::create_dir_all(&queue)?;
    std::fs::create_dir_all(&done)?;

    let running = Arc::new(AtomicBool::new(true));
    let handler_flag = running.clone();
    let _ = ctrlc::set_handler(move || {
        handler_flag.store(false, Ordering::SeqCst);
    });

    println!("majstackd watching {}", queue.display());
    loop {
        if !running.load(Ordering::SeqCst) {
            println!("majstackd shutting down");
            break;
        }
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&queue)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("goal"))
            .collect();
        entries.sort();
        let processed = entries.len();
        for file in entries {
            let goal = std::fs::read_to_string(&file)?;
            println!("majstackd: running goal from {}", file.display());
            match Engine::open(&cli.path) {
                Ok(engine) => match engine.run_goal(&goal, RunOptions::default()) {
                    Ok(outcome) => {
                        println!("majstackd: {} ({})", outcome.state.as_str(), outcome.run_id)
                    }
                    Err(error) => println!("majstackd: error: {error}"),
                },
                Err(error) => println!("majstackd: cannot open engine: {error}"),
            }
            let destination = done.join(file.file_name().unwrap_or_default());
            let _ = move_file(&file, &destination);
        }
        if cli.once {
            break;
        }
        if processed == 0 {
            std::thread::sleep(Duration::from_secs(cli.interval));
        }
    }
    Ok(())
}

fn move_file(source: &std::path::Path, destination: &std::path::Path) -> std::io::Result<()> {
    if std::fs::rename(source, destination).is_ok() {
        return Ok(());
    }
    std::fs::copy(source, destination)?;
    std::fs::remove_file(source)
}
