//! Vehicle physics (placeholder; filled in next).

use crate::body::BodyHandle;
use crate::arena::Arena;
use crate::world::SubstepContext;

pub struct Vehicle {
    pub chassis: BodyHandle,
}

impl Vehicle {
    pub fn can_sleep(&self) -> bool {
        true
    }
}

pub(crate) fn substep_vehicles(_v: &mut Arena<Vehicle>, _ctx: &mut SubstepContext) {}
