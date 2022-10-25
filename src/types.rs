// Generic bot error
pub type BotError = Box<dyn std::error::Error + Send + Sync + 'static>;
