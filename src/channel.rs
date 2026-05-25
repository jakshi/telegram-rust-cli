use chrono::{DateTime, Local};
use tdlib_rs::{
    enums::{Chat, ChatList, Chats, MessageContent, Messages, User},
    functions,
    types::FormattedText,
};

fn format_with_caption(label: &str, caption: &FormattedText) -> String {
    if caption.text.is_empty() {
        format!("[{label}]")
    } else {
        format!("[{label}] {}", caption.text)
    }
}

async fn resolve_chat_id(client_id: i32, target: &str) -> Result<(i64, String), String> {
    if target.eq_ignore_ascii_case("saved") {
        let User::User(me) = functions::get_me(client_id)
            .await
            .map_err(|e| format!("Failed to get current user: {}", e.message))?;
        Ok((me.id, "Saved Messages".to_string()))
    } else if let Ok(id) = target.parse::<i64>() {
        let Chat::Chat(chat) = functions::get_chat(id, client_id)
            .await
            .map_err(|e| format!("Failed to find chat {id}: {}", e.message))?;
        Ok((chat.id, chat.title))
    } else {
        let Chat::Chat(chat) = functions::search_public_chat(target.to_string(), client_id)
            .await
            .map_err(|e| format!("Failed to find channel @{target}: {}", e.message))?;
        Ok((chat.id, format!("@{target}")))
    }
}

pub async fn list_chats(client_id: i32, limit: i32) -> Result<(), String> {
    let _ = functions::load_chats(Some(ChatList::Main), limit, client_id).await;

    let Chats::Chats(chats) = functions::get_chats(Some(ChatList::Main), limit, client_id)
        .await
        .map_err(|e| format!("Failed to list chats: {}", e.message))?;

    println!("{:<16} TITLE", "ID");
    println!("{}", "-".repeat(60));

    for chat_id in chats.chat_ids {
        let Chat::Chat(chat) = functions::get_chat(chat_id, client_id)
            .await
            .map_err(|e| format!("Failed to get chat {chat_id}: {}", e.message))?;
        println!("{:<16} {}", chat.id, chat.title);
    }

    Ok(())
}

async fn fetch_messages(
    client_id: i32,
    chat_id: i64,
    count: i32,
    from_message_id: i64,
) -> Result<(Vec<tdlib_rs::types::Message>, i64), String> {
    let mut messages = Vec::new();
    let mut cursor = from_message_id;

    while (messages.len() as i32) < count {
        let batch_limit = (count - messages.len() as i32).min(100);
        let Messages::Messages(batch) =
            functions::get_chat_history(chat_id, cursor, 0, batch_limit, false, client_id)
                .await
                .map_err(|e| format!("Failed to fetch messages: {}", e.message))?;

        if batch.messages.is_empty() {
            break;
        }

        for msg in batch.messages.into_iter().flatten() {
            cursor = msg.id;
            messages.push(msg);
        }
    }

    Ok((messages, cursor))
}

pub async fn read_channel(
    client_id: i32,
    target: &str,
    limit: i32,
    skip: i32,
) -> Result<(), String> {
    let _ = functions::load_chats(Some(ChatList::Main), 100, client_id).await;

    let (chat_id, _display_name) = resolve_chat_id(client_id, target).await?;

    let _ = functions::open_chat(chat_id, client_id).await;

    let from_message_id = if skip > 0 {
        let (_, cursor) = fetch_messages(client_id, chat_id, skip, 0).await?;
        cursor
    } else {
        0
    };

    let (all_messages, _) = fetch_messages(client_id, chat_id, limit, from_message_id).await?;

    for msg in all_messages {
        let time = DateTime::from_timestamp(msg.date as i64, 0)
            .map(|dt| {
                dt.with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M %Z")
                    .to_string()
            })
            .unwrap_or_else(|| "unknown date".to_string());

        let text = match msg.content {
            MessageContent::MessageText(t) => t.text.text,
            MessageContent::MessagePhoto(p) => format_with_caption("Photo", &p.caption),
            MessageContent::MessageVideo(v) => format_with_caption("Video", &v.caption),
            MessageContent::MessageDocument(d) => format_with_caption("Document", &d.caption),
            MessageContent::MessageAudio(a) => format_with_caption("Audio", &a.caption),
            MessageContent::MessageSticker(s) => format!("[Sticker: {}]", s.sticker.emoji),
            MessageContent::MessageAnimation(_) => "[GIF]".to_string(),
            MessageContent::MessageVoiceNote(_) => "[Voice message]".to_string(),
            MessageContent::MessageVideoNote(_) => "[Video message]".to_string(),
            _ => "[Unsupported message type]".to_string(),
        };

        println!("[{time}] {text}");
    }

    let _ = functions::close_chat(chat_id, client_id).await;

    Ok(())
}
