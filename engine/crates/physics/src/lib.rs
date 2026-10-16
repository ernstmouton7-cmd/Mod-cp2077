#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]
//! # gridlock-physics
//!
//! A general-purpose 3D physics engine written for the Gridlock driving simulation:
//!
//! - rigid bodies (static, kinematic, dynamic) with compound colliders: spheres, capsules, boxes,
//!   convex hulls and static triangle meshes;
//! - a dynamic AABB tree broadphase and SAT/GJK narrowphase with speculative contacts;
//! - a sub-stepped soft-constraint solver (stable stacks, smooth penetration recovery);
//! - joints (ball, weld/spring, hinge with limits/motor/spring, distance) that can break;
//! - sleeping islands, contact/impact events, ray/sphere casts and overlap queries;
//! - vehicles: multi-wheel ray/sphere-cast suspension, combined-slip tyres, drivetrain
//!   and assists, plus body deformation and detachable parts.
//!
//! Units are SI (metres, kilograms, seconds). +Y is up.

pub mod aabb;
pub mod arena;
pub mod body;
pub mod broadphase;
pub mod bvh;
pub mod gjk;
pub mod joint;
pub mod math;
pub mod narrowphase;
pub mod query;
pub mod shape;
pub mod solver;
pub mod vehicle;
pub mod world;

pub use aabb::Aabb;
pub use body::{
    layers, BodyDesc, BodyHandle, BodyType, Collider, ColliderDesc, ColliderHandle,
    CollisionFilter, JointHandle, RigidBody, Surface,
};
pub use glam;
pub use joint::{HingeLimits, HingeMotor, HingeSpring, Joint, JointDesc, JointKind};
pub use math::Pose;
pub use query::{QueryFilter, QueryView, RayHit};
pub use shape::{ConvexHull, MassProperties, Shape, TriMesh};
pub use vehicle::{
    presets::VehicleClass, Assists, Vehicle, VehicleControls, VehicleDesc, VehicleHandle,
};
pub use world::{
    Contact, Manifold, ManifoldPoint, PhysicsEvent, PhysicsWorld, StepStats, SubstepContext,
};
