use std::collections::HashSet;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// [GetOwnedGames](https://developer.valvesoftware.com/wiki/Steam_Web_API#GetOwnedGames_.28v0001.29) response.
///
/// - クエリに `include_appinfo=true`. を含む必要がある
/// - ほかのフィールドを有効にするとユーザーごとに異なってきてしまうため除外している
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
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
}
