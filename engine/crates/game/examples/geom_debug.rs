use gridlock_game::*;
fn main() {
    let g = Game::new(GameConfig {
        traffic: 0,
        props: false,
        ..Default::default()
    });
    let net = g.traffic.net();
    println!("xs {:?}\nys {:?}", g.traffic.grid_xs, g.traffic.grid_ys);
    let l = net.lane(gridlock_traffic::LaneId(196));
    let r = net.road(l.road);
    let (a, b) = (net.node(r.a), net.node(r.b));
    println!("lane 196 road {:?} a ({},{}) b ({},{}) width {} lanes_ab {} lanes_ba {} median {} lane_w {} lane idx {} fwd {}", r.id, a.x, a.y, b.x, b.y, r.width, r.lanes_ab.len(), r.lanes_ba.len(), r.median_width, r.lane_width, l.index, l.forward);
    let p = l.path.sample(10.0);
    println!("lane point {:.2},{:.2}", p.x, p.y);
    for bl in &g.city.blocks {
        if bl.min.x < 180.0 && bl.max.x > 170.0 && (bl.min.y - -26.9).abs() < 30.0 {
            println!("block {:?}", bl);
        }
    }
}
