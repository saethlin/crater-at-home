#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Crate {
    pub name: String,
    pub recent_downloads: Option<u64>,
    pub version: String,
    pub status: Status,
    #[serde(default)]
    /// Time that the run took, in seconds
    pub time: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Unknown,
    Passing,
    Error(String),
    UB { cause: String, status: String },
}
