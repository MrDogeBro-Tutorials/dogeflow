use super::Context;
use anyhow::Result;
use serenity::futures::StreamExt;
use serenity::model::{
    channel::{ChannelType, Message},
    id::{ChannelId, GuildId, RoleId},
};
use uuid::Uuid;

pub async fn create_new(ctx: Context<'_>, message: Message) -> Result<()> {
    let uuid: String = Uuid::new_v4().to_string()[..6].to_string();
    let support_channel = ChannelId(ctx.data().config.env.support_channel_id);

    println!("{}", uuid);

    let thread = support_channel
        .create_public_thread(&ctx.discord().http, message.id, |t| {
            t.name("case-".to_string() + &uuid);
            t.auto_archive_duration(60);
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

    Ok(())
}
