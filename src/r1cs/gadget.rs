#![allow(non_snake_case)]
use std::borrow::Borrow;

use ark_ec::{AffineCurve, TEModelParameters};
use ark_ed_on_bls12_377::{
    constraints::{EdwardsVar, FqVar},
    EdwardsAffine, EdwardsParameters,
};
use ark_ff::Field;
use ark_r1cs_std::{
    alloc::AllocVar, eq::EqGadget, groups::curves::twisted_edwards::AffineVar, prelude::*, R1CSVar,
};
use ark_relations::ns;
use ark_relations::r1cs::{ConstraintSystemRef, SynthesisError, ToConstraintField};
use ark_std::One;

use crate::{r1cs::fqvar_ext::FqVarExtension, AffineElement, Element, Fq, Fr};

#[derive(Clone, Debug)]
/// Represents the R1CS equivalent of a `decaf377::Element`
pub struct Decaf377ElementVar {
    /// Inner type is an alias for `AffineVar<EdwardsParameters, FqVar>`
    pub(crate) inner: EdwardsVar,
}

impl Decaf377ElementVar {
    /// R1CS equivalent of `Element::vartime_compress_to_field`
    pub fn compress_to_field(&self) -> Result<FqVar, SynthesisError> {
        // We have affine x, y but our compression formulae are in projective.
        let affine_x = &self.inner.x;
        let affine_y = &self.inner.y;

        let X = affine_x;
        // We treat Z at a constant.
        let Z = FqVar::constant(Fq::one());
        let T = affine_x * affine_y;

        let A_MINUS_D = FqVar::constant(EdwardsParameters::COEFF_A - EdwardsParameters::COEFF_D);

        // 1.
        let u_1 = (X + T.clone()) * (X - T.clone());

        // 2.
        let den = u_1.clone() * A_MINUS_D.clone() * X.square()?;
        let one_over_den = den.inverse()?;
        let (_, v) = FqVar::isqrt(one_over_den)?;
        let v_var = FqVar::constant(v);

        // 3.
        let u_2: FqVar = (v_var * u_1).abs()?;

        // 4.
        let u_3 = u_2 * Z - T;

        // 5.
        let s = (A_MINUS_D * v * u_3 * X).abs()?;

        Ok(s)
    }

    /// R1CS equivalent of `Encoding::vartime_decompress`
    pub fn decompress_from_field(s: FqVar) -> Result<Decaf377ElementVar, SynthesisError> {
        let D4 = FqVar::constant(EdwardsParameters::COEFF_D * Fq::from(4u32));

        // 1. We do not check if canonically encoded here since we know FqVar is already
        // a valid Fq field element.

        // 2. Reject if negative.
        let is_nonnegative = s.is_nonnegative()?;
        let cs = s.cs();
        // TODO: Is constant the right allocation mode?
        let is_nonnegative_var = Boolean::new_variable(
            ns!(cs, "is_nonnegative"),
            || Ok(is_nonnegative),
            AllocationMode::Constant,
        )?;
        is_nonnegative_var.enforce_equal(&Boolean::TRUE)?;

        // 3. u_1 <- 1 - s^2
        let ss = s.square()?;
        let u_1 = FqVar::one() - ss.clone();

        // 4. u_2 <- u_1^2 - 4d s^2
        let u_2 = u_1.square()? - D4 * ss.clone();

        // 5. sqrt
        let den = u_2.clone() * u_1.square()?;
        let one_over_den = den.inverse()?;
        let (was_square, v) = FqVar::isqrt(one_over_den)?;
        let mut v_var = FqVar::constant(v);
        let was_square_var = Boolean::new_variable(
            ns!(cs, "is_square"),
            || Ok(was_square),
            AllocationMode::Constant,
        )?;
        was_square_var.enforce_equal(&Boolean::TRUE)?;

        // 6. Sign check
        let two_s_u_1 = (FqVar::one() + FqVar::one()) * s * u_1.clone();
        // In `vartime_decompress`, we check if it's negative prior to taking
        // the negative, which is effectively the absolute value:
        v_var = v_var.abs()?;

        // 7. (Extended) Coordinates
        let x = two_s_u_1 * v.square() * u_2;
        let y = (FqVar::one() + ss) * v_var * u_1;
        //let z = FqVar::one();
        //let t = x.clone() * y.clone();

        // Note that the above are in extended, but we need affine coordinates
        // for forming `AffineVar` where x = X/Z, y = Y/Z. However Z is
        // hardcoded to be 1 above, so we can use x and y as is.
        Ok(Decaf377ElementVar {
            inner: AffineVar::new(x, y),
        })
    }
}

