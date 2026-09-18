import asyncio
from collections import deque
import yt_dlp

YTDL_OPTIONS = {
    'format': 'bestaudio/best',
    'noplaylist': False,
    'quiet': True,
    'extract_flat': True,
    'skip_download': True
}


async def process_search(search: str):
    """Loads a playlist into the queue."""
    """Searches for a song or playlist and returns the info."""
    info = None
    def fetch_info(search, options=YTDL_OPTIONS):
        try:
            with yt_dlp.YoutubeDL(options) as ytdl:
                # Supports direct links or search keywords
                if search.startswith("http"):
                    return ytdl.extract_info(search, download=False)
                else:
                    return ytdl.extract_info(f"ytsearch:{search}", download=False)['entries'][0]
        except Exception as e:
            return None

    try:
        # Offload the blocking yt_dlp call to a separate background thread
        info = await asyncio.to_thread(fetch_info, search)
    except Exception as e:
        print(f"Error fetching info: {e}")
        return None

    url = None
    queue = deque()

    if isinstance(info, dict) and 'entries' in info:
        for entry in info['entries']:
            url = entry['url']
            print(f"Adding to queue: {entry['title']} ({url})")
            song_info = {'url': url, 'title': entry['title']}
            queue.append(song_info)
    else:
        song_info = {'url': info['url'], 'title': info['title']}
        queue.append(song_info)

    return queue, info['title'] if 'title' in info else None