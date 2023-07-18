use std::collections::HashSet;

use futures::future::join_all;
use serenity::client::Cache;

use super::prelude::*;
use crate::{db::DbClient, steam::SteamApiClient};

pub const COMMAND: &str = "get-common-games";

pub async fn run(
    ctx: impl AsRef<Cache> + AsRef<Http>,
    command: &ApplicationCommandInteraction,
    db: &DbClient,
    steam: &SteamApiClient,
) -> Result<()> {
    let Some(guild_id) = command.guild_id else {
        tracing::warn!("guild_id is not found.");
        return Ok(());
    };
    let Some(guild) = guild_id.to_guild_cached(&ctx) else {
        tracing::warn!("cache is not found.");
        return Ok(());
    };

    let Some(channel_id) = guild.voice_states.get(&command.user.id).and_then(|s| s.channel_id) else {
        // VC にいない
        tracing::info!("Not in vc.");
        return Ok(());
    };
    let ids = guild
        .voice_states
        .iter()
        .filter(|(_, s)| s.channel_id == Some(channel_id))
        .map(|(u, _)| u.to_string())
        .collect::<HashSet<_>>();
    dbg!(&ids);

    let users = join_all(ids.iter().map(|id| db.get_user(&id)))
        .await
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();
    let ids = users
        .iter()
        .map(|u| u.steam_id.as_str())
        .collect::<Vec<_>>();
    let games = steam.get_common_games(&ids, ids.len()).await?;

    dbg!(&games);

    // なんかうまいこと 2000 文字以内にして返したい
    command
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.content(
                        games
                            .iter()
                            .map(|(game, _)| game.name.as_str())
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                })
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name(COMMAND).description("Register your steam id")
}
