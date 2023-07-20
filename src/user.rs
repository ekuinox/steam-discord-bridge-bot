use anyhow::Result;
use serde::{Deserialize, Serialize};
use shuttle_persist::PersistInstance;

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Debug)]
pub struct User(String);

impl User {
    pub fn new(steam_id: String) -> User {
        User(steam_id)
    }

    pub fn steam_id(&self) -> &str {
        &self.0
    }

    pub fn save(&self, discord_id: &str, persist: &PersistInstance) -> Result<()> {
        persist.save(&Self::generate_persist_key(discord_id), &self)?;
        Ok(())
    }

    pub fn load(discord_id: &str, persist: &PersistInstance) -> Result<User> {
        let user = persist.load(&Self::generate_persist_key(discord_id))?;
        Ok(user)
    }

    fn generate_persist_key(discord_id: &str) -> String {
        format!("discord-user-{discord_id}")
    }
}
