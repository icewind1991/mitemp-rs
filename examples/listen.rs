use btleplug::api::Manager as _;
use btleplug::platform::Manager;
use futures_util::StreamExt;
use main_error::MainError;
use mitemp::{listen};

use tokio::pin;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    env_logger::init();

    let manager = Manager::new().await?;
    let adapter = manager.adapters().await?.pop().unwrap();

    let stream = listen(&adapter).await?;
    pin!(stream);

    while let Some(sensor) = stream.next().await {
        println!("{}: {:?}", sensor.mac, sensor.data);
    }
    Ok(())
}
