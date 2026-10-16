use gridlock_game::*;
use gridlock_input::ControlFrame;
use gridlock_physics::PhysicsEvent;
use std::collections::HashMap;
fn main() {
    let mut g = Game::new(GameConfig {
        traffic: 60,
        ..Default::default()
    });
    let frame = ControlFrame::default();
    let prop_kind: HashMap<_, _> = g.props.props.iter().map(|p| (p.body, p.kind)).collect();
    let car_bodies = |g: &Game| -> HashMap<gridlock_physics::BodyHandle, u32> {
        g.links
            .iter()
            .filter_map(|(id, l)| match &l.state {
                CarState::Lane { body } => Some((*body, id.0)),
                CarState::Physics { vehicle, .. } => {
                    g.physics.vehicle(*vehicle).map(|v| (v.chassis, id.0))
                }
            })
            .collect()
    };
    let mut tally: HashMap<String, usize> = HashMap::new();
    for _ in 0..(60 * 60) {
        g.step(&frame);
        let cars = car_bodies(&g);
        for e in g.physics.events() {
            if let PhysicsEvent::Impact {
                body_a,
                body_b,
                speed,
                point,
                ..
            } = *e
            {
                if speed < 2.0 {
                    continue;
                }
                for (me, other) in [(body_a, body_b), (body_b, body_a)] {
                    if cars.contains_key(&me) && !cars.contains_key(&other) {
                        let what = if let Some(k) = prop_kind.get(&other) {
                            format!("{k:?}")
                        } else if (other.0.index as usize) < 400 {
                            format!("static#{}", other.0.index)
                        } else {
                            "other".into()
                        };
                        let lane = g.links[&gridlock_traffic::VehicleId(cars[&me])]
                            .state
                            .is_lane();
                        let key = format!(
                            "{} {}",
                            if lane { "lane" } else { "phys" },
                            what.split('#').next().unwrap()
                        );
                        *tally.entry(key).or_default() += 1;
                        if lane && what.starts_with("static") {
                            let vid = gridlock_traffic::VehicleId(cars[&me]);
                            if let Some(v) = g.traffic.vehicle(vid) {
                                let (seg, lat, s) = (v.seg, v.lat, v.s);
                                let lw = match seg {
                                    Some(gridlock_traffic::SegRef::Lane(l)) => format!(
                                        "lane w {:.2} road w {:.2} sw {:.1}",
                                        g.traffic.net().lane(l).width,
                                        g.traffic.net().road(g.traffic.net().lane(l).road).width,
                                        g.traffic
                                            .net()
                                            .road(g.traffic.net().lane(l).road)
                                            .sidewalk_width
                                    ),
                                    _ => "connector".into(),
                                };
                                println!("{:?} {:?} lat {lat:.2} s {s:.1} {lw} hit at {point:?} car at {:?} sp {speed:.1} class {:?}", vid, seg, (v.x, v.y), v.spec.id);
                            }
                        }
                    }
                }
            }
        }
    }
    println!("{tally:?}");
}
