use crate::{Error};
use poise::serenity_prelude as serenity;

pub async fn resolve_reply(
    http: &serenity::Http,
    msg: &serenity::Message,
) -> Result<Option<serenity::Message>, Error> {
    // Fast path: Discord gave it to us
    if let Some(replied) = &msg.referenced_message {
        return Ok(Some((**replied).clone()));
    }

    // Fallback: fetch by ID from the reference
    if let Some(reference) = &msg.message_reference {
        if let Some(message_id) = reference.message_id {
            let fetched = msg.channel_id.message(http, message_id).await?;
            return Ok(Some(fetched));
        }
    }

    Ok(None)
}