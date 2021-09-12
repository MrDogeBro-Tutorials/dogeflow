use super::Context;
use anyhow::Result;
use serenity::futures::{future, StreamExt};
use serenity::model::{
    channel::{ChannelType, Message},
    id::{ChannelId, GuildId, RoleId},
};
use uuid::Uuid;

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
        .await?
        .id;

    support_channel
        .send_message(&ctx.discord().http, |m| {
            m.content("Support case opened.");

            m
        })
        .await?;

    let helpers = GuildId(*message.guild_id.unwrap().as_u64())
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

    for h in helpers.collect::<Vec<_>>().await.iter() {
        let helper = h.as_ref().unwrap();

        thread
            .add_thread_member(&ctx.discord().http, helper.user.id)
            .await?;
    }

    Ok(())
}
