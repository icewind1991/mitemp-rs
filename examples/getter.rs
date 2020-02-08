use mitemp::{adapter_by_mac, BDAddr, Sensor};
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), btleplug::Error> {
    env_logger::init();

    let addr = BDAddr::from_str("00:1A:7D:DA:71:08").unwrap();
    let adapter = adapter_by_mac(addr)?;
    let device = BDAddr::from_str("58:2d:34:35:f3:d4").unwrap();

    let sensor = Sensor::new(adapter, device).start();

    loop {
        let data = sensor.get_data();
        dbg!(data);
        sleep(Duration::from_secs(3));
    }
}
