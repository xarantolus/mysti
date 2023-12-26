mod rest;

fn main() {
    let config = common::client_config::find_parse_config().expect("Failed to parse config");

    let clients =
        rest::fetch_connected_clients(&config).expect("Failed to fetch connected clients");

    println!("{:#?}", clients);
}
