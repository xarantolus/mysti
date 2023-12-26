use common::name::client_name;
use dialoguer::FuzzySelect;

mod rest;

fn main() {
    let config = common::client_config::find_parse_config().expect("Failed to parse config");

    let clients =
        rest::fetch_connected_clients(&config).expect("Failed to fetch connected clients");

    if clients.is_empty() {
        println!("No clients are currently connected");
        return;
    }

    let current_client_name = client_name();

    // get index of the first client that is not ourselfes
    let default_idx = clients
        .iter()
        .position(|client| client.name != current_client_name)
        .unwrap_or(0);

    let selection = if clients.len() <= 1 {
        0
    } else {
        FuzzySelect::new()
            .with_prompt("What do you choose?")
            .items(&clients)
            .default(default_idx)
            .interact()
            .unwrap()
    };

    println!("Client: {}", clients[selection]);

    // TODO: Now actually do something with it
}
