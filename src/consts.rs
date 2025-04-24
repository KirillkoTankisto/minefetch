/*
  ____                _              _
 / ___|___  _ __  ___| |_ __ _ _ __ | |_ ___
| |   / _ \| '_ \/ __| __/ _` | '_ \| __/ __|
| |__| (_) | | | \__ \ || (_| | | | | |_\__ \
 \____\___/|_| |_|___/\__\__,_|_| |_|\__|___/

*/

use crate::helpmsg::{Help, Message};

pub const NAME: &'static str = "MineFetch";
pub const PROGRAM_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const USER_AGENT: &'static str = concat!("KirillkoTankisto/minefetch/", env!("CARGO_PKG_VERSION"), " (kirsergeev@icloud.com)");
pub const HELP_MESSAGE: Help = Help {
    header: "Commands:",
    program_name: "minefetch",
    message: &[
        &Message {
            name: "search",
            description: "search for mods and install them",
        },
        &Message {
            name: "add",
            description: "add a single mod",
        },
        &Message {
            name: "profile create",
            description: "create a new profile",
        },
        &Message {
            name: "profile delete",
            description: "delete a selected profile",
        },
        &Message {
            name: "profile delete all",
            description: "delete all profiles",
        },
        &Message {
            name: "profile switch",
            description: "switch between profiles",
        },
        &Message {
            name: "profile list",
            description: "list all profiles",
        },
        &Message {
            name: "update",
            description: "update all mods",
        },
        &Message {
            name: "upgrade",
            description: "same as 'minefetch update'",
        },
        &Message {
            name: "list",
            description: "list installed mods",
        },
        &Message {
            name: "lock add",
            description: "add a lock which stops selected mod from updating",
        },
        &Message {
            name: "lock remove",
            description: "delete a lock",
        },
        &Message {
            name: "lock list",
            description: "list locks",
        },
        &Message {
            name: "version",
            description: "display MineFetch version",
        },
    ],
};
