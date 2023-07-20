use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serenity::builder::CreateInteractionResponseData;
use shuttle_persist::PersistInstance;

use crate::steam::Game;

pub type AppId = u64;

pub const PAGE_SIZE: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommonGamesStore {
    games: HashMap<AppId, Game>,
    game_ids: Vec<AppId>,
}

impl CommonGamesStore {
    pub fn new(games: Vec<HashSet<Game>>) -> CommonGamesStore {
        let games = games
            .into_iter()
            .reduce(|acc, x| {
                acc.intersection(&x)
                    .map(ToOwned::to_owned)
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        let mut game_ids = games.iter().map(|game| game.appid).collect::<Vec<_>>();
        game_ids.sort_by(|a, b| a.cmp(&b));
        let games = games.into_iter().map(|game| (game.appid, game)).collect();
        CommonGamesStore { games, game_ids }
    }

    pub fn get(&self, page_idx: usize) -> Vec<&Game> {
        let ids = self
            .game_ids
            .chunks(PAGE_SIZE)
            .nth(page_idx)
            .unwrap_or_default();
        let mut games = self
            .games
            .iter()
            .filter(|(id, _)| ids.contains(id))
            .map(|(_, game)| game)
            .collect::<Vec<_>>();
        games.sort_by(|a, b| a.appid.cmp(&b.appid));
        games
    }

    pub fn load(key: &str, persist: &PersistInstance) -> Result<CommonGamesStore> {
        let self_ = persist.load(key)?;
        Ok(self_)
    }

    pub fn save(&self, key: &str, persist: &PersistInstance) -> Result<()> {
        persist.save(key, self)?;
        Ok(())
    }
}

/// ボタンに設定するカスタムID
#[derive(Serialize, Deserialize, Debug)]
pub struct CommonGamesButtonCustomId {
    /// 遷移先のページの ID
    pub page: usize,
    /// ストアのキー
    /// 呼び出したユーザーの ID に紐づけて保存する
    pub key: String,
}

impl CommonGamesButtonCustomId {
    pub fn new(page: usize, key: String) -> CommonGamesButtonCustomId {
        CommonGamesButtonCustomId { page, key }
    }

    pub fn prev(&self) -> Option<CommonGamesButtonCustomId> {
        self.page
            .checked_sub(1)
            .map(|page| CommonGamesButtonCustomId::new(page, self.key.clone()))
    }

    pub fn next(&self) -> CommonGamesButtonCustomId {
        CommonGamesButtonCustomId::new(self.page + 1, self.key.clone())
    }
}

impl FromStr for CommonGamesButtonCustomId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let id = serde_json::from_str(s)?;
        Ok(id)
    }
}

impl ToString for CommonGamesButtonCustomId {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).expect("convert id to string error")
    }
}

pub fn create_interaction_response(
    custom_id: CommonGamesButtonCustomId,
    games: Vec<&Game>,
    ephemeral: bool,
    msg: &mut CreateInteractionResponseData,
) {
    let text = games
        .iter()
        .map(|game| {
            format!(
                "- [{}](https://store.steampowered.com/app/{})\n",
                game.name, game.appid
            )
        })
        .collect::<String>();
    msg.ephemeral(ephemeral)
        .embed(|embed| embed.field(format!("Games: p{}", custom_id.page), text, false))
        .components(|c| {
            c.create_action_row(|r| {
                r.create_button(|b| {
                    b.label("PREV");
                    if let Some(prev) = custom_id.prev() {
                        b.custom_id(prev.to_string())
                    } else {
                        b.disabled(true).custom_id("Invalid")
                    }
                })
                .create_button(|b| {
                    b.custom_id(custom_id.next().to_string())
                        .label("NEXT")
                        .disabled(games.len() != PAGE_SIZE)
                })
            })
        });
}
