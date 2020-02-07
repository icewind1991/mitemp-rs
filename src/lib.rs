#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

mod cipher;

use btleplug::api::{
    BDAddr, Central, CentralEvent, CharPropFlags, Characteristic, Peripheral, UUID,
};
use btleplug::bluez::adapter::ConnectedAdapter;
use btleplug::bluez::manager::Manager;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub fn test() -> Result<(), btleplug::Error> {
    //    let addr = BDAddr::from_str("1C:4D:70:56:DA:AA").unwrap();
    let addr = BDAddr::from_str("00:1A:7D:DA:71:08").unwrap();
    //    let addr = BDAddr::from_str("40:4E:36:BF:E1:45").unwrap();
    let adapter = adapter_by_mac(addr)?;

    dbg!("discovering");

    let addr = BDAddr::from_str("58:2d:34:35:f3:d4").unwrap();

    let mut device = Device::discover(&adapter, addr).unwrap();

    println!("connecting");
    device.connect()?;

    device.auth([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])?;
    let _ = dbg!(device.get_firmware());
    let _ = dbg!(device.get_battery());
    device.listen(|| {})
}

pub fn adapter_by_mac(addr: BDAddr) -> Result<ConnectedAdapter, btleplug::Error> {
    let manager = Manager::new()?;
    let adapters = manager.adapters()?;

    let mut adapter = adapters
        .into_iter()
        .find(|adapter| adapter.addr == addr)
        .ok_or(btleplug::Error::DeviceNotFound)?;
    //
    //    adapter = manager.down(&adapter).unwrap();
    //    adapter = manager.up(&adapter).unwrap();

    adapter.connect()
}

const UUID_DEVICE_NAME: UUID = UUID::B16(0x2A00);
const UUID_APPEARANCE: UUID = UUID::B16(0x2A01);
const UUID_BATTERY: UUID = UUID::B16(0x2A19);
const UUID_MODEL: UUID = UUID::B16(0x2A24);
const UUID_SERIAL: UUID = UUID::B16(0x2A25);
const UUID_FIRMWARE: UUID = UUID::B16(0x2A26);
const UUID_REVISION: UUID = UUID::B16(0x2A27);
const UUID_MANUFACTURER: UUID = UUID::B16(0x2A29);
const UUID_WRITE_SENSOR: UUID = UUID::B16(0x2A05);
const UUID_AUTH_INIT: UUID = UUID::B16(0x0010);
const UUID_AUTH: UUID = UUID::B16(0x0001);
const UUID_AUTH_VER: UUID = UUID::B16(0x0004);

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
                    //                    println!("discovered {}", discovered_mac);
                    //                    device_count_clone.store(true, Ordering::Relaxed);
                }
                _ => {}
            };
        }));

        for _ in 0..150 {
            thread::sleep(Duration::from_millis(100));
            if found.load(Ordering::Relaxed) {
                break;
            }
        }

        adapter.stop_scan()?;

        match adapter.peripheral(mac) {
            Some(peripheral) => Ok(Device {
                peripheral: dbg!(peripheral),
                connected: false,
            }),
            None => Err(btleplug::Error::DeviceNotFound),
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
                    //                    dbg!(self.peripheral.characteristics());
                    return Ok(());
                }
                Err(e) => eprintln!("{}", e),
            }
        }

        let duration = Instant::now().duration_since(start);

        Err(btleplug::Error::TimedOut(duration))
    }

    fn get_characteristic(&self, uuid: UUID) -> Option<Characteristic> {
        self.peripheral
            .characteristics()
            .into_iter()
            .find(|c| c.uuid == uuid)
    }

    fn get_u8(&self, uuid: UUID) -> Result<u8, btleplug::Error> {
        let c = self.get_characteristic(uuid).unwrap();
        let data = self.peripheral.read(&c)?;

        Ok(data[1])
    }

    fn get_u16(&self, uuid: UUID) -> Result<u16, btleplug::Error> {
        let c = self.get_characteristic(uuid).unwrap();
        let data = self.peripheral.read(&c)?;
        dbg!(&data);

        Ok(u16::from_le_bytes([data[1], data[2]]))
    }

    pub fn get_battery(&self) -> Result<u8, btleplug::Error> {
        self.get_u8(UUID_BATTERY)
    }

    fn get_string(&self, uuid: UUID) -> Result<String, btleplug::Error> {
        let c = self.get_characteristic(uuid).unwrap();
        let data = self.peripheral.read(&c)?;
        let data = data[1..].to_vec();
        Ok(String::from_utf8(data).unwrap())
    }

    pub fn get_firmware(&self) -> Result<String, btleplug::Error> {
        self.get_string(UUID_FIRMWARE)
    }

    pub fn get_name(&self) -> Result<String, btleplug::Error> {
        self.get_string(UUID_DEVICE_NAME)
    }

    pub fn auth(&self, token: [u8; 12]) -> Result<(), btleplug::Error> {
        dbg!("auth");
        dbg!(self.peripheral.request(
            &self.get_characteristic(UUID_AUTH_INIT).unwrap(),
            //            &[0x90, 0xCA, 0x85, 0xDE],
            &[0x90, 0xCA, 0x85, 0xDE],
        ));

        let (auth_in, auth_out) = channel();
        self.peripheral
            .subscribe(&self.get_characteristic(UUID_AUTH).unwrap())?;
        self.peripheral
            .on_notification(Box::new(move |notification| {
                dbg!(notification.uuid);
                if notification.uuid == UUID_AUTH {
                    auth_in.send(notification.value).unwrap();
                }
            }));

        let mac = self.peripheral.address().address;
        let device_id = 426;
        //        let device_id = 131;
        let data = cipher::cipher(&cipher::mix_a(mac, device_id), &token);
        self.peripheral
            .request(&self.get_characteristic(UUID_AUTH).unwrap(), &data);

        let auth_response = auth_out.recv().unwrap();
        let verify = cipher::cipher(
            &cipher::mix_b(mac, device_id),
            &cipher::cipher(&cipher::mix_a(mac, device_id), &auth_response),
        );
        assert_eq!(&token, verify.as_slice());
        //                dbg!(self.peripheral.command(
        //                    &self.get_characteristic(UUID_AUTH).unwrap(),
        //                    &[0x92, 0xAB, 0x54, 0xFA],
        //                ));
        dbg!(self.peripheral.request(
            &self.get_characteristic(UUID_AUTH).unwrap(),
            &[0x92, 0xAB, 0x54, 0xFA],
        ));

        dbg!(&self
            .peripheral
            .read(&self.get_characteristic(UUID_AUTH_VER).unwrap())?);

        Ok(())
    }

    pub fn listen(&self, callback: impl Fn() -> ()) -> Result<(), btleplug::Error> {
        let uuid = UUID::from_str("22:6C:AA:55:64:76:45:66:75:62:66:73:44:70:66:6D").unwrap();
        let sensor = self.get_characteristic(uuid).unwrap();
        eprintln!("sub");
        self.peripheral.subscribe(&sensor)?;
        eprintln!("notify");
        self.peripheral.on_notification(Box::new(|notification| {
            dbg!(String::from_utf8(notification.value).unwrap());
        }));
        loop {
            eprintln!("sleep");
            thread::sleep(Duration::from_millis(5000));
            dbg!(self.get_battery());
        }
    }
}
