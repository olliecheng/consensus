use crate::seq::seq;

pub type Barcode = seq::Seq;
pub type Molecular = String;

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Identifier {
    pub bc: Barcode,
    pub umi: Molecular,
}
