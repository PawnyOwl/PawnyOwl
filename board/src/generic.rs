use crate::core::Color as Col;

pub trait Color {
    const COLOR: Col;
}

pub struct White;
pub struct Black;

impl Color for White {
    const COLOR: Col = Col::White;
}

impl Color for Black {
    const COLOR: Col = Col::Black;
}
