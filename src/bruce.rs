use poise::serenity_prelude::{Http, Member, RoleId, UserId};
use poise::{serenity_prelude as serenity, PrefixFrameworkOptions};
use rusqlite::Connection;

use crate::membership::Membership;

mod membership;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

struct Data {
    member_role_name: String,
    privileged_role_name: String,
    student_id_length: usize,
    membership_purchase_url: Option<String>,
    sqlite_file: String,
    http: Http,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("failed to load .env");
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "bruce=info");
    }
    env_logger::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let member_role_name =
        std::env::var("MEMBER_ROLE_NAME").unwrap_or_else(|_| "Member".to_string());
    let privileged_role_name =
        std::env::var("PRIVILEGED_ROLE_NAME").unwrap_or_else(|_| "Committee".to_string());
    let student_id_length = std::env::var("STUDENT_ID_LENGTH")
        .unwrap_or_else(|_| 8.to_string())
        .parse()
        .expect("Failed to parse STUDENT_ID_LENGTH as number");
    let membership_purchase_url = std::env::var("MEMBERSHIP_PURCHASE_URL").ok();
    let sqlite_file = std::env::var("SQLITE_FILE").unwrap_or_else(|_| "db.sqlite".to_string());

    let user_data = Data {
        member_role_name,
        privileged_role_name,
        student_id_length,
        sqlite_file: sqlite_file.clone(),
        membership_purchase_url,
        http: Http::new(token.as_str()),
    };

    let conn = Connection::open(sqlite_file).unwrap();
    Membership::init_table(&conn);

    let framework = poise::Framework::build()
        .options(poise::FrameworkOptions {
            commands: vec![setup_commands(), register(), unregister(), prune()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("bruce!".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })
        .token(token)
        .intents(
            serenity::GatewayIntents::non_privileged()
                | serenity::GatewayIntents::GUILD_MEMBERS
                | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .user_data_setup(move |_ctx, _ready, _framework| Box::pin(async move { Ok(user_data) }));

    framework.run().await.unwrap();
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
) -> Result<(), Error> {
    let data = ctx.data();
    let author_member = ctx.author_member().await.unwrap();
    if target_member.is_some()
        && author_member.user.id != target_member.as_ref().unwrap().user.id
        && !author_member.roles.contains(&get_privileged_role(ctx))
    {
        ctx.say("You don't have the required permissions to target a user")
            .await?;
        return Ok(());
    }
    let mut target_member = target_member.unwrap_or_else(|| author_member.clone());
    if student_id.to_string().len() != data.student_id_length {
        ctx.say("I don't think that's a student id!").await?;
        return Ok(());
    }

    ctx.say(format!(
        "Ok, I'm verifying membership for {} :rocket:",
        target_member.display_name()
    ))
    .await?;

    let conn = Connection::open(&data.sqlite_file).expect("failed to open sqlite connection");

    if Membership::get_by_discord_id(&conn, *target_member.user.id.as_u64()).is_some() {
        ctx.say(format!(
            "Target user ({}) is already registered, use /unregister to remove them",
            target_member.display_name()
        ))
        .await?;
        return Ok(());
    }

    let membership = Membership::get_by_student_id(&conn, student_id);

    if membership.is_none() {
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

    membership.update_disord_id(&conn, Some(*target_member.user.id.as_u64()));

    target_member
        .add_role(&data.http, get_member_role(ctx))
        .await?;

    let result = target_member
        .edit(&data.http, |edit| edit.nickname(&membership.name))
        .await;
    if result.is_err() {
        ctx.say(format!(
            "Done! Please change your nickname to: {}",
            &membership.name
        ))
        .await?;
    }
    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn unregister(
    ctx: Context<'_>,
    #[description = "The discord member to unregister"] mut target_member: Member,
) -> Result<(), Error> {
    let author_member = ctx.author_member().await.unwrap();
    if !author_member.roles.contains(&get_privileged_role(ctx)) {
        ctx.say("Only privileged users can run this command")
            .await?;
        return Ok(());
    }
    let conn = Connection::open(&ctx.data().sqlite_file).expect("open sqlite db");
    target_member
        .remove_role(&ctx.data().http, get_member_role(ctx))
        .await?;
    if let Some(mut m) = Membership::get_by_discord_id(&conn, *target_member.user.id.as_u64()) {
        m.update_disord_id(&conn, None)
    }
    ctx.say("User unregistered").await?;
    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn prune(ctx: Context<'_>) -> Result<(), Error> {
    let author_member = ctx.author_member().await.unwrap();
    log::info!("Prune called by {}", author_member.display_name());
    if !author_member.roles.contains(&get_privileged_role(ctx)) {
        ctx.say("Only privileged users can run this command")
            .await?;
        return Ok(());
    }
    let conn = Connection::open(&ctx.data().sqlite_file).expect("open sqlite db");
    let  memberships: Vec<Membership> = Membership::get_all(&conn).into_iter().filter(|m| m.should_drop).collect();
    ctx.say(format!("Dropping {} members", memberships.len())).await?;
    for membership in memberships {
        if let Some(discord_id) = membership.discord_id {
            let mut member = ctx.guild().unwrap().member(&ctx.data().http, UserId::from(discord_id)).await.unwrap();
            member.remove_role(&ctx.data().http, get_member_role(ctx)).await?;
            log::info!("Removing roles from {}", membership.student_id);
        }
        membership.delete(&conn);
    }
    Ok(())
}

fn get_member_role(ctx: Context<'_>) -> RoleId {
    ctx.guild()
        .unwrap()
        .role_by_name(ctx.data().member_role_name.as_str())
        .unwrap()
        .id
}

fn get_privileged_role(ctx: Context<'_>) -> RoleId {
    ctx.guild()
        .unwrap()
        .role_by_name(ctx.data().privileged_role_name.as_str())
        .unwrap()
        .id
}
