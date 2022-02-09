use std::error::Error;

use s3tui::App;
fn main() -> Result<(), Box<dyn Error>> {
    let mut app = App::new();
    app.run()?;
    Ok(())
}
