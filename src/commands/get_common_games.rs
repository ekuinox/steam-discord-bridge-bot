use std::collections::HashSet;

use futures::future::join_all;
use serenity::client::Cache;
use shuttle_persist::PersistInstance;

use super::prelude::*;
use crate::{
    common_games::{create_interaction_response, CommonGamesButtonCustomId, CommonGamesStore},
    steam::SteamApiClient,
    user::User,
};

pub const COMMAND: &str = "get-common-games";

pub async fn run(
    ctx: impl AsRef<Cache> + AsRef<Http>,
    command: &ApplicationCommandInteraction,
    steam: &SteamApiClient,
    persist: &PersistInstance,
) -> Result<()> {
    let Some(guild_id) = command.guild_id else {
        command
            .create_interaction_response(&ctx, |resp| {
                resp.kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|int| {
                        int.ephemeral(true).content("Please call at guild channel.")
                    })
            })
            .await?;
        return Ok(());
    };

    let Some(guild) = guild_id.to_guild_cached(&ctx) else {
        command
            .create_interaction_response(&ctx, |resp| {
                resp.kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|int| {
                        int.ephemeral(true).content("Internal error.")
                    })
            })
            .await?;
        return Ok(());
    };

    let Some(channel_id) = guild.voice_states.get(&command.user.id).and_then(|s| s.channel_id) else {
        command
            .create_interaction_response(&ctx, |resp| {
                resp.kind(InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|int| {
                        int.ephemeral(true).content("Please call when you joined voice channel.")
                    })
            })
            .await?;
        return Ok(());
    };

    // 呼び出したユーザーが参加している VC にいるすべてのユーザーの ID を取り出す
    let ids = guild
        .voice_states
        .iter()
        .filter(|(_, s)| s.channel_id == Some(channel_id))
        .map(|(u, _)| u.to_string())
        .collect::<HashSet<_>>();

    // Discord の ID から事前に登録された Steam の ID を引く
    // 引けなかったものは存在しないものとして除外する
    let users = ids
        .iter()
        .flat_map(|id| User::load(id, persist))
        .collect::<HashSet<_>>();

    let ids = users.iter().map(|u| u.steam_id()).collect::<Vec<_>>();

    let games = join_all(ids.iter().map(|id| steam.get_owned_games(id)))
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if games.len() < ids.len() {
        tracing::warn!("1つ以上のユーザーの所有ゲームを取得できませんでした");
    }

    let games = CommonGamesStore::new(games);
    let key = command.user.id.to_string();
    games.save(&key, persist)?;

    let games = games.get(0);
    let custom_id = CommonGamesButtonCustomId::new(0, key);

    command
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    create_interaction_response(custom_id, games, true, msg);
                    msg
                })
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name(COMMAND).description("Register your steam id")
}
