use std::time::Duration;
use tokio::time::sleep;
use zenoh::prelude::r#async::*;
use std::env;
mod types;
use types::{Dump1090Root, AircraftData};

#[tokio::main]
async fn main() -> zenoh::Result<()> {
    // Get URL from command-line args (default to local IP endpoint)
    let args: Vec<String> = env::args().collect();
    let url = if args.len() >= 2 {
        args[1].clone()
    } else {
        // default endpoint
        "http://localhost:8080/data/aircraft.json".to_string()
    };

    // Create Zenoh session
    let session = zenoh::open(Config::default()).res().await?;
    let publisher = session.declare_publisher("dump1090/aircraft").res().await?;

    loop {
        // Fetch JSON over HTTP
        match reqwest::get(&url).await {
            Ok(resp) => match resp.text().await {
                Ok(data) => match serde_json::from_str::<Dump1090Root>(&data) {
                  Ok(root) => {
                    let mut out: Vec<AircraftData> = Vec::new();
                    if let Some(list) = root.aircraft {
                      for a in list.into_iter() {
                        // skip entries without a hex
                        let hex = match a.hex {
                          Some(h) => h,
                          None => continue,
                        };
                        out.push(AircraftData {
                          hex: Some(hex),
                          flight: a.flight,
                          lat: a.lat,
                          lon: a.lon,
                          alt_baro: a.alt_baro,
                          alt_geom: a.alt_geom,
                          alt: a.alt,
                          gs: a.gs,
                          ias: a.ias,
                          tas: a.tas,
                          mach: a.mach,
                          track: a.track,
                          track_rate: a.track_rate,
                          roll: a.roll,
                          mag_heading: a.mag_heading,
                          true_heading: a.true_heading,
                          baro_rate: a.baro_rate,
                          geom_rate: a.geom_rate,
                          seen: a.seen,
                          rssi: a.rssi,
                        });
                      }
                    }

                    match serde_json::to_string(&out) {
                      Ok(json_string) => {
                        // Debug: print number of aircraft and short JSON preview
                        println!("Publishing {} aircraft", out.len());
                        let preview = if json_string.len() > 200 {
                          format!("{}... (len={})", &json_string[..200], json_string.len())
                        } else {
                          json_string.clone()
                        };
                        println!("payload preview: {}", preview);
                        if let Err(e) = publisher.put(json_string).res().await {
                          eprintln!("Zenoh publish error: {:?}", e);
                        }
                      }
                      Err(e) => eprintln!("Serialization error: {:?}", e),
                    }
                  }
                  Err(e) => eprintln!("JSON parse error (expected object with 'aircraft'): {:?}", e),
                },
                Err(e) => eprintln!("Failed to read response body from {}: {:?}", url, e),
            },
            Err(e) => eprintln!("HTTP GET {} failed: {:?}", url, e),
        }

        sleep(Duration::from_secs(1)).await;
    }
}
