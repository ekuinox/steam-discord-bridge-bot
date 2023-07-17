mod db;
mod steam;

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use db::DbClient;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_secrets::SecretStore;
use sqlx::PgPool;
use steam::{get_app_detail, Game, SteamApiClient, StoreAppDetail, StoreGameDetail};
use tracing::{error, info};

#[derive(Debug)]
struct Bot {
    steam: SteamApiClient,
    db: DbClient,
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        let splited = msg.content.split(' ').collect::<Vec<_>>();
        if splited.is_empty() {
            return;
        }

        match splited[0] {
            "!match-by-steam-ids" if splited.len() > 2 => {
                let ids = splited.iter().skip(1);
                async fn get<'a>(
                    steam: &'a SteamApiClient,
                    id: &'a str,
                ) -> Result<(&'a str, Vec<Game>)> {
                    let resp = steam.get_owned_games(id).await?;
                    Ok((id, resp))
                }

                async fn get_detail(id: u64) -> Result<StoreAppDetail> {
                    let detail = get_app_detail(&id.to_string()).await?;
                    Ok(detail)
                }

                let results = futures::future::join_all(ids.map(|id| get(&self.steam, id)))
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<HashMap<_, _>>();

                let games = results
                    .iter()
                    .flat_map(|(_, games)| games)
                    .collect::<HashSet<_>>();
                let games = games
                    .iter()
                    .filter(|game| results.values().all(|games| games.contains(&game)))
                    .collect::<Vec<_>>();
                dbg!(&games, results.len());

                let games =
                    futures::future::join_all(games.iter().map(|game| get_detail(game.appid)))
                        .await
                        .into_iter()
                        .flatten()
                        .flat_map(|app| {
                            if let StoreAppDetail::Game(game) = app {
                                Some(game)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                let names = games
                    .iter()
                    .map(|game| game.name.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                dbg!(&games);
                dbg!(&names);
                if let Err(e) = msg.reply_mention(ctx, format!("User = {}, Found = {names}", results.len())).await {
                    error!("{e:?}");
                }
            }
            "!profile" if splited.len() == 2 => {
                let steam_id = splited[1];
                match self.steam.get_owned_games(steam_id).await {
                    Ok(games) => {
                        if let Err(e) = msg
                            .reply_mention(ctx, format!("you have {} games", games.len()))
                            .await
                        {
                            error!("{e:?}");
                        }
                    }
                    Err(e) => {
                        error!("{e:?}");
                    }
                }
            }
            "!register" if splited.len() == 2 => {
                let steam_id = splited[1];
                let discord_id = msg.author.id.to_string();
                if self.db.insert_user(&discord_id, steam_id).await.is_ok() {
                    let _ = msg.reply_mention(ctx, "登録しました").await;
                }
            }
            "!update" if splited.len() == 2 => {
                let steam_id = splited[1];
                let discord_id = msg.author.id.to_string();
                if self.db.update_user(&discord_id, steam_id).await.is_ok() {
                    let _ = msg.reply_mention(ctx, "更新しました").await;
                }
            }
            "!show" => match self.db.get_user(&msg.author.id.to_string()).await {
                Ok(user) => {
                    let _ = msg
                        .reply_mention(ctx, format!("Found id = {}", user.steam_id))
                        .await;
                }
                Err(e) => {
                    error!("{e:?}");
                }
            },
            "!game" if splited.len() == 2 => {
                let appid = splited[1];
                match get_app_detail(appid).await {
                    Ok(StoreAppDetail::Game(StoreGameDetail {
                        name,
                        is_free,
                        price_overview,
                        categories,
                        ..
                    })) => {
                        let has_multi_player = categories.iter().any(|c| c.is_multi_player());
                        let price = price_overview
                            .as_ref()
                            .map(|price| price.final_formatted.as_str())
                            .unwrap_or_else(|| if is_free { "Free" } else { "Unknown" });
                        if let Err(e) = msg
                            .reply_mention(
                                ctx,
                                format!("App name = {name}. multi = {has_multi_player}. price = {price}"),
                            )
                            .await
                        {
                            error!("{e:?}")
                        }
                    }
                    Ok(_) => {
                        if let Err(e) = msg.reply_mention(ctx, "Is is not game.".to_string()).await
                        {
                            error!("{e:?}")
                        }
                    }
                    Err(e) => {
                        error!("{e:?}")
                    }
                }
            }
            _ => {}
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_aws_rds::Postgres] pool: PgPool,
) -> shuttle_serenity::ShuttleSerenity {
    let db = DbClient::new(pool).await.map_err(|e| anyhow!("{e:?}"))?;
    // Get the discord token set in `Secrets.toml`
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    let Some(api_key) = secret_store.get("STEAM_API_KEY") else {
        return Err(anyhow!("'STEAM_API_KEY' was not found").into());
    };

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&token, intents)
        .event_handler(Bot {
            steam: SteamApiClient::new(api_key),
            db,
        })
        .await
        .expect("Err creating client");

    Ok(client.into())
}
