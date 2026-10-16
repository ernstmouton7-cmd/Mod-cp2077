//! Street furniture with physical behaviour: posts that bend and snap, hydrants that burst,
//! loose objects that scatter. Each prop is a rigid body; anchored props are welded to a static
//! anchor with a springy, breakable weld (so a light tap wobbles a sign and a car flattens it).

use crate::body::{layers, BodyDesc, BodyHandle, ColliderDesc, Surface};
use crate::joint::{JointDesc, JointKind};
use crate::math::Pose;
use crate::shape::Shape;
use crate::world::{PhysicsEvent, PhysicsWorld};
use crate::JointHandle;
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropKind {
    StreetLamp,
    TrafficSign,
    SignalPole,
    Bollard,
    FireHydrant,
    TrashCan,
    TrafficCone,
    Bench,
    ConcreteBarrier,
    Mailbox,
    Newsstand,
    Planter,
}

#[derive(Clone, Copy, Debug)]
pub struct PropSpec {
    pub mass: f32,
    /// Anchored with a weld (None = free-standing).
    pub anchor: Option<Anchor>,
    pub surface: Surface,
}

#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    /// Weld spring (Hz) — low values bend visibly before breaking.
    pub hertz: f32,
    pub break_force: f32,
    pub break_torque: f32,
}

impl PropKind {
    pub fn spec(self) -> PropSpec {
        let a = |hertz, f, t| {
            Some(Anchor {
                hertz,
                break_force: f,
                break_torque: t,
            })
        };
        use PropKind::*;
        let (mass, anchor, surface) = match self {
            StreetLamp => (160.0, a(6.0, 90_000.0, 22_000.0), Surface::Metal),
            TrafficSign => (25.0, a(4.0, 15_000.0, 2_500.0), Surface::Metal),
            SignalPole => (450.0, a(9.0, 400_000.0, 140_000.0), Surface::Metal),
            Bollard => (120.0, a(20.0, 600_000.0, 90_000.0), Surface::Metal),
            FireHydrant => (110.0, a(25.0, 160_000.0, 18_000.0), Surface::Metal),
            Mailbox => (45.0, a(10.0, 20_000.0, 4_000.0), Surface::Metal),
            TrashCan => (14.0, None, Surface::Metal),
            TrafficCone => (3.0, None, Surface::Rubber),
            Bench => (55.0, None, Surface::Wood),
            ConcreteBarrier => (1100.0, None, Surface::Concrete),
            Newsstand => (70.0, None, Surface::Metal),
            Planter => (300.0, None, Surface::Concrete),
        };
        PropSpec {
            mass,
            anchor,
            surface,
        }
    }

