use std::path::Path;

use fastfile_iw4::load_zone;

use super::{CommonCensus, CommonWalkSink};
use asset_transport::{LoadProgress, StageId, ZoneImage, ZoneMemory};

/// Every weapon def is kept: ids are positions in the sorted catalog, and a
/// dropped def would shift them and the content digest.
pub(super) fn load_common_mp_view_only(
    path: &Path,
    image: &ZoneImage,
    progress: &LoadProgress,
    material_seed: asset_material::MaterialCatalog,
) -> CommonCensus {
    let zone_name = path.file_stem().map_or_else(
        || "common_mp".to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let header = match image.header() {
        Ok(header) => header,
        Err(error) => {
            return CommonCensus {
                material_population: material_seed,
                report: vec![format!("common_mp view walk: zone header: {error}")],
                ..Default::default()
            };
        }
    };
    let mut memory = ZoneMemory::for_header(&header);
    let mut stream = match memory.stream(&image.bytes) {
        Ok(stream) => stream,
        Err(error) => {
            return CommonCensus {
                material_population: material_seed,
                report: vec![format!("common_mp view walk: zone arenas: {error}")],
                ..Default::default()
            };
        }
    };
    let mut sink = CommonWalkSink::with_stage(progress.begin_scoped(
        StageId::CommonAssets,
        zone_name.clone(),
        None,
    ));
    sink.view_only = true;
    sink.seed_materials(material_seed);
    sink.set_capture_zone(asset_core::ZoneOwner::intern(&zone_name));
    sink.set_capture_ns(asset_core::AssetNamespace::Iw4);
    sink.sound = asset_audio::ZoneSoundCapture::claim_common(
        path,
        asset_audio::ZoneGame::Iw4,
        "common view walk",
    );
    let walk = load_zone(&mut stream, &mut sink);
    if let Some(sound) = sink.sound.take() {
        sound.deposit(walk.as_ref().map(|_| ()).map_err(|e| e.to_string()));
    }
    if let Some(stage) = sink.stage.take() {
        stage.finish_from(&walk);
    }
    let mut report = Vec::new();
    if let Err(error) = walk {
        report.push(format!(
            "common_mp view walk: stopped after {} assets — {error}",
            sink.walked
        ));
    }

    sink.weapons.resolve_reticles(&sink.materials);
    let graph = crate::resolve_after_absorb(
        &sink.materials,
        &mut sink.tracers,
        &mut sink.fx,
        Some(&mut sink.weapons),
        None,
        Some(&mut sink.world_weapons),
        None,
        Some(&mut sink.fpv_meshes),
        Some(&mut sink.projectile_meshes),
    );
    let mut weapons = sink.weapons.into_build();
    weapons.apply_stats_tables(sink.stats_tables.values());

    let named = weapons.view_asset_names();
    let fpv_walked = sink.fpv_meshes.len();
    sink.fpv_meshes.retain_keys(|key| {
        named.models.contains(&(key.namespace, key.name.clone()))
            || asset_model::is_arms_model(&key.name)
            || key.name == asset_model::VIEWHANDS_NAME
    });
    let world_walked = sink.world_weapons.len();
    sink.world_weapons
        .retain_keys(|key| named.world_models.contains(&(key.namespace, key.name.clone())));
    let clips_walked = sink.xanims.len();
    sink.xanims.retain_keys(|key| named.clips.contains(&(key.namespace, key.name.clone())));
    let fpv_meshes = sink.fpv_meshes.len();
    let world_models = sink.world_weapons.len();
    let clips = sink.xanims.len();

    weapons.resolve_sz_xanim_edges(&sink.xanims);
    weapons.resolve_fpv_mesh_edges(&sink.fpv_meshes);
    weapons.resolve_world_model_edges(&sink.world_weapons);
    let sz_xanims = weapons.sz_xanim_edge_census();
    report.push(format!(
        "common_mp view walk: weapons={} (catalog ids unchanged); FPV meshes kept={fpv_meshes}/{fpv_walked} (named by {} model names); world models kept={world_models}/{world_walked} (named by {}); clips kept={clips}/{clips_walked} (named by {} clip names); szXAnims bound={} unresolved={} absent={}; weapon HUD materials bound={} unresolved={}; projectiles, script models, shared surfaces and fx not captured",
        weapons.len(),
        named.models.len(),
        named.world_models.len(),
        named.clips.len(),
        sz_xanims.bound,
        sz_xanims.unresolved,
        sz_xanims.absent,
        graph.weapon_hud_materials.bound,
        graph.weapon_hud_materials.unresolved,
    ));

    let light_defs = asset_world::capture_light_defs(&stream, &sink.materials);
    drop(stream);
    let s1_common_bytes = memory.total_bytes();
    drop(memory);
    sink.materials.resolve_technique_set_edges();
    let materials = sink.materials.image_memory();
    report.push(format!(
        "common_mp view walk: materials={} images={} decoded={} (image decode deferred to the merged pass)",
        sink.materials.materials.len(),
        materials.images,
        materials.decoded_images,
    ));
    CommonCensus {
        weapons,
        cac_tables: sink.stats_tables.into_values().collect(),
        fpv: sink.fpv_meshes,
        world_weapons: sink.world_weapons,
        xanims: sink.xanims,
        player_anim_sources: sink.player_anim_sources,
        material_population: sink.materials,
        light_defs,
        report,
        pen_table: sink.pen_table.unwrap_or_default(),
        pen_table_loaded: sink.pen_table.is_some(),
        lochit_table: sink.lochit_table,
        xmodel_walk: sink.models.walk_census(),
        s1_common_bytes,
        scripts: sink.scripts,
        film_visions: sink.film_visions,
        ..Default::default()
    }
}
