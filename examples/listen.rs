use btleplug::api::Manager as _;
use btleplug::platform::Manager;
use main_error::MainError;
use mitemp::listen;
use tokio::pin;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    tracing_subscriber::fmt::init();

    let manager = Manager::new().await?;
    let adapter = manager.adapters().await?.pop().unwrap();

    let stream = listen(&adapter).await?;
    pin!(stream);

    while let Some(sensor) = stream.next().await {
        println!("{}: {:?}", sensor.mac, sensor.data);
    }
    Ok(())
}
