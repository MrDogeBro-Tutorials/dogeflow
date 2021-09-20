use super::Context;
use anyhow::Result;
use chrono::{prelude::Utc, DateTime, Duration, SecondsFormat};
use serenity::futures::{future, StreamExt};
use serenity::model::{
    channel::{ChannelType, GuildChannel, Message},
    id::{ChannelId, RoleId},
};
use uuid::Uuid;

// ========================================================================================
//                                  Create Support Thread
// ========================================================================================

pub async fn create_new(ctx: Context<'_>, message: Message) -> Result<()> {
    let uuid: String = Uuid::new_v4().to_string()[..6].to_string();
    let support_channel = ChannelId(ctx.data().config.env.support_channel_id);

    let thread = support_channel
        .create_public_thread(&ctx.discord().http, message.id, |t| {
            t.name("case-".to_string() + &uuid);
            t.auto_archive_duration(1440);
            t.kind(ChannelType::PublicThread);

            t
        })
        .await?;

    ctx.data().db.lock().unwrap().conn.execute(
        "INSERT INTO support (id, owner_id, thread_id, created_at) VALUES (:id, :owid, :thid, :creat)",
            &[(":id", &uuid),
            (":owid", &message.author.id.as_u64().to_string()),
            (":thid", &thread.id.as_u64().to_string()),
            (":creat", &Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true))]
        )?;

    support_channel
        .send_message(&ctx.discord().http, |m| {
            m.content("Support case opened.");

            m
        })
        .await?;

    Ok(())
}

// ========================================================================================
//                                  Call Command
// ========================================================================================

/// Calls the helpers to your support case.
///
/// Calls the helpers to your support case. However, this command cannot be used unless it has been at least 30m since the case opened. ```
/// <<prefix>>call
/// ```
#[poise::command(slash_command)]
pub async fn call(ctx: Context<'_>) -> Result<()> {
    let thread_id = ctx.channel_id();
    let thread: GuildChannel = thread_id
        .to_channel(&ctx.discord().http)
        .await?
        .guild()
        .unwrap();
    let mut query_successful: bool = false;

    let helpers = ctx
        .guild_id()
        .unwrap()
        .members_iter(&ctx.discord().http)
        .filter(|u| {
            if let Some(u) = u.as_ref().ok() {
                future::ready(
                    u.roles
                        .contains(&RoleId(ctx.data().config.env.helper_role_id)),
                )
            } else {
                future::ready(false)
            }
        });

    if thread.kind != ChannelType::PublicThread || !thread.name.starts_with("case-") {
        poise::send_reply(ctx, |m| {
            m.content("The call command can only be used within support cases.")
        })
        .await?;

        return Ok(());
    }

    let created_at: String = match ctx.data().db.lock().unwrap().conn.query_row_and_then(
        "SELECT created_at FROM support WHERE id = ?",
        [thread.name[5..].to_string()],
        |r| r.get(0),
    ) {
        Ok(timestamp) => {
            query_successful = true;
            timestamp
        }
        Err(_) => "Unable to get `created_at` for the current support case.".to_string(),
    };

    if query_successful {
        let duration: Duration = Utc::now() - created_at.parse::<DateTime<Utc>>()?;

        if duration.num_minutes() < 30 {
            poise::send_reply(ctx, |m| {
                m.content(
                    "You cannot call the helpers until at least 30m after opening your support case! \
                  We do this because all of our staff team is volunteers and we want to give them \
                  a chance to see and respond to your support case first before pinging them.",
                )
            })
            .await?;

            return Ok(());
        }
    }

    for h in helpers.collect::<Vec<_>>().await.iter() {
        let helper = h.as_ref().unwrap();

        thread_id
            .add_thread_member(&ctx.discord().http, helper.user.id)
            .await?;
    }

    poise::send_reply(ctx, |m| {
        m.content("The helpers have been called to your support case.")
    })
    .await?;

    Ok(())
}

// ========================================================================================
//                                  Close Command
// ========================================================================================

/// Closes your support case.
///
/// Closes your support case so its not just sitting open even though it is done. ```
/// <<prefix>>close
/// ```
#[poise::command(slash_command)]
pub async fn close(ctx: Context<'_>) -> Result<()> {
    let thread_id = ctx.channel_id();
    let thread: GuildChannel = thread_id
        .to_channel(&ctx.discord().http)
        .await?
        .guild()
        .unwrap();
    let mut query_successful: bool = false;

    if thread.kind != ChannelType::PublicThread || !thread.name.starts_with("case-") {
        poise::send_reply(ctx, |m| {
            m.content("The close command can only be used within support cases.")
        })
        .await?;

        return Ok(());
    }

    let owner_id: u64 = match ctx.data().db.lock().unwrap().conn.query_row_and_then(
        "SELECT owner_id FROM support WHERE id = ?",
        [thread.name[5..].to_string()],
        |r| r.get(0),
    ) {
        Ok(owner) => {
            query_successful = true;
            owner
        }
        Err(_) => 0,
    };

    if !query_successful {
        poise::send_reply(ctx, |m| m.content("Unable to get the owner of this support case. As a result, the support case must manually be closed by staff.")).await?;
        return Ok(());
    }

    if ctx.author().id.as_u64() != &owner_id
        && !ctx
            .author()
            .has_role(
                &ctx.discord().http,
                ctx.guild_id().unwrap(),
                RoleId(ctx.data().config.env.helper_role_id),
            )
            .await?
    {
        poise::send_reply(ctx, |m| {
            m.content("Only the support case author and staff members can close a support case!")
        })
        .await?;
        return Ok(());
    }

    poise::send_reply(ctx, |m| {
        m.content("This support case has been closed and can only be re-opened by a staff member.")
    })
    .await?;

    Ok(())
}
