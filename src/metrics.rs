use triple_accel::*;

#[derive(Debug)]
pub enum Distance {
    TooFar,
    Dist(u32),
}

pub trait Metric {
    fn distance_to(&self, other: &Self) -> Distance;
}

impl Metric for crate::record::RecordIdentifier {
    fn distance_to(&self, other: &Self) -> Distance {
        if self.bc != other.bc {
            Distance::TooFar
        } else {
            let umi1 = self.umi.as_bytes();
            let umi2 = other.umi.as_bytes();
            Distance::Dist(
                triple_accel::levenshtein(umi1, umi2)
            )
        }
    }
}