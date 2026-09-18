import discord

import requests
import asyncio
import random
from datetime import datetime, time
from zoneinfo import ZoneInfo
from discord.ext import tasks, commands
from helpers.utils import read_dict_from_disk, write_dict_to_disk
import dotenv
dotenv.load_dotenv()
import os

STEAM_DATA_FOLDER = "../data/steam_data"

STEAMAPI_KEY = os.getenv("STEAMAPI_KEY")
datetime_format = "%Y-%m-%d %H:%M:%S"


CHANNEL_ID = int(os.getenv("GOTD_CHANNEL"))  # Replace with your Channel ID
target_time = time(hour=9, minute=0, second=0, tzinfo=ZoneInfo("America/Denver"))  # MDT is UTC-6


class SteamCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot
        self.game_of_the_day_cache = read_dict_from_disk(f"{STEAM_DATA_FOLDER}/game_of_the_day.json")
        
        if not self.send_daily_message.is_running():
            self.send_daily_message.start()

    @commands.command(name="game")
    async def game_of_the_day(self, ctx):
        """Fetches a random game from steam and prints it's url and name."""

        channel = ctx.channel
        await self.game()  # Call the game function to send the message

    @tasks.loop(time=target_time)
    async def send_daily_message(self):
        channel = self.bot.get_channel(CHANNEL_ID)
        if channel:
            await self.game()  # Call the game function to send the message

    @send_daily_message.before_loop
    async def before_daily_message(self):
        await self.bot.wait_until_ready()
   

    async def game(self):
        def fetch_data(url):
            try:
                response = requests.get(url)
                response.raise_for_status()  # Raise an error for bad responses
                return response.json()
            except requests.RequestException as e:
                print(f"Error fetching data from Steam API: {e}")
                return None

        channel = self.bot.get_channel(CHANNEL_ID)
        cached_game = self.game_of_the_day_cache.get(str(channel.guild.id))
        list_of_games = []
        last_appid = 0
        file_number = 0
        cached_time = datetime.strptime(cached_game["time"], datetime_format) if cached_game else None


        if cached_time and (datetime.now().day == cached_time.day):  # Check if cached game is from the same day
            game = cached_game["game"]
        else:
            while True:

                apps = await asyncio.to_thread(read_dict_from_disk, f"{STEAM_DATA_FOLDER}/{file_number}.json")  # Read the cache from disk in a background thread
                if apps == {}:
                    url = f"https://api.steampowered.com/IStoreService/GetAppList/v1/?key={STEAMAPI_KEY}&format=json&max_results=50000&last_appid={last_appid}"  # Replace {STEAM_KEY} with your actual Steam API key
                    response = await asyncio.to_thread(fetch_data, url)
                    apps = response["response"]
                    await asyncio.to_thread(write_dict_to_disk, apps, f"{STEAM_DATA_FOLDER}/{file_number}.json")  # Save the cache to disk in a background thread


                list_of_games.extend(apps["apps"])
                last_appid = apps.get("last_appid", None)
                if not apps.get("have_more_results", False):
                    break

                file_number += 1

            if len(list_of_games) > 0:
                game = random.choice(list_of_games)
            else:
                return await channel.send("❌ Could not fetch games from Steam API.")
                

            self.game_of_the_day_cache[str(channel.guild.id)] = {"game": game,
                                                "time": datetime.now().strftime(datetime_format)}  # Cache the game of the day for future requests

            await asyncio.to_thread(write_dict_to_disk, self.game_of_the_day_cache, f"{STEAM_DATA_FOLDER}/game_of_the_day.json")  # Save the cache to disk
            

        await channel.send(f"🎮 Game of the Day: **{game['name']}**\n🔗 URL: https://store.steampowered.com/app/{game['appid']}/")


async def setup(bot):
    await bot.add_cog(SteamCog(bot))
