use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use futures::future::join_all;
use serde::Deserialize;

/// [GetOwnedGames](https://developer.valvesoftware.com/wiki/Steam_Web_API#GetOwnedGames_.28v0001.29) response.
///
/// - クエリに `include_appinfo=true`. を含む必要がある
/// - ほかのフィールドを有効にするとユーザーごとに異なってきてしまうため除外している
#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Game {
    pub appid: u64,
    pub name: String,
}

/// Steam Web API client.
///
/// https://steamcommunity.com/dev
#[derive(Clone, Debug)]
pub struct SteamApiClient {
    api_key: String,
}

impl SteamApiClient {
    pub fn new(api_key: String) -> SteamApiClient {
        SteamApiClient { api_key }
    }

    pub async fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<reqwest::Response> {
        let resp = reqwest::Client::default()
            .get(format!("http://api.steampowered.com{path}"))
            .query(&query)
            .query(&[("key", self.api_key.as_str()), ("format", "json")])
            .send()
            .await?;
        Ok(resp)
    }

    /// Returns a list of games a player owns along with some playtime information, if the profile is publicly visible.
    /// Private, friends-only, and other privacy settings are not supported unless you are asking for your own personal details (ie the WebAPI key you are using is linked to the steamid you are requesting).
    ///
    /// [GetOwnedGames](https://developer.valvesoftware.com/wiki/Steam_Web_API#GetOwnedGames_.28v0001.29)
    pub async fn get_owned_games(&self, steam_id: &str) -> Result<HashSet<Game>> {
        #[derive(Deserialize, Debug)]
        pub struct OwnedGames {
            pub game_count: usize,
            pub games: HashSet<Game>,
        }

        #[derive(Deserialize, Debug)]
        pub struct OwnedGamesResponse {
            pub response: OwnedGames,
        }

        let OwnedGamesResponse {
            response:
                OwnedGames {
                    game_count: _,
                    games,
                },
        } = self
            .get(
                "/IPlayerService/GetOwnedGames/v0001",
                &[("steamid", steam_id), ("include_appinfo", "true")],
            )
            .await
            .context("request failed")?
            .json()
            .await
            .context("invalid json")?;
        Ok(games)
    }

    /// Get owned games by steam id.
    pub async fn get_owned_games_by_steam_ids<'a>(
        &self,
        steam_ids: &'a [&str],
    ) -> Result<HashMap<&'a str, HashSet<Game>>> {
        let r = join_all(steam_ids.iter().map(|id| async {
            let resp = self.get_owned_games(id).await?;
            Result::<(&str, HashSet<Game>)>::Ok((id, resp))
        }))
        .await
        .into_iter()
        .flatten()
        .collect::<HashMap<_, _>>();
        Ok(r)
    }

    /// 共通のゲームの所有者を取得する
    ///
    /// # Parameters
    /// - `steam_ids` - 検索をかけるSteamのユーザID
    /// - `min` - 最低数必要な所有者の数
    pub async fn get_common_games<'a>(
        &self,
        steam_ids: &'a [&str],
        min: usize,
    ) -> Result<HashMap<Game, HashSet<&'a str>>> {
        let all = self.get_owned_games_by_steam_ids(steam_ids).await?;
        let games = all.values().flatten().collect::<HashSet<_>>();
        let r = games
            .into_iter()
            .map(|game| {
                (
                    game,
                    all.iter().fold(HashSet::new(), |mut owners, (id, games)| {
                        if games.contains(&game) {
                            owners.insert(*id);
                        }
                        owners
                    }),
                )
            })
            .filter_map(|(game, owners)| (owners.len() >= min).then(|| (game.clone(), owners)))
            .collect::<HashMap<_, _>>();
        Ok(r)
    }
}
