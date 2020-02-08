pub use btleplug::api::BDAddr;
use btleplug::api::{Central, CentralEvent};
use btleplug::bluez::adapter::ConnectedAdapter;
use btleplug::bluez::manager::Manager;
use btleplug::bluez::protocol::hci::LEAdvertisingData;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub trait SensorState {}

pub struct Inactive {}

impl SensorState for Inactive {}

pub struct Active {}

impl SensorState for Active {}

pub struct Sensor<State: SensorState> {
    mac: BDAddr,
    adapter: ConnectedAdapter,
    data: Arc<Mutex<SensorInnerData>>,
    state: PhantomData<State>,
}

#[derive(Default, Clone, Debug)]
struct SensorInnerData {
    battery: u8,
    temperature: i16,
    humidity: u16,
}

impl SensorInnerData {
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

impl Sensor<Inactive> {
    pub fn new(adapter: ConnectedAdapter, sensor_mac: BDAddr) -> Self {
        Sensor {
            mac: sensor_mac,
            adapter,
            data: Arc::default(),
            state: PhantomData,
        }
    }

    fn activate(self, tx: Option<Sender<SensorData>>) -> Sensor<Active> {
        let data = self.data.clone();
        let sensor_mac = self.mac.clone();

        self.adapter.on_event(Box::new(move |ev| {
            match ev {
                CentralEvent::DeviceDiscovered(discovered_mac, advertising_data)
                | CentralEvent::DeviceUpdated(discovered_mac, advertising_data)
                    if sensor_mac == discovered_mac =>
                {
                    if let (Some(sensor_update), Ok(mut data)) =
                        (parse_advertising_data(advertising_data), data.lock())
                    {
                        data.update(sensor_update);
                        if let Some(tx) = &tx {
                            let _ = tx.send(SensorData::from(data.deref()));
                        }
                    }
                }
                _ => {}
            };
        }));

        self.adapter.start_scan().unwrap();

        Sensor {
            mac: self.mac,
            adapter: self.adapter,
            data: self.data,
            state: PhantomData,
        }
    }

    pub fn start(self) -> Sensor<Active> {
        self.activate(None)
    }

    pub fn listen(self) -> Receiver<SensorData> {
        let (tx, rx) = channel();
        self.activate(Some(tx));
        rx
    }
}

impl Sensor<Active> {
    pub fn get_data(&self) -> SensorData {
        self.data
            .lock()
            .map(|data| SensorData::from(data.deref()))
            .unwrap_or_default()
    }
}

#[derive(Default, Clone, Debug)]
pub struct SensorData {
    pub battery: u8,
    pub temperature: f32,
    pub humidity: f32,
}

impl From<&SensorInnerData> for SensorData {
    fn from(inner: &SensorInnerData) -> Self {
        SensorData {
            battery: inner.battery,
            temperature: inner.temperature as f32 / 10.0,
            humidity: inner.humidity as f32 / 10.0,
        }
    }
}

pub fn adapter_by_mac(addr: BDAddr) -> Result<ConnectedAdapter, btleplug::Error> {
    let manager = Manager::new()?;
    let adapters = manager.adapters()?;

    let adapter = adapters
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
    Temperature(i16),
    Humidity(u16),
    TemperatureAndHumidity(i16, u16),
}

fn parse_advertising_data(advertising_data: &[LEAdvertisingData]) -> Option<SensorUpdate> {
    for item in advertising_data {
        if let LEAdvertisingData::ServiceData16(_, service_data) = item {
            let sensor_type = &service_data[1..4];
            assert_eq!(sensor_type, &[0x20, 0xaa, 0x01]);
            let sensor_type = SensorType::try_from(service_data[11]).ok()?;
            let data_length = service_data[13] as usize;
            assert_eq!(14 + data_length, service_data.len());
            let sensor_data = &service_data[14..14 + data_length];
            return match sensor_type {
                SensorType::Battery => Some(SensorUpdate::Battery(sensor_data[0])),
                SensorType::Temperature => Some(SensorUpdate::Temperature(i16::from_le_bytes([
                    sensor_data[0],
                    sensor_data[1],
                ]))),
                SensorType::Humidity => Some(SensorUpdate::Humidity(u16::from_le_bytes([
                    sensor_data[0],
                    sensor_data[1],
                ]))),
                SensorType::TemperatureAndHumidity => Some(SensorUpdate::TemperatureAndHumidity(
                    i16::from_le_bytes([sensor_data[0], sensor_data[1]]),
                    u16::from_le_bytes([sensor_data[2], sensor_data[3]]),
                )),
            };
        }
    }

    None
}
