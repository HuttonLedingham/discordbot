import asyncio
import os
import discord
from discord.ext import commands
import dotenv

dotenv.load_dotenv()

intents = discord.Intents.default()
intents.message_content = True
intents.voice_states = True

bot = commands.Bot(command_prefix="!", intents=intents)

@bot.event
async def on_ready():
    print(f"Logged in as {bot.user}")

async def setup_hook():
    # Load all .py files in the cogs directory
    for filename in os.listdir("./cogs"):
        if filename.endswith(".py"):
            await bot.load_extension(f"cogs.{filename[:-3]}")

# @bot.before_invoke
# async def before_invoke(ctx):   
#     if ctx.voice_client and (ctx.author.voice is None or ctx.author.voice.channel != ctx.voice_client.channel):
#         await ctx.send("❌ You must be in the same voice channel as the bot to use this command.")
#         return


@bot.after_invoke
async def after_invoke(ctx):
    guild_id = ctx.guild.id

    if ctx.message:
        await ctx.message.delete()  # Delete the command message to keep the chat clean

@bot.event
async def on_voice_state_update(member, before, after):

    if member.id == bot.user.id:
        # If 'before.channel' exists but 'after.channel' is None, the bot disconnected
        if before.channel is not None and after.channel is None:
            for cog in bot.cogs.values():
                if hasattr(cog, "vc_disconnect"):
                    await cog.vc_disconnect(member.guild.id)



    # Find the voice client for the specific server
    voice_client = member.guild.voice_client
    if not voice_client:
        return

    # Check if the bot is now the only one left in its voice channel
    bot_vc = voice_client.channel
    if len(bot_vc.members) == 1 and bot_vc.members[0] == bot.user:
        
        # If a timeout task is already running for this guild, do nothing
        if member.guild.id in vc_timeout_tasks:
            return

        # Start a background task to wait and disconnect
        async def timeout_timer(guild_id, vc):
            # Wait for 120 seconds (2 minutes)
            await asyncio.sleep(120) 
            
            # Double check if the bot is still alone before disconnecting
            guild = bot.get_guild(guild_id)
            current_vc = guild.voice_client if guild else None
            if current_vc and len(current_vc.channel.members) == 1:
                await current_vc.disconnect()

                print(f"Disconnected from {current_vc.channel.name} due to inactivity.")
            
            # Clean up the task from memory
            vc_timeout_tasks.pop(guild_id, None)

        # Create and track the asyncio task
        task = asyncio.create_task(timeout_timer(member.guild.id, voice_client))
        vc_timeout_tasks[member.guild.id] = task

    # If someone joins the channel, cancel the pending disconnect timeout
    elif len(bot_vc.members) > 1 and bot.user in bot_vc.members:
        if member.guild.id in vc_timeout_tasks:
            vc_timeout_tasks[member.guild.id].cancel()
            vc_timeout_tasks.pop(member.guild.id, None)
            print("Timeout canceled because a user joined the channel.")



bot.setup_hook = setup_hook

async def main():
    async with bot:
        await bot.start(os.getenv("DISCORD_TOKEN"))

if __name__ == "__main__":
    asyncio.run(main())