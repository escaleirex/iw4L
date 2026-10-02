use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::render::render_resource::{BindGroupLayoutDescriptor, ShaderModule};
use bevy::render::renderer::RenderDevice;

use super::sm3_wgsl::PASS_VERTEX_ENTRY;

pub(super) fn spirv_passthrough(device: &RenderDevice) -> bool {
    static SLIM: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SLIM.get_or_init(|| std::env::var("IW4L_ANDROID_SLIM").ok().as_deref() == Some("1"))
        && device
            .features()
            .contains(bevy::render::settings::WgpuFeatures::PASSTHROUGH_SHADERS)
}

/// The options the Vulkan backend compiles a WGSL entry point with when the
/// module was created without runtime checks, which is how
/// `RenderDevice::create_shader_module` creates the WGSL path's modules.
fn spirv_options(binding_map: &naga::back::spv::BindingMap) -> naga::back::spv::Options<'static> {
    use naga::back::spv;
    use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
    spv::Options {
        // SPIR-V 1.3 is core in Vulkan 1.1, the floor Android guarantees.
        lang_version: (1, 3),
        flags: spv::WriterFlags::FORCE_POINT_SIZE,
        fake_missing_bindings: false,
        binding_map: binding_map.clone(),
        capabilities: None,
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Unchecked,
            buffer: BoundsCheckPolicy::Unchecked,
            image_load: BoundsCheckPolicy::Unchecked,
            binding_array: BoundsCheckPolicy::Unchecked,
        },
        zero_initialize_workgroup_memory: spv::ZeroInitializeWorkgroupMemoryMode::None,
        force_loop_bounding: false,
        ray_query_initialization_tracking: false,
        use_storage_input_output_16: false,
        debug_info: None,
        task_dispatch_limits: None,
        mesh_shader_primitive_indices_clamp: false,
    }
}

pub(super) fn spirv_entry_modules(
    device: &RenderDevice,
    label: &str,
    wgsl: &str,
    binding_map: &naga::back::spv::BindingMap,
    missing: &[String],
) -> Result<HashMap<String, Arc<ShaderModule>>, String> {
    let module = naga::front::wgsl::parse_str(wgsl).map_err(|error| error.emit_to_string(wgsl))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|error| error.to_string())?;
    let options = spirv_options(binding_map);
    let mut created = HashMap::new();
    for entry in missing {
        let shader_stage = if entry == PASS_VERTEX_ENTRY {
            naga::ShaderStage::Vertex
        } else {
            naga::ShaderStage::Fragment
        };
        let words = naga::back::spv::write_vec(
            &module,
            &info,
            &options,
            Some(&naga::back::spv::PipelineOptions {
                shader_stage,
                entry_point: entry.clone(),
            }),
        )
        .map_err(|error| format!("{entry}: {error}"))?;
        let shader = unsafe {
            device
                .wgpu_device()
                .create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
                    label: Some(&format!("{label}/{entry}")),
                    spirv: Some(std::borrow::Cow::Owned(words)),
                    ..Default::default()
                })
        };
        created.insert(entry.clone(), Arc::new(shader));
    }
    Ok(created)
}

/// The Vulkan backend numbers a set's bindings in binding order and sizes each
/// binding array from its layout entry; a SPIR-V module written outside it has
/// to carry the same numbering.
pub(super) fn spirv_binding_map(groups: &[&BindGroupLayoutDescriptor]) -> naga::back::spv::BindingMap {
    let mut map = naga::back::spv::BindingMap::default();
    for (group, layout) in groups.iter().enumerate() {
        let mut bindings: Vec<_> = layout
            .entries
            .iter()
            .map(|entry| (entry.binding, entry.count))
            .collect();
        bindings.sort_unstable_by_key(|(binding, _)| *binding);
        for (vk_binding, (binding, count)) in bindings.into_iter().enumerate() {
            map.insert(
                naga::ResourceBinding {
                    group: group as u32,
                    binding,
                },
                naga::back::spv::BindingInfo {
                    descriptor_set: group as u32,
                    binding: vk_binding as u32,
                    binding_array_size: count.map(std::num::NonZeroU32::get),
                },
            );
        }
    }
    map
}
