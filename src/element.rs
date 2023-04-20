use ark_ec::{AffineRepr, ProjectiveCurve};
use ark_ed_on_bls12_377::{EdwardsAffine, EdwardsProjective};

use crate::{Fq, Fr};

pub mod affine;
pub mod projective;

pub use affine::AffineElement;
pub use projective::Element;

impl ProjectiveCurve for Element {
    // We implement `ProjectiveCurve` as it is required by the `CurveVar`
    // trait used in the R1CS feature. The `ProjectiveCurve` trait requires
    // an affine representation of `Element` to be defined, and `AffineRepr`
    // to be implemented on that type.
    const COFACTOR: &'static [u64] = &[1];

    type ScalarField = Fr;

    type BaseField = Fq;

    type Affine = AffineElement;

    fn prime_subgroup_generator() -> Self {
        crate::basepoint()
    }

    fn batch_normalization(v: &mut [Self]) {
        let mut v_inner = v.iter_mut().map(|g| g.inner).collect::<Vec<_>>();
        EdwardsProjective::batch_normalization(&mut v_inner[..]);
    }

    fn is_normalized(&self) -> bool {
        self.inner.is_normalized()
    }

    fn double_in_place(&mut self) -> &mut Self {
        let inner = *self.inner.double_in_place();
        *self = Element { inner };
        self
    }

    fn add_assign_mixed(&mut self, other: &Self::Affine) {
        let proj_other: Element = other.into();
        *self += proj_other;
    }
}

impl AffineRepr for AffineElement {
    const COFACTOR: &'static [u64] = &[1];

    type ScalarField = Fr;

    type BaseField = Fq;

    type Projective = Element;

    fn prime_subgroup_generator() -> Self {
        crate::basepoint().into()
    }

    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        EdwardsAffine::from_random_bytes(bytes).map(|inner| AffineElement { inner })
    }

    fn mul<S: Into<<Self::ScalarField as ark_ff::PrimeField>::BigInt>>(
        &self,
        other: S,
    ) -> Self::Projective {
        Element {
            inner: self.inner.mul(other),
        }
    }

    fn mul_by_cofactor_to_projective(&self) -> Self::Projective {
        Element {
            inner: self.inner.mul_by_cofactor_to_projective(),
        }
    }

    fn mul_by_cofactor_inv(&self) -> Self {
        AffineElement {
            inner: self.inner.mul_by_cofactor_inv(),
        }
    }
}

impl From<Element> for AffineElement {
    fn from(point: Element) -> Self {
        Self {
            inner: point.inner.into(),
        }
    }
}

impl From<AffineElement> for Element {
    fn from(point: AffineElement) -> Self {
        Self {
            inner: point.inner.into(),
        }
    }
}

impl From<&Element> for AffineElement {
    fn from(point: &Element) -> Self {
        Self {
            inner: point.inner.into(),
        }
    }
}

impl From<&AffineElement> for Element {
    fn from(point: &AffineElement) -> Self {
        Self {
            inner: point.inner.into(),
        }
    }
}
