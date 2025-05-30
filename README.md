# Moved to https://codeberg.org/icewind/mitemp-rs

# mitemp.rs

Read Xiaomi MI Temperature and Humidity Sensor over BLE

## Usage

```rust
use btleplug::api::Manager as _;
use btleplug::platform::Manager;
use mitemp::listen;
use tokio::pin;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), btleplug::Error> {
    let manager = Manager::new().await?;
    let adapter = manager.adapters().await?.pop().unwrap();

    let stream = listen(&adapter).await?;
    pin!(stream);

    while let Some(sensor) = stream.next().await {
        println!("{}: {:?}", sensor.mac, sensor.data);
    }
    Ok(())
}
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
  at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
