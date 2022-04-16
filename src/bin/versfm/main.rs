use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent},
    terminal::enable_raw_mode,
};
use std::env;
use std::error::Error;
use std::{
    io::{self, Stdout},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};
use versfm::{
    components::{FileList, FilesystemList, S3List},
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

    thread::spawn(move || {
        let mut last_tick = Instant::now();

        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("Timeout occured while polling event") {
                if let CEvent::Key(key) = event::read().expect("Couldn't read key") {
                    if key.code == KeyCode::Esc {
                        tx.send(Event::Shutdown)
                            .expect("Couldn't send shutdown event");
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

pub async fn run(bucket_name: &str) -> Result<(), Box<dyn Error>> {
    let input_channel = spawn_sender();
    let terminal = capture_terminal().expect("Coudn't capture terminal");
    let s3_client: Box<dyn FileList> = Box::new(S3List::new(S3Provider::new(bucket_name).await));
    let fs_client: Box<dyn FileList> = Box::new(FilesystemList::new());
    let mut main_screen = DualPaneList::new(terminal, s3_client, fs_client).await;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let result = run(&args[1]).await;
    result
}
