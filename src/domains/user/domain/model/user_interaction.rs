/// Status of a user's interaction with a specific record.
#[derive(Debug, Clone, Default)]
pub struct InteractionStatus {
    pub liked: bool,
    pub viewed: bool,
}
