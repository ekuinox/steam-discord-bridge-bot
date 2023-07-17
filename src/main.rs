mod steam;

use anyhow::anyhow;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use shuttle_secrets::SecretStore;
use steam::{get_app_detail, SteamApiClient, StoreAppDetail};
use tracing::{error, info};

#[derive(Debug)]
struct Bot {
    client: SteamApiClient,
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        let splited = msg.content.split(' ').collect::<Vec<_>>();
        if splited.is_empty() {
            return;
        }

        match splited[0] {
            "!profile" if splited.len() == 2 => {
                let steam_id = splited[1];
                match self.client.get_owned_games(steam_id).await {
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
            "!game" if splited.len() == 2 => {
                let appid = splited[1];
                match get_app_detail(appid).await {
                    Ok(StoreAppDetail::Game {
                        name,
                        is_free,
                        price_overview,
                        categories,
                        ..
                    }) => {
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
                        if let Err(e) = msg.reply_mention(ctx, format!("Is is not game.")).await {
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
) -> shuttle_serenity::ShuttleSerenity {
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
            client: SteamApiClient::new(api_key),
        })
        .await
        .expect("Err creating client");

    Ok(client.into())
}
