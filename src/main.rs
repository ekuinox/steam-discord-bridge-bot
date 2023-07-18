mod db;
mod steam;

use std::time::Instant;

use anyhow::anyhow;
use db::DbClient;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_secrets::SecretStore;
use sqlx::PgPool;
use steam::SteamApiClient;
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
                let ids = splited.into_iter().skip(1).collect::<Vec<_>>();
                let t = Instant::now();
                if let Ok(common_games) = self.steam.get_common_games(&ids, ids.len()).await {
                    dbg!(t.elapsed(), common_games.len());
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
