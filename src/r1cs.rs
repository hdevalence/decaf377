pub mod fqvar_ext;
pub mod gadget;
pub mod ops;

pub use ark_ed_on_bls12_377::constraints::FqVar;
pub use gadget::ElementVar;

use crate::Fq;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystem, OptimizationGoal, SynthesisMode,
};

pub trait CountConstraints: ConstraintSynthesizer<Fq> + Sized {
    fn num_constraints_and_instance_variables(self) -> (usize, usize) {
        let cs = ConstraintSystem::new_ref();
        cs.set_optimization_goal(OptimizationGoal::Constraints);
        cs.set_mode(SynthesisMode::Setup);

        // Synthesize the circuit.
        self.generate_constraints(cs.clone())
            .expect("can generate constraints");
        cs.finalize();
        (cs.num_constraints(), cs.num_instance_variables())
    }
}

impl<T> CountConstraints for T where T: ConstraintSynthesizer<Fq> + Sized {}