    /// Collision shapes in the prop frame (origin on the ground, +Y up).
    pub fn shapes(self) -> Vec<(Shape, Pose)> {
        use PropKind::*;
        let up = |y: f32| Pose::from_translation(Vec3::new(0.0, y, 0.0));
        match self {
            StreetLamp => vec![
                (Shape::capsule(3.6, 0.1), up(3.7)),
                (
                    Shape::cuboid(Vec3::new(0.06, 0.06, 0.9)),
                    Pose::from_translation(Vec3::new(0.0, 7.3, -0.8)),
                ),
                (
                    Shape::cuboid(Vec3::new(0.25, 0.12, 0.35)),
                    Pose::from_translation(Vec3::new(0.0, 7.2, -1.6)),
                ),
            ],
            TrafficSign => vec![
                (Shape::capsule(1.3, 0.035), up(1.35)),
                (Shape::cuboid(Vec3::new(0.38, 0.38, 0.01)), up(2.45)),
            ],
            SignalPole => vec![
                (Shape::capsule(2.8, 0.14), up(2.95)),
                (
                    Shape::cuboid(Vec3::new(0.08, 0.08, 3.0)),
                    Pose::from_translation(Vec3::new(0.0, 5.6, -3.0)),
                ),
            ],
            Bollard => vec![(Shape::capsule(0.38, 0.11), up(0.5))],
            FireHydrant => vec![
                (Shape::capsule(0.25, 0.14), up(0.4)),
                (Shape::cuboid(Vec3::new(0.24, 0.05, 0.05)), up(0.45)),
            ],
            Mailbox => vec![(Shape::cuboid(Vec3::new(0.28, 0.6, 0.25)), up(0.6))],
            TrashCan => vec![(cylinder(0.28, 0.5, 12), up(0.5))],
            TrafficCone => vec![(cone(0.18, 0.72, 12), up(0.0))],
            Bench => vec![
                (Shape::cuboid(Vec3::new(0.9, 0.04, 0.22)), up(0.45)),
                (
                    Shape::cuboid(Vec3::new(0.9, 0.2, 0.03)),
                    Pose::from_translation(Vec3::new(0.0, 0.7, 0.2)),
                ),
                (
                    Shape::cuboid(Vec3::new(0.04, 0.22, 0.22)),
                    Pose::from_translation(Vec3::new(-0.8, 0.22, 0.0)),
                ),
                (
                    Shape::cuboid(Vec3::new(0.04, 0.22, 0.22)),
                    Pose::from_translation(Vec3::new(0.8, 0.22, 0.0)),
                ),
            ],
            ConcreteBarrier => vec![(jersey_barrier(), up(0.0))],
            Newsstand => vec![(Shape::cuboid(Vec3::new(0.3, 0.55, 0.25)), up(0.55))],
            Planter => vec![(Shape::cuboid(Vec3::new(0.6, 0.35, 0.6)), up(0.35))],
        }
    }
}

fn cylinder(r: f32, half_h: f32, n: usize) -> Shape {
    let mut p = Vec::new();
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        p.push(Vec3::new(a.cos() * r, -half_h, a.sin() * r));
        p.push(Vec3::new(a.cos() * r, half_h, a.sin() * r));
    }
    Shape::convex(&p).unwrap()
}

fn cone(r: f32, h: f32, n: usize) -> Shape {
    let mut p = vec![Vec3::new(0.0, h, 0.0)];
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        p.push(Vec3::new(a.cos() * r, 0.0, a.sin() * r));
        p.push(Vec3::new(a.cos() * r * 1.4, 0.03, a.sin() * r * 1.4));
    }
    Shape::convex(&p).unwrap()
}

fn jersey_barrier() -> Shape {
    let profile = [
        (0.3, 0.0),
        (0.3, 0.08),
        (0.17, 0.33),
        (0.08, 0.81),
        (0.08, 0.81),
    ];
    let mut p = Vec::new();
    for (x, y) in profile {
        for z in [-1.5f32, 1.5] {
            p.push(Vec3::new(x, y, z));
            p.push(Vec3::new(-x, y, z));
        }
    }
    Shape::convex(&p).unwrap()
}

#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub kind: PropKind,
    pub body: BodyHandle,
    pub anchor: Option<BodyHandle>,
    pub weld: Option<JointHandle>,
    /// Knocked over / broken off its base.
    pub broken: bool,
    pub home: Pose,
}

#[derive(Clone, Copy, Debug)]
pub enum PropEvent {
    /// A post snapped off its base.
    Snapped { prop: usize, position: Vec3 },
    /// Hit hard enough to make a noise (for audio).
    Hit {
        prop: usize,
        position: Vec3,
        impulse: f32,
    },
    /// A hydrant was sheared off: start a water geyser here.
    HydrantBurst { prop: usize, position: Vec3 },
}

#[derive(Default)]
pub struct PropSystem {
    pub props: Vec<Prop>,
}

