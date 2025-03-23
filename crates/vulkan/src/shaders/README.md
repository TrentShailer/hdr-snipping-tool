# Shaders

## Compiling SPIR-V

```powershell
slangc `
    -target spirv `
    -profile spirv_1_4 `
    -fvk-use-scalar-layout `
    crates/vulkan/src/shaders/slang/maximum_reduction.slang `
    -o crates/vulkan/src/shaders/spv/maximum_reduction.spv

slangc `
    -target spirv `
    -profile spirv_1_4 `
    -fvk-use-scalar-layout `
    crates/vulkan/src/shaders/slang/render_capture.slang `
    -o crates/vulkan/src/shaders/spv/render_capture.spv

slangc `
    -target spirv `
    -profile spirv_1_4 `
    -fvk-use-scalar-layout `
    crates/vulkan/src/shaders/slang/render_line.slang `
    -o crates/vulkan/src/shaders/spv/render_line.spv
    
slangc `
    -target spirv `
    -profile spirv_1_4 `
    -fvk-use-scalar-layout `
    crates/vulkan/src/shaders/slang/render_selection.slang `
    -o crates/vulkan/src/shaders/spv/render_selection.spv

slangc `
    -target spirv `
    -profile spirv_1_4 `
    -fvk-use-scalar-layout `
    crates/vulkan/src/shaders/slang/tonemap_hdr_to_sdr.slang `
    -o crates/vulkan/src/shaders/spv/tonemap_hdr_to_sdr.spv
```

## Generating Bindings

```console
rspirv-bindgen crates/vulkan/src/shaders/spv/ -o crates/vulkan/src/shaders/mod.rs
```