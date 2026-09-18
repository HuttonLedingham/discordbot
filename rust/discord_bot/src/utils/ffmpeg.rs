use tokio::process::Command;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use crate::{Error};

pub async fn convert(full_bytes: Vec<u8>,  file_ext: &str, target_duration: Option<f64>, input_filter_complex: Option<&str>, audio_bitrate: Option<u32>, cmd_flags: Option<&str>)
    -> Result<Option<Vec<u8>>, Error> {

                        
    async fn probe_buffer(video_bytes: Vec<u8>) -> Option<serde_json::Value> {
        let cmd: Vec<&str> = vec![
            "ffprobe",
            "-v", "error",
            "-select_streams", "v:0",
            "-show_format",
            "-count_packets",
            "-show_entries",
            "stream=nb_read_packets,duration,r_frame_rate,bit_rate,width,height,size",           
            // "packet=pts_time,duration_time",     
            "-of",
            "json",      
             
            "pipe:0"
        ];
        
        let mut process = Command::new("ffprobe")
            .args(&cmd[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn ffprobe process");

        let mut stdin = process.stdin.take().expect("Failed to open ffprobe stdin");

        let _writer = tokio::spawn(async move{
            stdin.write_all(&video_bytes).await.expect("Failed to write to ffprobe stdin");
        });

        let output = process.wait_with_output().await.expect("Failed to read ffprobe output");

        if !output.status.success() {
            eprintln!("Probe Error: {}", String::from_utf8_lossy(&output.stderr));
            return None;
        }
        let output_json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Error probing video: {}", e);
                return None;
            }
        };
        return Some(output_json);

    }


    if full_bytes.is_empty() {
        return Ok(None);
    }
    let Some(probe) = probe_buffer(full_bytes.clone()).await else {
        return Ok(None);
    };

    let mut filter_complex = input_filter_complex.unwrap_or_default().to_string();
    let audio_bitrate = audio_bitrate.unwrap_or(128);

    let original_format: serde_json::Value = probe.get("format").cloned().unwrap_or_default();
    let original_format_name = original_format.get("format_name").and_then(|v| v.as_str()).unwrap_or("");
    
    let stream = probe["streams"][0].clone();

    let original_duration = if let Some(duration) = original_format.get("duration").
    and_then(|v| v.as_str()).
    and_then(|v| v.parse::<f64>().ok())
     {
        Some(duration)
    } 
    else if let (Some(nb_read_packets), Some(r_frame_rate)) = (stream.get("nb_read_packets")
    .and_then(|v| v.as_str())
    .and_then(|v| v.parse::<f64>().ok()), stream.get("r_frame_rate").and_then(|v| v.as_str())) {
        let parts: Vec<&str> = r_frame_rate.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                if den != 0.0 {
                    Some(nb_read_packets as f64 / (num / den))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let original_duration = match original_duration {
        Some(duration) => duration,
        None => {
            eprintln!("Failed to determine original video duration. {:?}", probe);
            return Ok(None);
        }
    };
    


    // 2. Calculate speed factor if the video exceeds target duration
    
    if let Some(target_duration) = target_duration{ 
        if original_duration > target_duration{
            
            let speed_factor = target_duration / original_duration;
            filter_complex = add_filter_complex(&filter_complex, &format!("setpts={:.4}*PTS", speed_factor));
        }
    }

    let mut cmd: Vec<String> = vec!["-i".to_string(), "pipe:0".to_string()];

    if let Some(cmd_flags) = cmd_flags {
        cmd.extend(cmd_flags.split_whitespace().map(String::from));
    }

    if file_ext == "mp4" {
        let mut bitrate = 750;
        if original_duration != 0.0 {
            let calculated_rate = (((8000.0 * 20.0) / original_duration) - audio_bitrate as f64) as i32;
            let old_rate = stream.get("bit_rate").and_then(|v| v.as_i64()).unwrap_or(6000000) as i32; // 1000
            bitrate = std::cmp::min(calculated_rate, old_rate);
        }
        let maxrate = format!("{}k", bitrate);
        let bufsize = format!("{}k", bitrate * 2);
        let audio_bitrate = format!("{}k", audio_bitrate);
        cmd.extend([
                    "-f", "mp4",
                    "-c:v", "hevc_nvenc",
                    "-rc", "vbr",            // explicit VBR mode
                    "-cq", "23",
                    "-maxrate", &maxrate,
                    "-bufsize", &bufsize,
                    "-preset", "p4",
                    "-profile:v", "main",    // HEVC main profile for compatibility
                    "-c:a", "aac",
                    "-b:a", &audio_bitrate,
                    "-movflags", "frag_keyframe+empty_moov+default_base_moof",
                    ].map(String::from));

        if original_format_name.contains("gif") {
            cmd.extend(vec!["-pix_fmt".into(), "yuv420p".into()]);
            filter_complex = add_filter_complex(&filter_complex, &format!("scale=trunc(iw/2)*2:trunc(ih/2)*2"));
        }
    } else if file_ext == "webm" {
        cmd.extend([ "-c:v", "vp9",
                    "-b:v", "0", 
                    "-crf", "33",
                    "-loop", "0"].map(String::from));
    } else if file_ext =="gif" {
        cmd.extend(["-f", "gif"].map(String::from));
    }

    if !filter_complex.is_empty() {
        cmd.push("-filter_complex".into());
        cmd.push(filter_complex);
    }




    cmd.push("pipe:1".into());

    let time1 = std::time::Instant::now();
    let output =  run_command(full_bytes, cmd).await;
    let time2 = std::time::Instant::now();
    println!("{:?}", time2.duration_since(time1));
    Ok(output)
}

fn add_filter_complex(existing: &str, addition: &str) -> String {
    if !existing.is_empty() {
        format!("{},{}", existing, addition)
    } else {
        addition.to_string()
    }
}

async fn run_command(full_bytes: Vec<u8>, cmd: Vec<String>) -> Option<Vec<u8>> {
    let mut ffmpeg = Command::new("ffmpeg")
        .args(&cmd.iter().map(|s| s.as_str()).collect::<Vec<&str>>())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ffmpeg failed to start");

    let mut stdin = ffmpeg.stdin.take().expect("Failed to open stdin");

    let _writer = tokio::spawn(async move{
        stdin.write_all(&full_bytes).await.expect("Failed to write to ffmpeg stdin");
    });

    let output = ffmpeg.wait_with_output().await.expect("Failed to read ffmpeg output");

    if !output.status.success() {
        eprintln!("FFmpeg failed with status: {}", output.status);
        eprintln!("FFmpeg stderr: {}", String::from_utf8_lossy(&output.stderr));
        return None;
    }

    Some(output.stdout)

}

pub fn create_deepfry_filter() -> String {
    let saturation = rand::random_range(2..=3); 
    let contrast = rand::random_range(1.5..=2.0);
    let noise = rand::random_range(30..=60);
    let gamma_r = rand::random_range(1..=3);
    let gamma_g = rand::random_range(1..=3);
    let gamma_b = rand::random_range(1..=3);


    let eq_str = format!("eq=saturation={}:contrast={}:gamma_r={}:gamma_g={}:gamma_b={}", saturation, contrast, gamma_r, gamma_g, gamma_b);
    let noise_str = format!("noise=alls={}:allf=t", noise);
    let sharpness_str = "unsharp=5:5:1.25:5:5:1";

    let combine_str = format!("{},{},{}", eq_str, noise_str, sharpness_str);
    return combine_str;
}
    // process = await asyncio.create_subprocess_exec(
    //     *cmd,
    //     stdin=asyncio.subprocess.PIPE,
    //     stdout=asyncio.subprocess.PIPE,
    //     stderr=asyncio.subprocess.PIPE
    // )

    // try:
    //     stdout_data, stderr_data = process.communicate(input=full_bytes)

    //     if process.returncode != 0:
    //         print(f"FFmpeg stderr: {stderr_data.decode()}", flush=True)
    //         return None

    //     return BytesIO(stdout_data)
    // except Exception as e:
    //     print(f"FFmpeg error: {e}", flush=True)
    //     process.kill()
    //     return None


//     let ytdl = Command::new("yt-dlp")
//         .args(["-f", "bestaudio", "-o", "-", url])
//         .stdout(Stdio::piped())
//         .stderr(Stdio::null())
//         .spawn()
//         .expect("yt-dlp failed to start");

//     let ffmpeg = Command::new("ffmpeg")
//         .args([
//             "-i", "pipe:0",
//             "-f", "s16le",
//             "-ar", "48000",
//             "-ac", "2",
//             "pipe:1",
//         ])
//         .stdin(Stdio::from(ytdl.stdout.unwrap().try_into().unwrap()))
//         .stdout(Stdio::piped())
//         .stderr(Stdio::null())
//         .spawn()
//         .expect("ffmpeg failed to start");

//     // RawAdapter wraps a reader of 48kHz stereo i16 PCM
//     RawAdapter::new(ffmpeg.stdout.unwrap(), 48000, 2).into()