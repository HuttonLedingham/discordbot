import asyncio

DEFAULT_YT_VIDEO_OPTIONS = [
    'yt-dlp',
    '-o', '-',  
    '--format', 'bestvideo[height<=360]+bestaudio/best',           # Pre-merged video/audio stream required for stdout/RAM
    '--quiet',
    '--no-warnings',
    '--socket-timeout', '15',
    '--retries', '10',
    '--fragment-retries', '10',
    '--skip-unavailable-fragments',
    '--no-playlist',
    '--merge-output-format', 'mkv'

]


# -- HELPER FUNCTIONS -- 


async def download_video(url, yt_dlp=DEFAULT_YT_VIDEO_OPTIONS):
    cmd = yt_dlp + [url]
    print(cmd)
    try:
        p1 = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            text=False,
        )

        output, error = await p1.communicate()
        if p1.returncode != 0:
            print(f"yt-dlp error: {error.decode()}")
            return None
        return output
        
    except Exception as e:
        print(f"Error running yt-dlp: {e}")
        return None
