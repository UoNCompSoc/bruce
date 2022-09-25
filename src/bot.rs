use anyhow::{anyhow, Error, Result};
use poise::serenity_prelude::{Member, RoleId, UserId};
use poise::{serenity_prelude as serenity, FrameworkBuilder, PrefixFrameworkOptions};

use crate::config::Config;
use crate::membership::Membership;

type Context<'a> = poise::Context<'a, Config, Error>;

pub fn build_framework(config: Config) -> FrameworkBuilder<Config, Error> {
    poise::Framework::build()
        .options(poise::FrameworkOptions {
            commands: vec![setup_commands(), register(), unregister(), prune()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("bruce!".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })
        .token(&config.discord_token)
        .intents(
            serenity::GatewayIntents::non_privileged()
                | serenity::GatewayIntents::MESSAGE_CONTENT
                | serenity::GatewayIntents::GUILD_MEMBERS,
        )
        .user_data_setup(|_ctx, _ready, _framework| Box::pin(async { Ok(config) }))
}

#[poise::command(prefix_command)]
async fn setup_commands(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands(ctx, false).await?;
    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn register(
    ctx: Context<'_>,
    #[description = "Student ID"]
    #[max = 99999999]
    student_id: u32,
    #[description = "Discord member to perform registration on, or if empty, yourself"]
    target_member: Option<Member>,
) -> Result<()> {
    let data = ctx.data();
    let author_member = ctx
        .author_member()
        .await
        .ok_or_else(|| anyhow!("Failed to retrieve calling user"))?;
    let mut target_member = if let Some(target_member) = target_member {
        if author_member.user.id != target_member.user.id
            && !author_member.roles.contains(&get_privileged_role(ctx)?)
        {
            ctx.say("You don't have the required permissions to target a user")
                .await?;
            return Ok(());
        }
        target_member
    } else {
        author_member.clone()
    };
    if student_id.to_string().len() != data.student_id_length {
        ctx.say("I don't think that's a student id!").await?;
        return Ok(());
    }

    ctx.say(format!(
        "Ok, I'm verifying membership for {} :rocket:",
        target_member.display_name()
    ))
    .await?;

    let conn = data.get_sqlite_conn()?;

    if Membership::get_by_discord_id(&conn, *target_member.user.id.as_u64()).is_ok() {
        ctx.say(format!(
            "Target user ({}) is already registered, use /unregister to remove them or @ a committee member",
            target_member.display_name()
        ))
        .await?;
        return Ok(());
    }

    let membership = Membership::get_by_student_id(&conn, student_id);

    if membership.is_err() {
        let mut membership_link = "".to_string();
        if let Some(x) = &data.membership_purchase_url {
            membership_link = format!("You can grab a membership at {}\n", x);
        }
        ctx.say(format!("I can't find that student id in my database :flushed:\n{}If you've purchased a membership recently, wait up to 30 minutes and try again.", membership_link)).await?;
        return Ok(());
    }

    let mut membership = membership.unwrap();

    if let Some(id) = membership.discord_id {
        if *target_member.user.id.as_u64() != id as u64 {
            ctx.say("Somebody else has already registered with that student id :eyes:\nIf you think this is a mistake, please @ someone on Committee.").await?;
            return Ok(());
        }
    }

    membership.update_disord_id(&conn, Some(*target_member.user.id.as_u64()))?;

    target_member
        .add_role(ctx.data().get_http(), get_member_role(ctx)?)
        .await?;

    let result = target_member
        .edit(ctx.data().get_http(), |edit| {
            edit.nickname(&membership.name)
        })
        .await;
    if result.is_err() {
        ctx.say(format!(
            "Done! Please change your nickname to: {}",
            &membership.name
        ))
        .await?;
    } else {
        log::info!(
            "Registered user {} with id {}",
            target_member.user.name,
            membership.student_id
        )
    }
    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn unregister(
    ctx: Context<'_>,
    #[description = "The discord member to unregister"] mut target_member: Member,
) -> Result<(), Error> {
    let author_member = ctx
        .author_member()
        .await
        .ok_or_else(|| anyhow!("Failed to retrieve calling user"))?;
    if !author_member.roles.contains(&get_privileged_role(ctx)?) {
        ctx.say("Only privileged users can run this command")
            .await?;
        return Ok(());
    }
    let conn = ctx.data().get_sqlite_conn()?;
    target_member
        .remove_role(ctx.data().get_http(), get_member_role(ctx)?)
        .await?;
    if let Ok(mut m) = Membership::get_by_discord_id(&conn, *target_member.user.id.as_u64()) {
        m.update_disord_id(&conn, None)?;
    }
    ctx.say("User unregistered").await?;
    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn prune(ctx: Context<'_>) -> Result<(), Error> {
    let author_member = ctx
        .author_member()
        .await
        .ok_or_else(|| anyhow!("Failed to retrieve calling user"))?;
    log::info!("Prune called by {}", author_member.display_name());
    if !author_member.roles.contains(&get_privileged_role(ctx)?) {
        ctx.say("Only privileged users can run this command")
            .await?;
        return Ok(());
    }

    let conn = ctx.data().get_sqlite_conn()?;
    let memberships: Vec<Membership> = Membership::get_all(&conn)?
        .into_iter()
        .filter(|m| m.discord_id.is_some())
        .collect();
    let users = ctx
        .guild()
        .ok_or_else(|| anyhow!("Failed to retrieve server information"))?
        .members(ctx.data().get_http(), None, None)
        .await?;
    let user_count = ctx
        .guild()
        .ok_or_else(|| anyhow!("Failed to retrieve server information"))?
        .member_count as usize;

    if users.len() != user_count {
        return Err(anyhow!(
            "Failed to retrieve all users; Expected: {}, Actual: {}",
            user_count,
            users.len()
        ));
    }

    ctx.say(format!("Checking {} users", users.len())).await?;

    let mut count = 0;
    for mut member in users {
        let membership = memberships
            .iter()
            .find(|m| m.discord_id.unwrap() == *member.user.id.as_u64());
        if membership.is_none() || membership.unwrap().should_drop {
            member
                .remove_role(ctx.data().get_http(), get_member_role(ctx)?)
                .await?;
            count += 1;
            log::info!("Removing roles from {}", member.user.name);
        }
    }

    ctx.say(format!("Pruned {count} users")).await?;

    for membership in memberships.into_iter().filter(|m| m.should_drop) {
        membership.delete(&conn)?;
    }

    Ok(())
}

fn get_member_role(ctx: Context<'_>) -> Result<RoleId, Error> {
    get_role_id(ctx, ctx.data().member_role_name.as_str())
}

fn get_privileged_role(ctx: Context<'_>) -> Result<RoleId, Error> {
    get_role_id(ctx, ctx.data().privileged_role_name.as_str())
}

fn get_role_id(ctx: Context<'_>, role_name: &str) -> Result<RoleId, Error> {
    Ok(ctx
        .guild()
        .ok_or_else(|| anyhow!("Failed to retrieve server information"))?
        .role_by_name(role_name)
        .ok_or_else(|| anyhow!("Role {} could not be found", role_name))?
        .id)
}
