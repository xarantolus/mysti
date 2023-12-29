use common::{action::Action, client_config::ClientConfig, name::client_name};
use dialoguer::FuzzySelect;

use crate::rest::post_action;

mod rest;

fn send_action_interactive(config: &ClientConfig) {
    let clients = match rest::fetch_connected_clients(config) {
        Ok(clients) => clients,
        Err(e) => {
            eprintln!("Failed to fetch connected clients:\n{}\nMake sure you are connected to the internet", e);
            return;
        }
    };

    if clients.is_empty() {
        println!("No clients are currently connected");
        return;
    }

    let current_client_name = client_name();

    let default_idx = clients
        .iter()
        .position(|client| client.name != current_client_name)
        .unwrap_or(0);

    let selection = if clients.len() <= 1 {
        println!("Only {} is connected", clients[0].name);
        0
    } else {
        FuzzySelect::new()
            .with_prompt("Select a client to run an action")
            .items(&clients)
            .default(default_idx)
            .interact()
            .unwrap()
    };

    let client = &clients[selection];

    // Now select which action to perform
    let action = FuzzySelect::new()
        .with_prompt("Which action do you want to run?")
        .items(
            &client
                .supported_actions
                .iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
        )
        .default(0)
        .interact()
        .unwrap();

    let (selected_action, required_args) = &client.supported_actions[action];

    let args = (1..=*required_args)
        .into_iter()
        .map(|arg| {
            dialoguer::Input::<String>::new()
                .with_prompt(format!(
                    "Enter value for argument {}/{}",
                    arg, required_args
                ))
                .interact()
                .unwrap()
        })
        .collect::<Vec<_>>();

    let action = Action {
        action: selected_action.clone(),
        args: args,
    };

    println!("Running action {} on client {}", &action, client.name);

    post_action(&config, client.id, &action).expect("Failed to post action");

    println!("Sent action.");
}

fn main() {
    let config = common::client_config::find_parse_config().expect("Failed to parse config");

    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        1 => send_action_interactive(&config),
        2 if match config.wol_shortcut {
            Some(ref shortcut) => args[1].eq(shortcut),
            None => false,
        } =>
        {
            rest::send_wol(&config).expect("Failed to send WOL packet");
            println!("Sent WOL packet");
        }
        _ => {
            // This is where the fun begins
            unimplemented!(
                "Arguments are not yet supported. Please use the interactive mode for now."
            );
        }
    }
}
