# mitemp.rs

Read Xiaomi MI Temperature and Humidity Sensor over BLE

## Usage

```rust
use mitemp::{adapter_by_mac, listen, BDAddr};
use std::str::FromStr;

fn main() -> Result<(), btleplug::Error> {
    env_logger::init();

    let addr = BDAddr::from_str("00:1A:7D:DA:71:08").unwrap();
    let adapter = adapter_by_mac(addr)?;

    let rx = listen(adapter)?;
    loop {
        let data = rx.recv().unwrap();
        println!("{}: {:?}", data.mac, data.data);
    }
}
```

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.