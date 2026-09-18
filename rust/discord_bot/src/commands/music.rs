use poise::serenity_prelude as serenity;
use songbird::input::YoutubeDl;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use serenity::{async_trait};
use songbird::tracks::TrackHandle;

use crate::{Context, Error};
use crate::utils::search::{yt_query};
use std::collections::VecDeque;
use std::sync::Arc;
use reqwest::Client as HttpClient;
use std::sync::Mutex;

#[derive(Clone)]
pub struct Song {
    pub url: String,
    pub title: String,
}

pub struct MusicManager {
    pub queue: VecDeque<Song>,
    pub current_track: Option<TrackHandle>,
}

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn play(
    ctx: Context<'_ >,
    #[description = "The URL or search query for the song"] 
    #[rest] 
    url: String
) -> Result<(), Error> {
        
        play_link(ctx, &url).await?;
        Ok(())
}

struct SongEnd {
    guild_id: serenity::model::id::GuildId,
    http: HttpClient,
    ctx: serenity::client::Context,
    music_manager: Arc<Mutex<MusicManager>>,
    channel: serenity::model::id::ChannelId,
}

#[async_trait]
impl VoiceEventHandler for SongEnd {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = ctx {
            let next_song = {
                let mut mm = self.music_manager.lock().unwrap();
                mm.queue.pop_front()?
            };

            let manager = songbird::get(&self.ctx).await.expect("Songbird Voice client placed in at initialisation.").clone();
        
            if let Some(handler_lock) = manager.get(self.guild_id) {
                let src = YoutubeDl::new(self.http.clone(), next_song.url.clone());
                let _song = {
                    let mut handler = handler_lock.lock().await;
                    handler.play_input(src.into())
                };
                
                let _ = _song.add_event(
                    Event::Track(TrackEvent::End),
                    SongEnd {
                        guild_id: self.guild_id,
                        http: self.http.clone(),
                        ctx: self.ctx.clone(),
                        music_manager: self.music_manager.clone(),
                        channel: self.channel.clone(),
                    },
                );
                let _ = self.channel.say(&self.ctx.http, format!("🎶Now playing: **{}**", next_song.title)).await.ok();
                {
                    let mut mm = self.music_manager.lock().unwrap();
                    mm.current_track = Some(_song.clone());
                }
            }
        }
        None

    }
}

#[poise::command(prefix_command, guild_only)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    ensure_joined(ctx).await?;
    Ok(())
}

pub async fn ensure_joined(ctx: Context<'_>) -> Result<(), Error> {
    let (guild_id, channel_id) = {
        let guild = ctx.guild().unwrap();
        let channel_id = guild.voice_states.get(&ctx.author().id).and_then(|vs| vs.channel_id);
        (guild.id, channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("You are not in a voice channel!").await?;
            return Ok(());
        }
    };
    let manager = songbird::get(ctx.serenity_context()).await.expect("Songbird Voice client placed in at initialisation.").clone();
    if manager.get(guild_id).is_some() {
        return Ok(());
    }
    
    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        // Attach an event handler to see notifications of all track errors.
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        {
            ctx.data().music_manager.lock().unwrap().insert(guild_id, Arc::new(Mutex::new(MusicManager {
                current_track: None,
                queue: VecDeque::new(),
            })));
        }
    }    
    ctx.say(format!("Joined <#{}>", connect_to)).await?;

    Ok(())

}
struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

#[poise::command(prefix_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.expect("Songbird Voice client placed in at initialisation.").clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        let _ =handler.stop();
        ctx.say("Stopped the current track!").await?;
    } else {
        ctx.say("Not in a voice channel!").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let track = {
        let music_manager = ctx.data().music_manager.lock().unwrap().get(&guild_id).cloned();
        music_manager.and_then(|m| m.lock().unwrap().current_track.clone())
    };

    if let Some(handle) = track {
        let _ = handle.stop();   // fires TrackEnd -> SongEnd::act advances the queue
        ctx.say("⏭️ Skipped!").await?;
    } else {
        ctx.say("Nothing playing!").await?;
    }
    Ok(())
}


