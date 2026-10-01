mod commands;
mod utils;
use std::env;
use dotenvy::from_path;
use songbird::SerenityInit;

use serenity::model::id::GuildId;
use reqwest::Client as HttpClient;
use std::{
    collections::{HashMap},
    sync::{Arc, Mutex},
    time::Duration,
};
use commands::music::MusicManager;
use serenity::client::Context as SerenityContext;

use serenity::{
    async_trait,
    client::{EventHandler},
    model::{gateway::Ready},
    prelude::{GatewayIntents, TypeMapKey},
};

use poise::serenity_prelude as serenity;

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

// poise wants a shared data struct and an error type
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    http_client: HttpClient,
    pub music_manager: Mutex<HashMap<GuildId, Arc<Mutex<MusicManager>>>>,
}


struct Handler; 

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: SerenityContext, ready: Ready) {
        commands::steam::start_daily_task(_ctx.http);
        println!("{} is connected!", ready.user.name);
    }
}

fn commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        commands::ping::ping(),
        commands::music::play(),
        commands::music::join(),
        commands::music::stop(),
        commands::music::leave(),
        commands::music::skip(),
        commands::music::deadlock(),
        commands::steam::game(),
        commands::convert::video(),
        commands::convert::deepfry(),
        commands::convert::gif(),
        commands::convert::corrupt(),
    ]
}

#[tokio::main]
async fn main() {
    from_path("../../.env").ok();
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    
    //for (key, value) in env::vars() {

    let options = poise::FrameworkOptions {
        commands: commands(),
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            ..Default::default()
        },
        post_command: |ctx| {
            Box::pin(async move {
                if let poise::Context::Prefix(prefix_ctx) = ctx {
                    let _ = prefix_ctx.msg.delete(&ctx.http()).await;
                }                
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    http_client: HttpClient::new(),
                    music_manager: Mutex::new(HashMap::new()),
                })
            })
        })
        .options(options)
        .build();


    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_VOICE_STATES;
    

    let mut client = serenity::ClientBuilder::new(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Error creating client");

    client.start().await.unwrap()
}


#[poise::command(prefix_command, guild_only)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

