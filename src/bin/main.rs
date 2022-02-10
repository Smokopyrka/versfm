use std::error::Error;

use s3tui::App;
use s3tui::aws::s3::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Cli::new().await;
    let mut app = App::new(&client);
    app.run().await?;
    Ok(())
}
