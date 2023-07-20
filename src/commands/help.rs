use super::prelude::*;

pub const COMMAND: &str = "help";
const HOW_TO_USE: &str = r#"
このbotは通話中のユーザーが共通して所持しているゲームを表示するためのものです。
事前に登録の手順があります。
1. [アカウント詳細](https://store.steampowered.com/account/)にアクセスして、左上にある*Steam ID*をコピーしておく
1. このbotとのチャットを開き、`/register` を入力する。 `steam-id` にさきほどコピーした*Steam ID*を貼り付け送信する。
1. 通話に参加し、そのサーバー内で `/get-common-games` を入力することで共通のゲーム一覧を取得できます。
"#;

pub async fn run(ctx: impl AsRef<Http>, command: &ApplicationCommandInteraction) -> Result<()> {
    command
        .create_interaction_response(ctx, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.ephemeral(true)
                        .embed(|embed| embed.field("使い方", HOW_TO_USE, true))
                })
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name(COMMAND).description("使い方を表示します。")
}
