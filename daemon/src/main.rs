use client::MystiClient;
use image::ImageOutputFormat;

mod client;
mod clipboard;

#[tokio::main]
async fn main() {
    fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("mysti-daemon.log").expect("Failed to open log file"))
        .apply()
        .expect("Failed to initialize logger");

    let config = common::client_config::find_parse_config().expect("Failed to parse config");

    let mut client = MystiClient::new(config, ImageOutputFormat::Jpeg(100));

    client.run().await.expect("Failed to run client");
}
