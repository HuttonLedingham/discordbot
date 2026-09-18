import os

import discord
from discord.ext import commands
from .helpers.search import process_search
from .helpers.utils import read_dict_from_disk

import yt_dlp
import asyncio
from collections import deque
import random
import json

MUSIC_DATA_PATH = "../data/music_data"

FFMPEG_OPTIONS = {
    'before_options': '-nostdin -reconnect 1 -reconnect_streamed 1 -reconnect_delay_max 5',
    'options': '-vn' 
}

YTDL_OPTIONS = {

    'format': 'ba/ba*/bestaudio/b/best',
    'noplaylist': False,
    'quiet': True,
    'extract_flat': 'in_playlist',
    'skip_download': True,
    'rm_cachedir': True,

    'nocheckcertificate': True,
    'ignoreerrors': False,
    'logtostderr': False,
    'no_warnings': True,
    'default_search': 'auto',
    'source_address': '0.0.0.0',

    'extractor_args': {
            'youtube': {
                'player_client': ['android', 'ios', 'web'],
                'player_skip': ['webpage', 'configs'],
            }
        },
}


class Music(commands.Cog):
    def __init__(self, bot):

        self.queues = {}
        self.current_song = {}
        self.loop_state = {}
        self.saved_playlists = {}
        self.current_playlist = {}

        self.saved_playlists = read_dict_from_disk(f"{MUSIC_DATA_PATH}/playlist.json")
        if self.saved_playlists == {}:
            print("No saved playlists found. Starting with an empty playlist dictionary.")
        self.bot = bot


    async def play_song(self, ctx, song_info=None):
        async def validate_song(song_info, guild_id=None):
            if not song_info:
                return None

            url = song_info['url']

            if len(url) < 100:  # If the URL is likely a YouTube link, re-extract it
                def _extract():
                    with yt_dlp.YoutubeDL(YTDL_OPTIONS) as ytdl:
                        return ytdl.extract_info(url, download=False)
                song_info = _extract()

            return song_info

        song_info = await validate_song(song_info, ctx.guild.id)
        if not song_info:
            return
        
        if not ctx.guild.voice_client:
            return


        ctx.guild.voice_client.play(
            discord.FFmpegOpusAudio(song_info['url'], **FFMPEG_OPTIONS),
            after=lambda e: asyncio.run_coroutine_threadsafe(self.check_queue(ctx), self.bot.loop)
        )
        await ctx.channel.send(f"🎶 Now playing: **{song_info['title']}**")
        await self.bot.change_presence(activity=discord.Game(name=f"🎶 {song_info['title']}"))

        self.current_song[ctx.guild.id] = song_info


    async def check_queue(self, ctx):
        """Checks if there are more songs in the queue and plays them."""
        if not ctx.guild.voice_client:
            return

        guild_id = ctx.guild.id

        queue = self.queues.get(guild_id, deque())

        if len(queue) > 0:
            if not self.loop_state.get(guild_id, False):
                next_song = queue.popleft()
            else:
                next_song = self.current_song[guild_id]

            await self.play_song(ctx, next_song)
        else:
            self.queues[guild_id] = None # Clear the queue if no more songs

    async def cache_playlist(self, playlist):
        """Caches a playlist for the guild."""
        video_list = playlist.get("playlist", [])

        if len(video_list) == 0:
            video_list, title = await process_search(playlist["url"])  # Load the playlist from the URL if not already loaded
            
            if len(video_list) == 0:
                return deque()  # Return an empty queue if no songs were found

            playlist["playlist"] = list(video_list)  # Cache the playlist in memory

            await asyncio.to_thread(write_dict_to_disk, self.saved_playlists)  # Save the updated playlists to JSON in a background thread
        
        return deque(video_list)  # Load the saved playlist into the queue


    @commands.command(name="join")
    async def join(self, ctx):
        """Commands the bot to join the user's voice channel."""
        if not ctx.author.voice:
            return await ctx.send("❌ You must be in a voice channel first!")
        
        channel = ctx.author.voice.channel

        if ctx.voice_client is None:
            guild_id = ctx.guild.id
            self.queues[guild_id] = deque()  # Ensure the queue exists

            await channel.connect()
            await ctx.send(f"🔊 Joined **{channel.name}**")
        elif ctx.voice_client.channel == channel:
            return
        elif ctx.voice_client is not None:
            return await ctx.voice_client.move_to(channel)


    @commands.command(name="loop")
    async def loop(self, ctx):
        """Repeats the currently playing song."""
        guild_id = ctx.guild.id
        if ctx.voice_client and ctx.voice_client.is_playing():
            self.loop_state[guild_id] = not self.loop_state.get(guild_id, False)  # Toggle loop state
            song_info = self.current_song[guild_id]  # Get the currently playing song

            if self.loop_state[guild_id]:
                await ctx.send(f"🔁 Looping: **{song_info['title']}**")
            else:
                await ctx.send(f"🔁 No Longer Looping: **{song_info['title']}**")
        else:
            await ctx.send("❌ No song is currently playing to repeat.")

    @commands.command(name="nowplaying")
    async def now_playing(self, ctx):
        """Shows the currently playing song."""
        guild_id = ctx.guild.id
        if ctx.voice_client and ctx.voice_client.is_playing():
            song_info = self.current_song[guild_id]
            await ctx.send(f"🎶 Now playing: **{song_info['title']}**")
        else:
            await ctx.send("❌ No song is currently playing.")

    @commands.command(name="restart")
    async def restart(self, ctx):
        """Restarts the currently playing song."""
        guild_id = ctx.guild.id
        if ctx.voice_client and (ctx.voice_client.is_playing() or ctx.voice_client.is_paused()):
            song_info =  self.current_song[guild_id]
            self.queues[guild_id].appendleft(song_info)  # Add the current song to the front of the queue
            ctx.voice_client.stop()  # This will trigger check_queue and play the same song again
            
        else:
            await ctx.send("❌ No song is currently playing to restart.")

    @commands.command(name="play")
    async def play(self, ctx, *, search: str = None):
        """Plays a song from a link or keywords, adding to queue if active."""
        if not ctx.voice_client:
            await ctx.invoke(self.bot.get_command("join"))
            if not ctx.voice_client:
                return

        if ctx.voice_client and ctx.voice_client.is_paused():
            ctx.voice_client.resume()
            await ctx.send("▶️ Stream resumed.")
            
        if not search:
            return

        await ctx.send("🔍 Searching... Please wait.")
        guild_id = ctx.guild.id


        queue, title = await process_search(search)

        if len(queue) == 0:
            await ctx.send("❌ No results found.")
            return
        elif len(queue) > 1:
            await ctx.send(f"📋 Added [{title}]({search}) to the queue.")

        song_info = queue.popleft()  # Get the first song in the queue
        self.queues[guild_id].extend(queue)  # Add the new songs to the queue

        if ctx.voice_client.is_playing() or ctx.voice_client.is_paused():
            self.queues[guild_id].appendleft(song_info)
            ctx.voice_client.stop()
            # Skip to the next song to play the newly added song
        else:
            await self.play_song(ctx, song_info)
        

    @commands.command(name="pause")
    async def pause(self, ctx):
        """Pauses the currently playing audio."""
        if ctx.voice_client and ctx.voice_client.is_playing():
            ctx.voice_client.pause()
            await ctx.send("⏸️ Stream paused.")
        else:
            await ctx.send("❌ Nothing is playing right now.")

    @commands.command(name="resume")
    async def resume(self, ctx):
        """Resumes paused audio."""
        if ctx.voice_client and ctx.voice_client.is_paused():
            ctx.voice_client.resume()
            await ctx.send("▶️ Stream resumed.")
        else:
            await ctx.send("❌ Audio is not paused.")

    @commands.command(name="skip")
    async def skip(self, ctx):
        """Skips the currently playing song."""
        if ctx.voice_client and (ctx.voice_client.is_playing() or ctx.voice_client.is_paused()):
            ctx.voice_client.stop() # Triggers the 'after' lambda automatically to load next song
            await ctx.send("⏭️ Skipped song.")
        else:
            await ctx.send("❌ Nothing to skip.")

    @commands.command(name="stop")
    async def stop(self, ctx):
        """Clears the queue and disconnects the bot from voice."""
        guild_id = ctx.guild.id
        self.queues[guild_id] = None # Clear the queue
        self.current_song[guild_id] = None  # Clear the current song
        if ctx.voice_client:
            ctx.voice_client.stop()

            await ctx.voice_client.disconnect()

            

    @commands.command(name="clear")
    async def clear(self, ctx):
        """Clears the current queue without disconnecting."""
        guild_id = ctx.guild.id
        if guild_id in self.queues:
            self.queues[guild_id] = deque()
            await ctx.send("🗑️ Queue cleared.")
        else:
            await ctx.send("❌ No queue to clear.")

    @commands.command(name="add")
    async def add(self, ctx, *, search: str):
        """Adds a song to the queue."""

        if not ctx.voice_client:
            await ctx.invoke(self.bot.get_command("join"))
            if not ctx.voice_client:
                return

        queue, title = await process_search(search)
        guild_id = ctx.guild.id
        self.queues.setdefault(guild_id, deque())  # Ensure the queue exists
        self.queues[guild_id].extend(queue)  # Add the new songs to the queue

        song_info = {'url': info['url'], 'title': info['title']}
        guild_id = ctx.guild.id

        if ctx.voice_client and not ctx.voice_client.is_playing():
            await self.play_song(ctx, song_info)  # If nothing is playing, start playing the newly added song

        await ctx.send(f"📋 Added to queue: **{search}**")

    @commands.command(name="shuffle")
    async def shuffle(self, ctx):
        """Shuffles the current queue."""


        guild_id = ctx.guild.id
        queue = self.queues.get(guild_id, deque())
        if len(queue) > 1:
            shuffle = list(queue)  # Convert deque to list for shuffling
            random.shuffle(shuffle)
            self.queues[guild_id] = deque(shuffle)  # Convert back to deque
            self.queues[guild_id].appendleft(self.current_song[guild_id])  # Keep the current song at the front
            ctx.voice_client.stop()   # Skip to the next song to play the shuffled queue
        else:
            await ctx.send("❌ No queue to shuffle.")

    @commands.command(name="deadlock")
    async def deadlock(self, ctx, shuffle: bool = False):
        """Plays the 'Deadlock' playlist."""
        await ctx.invoke(self.bot.get_command("loadplaylist"), name="deadlock", shuffle=shuffle)
        

    @commands.command(name="saveplaylist")
    async def saveplaylist(self, ctx, name: str, url: str):
        """Saves a playlist under the given name."""
        guild_id = ctx.guild.id

        guild_playlists = self.saved_playlists.setdefault(str(guild_id), {})

        for saved_name, saved_data in guild_playlists.items():
            if saved_name.lower() == name.lower():
                await ctx.send(f"❌ A playlist named '{name}' already exists. Please choose a different name.")
                return
            if url.lower() == saved_data["url"].lower():
                await ctx.send(f"❌ A playlist with the URL '{url}' already exists under the name '{saved_name}'. Please choose a different URL.")
                return

        new_playlist = guild_playlists.get(name, {})
        new_playlist["url"] = url

        cached_queue = await self.cache_playlist(new_playlist)
        new_playlist["playlist"] = cached_queue  # Store the cached playlist in memory
        
        if len(cached_queue) == 0:
            await ctx.send("❌ Could not find any songs in the playlist.")
            return

        await ctx.send(f"💾 Playlist '({name})[{url}]' saved.")

    @commands.command(name="loadplaylist")
    async def loadplaylist(self, ctx, name: str = None, shuffle: bool = False):
        """Loads a saved playlist by name."""
        if ctx.voice_client is None:
            await ctx.invoke(self.bot.get_command("join"))
            if ctx.voice_client is None:
                return

        guild_id = ctx.guild.id
        guild_playlists = self.saved_playlists.get(str(guild_id), {})  # Merge universal playlists into guild playlists

        playlist = guild_playlists.get(name)

        if not playlist:
            await ctx.send(f"❌ No playlist named '{name}' found.")
            return


        self.queues[guild_id] = await self.cache_playlist(playlist)  # Load the saved playlist into the queue

        if len(self.queues[guild_id]) > 0:
            if shuffle:
                import random
                random.shuffle(self.queues[guild_id])

            await self.play_song(ctx, self.queues[guild_id].popleft())  # Start playing the first song in the playlist
            self.current_playlist[guild_id] = name  # Set the current playlist name
            await ctx.send(f"🎶 Loaded playlist '{name}' with {len(self.queues[guild_id])} songs.")
        else:
            await ctx.send(f"❌ Playlist '{name}' is empty.")


    @commands.command(name="listplaylists")
    async def listplaylists(self, ctx):
        """Lists all saved playlists for the guild."""
        guild_id = ctx.guild.id
        guild_playlists = self.saved_playlists.get(str(guild_id), {})  # Merge universal playlists into guild playlists

        if guild_playlists:
            playlist_names = "\n".join(f"- {name}" for name in guild_playlists.keys())
            await ctx.send(f"📋 Saved playlists:\n{playlist_names}")
        else:
            await ctx.send("❌ No saved playlists found.")

    @commands.Cog.listener()
    async def on_command_error(self, ctx, error):
        # Print the raw exception to console
        print(f"Error in command '{ctx.command}':", error)
            
    async def vc_disconnect(self, guild_id):
        """Handles the bot's disconnection from a voice channel."""
        print(f"Bot has disconnected from the voice channel in guild {guild_id}.")
        self.queues.pop(guild_id, None)  # Clear the queue for this guild
        self.current_song.pop(guild_id, None)  # Clear the current song for this guild


        # await delete_all_bot_messages(guild_id)  # Delete all bot messages in the guild
        await self.bot.change_presence(activity=None)  # Clear activity if no song is playing


async def setup(bot):
    await bot.add_cog(Music(bot))

