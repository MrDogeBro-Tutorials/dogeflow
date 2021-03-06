mod commands;
mod config;
mod db;
mod hub;
mod utils;

extern crate serde_json;

use anyhow::{Error, Result};
use chrono::{prelude::Utc, DateTime};
use serenity::{
    builder::CreateApplicationCommands, model::prelude::ApplicationId,
    prelude::Context as SerenityContext,
};
use std::sync::Mutex;
use std::time::Duration;

pub type Context<'a> = poise::Context<'a, State, Error>;
pub type PrefixContext<'a> = poise::PrefixContext<'a, State, Error>;

pub struct State {
    config: config::Config,
    hub: hub::Hub,
    start_time: DateTime<Utc>,
    connected: Mutex<bool>,
    db: Mutex<db::Database>,
}

impl State {
    pub async fn load() -> Result<Self> {
        let config = config::Config::load()?;

        Ok(Self {
            hub: hub::Hub::load(&config)?,
            start_time: Utc::now(),
            connected: Mutex::new(false),
            db: Mutex::new(db::Database::load(&config.data_path.dynamic)?),
            config,
        })
    }

    pub async fn set_connected(&self) -> Result<()> {
        let mut conn = self.connected.lock().unwrap();
        *conn = true;

        Ok(())
    }
}

async fn listener(
    ctx: &SerenityContext,
    event: &poise::Event<'_>,
    framework: &poise::Framework<State, Error>,
    state: &State,
) -> Result<()> {
    match event {
        poise::Event::Ready { .. } => {
            if *state.connected.lock().unwrap() {
                println!("Bot reconnected!");
                return Ok(());
            }

            state.set_connected().await?;
            println!("Bot connected!");

            state
                .hub
                .stdout
                .send_message(&ctx.http, |m| {
                    m.content(format!("DogeFlow v{} started.", env!("CARGO_PKG_VERSION")))
                })
                .await?;

            if cfg!(debug_assertions) {
                // register only for test guild in develop
                let commands = ctx
                    .http
                    .get_guild_application_commands(state.config.env.hub_server_id)
                    .await?;

                for cmd in commands {
                    ctx.http
                        .delete_guild_application_command(state.config.env.hub_server_id, cmd.id.0)
                        .await?;
                }

                println!("Commands unregistered (develop)");

                let mut commands_builder = CreateApplicationCommands::default();
                let commands = &framework.options().application_options.commands;

                for cmd in commands {
                    commands_builder.create_application_command(|f| cmd.create(f));
                }

                let json_value = serde_json::Value::Array(commands_builder.0);
                ctx.http
                    .create_guild_application_commands(state.config.env.hub_server_id, &json_value)
                    .await?;

                println!("Commands registered (develop)");
            } else {
                // register globally in prod
                let commands = ctx.http.get_global_application_commands().await?;

                for cmd in commands {
                    ctx.http.delete_global_application_command(cmd.id.0).await?;
                }

                println!("Commands unregistered");

                let mut commands_builder = CreateApplicationCommands::default();
                let commands = &framework.options().application_options.commands;

                for cmd in commands {
                    commands_builder.create_application_command(|f| cmd.create(f));
                }

                let json_value = serde_json::Value::Array(commands_builder.0);
                ctx.http
                    .create_global_application_commands(&json_value)
                    .await?;

                println!("Commands registered");
            }
        }
        poise::Event::Message { new_message, .. } => {
            if new_message.channel_id == state.config.env.support_channel_id
                && !new_message.author.bot
            {
                let new_ctx = poise::PrefixContext {
                    data: state,
                    discord: ctx,
                    msg: new_message,
                    framework,
                    command: None,
                };
                commands::support::create_new(
                    poise::Context::Prefix(new_ctx),
                    new_message.to_owned(),
                )
                .await?;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn on_error(error: Error, ctx: poise::ErrorContext<'_, State, Error>) {
    match ctx {
        poise::ErrorContext::Setup => panic!("Failed to start bot: {:?}", error),
        poise::ErrorContext::Command(ctx) => {
            println!("Error in command `{}`: {:?}", ctx.command().name(), error)
        }
        _ => println!("Other error: {:?}", error),
    }
}

fn init_framework() -> Result<poise::FrameworkOptions<State, Error>> {
    let mut options = poise::FrameworkOptions {
        listener: |ctx, event, framework, state| Box::pin(listener(ctx, event, framework, state)),
        prefix_options: poise::PrefixFrameworkOptions {
            edit_tracker: Some(poise::EditTracker::for_timespan(Duration::from_secs(3600))),
            ..Default::default()
        },
        on_error: |error, ctx| Box::pin(on_error(error, ctx)),
        ..Default::default()
    };

    options = commands::command_list(options)?;
    Ok(options)
}

#[tokio::main]
async fn main() -> Result<()> {
    let env = config::Env::load()?;

    let framework = poise::Framework::new(
        env.prefix.to_owned(),
        ApplicationId(env.application_id),
        |_, _, _| Box::pin(State::load()),
        init_framework()?,
    );
    framework
        .start(serenity::client::ClientBuilder::new(env.token))
        .await?;

    Ok(())
}
