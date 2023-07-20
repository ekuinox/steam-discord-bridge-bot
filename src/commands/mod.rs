pub mod get_common_games;
pub mod help;
pub mod register;
pub mod show;

pub(self) mod prelude {
    pub use anyhow::Result;
    pub use serenity::{
        builder::{CreateApplicationCommand, CreateApplicationCommandOption},
        http::Http,
        model::prelude::{
            application_command::ApplicationCommandInteraction, command::CommandOptionType, *,
        },
        prelude::*,
    };
}
