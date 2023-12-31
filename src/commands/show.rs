use shuttle_persist::PersistInstance;

use super::prelude::*;
use crate::user::User;

pub const COMMAND: &str = "show";

pub async fn run(
    ctx: impl AsRef<Http>,
    command: &ApplicationCommandInteraction,
    persist: &PersistInstance,
) -> Result<()> {
    let content = match User::load(&command.user.id.to_string(), persist) {
        Ok(user) => format!(
            "あなたのSteamIDは[{}](https://steamcommunity.com/profiles/{})として登録されています。",
            user.steam_id(),
            user.steam_id(),
        ),
        Err(_e) => "あなたのSteamIDは未登録のようです。".to_string(),
    };

    command
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| msg.ephemeral(true).content(content))
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND)
        .description("あなたが現在登録しているSteamのIDを返します。")
}
