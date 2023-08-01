use crate::seq;

pub type Barcode = seq::Seq;
pub type Molecular = seq::Seq;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Identifier {
    pub bc: Barcode,
    pub umi: Molecular,
}
