use std::env;
use std::error::Error;
use std::sync::Arc;

use s3tui::providers::s3::S3Provider;
use s3tui::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let client = Arc::new(S3Provider::new(args[1].as_str()).await);
    let mut app = App::new(client.clone()).await;
    app.run().await?;
    Ok(())
}
