use std::thread;
use std::time::{Duration, Instant};
use btleplug::bluez::manager::Manager;
use btleplug::api::{UUID, Central, Peripheral, BDAddr, CentralEvent, Characteristic};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use btleplug::bluez::adapter::ConnectedAdapter;

pub fn test() -> Result<(), btleplug::Error> {
    let addr = BDAddr {
        address: [0xAA, 0xDA, 0x56, 0x70, 0x4D, 0x1C]
    };
    let adapter = adapter_by_mac(addr)?;

    // reset the adapter -- clears out any errant state

    let addr = BDAddr {
        address: [0xD4, 0xF3, 0x35, 0x34, 0x2D, 0x58]
    };

    let mut device = Device::discover(&adapter, addr).unwrap();

    println!("connecting");
    device.connect()?;
    let _ = dbg!(device.get_firmware());
    let _ = dbg!(device.get_battery());
    Ok(())
}

pub fn adapter_by_mac(addr: BDAddr) -> Result<ConnectedAdapter, btleplug::Error> {
    let manager = Manager::new()?;
    let adapters = manager.adapters()?;

    let mut adapter = adapters.into_iter().find(|adapter| adapter.addr == addr).ok_or(btleplug::Error::DeviceNotFound)?;

    adapter = manager.down(&adapter).unwrap();
    adapter = manager.up(&adapter).unwrap();

    adapter.connect()
}

const UUID_BATTERY: UUID = UUID::B16(0x2A19);
const UUID_FIRMWARE: UUID = UUID::B16(0x2A26);
const UUID_WRITE_SENSOR: UUID = UUID::B16(0x2A05);

struct Device<P: Peripheral> {
    peripheral: P,
    connected: bool,
}

impl<P: Peripheral> Device<P> {
    pub fn discover<C: Central<P>>(adapter: &C, mac: BDAddr) -> Result<Self, btleplug::Error> {
        let found = Arc::new(AtomicBool::new(false));
        adapter.start_scan()?;
        let device_count_clone = found.clone();

        adapter.on_event(Box::new(move |ev| {
            match ev {
                CentralEvent::DeviceDiscovered(discovered_mac) if mac == discovered_mac => {
                    device_count_clone.store(true, Ordering::Relaxed);
                }
                _ => {}
            };
        }));

        for _ in 0..1500 {
            thread::sleep(Duration::from_millis(100));
            if found.load(Ordering::Relaxed) {
                break;
            }
        }

        adapter.stop_scan()?;

        match adapter.peripheral(mac) {
            Some(peripheral) => Ok(Device {
                peripheral,
                connected: false,
            }),
            None => Err(btleplug::Error::DeviceNotFound)
        }
    }

    pub fn connect(&mut self) -> Result<(), btleplug::Error> {
        if self.connected {
            return Ok(());
        }

        let start = Instant::now();
        for _ in 0..3 {
            match self.peripheral.connect() {
                Ok(_) => {
                    self.connected = true;

                    self.peripheral.discover_characteristics()?;
                    return Ok(());
                },
                Err(e) => eprintln!("{}", e)
            }
        }

        let duration = Instant::now().duration_since(start);

        Err(btleplug::Error::TimedOut(duration))
    }

    fn get_characteristic(&self, uuid: UUID) -> Option<Characteristic> {
        self.peripheral.characteristics().into_iter().find(|c| c.uuid == uuid)
    }

    pub fn get_battery(&self) -> Result<u8, btleplug::Error> {
        let bat = self.get_characteristic(UUID_BATTERY).unwrap();
        let data = self.peripheral.read(&bat)?;

        Ok(data[1])
    }

    pub fn get_firmware(&self) -> Result<String, btleplug::Error> {
        let bat = self.get_characteristic(UUID_FIRMWARE).unwrap();
        let data = self.peripheral.read(&bat)?;
        let data = data[1..].to_vec();
        Ok(String::from_utf8(data).unwrap())
    }
}