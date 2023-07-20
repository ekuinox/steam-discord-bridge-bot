use anyhow::bail;
use shuttle_persist::PersistInstance;

use super::prelude::*;
use crate::user::User;

pub const COMMAND: &str = "register";

pub async fn run(
    ctx: impl AsRef<Http>,
    command: &ApplicationCommandInteraction,
    persist: &PersistInstance,
) -> Result<()> {
    let Some(steam_id) = command.data.options
        .iter()
        .find(|opt| opt.name == "steam-id")
        .and_then(|opt| opt.value.as_ref())
        .and_then(|v| v.as_str()) else {
            bail!("steam id is missing.");
        };

    let user = User::new(steam_id.to_string());
    if let Err(e) = user.save(&command.user.id.to_string(), persist) {
        bail!("Insert user error. {e:?}");
    }

    command
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.ephemeral(true)
                        .content(format!("あなたのSteamIDは[{steam_id}](https://steamcommunity.com/profiles/{steam_id})として登録されました。"))
                })
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND)
        .description("あなたのSteamIDを登録してください。")
        .create_option(|option| {
            option
                .name("steam-id")
                .description(
                    "アカウント詳細ページからSteamIDを取得してください。 https://store.steampowered.com/account/",
                )
                .kind(CommandOptionType::String)
                .required(true)
        })
}
