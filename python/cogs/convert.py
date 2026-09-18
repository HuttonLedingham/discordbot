import discord
from discord.ext import commands
import contextlib
from io import BytesIO
import asyncio
import json
import sys

from .helpers.ffmpeg import apply_ffmpeg, convert_video_to_audio, create_eq, create_sharpness, create_filter_args, generate_thumbnail
from .helpers.yt_utils import download_video, DEFAULT_YT_VIDEO_OPTIONS

class ConvertFlags(commands.FlagConverter, prefix='--', delimiter=" "):
    url: str = None
    intensity: int = 19000
    length: int = None
    audio_bitrate: int = 128


class ConvertCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.command(name="gif")
    async def gif(self, ctx, *, args: ConvertFlags):
        """Converts a twitter video to a GIF and sends it to the channel."""
        await convert_video(ctx, file_ext="gif", url=args.url, length=args.length,
        filter_complex= "[0:v] fps=15,scale=360:-1:flags=lanczos,split [a][b];[a] palettegen [p];[b][p] paletteuse")

    @commands.command(name="deepfry")
    async def deepfry(self, ctx, *, args: ConvertFlags):
        """Converts a twitter video to a deep-fried GIF and ConvertFlags it to the channel."""
        await convert_video(ctx, file_ext="gif", url=args.url, length=args.length,
        filter_complex=f"[0:v] fps=15,{create_filter_args()},scale=iw/4:ih/4,scale=iw*4:ih*4,scale=360:-1:flags=neighbor")

    @commands.command(name="interpolate")
    async def interpolate(self, ctx, *, args: ConvertFlags):
        """Converts a twitter video to an interpolated GIF and sends it to the channel."""
        await convert_video(ctx, file_ext="mp4", url=args.url, 
        filter_complex=f"[0:v] fps=60,minterpolate='mi_mode=mci:mc_mode=aobmc:vsbmc=1', scale=720:-1:flags=neighbor")

    @commands.command(name="video")
    async def video(self, ctx, *, args: ConvertFlags):
        """Downloads a twitter video and sends it to the channel."""
        options = DEFAULT_YT_VIDEO_OPTIONS.copy()
        options[options.index('--format') + 1] = "bestvideo[height<=721]+bestaudio/b"

        await convert_video(ctx, "mp4", args.url, options=options)


    @commands.command(name="corrupt")
    async def corrupt(self, ctx, *, args: ConvertFlags):
        """Downloads a twitter video and sends it to the channel."""
        options = DEFAULT_YT_VIDEO_OPTIONS.copy()
        options[options.index('--format') + 1] = "bestvideo[height<=720]+bestaudio/best"

        status_message = await ctx.send("🔄 Downloading... Please wait.")

        full_bytes = await get_video_bytes(ctx, args.url, options)
        if not full_bytes:
            await status_message.edit(content="❌ Failed to retrieve video.")
            return

        await status_message.edit(content="🔄 Converting... Please wait.")

        first_pass = await apply_ffmpeg(full_bytes, file_ext="mp4", 
                                        cmd_flags = ["-bsf:v", f"noise={args.intensity}",
                                                    
                                                     ])

        if first_pass is None:
            await status_message.edit(content="First Pass Failed")

        output = await apply_ffmpeg(first_pass.getvalue(), file_ext="mp4", audio_bitrate = args.audio_bitrate)

        await send_file(ctx, output, "mp4", status_message)

    @commands.command(name="audio")
    async def audio(self, ctx, *, args: ConvertFlags):
        """Converts a twitter video to an audio file and sends it to the channel."""

        status_message = await ctx.send("🔄 Converting... Please wait.")

        full_bytes = await get_video_bytes(ctx, args.url)
        if not full_bytes:
            await status_message.edit(content="❌ Failed to retrieve video.")
            return


        output = await convert_video_to_audio(full_bytes)
        thumbnail = await generate_thumbnail(full_bytes)
        if thumbnail:
            await send_file(ctx, thumbnail, "jpg", status_message)
        await send_file(ctx, output, "mp3", status_message)




# -- HELPER FUNCTIONS -- 

async def convert_video(ctx, file_ext: str, url: str = None, filter_complex: str = None, length: float = None, options: str = DEFAULT_YT_VIDEO_OPTIONS, cmd_flags = None, audio_bitrate: int = 128):
    """Converts a twitter video to a GIF and sends it to the channel."""
    status_message = await ctx.send("🔄 Downloading... Please wait.")

    full_bytes = await get_video_bytes(ctx, url, options)
    if not full_bytes:
        await status_message.edit(content="❌ Failed to retrieve video.")
        return

    await status_message.edit(content="🔄 Converting... Please wait.")

    output = await apply_ffmpeg(full_bytes, file_ext=file_ext, filter_complex=filter_complex, 
                                target_duration=length, cmd_flags=cmd_flags, audio_bitrate=audio_bitrate)

    if not output:
        await status_message.edit(content="❌ File conversion failed")
        return

    await send_file(ctx, output, file_ext, status_message)

async def send_file(ctx, file_bytes:bytes, file_ext, status_message):

    unique_id = ctx.message.id  # Use the message ID to create a unique identifier for this request

    try:
        await ctx.send(file=discord.File(file_bytes, filename=f"{unique_id}.{file_ext}"))
        await status_message.edit(content="✅ Upload complete!")  # Send the status message after processing
    except Exception as e:
        await ctx.send(f"❌ Failed to send the file: {e}")
        await status_message.delete()
    finally:
        if file_bytes:
            file_bytes.close()  # Clean up the file

async def get_video_bytes(ctx, url: str = None, yt_options=DEFAULT_YT_VIDEO_OPTIONS):
    full_bytes = None
    if ctx.message.attachments:
        attachment = ctx.message.attachments[0]
        full_bytes = await attachment.read()
    elif url:
        full_bytes = await download_video(url, yt_options)
    elif ctx.message.reference and ctx.message.reference.message_id:
        referenced_message = await ctx.channel.fetch_message(ctx.message.reference.message_id)
        if referenced_message.attachments:
            attachment = referenced_message.attachments[0]
            full_bytes = await attachment.read()
        else:
            potential_url = referenced_message.content.split(" ")[-1]
            full_bytes = await download_video(potential_url, yt_options)

    else:
        potential_url = ctx.message.content.split(" ")[-1]
        full_bytes = await download_video(potential_url, yt_options)
    return full_bytes


async def setup(bot):
    await bot.add_cog(ConvertCog(bot))

    