impl EqGadget<Fq> for Decaf377ElementVar {
    fn is_eq(&self, other: &Self) -> Result<Boolean<Fq>, SynthesisError> {
        // Section 4.5 of Decaf paper: X_1 * Y_2 = X_2 * Y_1
        // in extended coordinates, but note that x, y are affine here:
        let X_1 = &self.inner.x;
        let Y_1 = &self.inner.y;
        let X_2 = &other.inner.x;
        let Y_2 = &other.inner.y;
        let lhs = X_1 * Y_2;
        let rhs = X_2 * Y_1;
        lhs.is_eq(&rhs)
    }

    fn conditional_enforce_equal(
        &self,
        other: &Self,
        should_enforce: &Boolean<Fq>,
    ) -> Result<(), SynthesisError> {
        // should_enforce = true
        //      return self == other
        // should_enforce = false
        //      return true
        self.is_eq(other)?
            .conditional_enforce_equal(&Boolean::constant(true), should_enforce)
    }

    fn conditional_enforce_not_equal(
        &self,
        other: &Self,
        should_enforce: &Boolean<Fq>,
    ) -> Result<(), SynthesisError> {
        self.is_eq(other)?
            .conditional_enforce_equal(&Boolean::constant(false), should_enforce)
    }
}

impl R1CSVar<Fq> for Decaf377ElementVar {
    type Value = Element;

    fn cs(&self) -> ConstraintSystemRef<Fq> {
        self.inner.cs()
    }

    fn value(&self) -> Result<Self::Value, SynthesisError> {
        let (x, y) = (self.inner.x.value()?, self.inner.y.value()?);
        let result = EdwardsAffine::new(x, y);
        Ok(Element {
            inner: result.into(),
        })
    }
}

impl CondSelectGadget<Fq> for Decaf377ElementVar {
    fn conditionally_select(
        cond: &Boolean<Fq>,
        true_value: &Self,
        false_value: &Self,
    ) -> Result<Self, SynthesisError> {
        let x = cond.select(&true_value.inner.x, &false_value.inner.x)?;
        let y = cond.select(&true_value.inner.y, &false_value.inner.y)?;

        Ok(Decaf377ElementVar {
            inner: EdwardsVar::new(x, y),
        })
    }
}

