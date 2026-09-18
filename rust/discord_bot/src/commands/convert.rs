use crate::utils::ffmpeg;
use poise::serenity_prelude as serenity;
use poise::CreateReply;
use crate::{Context, Error};
use crate::utils::search::{yt_download};
use crate::utils::discord::{resolve_reply};


#[poise::command(prefix_command, guild_only)]
pub async fn video(
    ctx: Context<'_>,
    #[description = "The file to convert"]
    file: Option<serenity::Attachment>,
    #[description = "The URL of the video to convert"]
    url: Option<String>,
) -> Result<(), Error> {

    let status = ctx.say("🔄 Processing...").await?;
    let msg = get_message(ctx)?;

    let bytes = get_video_bytes(&msg, url.as_deref(), file, &ctx.http()).await?;
    let output = ffmpeg::convert(bytes, "mp4", None, None, None, None).await?;

    let _ = send_output(ctx, output, "mp4").await;
    status.delete(ctx).await?;

    Ok(())
}

#[poise::command(prefix_command, guild_only)]
pub async fn gif(
    ctx: Context<'_>,
    #[description = "The file to convert"]
    file: Option<serenity::Attachment>,
    #[description = "The URL of the video to convert"]
    url: Option<String>,
) -> Result<(), Error> {

    let status = ctx.say("🔄 Processing...").await?;

    let msg = get_message(ctx)?;
    let bytes = get_video_bytes(&msg, url.as_deref(), file, &ctx.http()).await?;

    let output = ffmpeg::convert(bytes, "gif", Some(5.0), Some("[0:v] fps=15,scale=360:-1:flags=lanczos,split [a][b];[a] palettegen [p];[b][p] paletteuse"),
                                 None, None).await?;

    let _ = send_output(ctx, output, "gif").await;
    status.delete(ctx).await?;
    Ok(())
}

#[poise::command(prefix_command, guild_only)]
pub async fn deepfry(
    ctx: Context<'_>,
    #[description = "The file to convert"]
    file: Option<serenity::Attachment>,
    #[description = "The URL of the video to convert"]
    url: Option<String>,
) -> Result<(), Error> {

    let status = ctx.say("🔄 Processing...").await?;


    let msg = get_message(ctx)?;
    let filter = ffmpeg::create_deepfry_filter();
    let bytes = get_video_bytes(&msg, url.as_deref(), file, &ctx.http()).await?;
    let output = ffmpeg::convert(bytes, "gif", Some(5.0),
                                Some(format!("[0:v] fps=15,{},scale=iw/4:ih/4,scale=iw*4:ih*4,scale=360:-1:flags=neighbor", filter).as_str()),
                                 None, None).await?;

    let _ = send_output(ctx, output, "gif").await;
    status.delete(ctx).await?;
    Ok(())
}

#[poise::command(prefix_command, guild_only)]
pub async fn corrupt(
    ctx: Context<'_>,
    #[description = "The file to convert"]
    file: Option<serenity::Attachment>,
    #[description = "The URL of the video to convert"]
    url: Option<String>,
) -> Result<(), Error> {
    let status = ctx.say("🔄 Processing...").await?;
    let msg = get_message(ctx)?;
    let bytes = get_video_bytes(&msg, url.as_deref(), file, &ctx.http()).await?;

    if bytes.is_empty() {
        ctx.send(CreateReply::default().content("No video source provided.")).await?;
        return Ok(());
    }

    let first_pass = ffmpeg::convert(bytes, "mp4", None, None, Some(32), Some(&"-bsf:v noise=19000")).await?;

    if let Some(first_pass) = first_pass {
        let final_pass = ffmpeg::convert(first_pass, "mp4", None,None, None, None).await?;
        let _ = send_output(ctx, final_pass, "mp4").await;
    }
    else{
        let _ = ctx.send(CreateReply::default().content("Failed to convert the file on the first pass.")).await?;
    }
    status.delete(ctx).await?;
    Ok(())
}

async fn get_video_bytes(msg: &serenity::Message, url: Option<&str>, file: Option<serenity::Attachment>, http: &serenity::Http) -> Result<Vec<u8>, Error> {
    if let Some(file) = file {
        return Ok(file.download().await?);
    }

    let replied = resolve_reply(http, &msg).await?;
    if let Some(replied) = replied {
        if let Some(attachment) = replied.attachments.first().cloned() {
            return Ok(attachment.download().await?);
        }
        else {
            return yt_download(&replied.content).await;
        }
    }

    if let Some(url) = url {
        return yt_download(url).await;
    }

    Err("No video source provided".into())
}

fn get_message(ctx: Context<'_>) -> Result<&serenity::Message, Error> {
    match ctx {
        poise::Context::Prefix(p) => Ok(p.msg),
        poise::Context::Application(_) => Err("No message found".into()),
    }
}

async fn send_output(ctx: Context<'_>, output: Option<Vec<u8>>, file_extension: &str) -> Result<(), Error> {
    if let Some(output) = output {
        let attachment = serenity::CreateAttachment::bytes(output, &format!("output.{}", file_extension));
        let _ = ctx.send(CreateReply::default().attachment(attachment).content("✅ Converstion Done!")).await?;
    }
    else{
        let _ = ctx.say("Failed to convert the file.").await?;
    }
    Ok(())
}