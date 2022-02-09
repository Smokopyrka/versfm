mod view;

use std::{io::{self, Stdout}, time::{Duration, Instant}, sync::mpsc::{self, Receiver}, thread, error::Error};
use crossterm::{event::{self, Event as CEvent, KeyCode, KeyEvent}, terminal::enable_raw_mode};
use tui::{
    backend::{CrosstermBackend},
    Terminal,
};
use view::screens::MainScreen;

enum Event<I> {
    Input(I),
    Shutdown,
    Tick,
}

pub struct App {
    main_screen: MainScreen,
    input_channel: Receiver<Event<KeyEvent>>,
}

impl App {
    fn spawn_sender() -> Receiver<Event<KeyEvent>> {
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(200);

        thread::spawn(move || {
            let mut last_tick = Instant::now();

            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).expect("timeout") {
                    if let CEvent::Key(key) = event::read().expect("key") {
                            if key.code == KeyCode::Esc {
                                tx.send(Event::Shutdown).expect("Can send events");
                            } else {
                                tx.send(Event::Input(key)).expect("Can send events");
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

    pub fn new() -> App {
        let input_channel = App::spawn_sender();
        let terminal = App::capture_terminal().unwrap();
        let main_screen = MainScreen::new(terminal);
        App { main_screen, input_channel }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.main_screen.render()?;
        loop {
            match self.input_channel.recv().unwrap() {
                Event::Input(event) => self.handle_key(event),
                Event::Shutdown => {
                    self.main_screen.shutdown()?;
                    break;
                }
                Event::Tick => ()
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key_event: KeyEvent) {
            self.main_screen.handle_event(key_event);
            self.main_screen.render().unwrap()
    }
}