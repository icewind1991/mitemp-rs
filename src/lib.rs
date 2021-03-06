pub use btleplug::api::BDAddr;
use btleplug::api::{Central, CentralEvent, Peripheral};
use btleplug::platform::{Adapter, Manager};
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use std::convert::TryFrom;

/// Detected mitemp sensor and the data read from it
#[derive(Clone, Debug)]
pub struct Sensor {
    pub mac: BDAddr,
    pub data: SensorData,
}

#[derive(Default, Clone, Copy, Debug)]
struct SensorRawData {
    battery: u8,
    temperature: i16,
    humidity: u16,
}

impl SensorRawData {
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

/// Listen for sensor data
///
/// Returns an iterator that will block waiting for new sensor data
pub fn listen<A: Central + 'static>(
    adapter: A,
) -> Result<impl Iterator<Item = Sensor>, btleplug::Error> {
    let mut sensors: HashMap<BDAddr, SensorRawData> = HashMap::new();

    let event_receiver = adapter.event_receiver().unwrap();

    // start scanning for devices
    adapter.start_scan()?;

    Ok(event_receiver
        .into_iter()
        .filter_map(|event| match event {
            CentralEvent::DeviceDiscovered(bd_addr) | CentralEvent::DeviceUpdated(bd_addr) => {
                Some(bd_addr)
            }
            _ => None,
        })
        .filter_map(move |bd_addr| adapter.peripheral(bd_addr))
        .flat_map(|peripheral| {
            peripheral
                .properties()
                .service_data
                .into_iter()
                .map(move |(_, data)| (peripheral.address(), data))
        })
        .filter_map(|(bd_addr, data)| Some((bd_addr, parse_advertising_data(&data).ok()?)))
        .map(move |(bd_addr, update)| {
            let sensor_data = sensors.entry(bd_addr).or_default();
            sensor_data.update(update);
            Sensor {
                mac: bd_addr,
                data: (*sensor_data).into(),
            }
        }))
}

/// Collected data from a sensor
///
/// Because not all data is emitted at the same time, some fields might not be populated yet
/// in which case they are set to 0
#[derive(Default, Clone, Debug)]
pub struct SensorData {
    /// Battery percentage
    pub battery: u8,
    /// Temperature in °C
    pub temperature: f32,
    /// Humidity in %H
    pub humidity: f32,
}

impl From<SensorRawData> for SensorData {
    fn from(inner: SensorRawData) -> Self {
        SensorData {
            battery: inner.battery,
            temperature: inner.temperature as f32 / 10.0,
            humidity: inner.humidity as f32 / 10.0,
        }
    }
}

pub fn adapter_by_mac(addr: BDAddr) -> Result<Adapter, btleplug::Error> {
    let manager = Manager::new()?;
    manager
        .adapters()?
        .into_iter()
        .find(|adapter| {
            adapter
                .address()
                .ok()
                .filter(|adapter_addr| *adapter_addr == addr)
                .is_some()
        })
        .ok_or(btleplug::Error::DeviceNotFound)
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
    Temperature(i16),
    Humidity(u16),
    TemperatureAndHumidity(i16, u16),
}

struct InvalidServiceData;

fn parse_advertising_data(service_data: &[u8]) -> Result<SensorUpdate, InvalidServiceData> {
    let sensor_type = &service_data[1..4];
    if sensor_type != [0x20, 0xaa, 0x01] {
        return Err(InvalidServiceData);
    }
    let sensor_type = SensorType::try_from(service_data[11]).map_err(|_| InvalidServiceData)?;
    let data_length = service_data[13] as usize;

    if 14 + data_length != service_data.len() {
        return Err(InvalidServiceData);
    }

    let sensor_data = &service_data[14..14 + data_length];
    Ok(match sensor_type {
        SensorType::Battery => SensorUpdate::Battery(sensor_data[0]),
        SensorType::Temperature => {
            SensorUpdate::Temperature(i16::from_le_bytes([sensor_data[0], sensor_data[1]]))
        }
        SensorType::Humidity => {
            SensorUpdate::Humidity(u16::from_le_bytes([sensor_data[0], sensor_data[1]]))
        }
        SensorType::TemperatureAndHumidity => SensorUpdate::TemperatureAndHumidity(
            i16::from_le_bytes([sensor_data[0], sensor_data[1]]),
            u16::from_le_bytes([sensor_data[2], sensor_data[3]]),
        ),
    })
}
