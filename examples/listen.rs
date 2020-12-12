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
