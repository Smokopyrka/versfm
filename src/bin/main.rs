use std::env;
use std::error::Error;

use s3tui::aws::s3::Cli;
use s3tui::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let client = Cli::new(args[1].as_str()).await;
    let mut app = App::new(&client).await;
    app.run().await?;
    Ok(())
}
