pub mod providers;
mod view;

use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent},
    terminal::enable_raw_mode,
};
use providers::s3::S3Provider;
use std::{
    error::Error,
    io::{self, Stdout},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use tui::{backend::CrosstermBackend, Terminal};
use view::{
    components::{FileList, FilesystemList, S3List},
    screens::DualPaneList,
};

#[derive(Clone)]
pub enum Kind {
    File,
    Directory,
}

enum Event<I> {
    Input(I),
    Shutdown,
    Tick,
}

pub struct App {
    main_screen: DualPaneList,
    input_channel: Receiver<Event<KeyEvent>>,
}

impl App {
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

    pub async fn new(bucket_name: &str) -> App {
        let input_channel = App::spawn_sender();
        let terminal = App::capture_terminal().expect("Coudn't capture terminal");
        let s3_client: Box<dyn FileList> =
            Box::new(S3List::new(S3Provider::new(bucket_name).await));
        let fs_client: Box<dyn FileList> = Box::new(FilesystemList::new());
        let main_screen = DualPaneList::new(terminal, s3_client, fs_client).await;
        App {
            main_screen,
            input_channel,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.input_channel.recv().unwrap() {
                Event::Input(event) => self.main_screen.handle_event(event).await,
                Event::Shutdown => {
                    self.main_screen.shutdown()?;
                    break;
                }
                Event::Tick => self
                    .main_screen
                    .render()
                    .expect("Couldn't render DualPaneList screen"),
            }
        }
        Ok(())
    }
}
