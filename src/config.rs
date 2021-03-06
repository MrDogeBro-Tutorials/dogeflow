use anyhow::Result;
use dotenv::dotenv;
use std::env::var;
use std::time::Duration;

#[derive(Clone)]
pub struct Config {
    pub env: Env,
    pub data_path: DataPath,
}

#[derive(Clone)]
pub struct Env {
    pub token: String,
    pub owner_id: String,
    pub application_id: u64,
    pub prefix: String,
    pub default_embed_color: serenity::utils::Colour,
    pub default_interaction_timeout: Duration,
    pub hub_server_id: u64,
    pub hub_stdout_id: u64,
    pub support_channel_id: u64,
    pub helper_role_id: u64,
    pub staff_role_id: u64,
}

#[derive(Clone)]
pub struct DataPath {
    pub dynamic: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let base_data_path = "data";

        Ok(Self {
            env: Env::load()?,
            data_path: DataPath {
                dynamic: format!("{}/dynamic", base_data_path),
            },
        })
    }
}

impl Env {
    pub fn load() -> Result<Self> {
        dotenv().ok();

        let default_embed_color: Vec<u8> = var("DEFAULT_EMBED_COLOR")?
            .replace("(", "")
            .replace(")", "")
            .replace(" ", "")
            .split(",")
            .collect::<Vec<&str>>()
            .iter()
            .map(|c| -> u8 { c.parse::<u8>().unwrap() })
            .collect();

        Ok(Self {
            token: var("TOKEN")?,
            owner_id: var("OWNER_ID")?,
            application_id: var("APPLICATION_ID")?.parse()?,
            prefix: var("PREFIX")?,
            default_embed_color: serenity::utils::Colour::from_rgb(
                *default_embed_color.get(0).unwrap(),
                *default_embed_color.get(1).unwrap(),
                *default_embed_color.get(2).unwrap(),
            ),
            default_interaction_timeout: Duration::from_secs(
                var("DEFAULT_INTERACTION_TIMEOUT")?.parse()?,
            ),
            hub_server_id: var("HUB_SERVER_ID")?.parse()?,
            hub_stdout_id: var("HUB_STDOUT_ID")?.parse()?,
            support_channel_id: var("SUPPORT_CHANNEL_ID")?.parse()?,
            helper_role_id: var("HELPER_ROLE_ID")?.parse()?,
            staff_role_id: var("STAFF_ROLE_ID")?.parse()?,
        })
    }
}
