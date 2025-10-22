use foxglove::{
    WebSocketServer, log,
    schemas::{Log, Timestamp, log::Level},
    Channel,
    McapWriter,
};
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use tokio::time::sleep;
use zenoh::prelude::r#async::*;
mod types;
use types::AircraftData;

fn main() -> zenoh::Result<()> {
    // Spawn the blocking Foxglove server on its own thread so it can create its own runtime.
    thread::spawn(|| {
        WebSocketServer::new()
            .start_blocking()
            .expect("Foxglove server failed to start");
    });

    // Create the MCAP writer to log messages to a file.
    let mcap_handle = McapWriter::new()
        .create_new_buffered_file(format!("{}-adsb-decode.mcap", chrono::Local::now().format("%Y%m%d-%H%M%S")))
        .expect("create failed");
    // share mcap with a signal handler so we can close it on SIGINT
    // store as Option so we can take ownership later
    let mcap = Arc::new(Mutex::new(Some(mcap_handle)));


    // Build a Tokio runtime manually and run async zenoh subscriber code inside it.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    rt.block_on(async move {
        // spawn a task to handle SIGINT and close the mcap file
        let mcap_for_signal = Arc::clone(&mcap);
        tokio::spawn(async move {
            // wait for Ctrl-C
            if let Err(e) = tokio::signal::ctrl_c().await {
                eprintln!("Failed to listen for ctrl_c: {:?}", e);
                return;
            }
            eprintln!("SIGINT received, closing mcap");
            // take the writer out of the mutex so we own it and can call close()
            let maybe = mcap_for_signal.lock().unwrap().take();
            if let Some(writer) = maybe {
                writer.close().expect("close failed");
            } else {
                eprintln!("mcap writer was already taken");
            }
            std::process::exit(0);
        });
        // Open a zenoh session and subscribe to the topic where dump1090_publisher publishes
        let session = zenoh::open(Config::default()).res().await?;
        let subscriber = session
            .declare_subscriber("dump1090/aircraft")
            .res()
            .await?;

        // Process incoming samples in a loop
        loop {
            // receive samples (returns Result<Sample, RecvError>)
            while let Ok(sample) = subscriber.recv_async().await {
                let payload_bytes = sample.value.payload.contiguous().to_vec();
                if let Ok(payload_str) = String::from_utf8(payload_bytes) {
                    // Debug: show received raw payload (short)
                    let preview = if payload_str.len() > 200 {
                        format!("{}... (len={})", &payload_str[..200], payload_str.len())
                    } else {
                        payload_str.clone()
                    };
                    println!("Received zenoh payload preview: {}", preview);

                    match serde_json::from_str::<Vec<AircraftData>>(&payload_str) {
                        Ok(list) => {
                            println!("Parsed aircraft count: {}", list.len());
                            // Publish the entire array as one foxglove log message
                            log!(
                                "/aircraft",
                                Log {
                                    level: Level::Info.into(),
                                    timestamp: Some(Timestamp::now()),
                                    message: payload_str.clone(),
                                    ..Default::default()
                                }
                            );
                        }
                        Err(e) => eprintln!("Failed to parse aircraft JSON from zenoh payload: {:?}\npayload={}", e, payload_str),
                    }
                } else {
                    eprintln!("Received non-utf8 payload from zenoh");
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
    })
}
