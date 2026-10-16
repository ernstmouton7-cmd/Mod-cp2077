//! Rigid bodies and colliders.

use crate::aabb::Aabb;
use crate::arena::Handle;
use crate::math::Pose;
use crate::shape::{MassProperties, Shape};
use glam::{Mat3, Vec3};
use smallvec::SmallVec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BodyHandle(pub Handle);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColliderHandle(pub Handle);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JointHandle(pub Handle);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyType {
    /// Never moves (roads, buildings).
    Static,
    /// Moved by velocity set by the user; pushes dynamic bodies but is not affected by them.
    Kinematic,
    /// Fully simulated.
    Dynamic,
}

/// Construction parameters for a body.
#[derive(Clone, Debug)]
pub struct BodyDesc {
    pub body_type: BodyType,
    pub pose: Pose,
    pub lin_vel: Vec3,
    pub ang_vel: Vec3,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    pub can_sleep: bool,
    pub awake: bool,
    /// Treat fast motion conservatively (bigger speculative margin).
    pub bullet: bool,
    pub user_data: u64,
}

impl Default for BodyDesc {
    fn default() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            pose: Pose::IDENTITY,
            lin_vel: Vec3::ZERO,
            ang_vel: Vec3::ZERO,
            linear_damping: 0.0,
            angular_damping: 0.05,
            gravity_scale: 1.0,
            can_sleep: true,
            awake: true,
            bullet: false,
            user_data: 0,
        }
    }
}

impl BodyDesc {
    pub fn dynamic(pose: Pose) -> Self {
        Self {
            pose,
            ..Default::default()
        }
    }
    pub fn fixed(pose: Pose) -> Self {
        Self {
            pose,
            body_type: BodyType::Static,
            ..Default::default()
        }
    }
    pub fn kinematic(pose: Pose) -> Self {
        Self {
            pose,
            body_type: BodyType::Kinematic,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct RigidBody {
    pub body_type: BodyType,
    /// Pose of the body origin.
    pub pose: Pose,
    /// Pose at the start of the last step (for render interpolation and motion vectors).
    pub prev_pose: Pose,
    /// Centre of mass in the body frame.
    pub local_com: Vec3,
    pub lin_vel: Vec3,
    pub ang_vel: Vec3,
    pub force: Vec3,
    pub torque: Vec3,
    pub mass: f32,
    pub inv_mass: f32,
    /// Inertia about the centre of mass, body frame.
    pub inertia: Mat3,
    pub inv_inertia_local: Mat3,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub gravity_scale: f32,
    pub can_sleep: bool,
    pub sleeping: bool,
    pub sleep_time: f32,
    pub bullet: bool,
    pub user_data: u64,
    /// Fixed mass properties set by the user (otherwise derived from colliders).
    pub mass_override: Option<MassProperties>,
    pub colliders: SmallVec<[ColliderHandle; 4]>,
}

impl RigidBody {
    pub(crate) fn from_desc(d: &BodyDesc) -> Self {
        RigidBody {
            body_type: d.body_type,
            pose: d.pose,
            prev_pose: d.pose,
            local_com: Vec3::ZERO,
            lin_vel: d.lin_vel,
            ang_vel: d.ang_vel,
            force: Vec3::ZERO,
            torque: Vec3::ZERO,
            mass: 0.0,
            inv_mass: 0.0,
            inertia: Mat3::ZERO,
            inv_inertia_local: Mat3::ZERO,
            linear_damping: d.linear_damping,
            angular_damping: d.angular_damping,
            gravity_scale: d.gravity_scale,
            can_sleep: d.can_sleep,
            sleeping: !d.awake && d.body_type != BodyType::Static,
            sleep_time: 0.0,
            bullet: d.bullet,
            user_data: d.user_data,
            mass_override: None,
            colliders: SmallVec::new(),
        }
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        self.body_type == BodyType::Dynamic
    }
    #[inline]
    pub fn is_static(&self) -> bool {
        self.body_type == BodyType::Static
    }
    /// World-space centre of mass.
    #[inline]
    pub fn com(&self) -> Vec3 {
        self.pose.transform_point(self.local_com)
    }
    /// World inverse inertia tensor.
    #[inline]
    pub fn inv_inertia_world(&self) -> Mat3 {
        let r = Mat3::from_quat(self.pose.q);
        r * self.inv_inertia_local * r.transpose()
    }
    /// Velocity of a world point attached to the body.
    #[inline]
    pub fn velocity_at(&self, p: Vec3) -> Vec3 {
        self.lin_vel + self.ang_vel.cross(p - self.com())
    }
    /// Kinetic energy (for tests/diagnostics).
    pub fn kinetic_energy(&self) -> f32 {
        if self.inv_mass == 0.0 {
            return 0.0;
        }
        let r = Mat3::from_quat(self.pose.q);
        let iw = r * self.inertia * r.transpose();
        0.5 * self.mass * self.lin_vel.length_squared() + 0.5 * self.ang_vel.dot(iw * self.ang_vel)
    }
}

/// Collision group filtering: two colliders interact iff each one's `memberships` intersects the other's `filter`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionFilter {
    pub memberships: u32,
    pub filter: u32,
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self {
            memberships: 1,
            filter: u32::MAX,
        }
    }
}

impl CollisionFilter {
    #[inline]
    pub fn test(&self, o: &CollisionFilter) -> bool {
        (self.memberships & o.filter) != 0 && (o.memberships & self.filter) != 0
    }
}

/// Collision layers used by the engine (bit masks for `CollisionFilter`).
pub mod layers {
    pub const STATIC: u32 = 1 << 0;
    pub const VEHICLE: u32 = 1 << 1;
    pub const PROP: u32 = 1 << 2;
    pub const DEBRIS: u32 = 1 << 3;
    pub const CHARACTER: u32 = 1 << 4;
    pub const WHEEL_RAY: u32 = 1 << 5;
    pub const SENSOR: u32 = 1 << 6;
    pub const ALL: u32 = u32::MAX;
}

/// Surface types (affect tyre grip, sounds and particles).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Surface {
    #[default]
    Asphalt = 0,
    Concrete = 1,
    Cobblestone = 2,
    Grass = 3,
    Dirt = 4,
    Gravel = 5,
    Metal = 6,
    Wood = 7,
    Glass = 8,
    Rubber = 9,
    Water = 10,
}

