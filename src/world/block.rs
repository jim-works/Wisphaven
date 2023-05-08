#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockType {
    #[default]
    Empty,
    Basic(u32)
}