#[poise::command(prefix_command, guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let manager = songbird::get(ctx.serenity_context()).await.expect("Songbird Voice client placed in at initialisation.").clone();
    if manager.get(guild_id).is_some() {

        {
            let mut mm = ctx.data().music_manager.lock().unwrap();
            let _ = mm.remove(&guild_id);
        }


        {
            let handler_lock = manager.get(guild_id).unwrap();
            let mut handler = handler_lock.lock().await;
            handler.stop();
        }
        manager.remove(guild_id).await?;
        ctx.say("Left the voice channel!").await?;


    } else {
        ctx.say("Not in a voice channel!").await?;
    }
    Ok(())
}

#[poise::command(prefix_command, guild_only)]
pub async fn deadlock(ctx: Context<'_>) -> Result<(), Error> {
    let url = "https://www.youtube.com/playlist?list=PLgsHQI3t5TmisHbRYDnNj9bahlT_5YtG8";
    play_link(ctx, url).await?;
    Ok(())
}

async fn play_link(
    ctx: Context<'_ >,
    url: &str)     
    -> Result<(), Error> 
{
    if url.is_empty() {
        ctx.say("You must provide a URL!").await?;
        return Ok(());
    }
    println!("Playing URL: {}", url);
    ensure_joined(ctx).await?;

    let guild_id = ctx.guild_id().unwrap();

    let http_client = ctx.data().http_client.clone();

    let manager = songbird::get(ctx.serenity_context()).await.expect("Songbird Voice client placed in at initialisation.").clone();
    
    let mut query_result = VecDeque::from(yt_query(url).await?);

    if query_result.is_empty() {
        ctx.say("No results found for the query!").await?;
        return Ok(());
    }

    let music_manager = if let Some(manager) = ctx.data().music_manager.lock().unwrap().get(&guild_id) {
        manager.clone()
    } else {
        let new_manager = Arc::new(Mutex::new(MusicManager {
            queue: VecDeque::new(),
            current_track: None,
        }));
        ctx.data().music_manager.lock().unwrap().insert(guild_id, new_manager.clone());
        new_manager
    };

    let old_handle = {
        let tracks = music_manager.lock().unwrap();
        tracks.current_track.clone()
    };

    if let Some(old) = old_handle {
        if let Ok(info) = old.get_info().await {
            if info.playing == songbird::tracks::PlayMode::Play {
                if !query_result.is_empty() {
                    {
                        let mut queue = music_manager.lock().unwrap().queue.clone();
                        queue.extend(query_result.clone().into_iter());
                    }
                    ctx.say(format!("Added **{}** to the queue!", query_result.clone()[0].title)).await?;
                    return Ok(());
                }
            }
            
        }
    }
    

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;


        let upcoming_song = query_result.pop_front().unwrap();
        println!("Playing title: {}", upcoming_song.title);

        let src = YoutubeDl::new(http_client.clone(), upcoming_song.url.clone());

        let song = handler.play_input(src.clone().into());

        let _ = song.add_event(
            Event::Track(TrackEvent::End),
            SongEnd {
                guild_id: guild_id,
                http: http_client.clone(),
                ctx: ctx.serenity_context().clone(),
                music_manager: music_manager.clone(),
                channel: ctx.channel_id().clone(),
            },
        );

        ctx.say(format!("🎶Now playing: **{}**", upcoming_song.title)).await?;
        {
            music_manager.lock().unwrap().current_track = Some(song.clone());
        }
    }
    else{
        ctx.say("Not in a voice channel!").await?;
        return Ok(());
    }

    
    if !query_result.is_empty() {
        let mut manager = music_manager.lock().unwrap();
        
        println!("Adding {} songs to the queue.", query_result.clone().len());
        manager.queue.extend(query_result.clone().into_iter());
    }

    Ok(())
}


// async fn play_song(src: YouTubeDl) -> Result<(), Error> {
// }