use std::collections::HashMap;

use anyhow::{anyhow, Context, Ok, Result};
use monostate::MustBe;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Game {
    pub appid: u64,
    pub playtime_forever: u64,
    pub playtime_windows_forever: u64,
    pub playtime_mac_forever: u64,
    pub playtime_linux_forever: u64,
    pub rtime_last_played: u64,
    pub playtime_disconnected: u64,
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
    pub async fn get_owned_games(&self, steam_id: &str) -> Result<Vec<Game>> {
        #[derive(Deserialize, Debug)]
        pub struct OwnedGames {
            pub game_count: usize,
            pub games: Vec<Game>,
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

#[derive(Deserialize, Debug)]
pub struct StorePriceOverview {
    pub currency: String,
    pub initial: usize,
    pub r#final: usize,
    pub discount_percent: usize,
    pub initial_formatted: String,
    pub final_formatted: String,
}

#[derive(Deserialize, Debug)]
pub struct StoreCategory {
    pub id: usize,
    pub description: String,
}

impl StoreCategory {
    const MULTI_PLAYER_ID: usize = 1;

    pub fn is_multi_player(&self) -> bool {
        self.id == Self::MULTI_PLAYER_ID
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum StoreAppDetail {
    #[serde(rename = "game")]
    Game(StoreGameDetail),
    #[serde(rename = "dlc")]
    Dlc,
    #[serde(rename = "music")]
    Music,
}

#[derive(Deserialize, Debug)]
pub struct StoreGameDetail {
    pub name: String,
    pub steam_appid: usize,
    pub is_free: bool,
    pub price_overview: Option<StorePriceOverview>,
    pub categories: Vec<StoreCategory>,
}

pub async fn get_app_detail(appid: &str) -> Result<StoreAppDetail> {
    #[derive(Deserialize, Debug)]
    struct StoreAppDetailResponse {
        #[allow(unused)]
        success: MustBe!(true),
        data: StoreAppDetail,
    }
    let mut responses: HashMap<String, StoreAppDetailResponse> = reqwest::Client::default()
        .get("http://store.steampowered.com/api/appdetails")
        .query(&[("appids", appid)])
        .send()
        .await
        .context("request errorr")?
        .json()
        .await
        .context("invalid json")?;
    match responses.remove(appid) {
        Some(StoreAppDetailResponse { data, .. }) => Ok(data),
        None => Err(anyhow!("Not found {appid}")),
    }
}
