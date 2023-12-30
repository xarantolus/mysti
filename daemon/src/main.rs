use common::client_config::ClientConfig;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::{mpsc::channel, Mutex};

use client::MystiClient;
use image::ImageOutputFormat;
use tokio::task;

use crate::{client::LocalEvent, clipboard::Watcher as ClipboardWatcher};

mod client;
mod clipboard;

const IMAGE_FORMAT: ImageOutputFormat = ImageOutputFormat::Bmp;

#[tokio::main]
async fn main() {
    fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("mysti-daemon.log").expect("Failed to open log file"))
        .apply()
        .expect("Failed to initialize logger");

    // Basically, we parse a config file
    let (mut config, config_path) =
        common::client_config::find_parse_config().expect("Failed to parse config");
    log::info!("Using config file {}", config_path);

    // Notify on config changes
    let (config_events, config_receiver) = std::sync::mpsc::sync_channel::<ClientConfig>(1);
    let config_path_cloned = config_path.clone();
    let mut config_watcher =
        new_debouncer(Duration::from_secs(2), move |res: DebounceEventResult| {
            let Ok(_events) = res else {
                log::error!("Error watching config file: {:?}", res);
                return;
            };

            let cfg = match common::client_config::parse_file(&config_path_cloned) {
                Ok(cfg) => cfg,
                Err(e) => {
                    log::error!("Error parsing updated config file: {}", e);
                    return;
                }
            };

            log::info!("Config file changed, reloading");
            config_events.send(cfg).expect("sending config update");
            log::info!("Sent reload event");
        })
        .expect("creating configuration file watcher");

    config_watcher
        .watcher()
        .watch(Path::new(&config_path), notify::RecursiveMode::NonRecursive)
        .expect("watching configuration file");

    // Then we watch the clipboard for changes - this watcher stays the same,
    // even when we restart the client
    let (clipboard_events, clipboard_receiver) = channel::<LocalEvent>(10);
    let rec_multi = Arc::new(Mutex::new(clipboard_receiver));
    let mut clipboard_watcher = ClipboardWatcher::new(IMAGE_FORMAT, clipboard_events.clone());
    tokio::spawn(async move {
        clipboard_watcher
            .run()
            .expect("Failed to run clipboard watcher");
    });

    // Now we do our loop, restarting every time our configuration changed
    loop {
        let client = Arc::new(Mutex::new(MystiClient::new(config.clone())));

        let client_clone = Arc::clone(&client);
        let clipboard_recv = Arc::clone(&rec_multi);

        let cancellation_token = tokio_util::sync::CancellationToken::new();

        let cclone = cancellation_token.clone();
        let run_task = task::spawn(async move {
            let mut client = client_clone.lock().await;
            loop {
                tokio::select! {
                    err = client.run(clipboard_recv, cclone.clone()) => {
                        if let Err(e) = err {
                            log::error!("Client error: {}", e);
                        }
                        return;
                    },
                    _ = cclone.cancelled() => {
                        return;
                    },
                }
            }
        });

        config = config_receiver.recv().expect("receiving config update");

        log::info!("Configuration changed - restarting client");

        cancellation_token.cancel();
        run_task.abort();
        let stop_result = client.lock().await.abort().await;
        if let Err(e) = stop_result {
            log::error!("Failed to stop client tasks in time: {}", e);
            panic!("Failed to stop client tasks in time");
        }
        if let Err(_) = tokio::time::timeout(Duration::from_secs(10), run_task).await {
            log::error!("Failed to stop client in time");
            panic!("Failed to stop client tasks in time");
        }

        log::info!("Ready to restart");
    }
}