// This lets us use `new_constant`, `new_input` (public), or `new_witness` to add
// decaf elements to an R1CS constraint system.
impl AllocVar<Element, Fq> for Decaf377ElementVar {
    fn new_variable<T: std::borrow::Borrow<Element>>(
        cs: impl Into<ark_relations::r1cs::Namespace<Fq>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let ns = cs.into();
        let cs = ns.cs();
        let f = || Ok(*f()?.borrow());
        let group_projective_point = f()?;

        // `new_variable` should *not* allocate any new variables or constraints in `cs` when
        // the mode is `AllocationMode::Constant` (see `AllocVar::new_constant`).
        //
        // Compare this with the implementation of this trait for `EdwardsVar`
        // where they check that the point is in the right subgroup prior to witnessing.
        match mode {
            AllocationMode::Constant => Ok(Self {
                inner: EdwardsVar::new_variable_omit_prime_order_check(
                    cs,
                    || Ok(group_projective_point.inner),
                    mode,
                )?,
            }),
            AllocationMode::Input => Ok(Self {
                inner: EdwardsVar::new_variable_omit_prime_order_check(
                    cs,
                    || Ok(group_projective_point.inner),
                    mode,
                )?,
            }),
            AllocationMode::Witness => {
                let ge: EdwardsAffine = group_projective_point.inner.into();
                let P = AffineVar::new_variable(ns!(cs, "P_affine"), || Ok(ge), mode)?;

                // At this point P might not be a valid representative of a decaf point.
                //
                // One way that is secure but provides stronger constraints than we need:
                // 1. Encode (out of circuit) to an Fq
                // 2. Witness the encoded value
                // 3. Decode (in circuit)
                //
                // But a cheaper option is to prove this point is in the
                // image of the encoding map. We can do so
                // by checking if the point is even (see section 1.2 Decaf paper):

                // 1. Outside circuit, compute Q = 1/2 * P
                let half = Fr::from(2)
                    .inverse()
                    .expect("inverse of 2 should exist in Fr");
                // To do scalar mul between `Fr` and `GroupProjective`, need to
                // use `std::ops::MulAssign`
                let mut Q = ge;
                Q *= half;

                // 2. Inside the circuit, witness Q
                let Q_var = AffineVar::new_variable(ns!(cs, "Q_affine"), || Ok(Q), mode)?;

                // 3. Add equality constraint that Q + Q = P
                (Q_var.clone() + Q_var).enforce_equal(&P)?;

                Ok(Self { inner: P })
            }
        }
    }
}

impl AllocVar<AffineElement, Fq> for Decaf377ElementVar {
    fn new_variable<T: Borrow<AffineElement>>(
        cs: impl Into<ark_relations::r1cs::Namespace<Fq>>,
        f: impl FnOnce() -> Result<T, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        Self::new_variable(cs, || f().map(|b| b.borrow().into_projective()), mode)
    }
}

impl ToBitsGadget<Fq> for Decaf377ElementVar {
    fn to_bits_le(&self) -> Result<Vec<Boolean<Fq>>, SynthesisError> {
        let compressed_fq = self.compress_to_field()?;
        let encoded_bits = compressed_fq.to_bits_le()?;
        Ok(encoded_bits)
    }
}

impl ToBytesGadget<Fq> for Decaf377ElementVar {
    fn to_bytes(&self) -> Result<Vec<UInt8<Fq>>, SynthesisError> {
        let compressed_fq = self.compress_to_field()?;
        let encoded_bytes = compressed_fq.to_bytes()?;
        Ok(encoded_bytes)
    }
}

impl<'a> GroupOpsBounds<'a, Element, Decaf377ElementVar> for Decaf377ElementVar {}

impl CurveVar<Element, Fq> for Decaf377ElementVar {
    fn zero() -> Self {
        Self {
            inner: AffineVar::<EdwardsParameters, FqVar>::zero(),
        }
    }

    fn constant(other: Element) -> Self {
        Self {
            inner: AffineVar::<EdwardsParameters, FqVar>::constant(other.inner),
        }
    }

    fn new_variable_omit_prime_order_check(
        cs: impl Into<ark_relations::r1cs::Namespace<Fq>>,
        f: impl FnOnce() -> Result<Element, SynthesisError>,
        mode: AllocationMode,
    ) -> Result<Self, SynthesisError> {
        let ns = cs.into();
        let cs = ns.cs();

        match f() {
            Ok(ge) => {
                let P = AffineVar::new_variable_omit_prime_order_check(cs, || Ok(ge.inner), mode)?;
                Ok(Self { inner: P })
            }
            _ => Err(SynthesisError::AssignmentMissing),
        }
    }

    fn enforce_prime_order(&self) -> Result<(), SynthesisError> {
        // This is decaf
        Ok(())
    }

    fn double_in_place(&mut self) -> Result<(), SynthesisError> {
        self.inner.double_in_place()?;
        Ok(())
    }

    fn negate(&self) -> Result<Self, SynthesisError> {
        let negated = self.inner.negate()?;
        Ok(Self { inner: negated })
    }
}

impl ToConstraintField<Fq> for Element {
    fn to_field_elements(&self) -> Option<Vec<Fq>> {
        self.inner.to_field_elements()
    }
}
