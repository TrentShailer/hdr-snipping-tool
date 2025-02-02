## Generating Vulkan Profiles
```powershell
python $Env:VULKAN_SDK/share/vulkan/registry/gen_profiles_solution.py `
    --registry $Env:VULKAN_SDK/share/vulkan/registry/vk.xml `
    --input crates/vulkan/vulkan_profiles/profiles `
    --output-library-src crates/vulkan/vulkan_profiles `
    --output-library-inc crates/vulkan/vulkan_profiles/vulkan
```
