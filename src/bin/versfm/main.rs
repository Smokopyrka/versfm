use clap::Parser;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent},
    terminal::enable_raw_mode,
};
use rusoto_core::Region;
use std::{error::Error, process, str::FromStr};
use std::{
    io::{self, Stdout},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};
use versfm::{
    components::{FileCRUDListWidget, FilesystemList, S3List},
    providers::s3::S3Provider,
    screens::DualPaneList,
};

enum Event<I> {
    Input(I),
    Shutdown,
    Tick,
}

fn spawn_sender() -> Receiver<Event<KeyEvent>> {
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(75);

    tokio::spawn(async move {
        let mut last_tick = Instant::now();

        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("Timeout ocurred while polling event") {
                if let CEvent::Key(key) = event::read().expect("Couldn't read key") {
                    if key.code == KeyCode::Esc {
                        tx.send(Event::Shutdown)
                            .expect("Couldn't send shutdown event");
                        break;
                    } else {
                        tx.send(Event::Input(key))
                            .expect("Couldn't send user input event");
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });
    rx
}

fn capture_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

async fn get_pane(pane_str: &str) -> Box<dyn FileCRUDListWidget> {
    match pane_str {
        "s3" => {
            let s3_args = Args::parse();
            if s3_args.aws_region.is_none() || s3_args.s3_bucket_name.is_none() {
                println!("Error: Please provide a valid name of the bucket you want to connect to, and the region it is located in");
                process::exit(1);
            }
            if let Ok(region) = Region::from_str(&s3_args.aws_region.unwrap()) {
                Box::new(S3List::new(
                    S3Provider::new(&s3_args.s3_bucket_name.unwrap(), region).await,
                ))
            } else {
                println!("Error: Provided AWS region is incorrect");
                process::exit(1);
            }
        }
        "fs" => Box::new(FilesystemList::new()),
        _ => {
            println!("Error: Please provide a valid provider");
            process::exit(1);
        }
    }
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let left_pane = get_pane(&args.left_pane).await;
    let right_pane = get_pane(&args.right_pane).await;

    let terminal = capture_terminal().expect("Couldn't capture terminal");
    let mut main_screen = DualPaneList::new(terminal, left_pane, right_pane).await;

    let input_channel = spawn_sender();
    loop {
        match input_channel.recv().unwrap() {
            Event::Input(event) => main_screen.handle_event(event).await,
            Event::Shutdown => {
                main_screen.shutdown()?;
                break;
            }
            Event::Tick => main_screen
                .render()
                .expect("Couldn't render DualPaneList screen"),
        }
    }
    Ok(())
}

/// VersFM - A versatile file manager written in Rust
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about=None)]
struct Args {
    /// Provider for the left pane [Options: "fs", "s3"]
    #[clap(long, short, default_value = "fs")]
    left_pane: String,
    /// Provider for the right pane [Options: "fs", "s3"]
    #[clap(long, short, default_value = "fs")]
    right_pane: String,
    /// Name of the aws region your bucket is located in
    #[clap(long)]
    aws_region: Option<String>,
    /// Name of the bucket you want to connect to
    #[clap(long)]
    s3_bucket_name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    run().await?;
    Ok(())
}
