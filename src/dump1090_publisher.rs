use std::time::Duration;
use tokio::time::sleep;
use zenoh::prelude::r#async::*;
use std::env;
mod types;
use types::AircraftData;

#[tokio::main]
async fn main() -> zenoh::Result<()> {
    // Get URL from command-line args (default to local IP endpoint)
    let args: Vec<String> = env::args().collect();
    let url = if args.len() >= 2 {
        args[1].clone()
    } else {
        // default endpoint
        "http://192.168.50.50:8080/data/aircraft.json".to_string()
    };

    // Create Zenoh session
    let session = zenoh::open(Config::default()).res().await?;
    let publisher = session.declare_publisher("dump1090/aircraft").res().await?;

    loop {
        // Fetch JSON over HTTP
        match reqwest::get(&url).await {
            Ok(resp) => match resp.text().await {
                Ok(data) => {
                  // First parse into a generic JSON value so we can handle slight variations
                  let mut out: Vec<AircraftData> = Vec::new();
                  match serde_json::from_str::<serde_json::Value>(&data) {
                    Ok(val) => {
                      match val.get("aircraft") {
                        Some(aircraft_val) => {
                          if let Some(list) = aircraft_val.as_array() {
                            for elem in list.iter() {
                              match serde_json::from_value::<AircraftData>(elem.clone()) {
                                Ok(a) => {
                                  // skip entries without a hex
                                  match &a.hex {
                                    Some(h) if !h.is_empty() => out.push(a),
                                    _ => continue,
                                  }
                                }
                                Err(e) => eprintln!("Failed to deserialize aircraft entry: {:?}\nelement={}", e, elem),
                              }
                            }
                          } else {
                            eprintln!("'aircraft' field is present but not an array");
                          }
                        }
                        None => eprintln!("No 'aircraft' field in JSON response"),
                      }
                    }
                    Err(e) => {
                      let preview = if data.len() > 500 {
                        format!("{}... (len={})", &data[..500], data.len())
                      } else {
                        data.clone()
                      };
                      eprintln!("JSON parse error (invalid JSON): {:?}\nresponse preview: {}", e, preview);
                      continue;
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
                },
                Err(e) => eprintln!("Failed to read response body from {}: {:?}", url, e),
            },
            Err(e) => eprintln!("HTTP GET {} failed: {:?}", url, e),
        }

        sleep(Duration::from_secs(1)).await;
    }
}
