use std::env;
use std::error::Error;

use versfm::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mut app = App::new(&args[1]).await;
    app.run().await?;
    Ok(())
}
