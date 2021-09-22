use ark_ed_on_bls12_377::Fq;
use ark_ff::{Field, SquareRootField, Zero};

use crate::constants;

pub trait SqrtRatioZeta: Sized {
    /// Computes the square root of a ratio of field elements, returning:
    ///
    /// - `(true, sqrt(u/v))` if `u` and `v` are both nonzero and `u/v` is square;
    /// - `(true, 0)` if `u` is zero;
    /// - `(false, 0)` if `v` is zero;
    /// - `(false, sqrt(zeta*u/v))` if `u` and `v` are both nonzero and `u/v` is nonsquare;
    ///
    fn sqrt_ratio_zeta(u: &Self, v: &Self) -> (bool, Self);
}

impl SqrtRatioZeta for Fq {
    fn sqrt_ratio_zeta(u: &Self, v: &Self) -> (bool, Self) {
        // TODO: optimized implementation
        if u.is_zero() {
            return (true, *u);
        }
        if v.is_zero() {
            return (false, *v);
        }

        let uv = v.inverse().expect("nonzero") * u;
        if let Some(sqrt_uv) = uv.sqrt() {
            return (true, sqrt_uv);
        } else {
            let sqrt_zeta_uv = (*constants::ZETA * uv)
                .sqrt()
                .expect("must be square if u/v nonsquare");
            return (false, sqrt_zeta_uv);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn fq_strategy() -> BoxedStrategy<Fq> {
        use ark_serialize::CanonicalDeserialize;
        any::<[u8; 32]>()
            .prop_filter_map("non-canonical bytes", |bytes| {
                Fq::deserialize(&bytes[..]).ok()
            })
            .boxed()
    }

    proptest! {
        #[test]
        fn sqrt_ratio_zeta(u in fq_strategy(), v in fq_strategy()) {
            if u.is_zero() {
                assert_eq!(Fq::sqrt_ratio_zeta(&u, &v), (true, u));
            } else if v.is_zero() {
                assert_eq!(Fq::sqrt_ratio_zeta(&u, &v), (false, v));
            } else {
                let (was_square, sqrt_zeta_uv) = Fq::sqrt_ratio_zeta(&u, &v);
                let zeta_uv = sqrt_zeta_uv * sqrt_zeta_uv;
                if was_square {
                    // check zeta_uv = u/v
                    assert_eq!(u, v * zeta_uv);
                } else {
                    // check zeta_uv = zeta * u / v
                    assert_eq!(*constants::ZETA * u, v * zeta_uv);
                }
            }
        }
    }
}
