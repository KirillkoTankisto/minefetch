use crate::async_println;
use crate::MFText;

pub struct Help {
    pub header: &'static str,
    pub program_name: &'static str,
    pub message: &'static [&'static Message],
}
pub struct Message {
    pub name: &'static str,
    pub description: &'static str,
}

pub async fn display_help_msg(helpmsg: &Help) -> () {
    let space_size = find_the_largest_msg(helpmsg);

    async_println!(
        ":hlp: {}{}{}\n:   :",
        MFText::Bold,
        helpmsg.header,
        MFText::Reset
    )
    .await;

    for message in helpmsg.message {
        async_println!(
            ":hlp: {}{} {}{}{}  ===>  {};",
            MFText::Bold,
            &helpmsg.program_name,
            message.name,
            MFText::Reset,
            " ".repeat(space_size - message.name.len()),
            message.description
        )
        .await;

        async_println!(":   :").await;
    }
}

pub fn find_the_largest_msg(helpmsg: &Help) -> usize {
    let mut lenght: usize = 0;

    for message in helpmsg.message {
        if message.name.len() > lenght {
            lenght = message.name.len()
        }
    }

    return lenght;
}
