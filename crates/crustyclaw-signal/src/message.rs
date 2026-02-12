//! Signal message types — models for messages, attachments, and groups.

use std::time::SystemTime;

/// A message received from or sent via Signal.
#[derive(Debug, Clone)]
pub struct SignalMessage {
    /// The sender's phone number or UUID.
    pub sender: String,

    /// The recipient's phone number or UUID (for outbound).
    pub recipient: Option<String>,

    /// Message text body (may be empty if attachment-only).
    pub body: String,

    /// Timestamp of the message.
    pub timestamp: SystemTime,

    /// Optional group information (if this is a group message).
    pub group: Option<GroupInfo>,

    /// Attached media files.
    pub attachments: Vec<Attachment>,
}

impl SignalMessage {
    /// Create a simple text message.
    pub fn text(sender: &str, body: &str) -> Self {
        Self {
            sender: sender.to_string(),
            recipient: None,
            body: body.to_string(),
            timestamp: SystemTime::now(),
            group: None,
            attachments: Vec::new(),
        }
    }

    /// Create an outbound text message to a specific recipient.
    pub fn outbound(recipient: &str, body: &str) -> Self {
        Self {
            sender: String::new(), // filled in by adapter
            recipient: Some(recipient.to_string()),
            body: body.to_string(),
            timestamp: SystemTime::now(),
            group: None,
            attachments: Vec::new(),
        }
    }

    /// Whether this message belongs to a group conversation.
    pub fn is_group(&self) -> bool {
        self.group.is_some()
    }

    /// Whether this message has attachments.
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }
}

/// Information about a Signal group.
#[derive(Debug, Clone)]
pub struct GroupInfo {
    /// Group identifier.
    pub id: String,

    /// Human-readable group name.
    pub name: String,

    /// List of member phone numbers / UUIDs.
    pub members: Vec<String>,
}

impl GroupInfo {
    /// Create a new group.
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            members: Vec::new(),
        }
    }

    /// Add a member to the group.
    pub fn with_member(mut self, member: &str) -> Self {
        self.members.push(member.to_string());
        self
    }
}

/// A file attachment on a Signal message.
#[derive(Debug, Clone)]
pub struct Attachment {
    /// MIME content type (e.g. "image/jpeg", "audio/ogg").
    pub content_type: String,

    /// Original filename, if available.
    pub filename: Option<String>,

    /// Size in bytes.
    pub size: u64,

    /// Local file path where the attachment is stored.
    pub local_path: Option<String>,
}

impl Attachment {
    /// Create a new attachment.
    pub fn new(content_type: &str, size: u64) -> Self {
        Self {
            content_type: content_type.to_string(),
            filename: None,
            size,
            local_path: None,
        }
    }

    /// Whether this is an image attachment.
    pub fn is_image(&self) -> bool {
        self.content_type.starts_with("image/")
    }

    /// Whether this is an audio attachment (e.g. voice note).
    pub fn is_audio(&self) -> bool {
        self.content_type.starts_with("audio/")
    }

    /// Whether this is a video attachment.
    pub fn is_video(&self) -> bool {
        self.content_type.starts_with("video/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_message() {
        let msg = SignalMessage::text("+1234567890", "Hello");
        assert_eq!(msg.sender, "+1234567890");
        assert_eq!(msg.body, "Hello");
        assert!(!msg.is_group());
        assert!(!msg.has_attachments());
    }

    #[test]
    fn test_outbound_message() {
        let msg = SignalMessage::outbound("+0987654321", "Reply");
        assert_eq!(msg.recipient.as_deref(), Some("+0987654321"));
        assert_eq!(msg.body, "Reply");
    }

    #[test]
    fn test_group_info() {
        let group = GroupInfo::new("g1", "Test Group")
            .with_member("+111")
            .with_member("+222");
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.members.len(), 2);
    }

    #[test]
    fn test_attachment_types() {
        let img = Attachment::new("image/jpeg", 1024);
        assert!(img.is_image());
        assert!(!img.is_audio());
        assert!(!img.is_video());

        let audio = Attachment::new("audio/ogg", 2048);
        assert!(audio.is_audio());
        assert!(!audio.is_image());

        let video = Attachment::new("video/mp4", 4096);
        assert!(video.is_video());
    }

    #[test]
    fn test_message_with_attachments() {
        let mut msg = SignalMessage::text("+1", "photo");
        msg.attachments.push(Attachment::new("image/png", 5000));
        assert!(msg.has_attachments());
    }

    #[test]
    fn test_group_message() {
        let mut msg = SignalMessage::text("+1", "hello group");
        msg.group = Some(GroupInfo::new("g1", "Friends"));
        assert!(msg.is_group());
    }
}
