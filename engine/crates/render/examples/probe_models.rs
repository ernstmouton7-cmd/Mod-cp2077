fn main() {
    let store = gridlock_assets::AssetStore::discover().unwrap();
    for id in [
        "model.tree_small",
        "model.tree_jacaranda",
        "model.fire_hydrant",
        "model.trash_can",
        "model.bench",
        "model.street_lamp_victorian",
        "model.shrub",
        "model.utility_box",
        "model.manhole_cover",
    ] {
        match store.load_model(id) {
            Ok(m) => println!(
                "{id}: meshes {} tris {} bounds {:?} mats {:?}",
                m.meshes.len(),
                m.triangle_count(),
                m.bounds,
                m.materials
                    .iter()
                    .map(|x| (x.name.clone(), x.alpha_mode, x.base_color_texture.is_some()))
                    .collect::<Vec<_>>()
            ),
            Err(e) => println!("{id}: {e}"),
        }
    }
    let h = store.load_hdri("hdri.overcast").unwrap();
    println!("hdri {}x{} len {}", h.width, h.height, h.data.len());
}
