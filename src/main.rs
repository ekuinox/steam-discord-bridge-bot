mod commands;
mod common_games;
mod db;
mod steam;

use std::str::FromStr;

use anyhow::anyhow;
use db::DbClient;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{async_trait, model::prelude::command::Command};
use shuttle_persist::PersistInstance;
use shuttle_secrets::SecretStore;
use sqlx::PgPool;
use steam::SteamApiClient;
use tracing::{error, info};

use crate::common_games::{
    create_interaction_response, CommonGamesButtonCustomId, CommonGamesStore,
};

struct Bot {
    steam: SteamApiClient,
    db: DbClient,
    persist: PersistInstance,
}

#[async_trait]
impl EventHandler for Bot {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                let resp = match command.data.name.as_str() {
                    commands::register::COMMAND => {
                        commands::register::run(ctx.clone(), &command, &self.db).await
                    }
                    commands::show::COMMAND => {
                        commands::show::run(ctx.clone(), &command, &self.db).await
                    }
                    commands::get_common_games::COMMAND => {
                        commands::get_common_games::run(
                            ctx.clone(),
                            &command,
                            &self.db,
                            &self.steam,
                            &self.persist,
                        )
                        .await
                    }
                    c => {
                        tracing::warn!("Not implimented {c}");
                        return;
                    }
                };
                if let Err(e) = resp {
                    tracing::warn!("{e:?}");
                }
            }
            Interaction::MessageComponent(component) => {
                if let Ok(custom_id) =
                    CommonGamesButtonCustomId::from_str(&component.data.custom_id)
                {
                    if let Ok(store) = CommonGamesStore::load(&custom_id.key, &self.persist) {
                        let games = store.get(custom_id.page);
                        if let Err(e) = component
                            .create_interaction_response(&ctx, |response| {
                                response.interaction_response_data(|msg| {
                                    create_interaction_response(custom_id, games, true, msg);
                                    msg
                                })
                            })
                            .await
                        {
                            tracing::error!("{e:?}")
                        }
                        if let Err(e) = component
                            .delete_followup_message(&ctx, component.message.id)
                            .await
                        {
                            tracing::error!("{e:?}")
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        for register in [
            commands::show::register,
            commands::register::register,
            commands::get_common_games::register,
        ] {
            if let Err(e) =
                Command::create_global_application_command(&ctx, |command| register(command)).await
            {
                error!("{e:?}");
            }
        }
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_aws_rds::Postgres] pool: PgPool,
    #[shuttle_persist::Persist] persist: PersistInstance,
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
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let client = Client::builder(&token, intents)
        .event_handler(Bot {
            steam: SteamApiClient::new(api_key),
            db,
            persist,
        })
        .await
        .expect("Err creating client");

    Ok(client.into())
}
