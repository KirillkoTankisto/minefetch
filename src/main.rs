/*
 __  __       _         _____                 _   _
|  \/  | __ _(_)_ __   |  ___|   _ _ __   ___| |_(_) ___  _ __
| |\/| |/ _` | | '_ \  | |_ | | | | '_ \ / __| __| |/ _ \| '_ \
| |  | | (_| | | | | | |  _|| |_| | | | | (__| |_| | (_) | | | |
|_|  |_|\__,_|_|_| |_| |_|   \__,_|_| |_|\___|\__|_|\___/|_| |_|

*/

// Standard imports
use std::result::Result;

// Internal modules
mod api;
mod consts;
mod downloader;
mod front;
mod helpmsg;
mod mfio;
mod profile;
mod structs;
mod utils;

use crate::consts::{NAME, PROGRAM_VERSION, USER_AGENT};
use crate::front::*;
use crate::helpmsg::display_help;

#[tokio::main]
// The start of the main function
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Calling initialise to start the program
    match initialise().await {
        Ok(_) => (),
        Err(error) => eprintln!(":err: {error}"),
    };

    // Exit
    Ok(())
}

/// The start of the main async function
async fn initialise() -> Result<(), Box<dyn std::error::Error>> {
    // Read the commandline arguments
    let args: Vec<String> = std::env::args().collect();

    // Get the first argument
    match args.get(1).map(String::as_str) {
        // minefetch add %mod_id_or_slug%
        Some("add") => match args.get(2).map(String::as_str) // Get the mod id / slug
        {
            Some(modname) => {
                add_mod(modname).await?;
            }

            // If the prompt is empty
            _ => display_help().await,
        },

        // minefetch profile %subcommand%
        Some("profile") => match args.get(2).map(String::as_str) {
            // minefetch profile create
            Some("create") => create_profile().await?,

            // minefetch profile delete (all or only one)
            Some("delete") => match args.get(3).map(String::as_str) {
                // minefetch profile delete all
                Some("all") => delete_profile(1).await?,

                // minefetch profile delete
                _ => delete_profile(0).await?,
            },

            // minefetch profile switch
            Some("switch") => switch_profile().await?,

            // minefetch profile list
            Some("list") => list_profiles().await?,

            // If the prompt is empty
            _ => display_help().await,
        },

        // minefetch version
        Some("version") => println!(":out: {} {}", NAME, PROGRAM_VERSION),

        // minefetch search %query%
        Some("search") => match args.get(2) {
            /*
                Doesn't use the value from args because
                the program joins all the strings starting
                from the third argument
            */
            Some(_) => search(args).await?,

            // Display the help message if empty
            None => display_help().await,
        },

        // minefetch update OR minefetch upgrade
        Some("upgrade") | Some("update") => upgrade().await?,

        // minefetch list
        Some("list") => list().await?,

        // minefetch lock %subcommand%
        Some("lock") => match args.get(2).map(String::as_str) {
            // minefetch lock add
            Some("add") => {
                fadd_lock().await?;
            }

            // minefetch lock remove
            Some("remove") => rm_lock().await?,

            // minefetch lock list
            Some("list") => ls_lock().await?,

            // Display the help message in other cases
            _ => display_help().await,
        },

        // minefetch edit
        Some("edit") => fedit_mod().await?,

        // minefetch debug
        Some("debug") => println!(":dbg: {} / {} / {}", NAME, PROGRAM_VERSION, USER_AGENT),

        // Display a help message in other case
        Some(_) => display_help().await,

        // If arguments are empty
        None => println!(":out: No arguments provided"),
    }

    // Success
    Ok(())
}
