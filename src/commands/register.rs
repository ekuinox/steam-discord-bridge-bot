use super::prelude::*;
use crate::db::DbClient;

pub const COMMAND: &str = "register";

pub async fn run(
    ctx: impl AsRef<Http>,
    command: &ApplicationCommandInteraction,
    db: &DbClient,
) -> Result<()> {
    let Some(steam_id) = command.data.options
        .iter()
        .find(|opt| opt.name == "steam-id")
        .and_then(|opt| opt.value.as_ref())
        .and_then(|v| v.as_str()) else {
            command
                .create_interaction_response(ctx, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|msg| msg.content("Missing steam-id"))
                })
                .await?;
            return Ok(());
        };

    if let Err(e) = db.insert_user(&command.user.id.to_string(), steam_id).await {
        command
            .create_interaction_response(ctx, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|msg| msg.content("Internal error"))
            })
            .await?;
        tracing::error!("Insert user error. {e:?}");
        return Ok(());
    }

    command
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| msg.content("OK"))
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND)
        .description("Register your steam id")
        .create_option(|option| {
            option
                .name("steam-id")
                .description("Steam ID for register")
                .kind(CommandOptionType::String)
                .required(true)
        })
}
