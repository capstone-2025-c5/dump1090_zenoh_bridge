use serde::{Deserialize, Serialize};

#[serde(untagged)]
#[derive(Deserialize, Serialize, Debug)]
pub enum AltitudeValue {
    Numeric(f64),
    Ground(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Dump1090Root {
    pub now: Option<f64>,
    pub messages: Option<u64>,
    pub aircraft: Option<Vec<AircraftData>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AircraftData {
    pub hex: Option<String>,
    pub flight: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub alt_baro: Option<AltitudeValue>,
    pub alt_geom: Option<f64>,
    pub alt: Option<f64>,
    pub gs: Option<f64>,
    pub ias: Option<f64>,
    pub tas: Option<f64>,
    pub mach: Option<f64>,
    pub track: Option<f64>,
    pub track_rate: Option<f64>,
    pub roll: Option<f64>,
    pub mag_heading: Option<f64>,
    pub true_heading: Option<f64>,
    pub baro_rate: Option<f64>,
    pub geom_rate: Option<f64>,
    pub seen: Option<f64>,
    pub rssi: Option<f64>,
}