impl PropSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, world: &mut PhysicsWorld, kind: PropKind, pose: Pose) -> usize {
        let spec = kind.spec();
        let shapes = kind.shapes();
        // Start asleep: city props are many and still until something hits them.
        let body = world.add_body(BodyDesc {
            awake: false,
            angular_damping: 0.1,
            ..BodyDesc::dynamic(pose)
        });
        let mut parts = Vec::new();
        for (s, local) in &shapes {
            parts.push(s.mass_properties(1.0).transformed(local));
        }
        let vol: f32 = parts.iter().map(|p| p.mass).sum();
        let density = spec.mass / vol.max(1e-6);
        for (s, local) in shapes {
            world.add_collider(
                body,
                ColliderDesc::new(s)
                    .at(local)
                    .density(density)
                    .surface(spec.surface)
                    .friction(0.6)
                    .filter(layers::PROP, layers::ALL),
            );
        }
        let mut anchor = None;
        let mut weld = None;
        if let Some(a) = spec.anchor {
            let base = world.add_body(BodyDesc::fixed(pose));
            let j = world.add_joint(
                JointDesc::new(
                    base,
                    body,
                    Pose::IDENTITY,
                    Pose::IDENTITY,
                    JointKind::Weld {
                        linear_hertz: a.hertz * 3.0,
                        angular_hertz: a.hertz,
                        damping_ratio: 0.7,
                    },
                )
                .breakable(a.break_force, a.break_torque),
            );
            anchor = Some(base);
            weld = Some(j);
        }
        if let Some(b) = world.body_mut(body) {
            b.sleeping = true;
        }
        self.props.push(Prop {
            kind,
            body,
            anchor,
            weld,
            broken: false,
            home: pose,
        });
        self.props.len() - 1
    }

    /// Inspect the last step's events.
    pub fn update(&mut self, world: &PhysicsWorld) -> Vec<PropEvent> {
        let mut out = Vec::new();
        for e in world.events() {
            match *e {
                PhysicsEvent::JointBroken { joint, .. } => {
                    if let Some((i, p)) = self
                        .props
                        .iter_mut()
                        .enumerate()
                        .find(|(_, p)| p.weld == Some(joint))
                    {
                        p.broken = true;
                        p.weld = None;
                        let pos = p.home.p;
                        out.push(PropEvent::Snapped {
                            prop: i,
                            position: pos,
                        });
                        if p.kind == PropKind::FireHydrant {
                            out.push(PropEvent::HydrantBurst {
                                prop: i,
                                position: pos + Vec3::Y * 0.3,
                            });
                        }
                    }
                }
                PhysicsEvent::Impact {
                    body_a,
                    body_b,
                    point,
                    impulse,
                    ..
                } => {
                    if let Some((i, _)) = self
                        .props
                        .iter()
                        .enumerate()
                        .find(|(_, p)| p.body == body_a || p.body == body_b)
                    {
                        if impulse > 150.0 {
                            out.push(PropEvent::Hit {
                                prop: i,
                                position: point,
                                impulse,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        // Free-standing props count as knocked over once tipped past 60°.
        for p in &mut self.props {
            if !p.broken && p.anchor.is_none() {
                if let Some(b) = world.body(p.body) {
                    if (b.pose.q * Vec3::Y).y < 0.5 {
                        p.broken = true;
                    }
                }
            }
        }
        out
    }

    /// Put a prop back where it started (city maintenance / respawn).
    pub fn reset(&mut self, world: &mut PhysicsWorld, i: usize) {
        let p = self.props[i];
        world.set_pose(p.body, p.home);
        world.set_velocity(p.body, Vec3::ZERO, Vec3::ZERO);
        if let (Some(a), None) = (p.anchor, p.weld) {
            if let Some(spec) = p.kind.spec().anchor {
                let j = world.add_joint(
                    JointDesc::new(
                        a,
                        p.body,
                        Pose::IDENTITY,
                        Pose::IDENTITY,
                        JointKind::Weld {
                            linear_hertz: spec.hertz * 3.0,
                            angular_hertz: spec.hertz,
                            damping_ratio: 0.7,
                        },
                    )
                    .breakable(spec.break_force, spec.break_torque),
                );
                self.props[i].weld = Some(j);
            }
        }
        self.props[i].broken = false;
    }
}
