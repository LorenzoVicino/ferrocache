#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(i64),
    Bulk(Vec<u8>),
    Null,
    Array(Vec<Frame>),
}
