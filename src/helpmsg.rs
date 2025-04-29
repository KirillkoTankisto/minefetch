// Internal modules
use crate::MFText;
use crate::async_println;

// Help message root
pub struct Help {
    pub header: &'static str,
    pub program_name: &'static str,
    pub message: &'static [&'static Message],
}

// Message structure
pub struct Message {
    pub name: &'static str,
    pub description: &'static str,
}

/// Display a help message
pub async fn display_help_msg(helpmsg: &Help) -> () {
    // The amount of space that the largest 'name' takes
    let space_size = find_the_largest_msg(helpmsg);

    // Print the header
    async_println!(
        ":hlp: {}{}{}\n:   :",
        MFText::Bold,
        helpmsg.header,
        MFText::Reset
    )
    .await;

    // Print all messages
    for message in helpmsg.message {
        async_println!(
            ":hlp: {}{} {}{}{}  ===>  {};",
            MFText::Bold,
            &helpmsg.program_name, // program name
            message.name,          // name of the command
            MFText::Reset,
            " ".repeat(space_size - message.name.len()), // indent
            message.description                          // the description of the command
        )
        .await;

        // footer
        async_println!(":   :").await;
    }
}

/// Gets the largest name in the 'Help' structure
pub fn find_the_largest_msg(helpmsg: &Help) -> usize {
    // Create a mutable value
    let mut lenght: usize = 0;

    // Go through all messages
    for message in helpmsg.message {
        /*
        If this name is larger than
        previous one then rewrite the
        length value with the new one
        */
        if message.name.len() > lenght {
            lenght = message.name.len()
        }
    }

    // Return the result
    return lenght;
}
