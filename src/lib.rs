#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

pub use btleplug::api::BDAddr;
use btleplug::api::{Central, CentralEvent};
use btleplug::bluez::adapter::ConnectedAdapter;
use btleplug::bluez::manager::Manager;
use btleplug::bluez::protocol::hci::LEAdvertisingData;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

pub fn listen(adapter: ConnectedAdapter, sensor_mac: BDAddr) -> Receiver<SensorData> {
    let (tx, rx) = channel();

    let mut sensor_data = Arc::new(Mutex::new(SensorData::default()));

    adapter.on_event(Box::new(move |ev| {
        match ev {
            CentralEvent::DeviceDiscovered(discovered_mac, data)
            | CentralEvent::DeviceUpdated(discovered_mac, data)
                if sensor_mac == discovered_mac =>
            {
                //                    dbg!(data);
                for item in data {
                    match item {
                        LEAdvertisingData::ServiceData16(id, data) => {
                            if let Some(sensor_update) = parse_advertising_data(data) {
                                let updated = {
                                    let mut data = sensor_data.lock().unwrap();
                                    data.update(sensor_update);
                                    data.clone()
                                };
                                tx.send(updated).unwrap()
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };
    }));

    adapter.start_scan().unwrap();

    rx
}

#[derive(Default, Clone, Debug)]
pub struct SensorData {
    battery: u8,
    temperature: f32,
    humidity: f32,
}

impl SensorData {
    fn update(&mut self, update: SensorUpdate) {
        match update {
            SensorUpdate::Temperature(temp) => self.temperature = temp,
            SensorUpdate::Humidity(hum) => self.humidity = hum,
            SensorUpdate::Battery(bat) => self.battery = bat,
            SensorUpdate::TemperatureAndHumidity(temp, hum) => {
                self.humidity = hum;
                self.temperature = temp
            }
        }
    }
}

pub fn adapter_by_mac(addr: BDAddr) -> Result<ConnectedAdapter, btleplug::Error> {
    let manager = Manager::new()?;
    let adapters = manager.adapters()?;

    let mut adapter = adapters
        .into_iter()
        .find(|adapter| adapter.addr == addr)
        .ok_or(btleplug::Error::DeviceNotFound)?;

    adapter.connect()
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone, Copy)]
#[repr(u8)]
enum SensorType {
    Temperature = 0x04,
    Humidity = 0x06,
    Battery = 0x0A,
    TemperatureAndHumidity = 0x0D,
}

#[derive(Debug)]
enum SensorUpdate {
    Battery(u8),
    Temperature(f32),
    Humidity(f32),
    TemperatureAndHumidity(f32, f32),
}

fn parse_advertising_data(data: &[u8]) -> Option<SensorUpdate> {
    const SENSOR_HUMIDITY: u8 = 0x06;
    const SENSOR_TEMPERATURE: u8 = 0x04;

    let sensor_type = &data[1..4];
    assert_eq!(sensor_type, &[0x20, 0xaa, 0x01]);
    let sensor_type = SensorType::try_from(data[11]).ok()?;
    let data_length = data[13] as usize;
    assert_eq!(14 + data_length, data.len());
    let sensor_data = &data[14..14 + data_length];
    match sensor_type {
        SensorType::Battery => Some(SensorUpdate::Battery(sensor_data[0])),
        SensorType::Temperature => Some(SensorUpdate::Temperature(
            i16::from_le_bytes([sensor_data[0], sensor_data[1]]) as f32 / 10.0,
        )),
        SensorType::Humidity => Some(SensorUpdate::Humidity(
            u16::from_le_bytes([sensor_data[0], sensor_data[1]]) as f32 / 10.0,
        )),
        SensorType::TemperatureAndHumidity => Some(SensorUpdate::TemperatureAndHumidity(
            i16::from_le_bytes([sensor_data[0], sensor_data[1]]) as f32 / 10.0,
            u16::from_le_bytes([sensor_data[2], sensor_data[3]]) as f32 / 10.0,
        )),
    }
}
