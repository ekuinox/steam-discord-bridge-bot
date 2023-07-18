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
    let users = join_all(ids.iter().map(|id| db.get_user(&id)))
        .await
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();

    let ids = users
        .iter()
        .map(|u| u.steam_id.as_str())
        .collect::<Vec<_>>();
    // すべてのユーザーで共通しているゲームを名前付きで取得する
    let games = steam.get_common_games(&ids, ids.len()).await?;

    // TODO: 状態として、リクエストの ID をキーとして `games` と表示したページの番号を保存する
    // 保存する場所は DB に置けたらいいけど、オンメモリにするにしても一定期間で expire するようにしないとパンパンになってしまう
    // ringbuffer とかどう
    // 先頭の数件だけを embed として表示して返す

    // なんかうまいこと 2000 文字以内にして返したい
    command
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    let texts = games
                        .iter()
                        .filter(|(_, users)| users.len() == ids.len())
                        .map(|(game, _)| {
                            format!(
                                "- [{}](https://store.steampowered.com/app/{})\n",
                                game.name, game.appid
                            )
                        })
                        .fold(Vec::<String>::new(), |mut texts, line| {
                            match texts.last_mut() {
                                Some(last) if last.len() + line.len() < 1024 => {
                                    last.push_str(&line);
                                }
                                _ => {
                                    texts.push(line);
                                }
                            }
                            texts
                        });
                    for text in texts.iter().take(1) {
                        msg.embed(|embed| embed.title("Games").field("ALL", text, false));
                    }
                    msg.components(|c| {
                        c.create_action_row(|r| {
                            // この next-button を next-button-[a-z]{10} にして紐づけるしかない?
                            r.create_button(|b| b.custom_id("next-button").label("NEXT"))
                        })
                    });
                    msg.custom_id("hogeeee");
                    msg
                })
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name(COMMAND).description("Register your steam id")
}
