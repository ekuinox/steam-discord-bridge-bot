mod commands;
mod db;
mod steam;

use anyhow::anyhow;
use db::DbClient;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{async_trait, model::prelude::command::Command};
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
    async fn voice_state_update(&self, _ctx: Context, _old: Option<VoiceState>, _new: VoiceState) {
        dbg!(&_new);
    }
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
            Interaction::MessageComponent(mut component) => {
                dbg!(component.message.id, component.data.custom_id);
                let _r = component
                    .message
                    .edit(ctx, |msg| msg.content("aaaaaa"))
                    .await;
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
        })
        .await
        .expect("Err creating client");

    Ok(client.into())
}