#[derive(Clone, Debug)]
pub struct ColliderDesc {
    pub shape: Shape,
    pub local: Pose,
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    /// Rolling resistance coefficient (resisting torque = coefficient * radius * normal force).
    pub rolling_resistance: f32,
    pub filter: CollisionFilter,
    pub sensor: bool,
    pub surface: Surface,
    pub user_data: u64,
}

impl ColliderDesc {
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            local: Pose::IDENTITY,
            density: 1000.0,
            friction: 0.6,
            restitution: 0.0,
            rolling_resistance: 0.03,
            filter: CollisionFilter::default(),
            sensor: false,
            surface: Surface::Asphalt,
            user_data: 0,
        }
    }
    pub fn at(mut self, local: Pose) -> Self {
        self.local = local;
        self
    }
    pub fn density(mut self, d: f32) -> Self {
        self.density = d;
        self
    }
    pub fn friction(mut self, f: f32) -> Self {
        self.friction = f;
        self
    }
    pub fn restitution(mut self, r: f32) -> Self {
        self.restitution = r;
        self
    }
    pub fn rolling_resistance(mut self, r: f32) -> Self {
        self.rolling_resistance = r;
        self
    }
    pub fn filter(mut self, memberships: u32, filter: u32) -> Self {
        self.filter = CollisionFilter {
            memberships,
            filter,
        };
        self
    }
    pub fn sensor(mut self, s: bool) -> Self {
        self.sensor = s;
        self
    }
    pub fn surface(mut self, s: Surface) -> Self {
        self.surface = s;
        self
    }
    pub fn user_data(mut self, u: u64) -> Self {
        self.user_data = u;
        self
    }
}

#[derive(Clone, Debug)]
pub struct Collider {
    pub body: BodyHandle,
    pub shape: Shape,
    pub local: Pose,
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub rolling_resistance: f32,
    pub filter: CollisionFilter,
    pub sensor: bool,
    pub surface: Surface,
    pub user_data: u64,
    /// World pose (body pose * local), refreshed after every step.
    pub world: Pose,
    pub(crate) aabb: Aabb,
    pub(crate) proxy: u32,
    pub(crate) is_static_tree: bool,
}

impl Collider {
    pub fn aabb(&self) -> Aabb {
        self.aabb
    }
}
