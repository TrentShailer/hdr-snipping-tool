use std::io::Cursor;

use ash::{
    util::read_spv,
    vk::{ShaderModule, ShaderModuleCreateInfo},
};
use tracing::instrument;

use crate::{VulkanError, VulkanInstance};

impl VulkanInstance {
    #[instrument("VulkanInstance::create_shader_module", skip_all, err)]
    pub fn create_shader_module(&self, bytes: &[u8]) -> Result<ShaderModule, VulkanError> {
        let mut shader_file = Cursor::new(bytes);
        let shader_code = read_spv(&mut shader_file).unwrap();

        let shader_info = ShaderModuleCreateInfo::default().code(&shader_code);

        let shader_module = unsafe {
            self.device
                .create_shader_module(&shader_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating shader module"))?
        };

        Ok(shader_module)
    }
}
