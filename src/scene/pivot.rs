use crate::scene::Node;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Pivot {}

impl Pivot {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<Pivot> for Node {
    fn from(value: Pivot) -> Node {
        Node::Pivot(value)
    }
}
