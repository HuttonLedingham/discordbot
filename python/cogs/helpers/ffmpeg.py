import asyncio
from io import BytesIO
import json
from .utils import make_random_value
import time


async def apply_ffmpeg(full_bytes: bytes,  file_ext: str, target_duration: float =  None, 
                        filter_complex: str = None, audio_bitrate = 128, cmd_flags = []): 
    async def probe_buffer(video_bytes: bytes):
        cmd = [
            "ffprobe",
            "-v", "error",
            "-select_streams", "v:0",
            "-show_format",
            "-count_packets",
            "-show_entries",
            "stream=nb_read_packets,duration,r_frame_rate,bit_rate,width,height,size",              # Target video stream 0
            #"packet=pts_time,duration_time",     
            "-of",
            "json",        # 
             
            "pipe:0"
        ] 
        
        process = await asyncio.create_subprocess_exec(
            *cmd,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        try:
            stdout_data, stderr_data = await process.communicate(input=video_bytes)

            if process.returncode != 0:
                print(f"Probe Error: {stderr_data.decode()}")
                return None

            output = json.loads(stdout_data.decode())
            return output
        except Exception as e:
            print(f"Error probing video: {e}")
            process.kill()
            return None

    if full_bytes is None:
        return None

    probe = await probe_buffer(full_bytes)

    if probe is None:
        print(f"Failed to probe video. {probe}")
        return None

    original_format = probe.get('format', {})
    original_format_name = original_format.get('format_name', "")

    stream = probe["streams"][0]
    print(probe)
    original_duration = None

    if 'duration' in stream:
        original_duration = float(stream['duration'])
    elif 'nb_read_packets' in stream and 'r_frame_rate' in stream:
        original_duration = int(stream['nb_read_packets']) / eval(stream['r_frame_rate'])

    if original_duration is None:
        print(f"Failed to determine original video duration. {probe}")
        return None



    # 2. Calculate speed factor if the video exceeds target duration
    if target_duration and original_duration > target_duration:
        speed_factor = target_duration / original_duration
        filter_complex = add_filter_complex(filter_complex, f"setpts={speed_factor:.4f}*PTS")



    cmd = ["ffmpeg", "-i", "pipe:0"]

    if cmd_flags:
        cmd.extend(cmd_flags)

    if file_ext == "mp4":
        bitrate = 750
        if original_duration != 0.0:
            calculated_rate = int(((8000 * 20) / original_duration) - audio_bitrate)
            old_rate = int(stream.get("bit_rate", 6000000)) // 1000
            bitrate = min(calculated_rate, old_rate)
        
        cmd.extend([
                    "-f", "mp4",
                    "-c:v", "hevc_nvenc",
                    "-rc", "vbr",            # explicit VBR mode
                    "-cq", "23",
                    "-maxrate", f"{bitrate}k",
                    "-bufsize", f"{bitrate * 2}k",
                    "-preset", "p4",
                    "-profile:v", "main",    # HEVC main profile for compatibility
                    "-c:a", "aac",
                    "-b:a", f"{audio_bitrate}k",
                    "-movflags", "frag_keyframe+empty_moov+default_base_moof",
                    ])

        if "gif" in original_format_name:
            cmd.extend(["-pix_fmt", "yuv420p"])
            filter_complex = add_filter_complex(filter_complex, f"scale=trunc(iw/2)*2:trunc(ih/2)*2")

    elif file_ext == "webm":
        cmd.extend([ "-c:v", "vp9",
                    "-b:v", "0", 
                    "-crf", "33",
                    "-loop", "0"])
    elif file_ext =="gif":
        cmd.extend(["-f", "gif"])

    if filter_complex:
        cmd.extend(["-filter_complex", filter_complex])




    cmd.append("pipe:1")

    time1 = time.perf_counter()
    output =  await run_ffmpeg(full_bytes, cmd)
    time2 = time.perf_counter()
    print(time2 - time1)
    return output


async def convert_video_to_audio(full_bytes: bytes) -> BytesIO | None:
    cmd = [
        "ffmpeg",
        "-i", "pipe:0",
        "-f", "mp3",
        "pipe:1"
    ]

    return await run_ffmpeg(full_bytes, cmd)

async def generate_thumbnail(full_bytes: bytes) -> BytesIO | None:
    cmd = [
        "ffmpeg",
        "-i", "pipe:0",
        "-vf", "thumbnail=100",
        "-frames:v", "1",
        "-f", "image2",
        "pipe:1"
    ]

    return await run_ffmpeg(full_bytes, cmd)

async def run_ffmpeg(full_bytes: bytes, cmd: list) -> BytesIO | None:
    process = await asyncio.create_subprocess_exec(
        *cmd,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE
    )

    try:
        stdout_data, stderr_data = await process.communicate(input=full_bytes)

        if process.returncode != 0:
            print(f"FFmpeg stderr: {stderr_data.decode()}", flush=True)
            return None

        return BytesIO(stdout_data)
    except Exception as e:
        print(f"FFmpeg error: {e}", flush=True)
        process.kill()
        return None

def create_filter_args():
    """
    Create randomized "deep fried" visual filters
    returns command line args for ffmpeg's filter flag -vf
    """
    saturation = make_random_value([2, 3])
    contrast = make_random_value([1.5, 2])
    noise = make_random_value([30, 60])
    gamma_r = make_random_value([1, 3])
    gamma_g = make_random_value([1, 3])
    gamma_b = make_random_value([1, 3])

    eq_str = f'eq=saturation={saturation}:contrast={contrast}'
    eq_str += f':gamma_r={gamma_r}:gamma_g={gamma_g}:gamma_b={gamma_b}'eq
    noise_str = f'noise=alls={noise}:allf=t'
    sharpness_str = 'unsharp=5:5:1.25:5:5:1'

    combine_str = ','.join([eq_str, noise_str, sharpness_str])
    return combine_str

def add_filter_complex(existing: str, new_filter: str) -> str:
    if existing:
        return f"{existing},{new_filter}"
    return new_filter


def create_eq(saturation: float = 5.0, contrast: float = 2.5, brightness: float = 0.1) -> str:
        return f"eq=saturation={saturation}:contrast={contrast}:brightness={brightness}"

def create_sharpness(luma_msize_x: int = 5, luma_msize_y: int = 5, luma_amount: float = 5.0) -> str:
    return f"unsharp={luma_msize_x}:{luma_msize_y}:{luma_amount}"