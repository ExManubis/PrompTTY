/// Where a conversation restore loads from.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ConversationRestoreTarget {
    Local,
    Server,
}
