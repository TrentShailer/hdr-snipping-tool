use super::Renderer;

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.capture.drop(&self.device);
            self.selection.drop(&self.device);
            self.mouse_guides.drop(&self.device);

            self.descriptor_layouts
                .iter()
                .for_each(|&layout| self.device.destroy_descriptor_set_layout(layout, None));
            self.shaders
                .iter()
                .for_each(|&shader| self.device.destroy_shader_module(shader, None));
            self.pipeline_layouts
                .iter()
                .for_each(|&layout| self.device.destroy_pipeline_layout(layout, None));
            self.pipelines
                .iter()
                .for_each(|&pipeline| self.device.destroy_pipeline(pipeline, None));

            self.cleanup_swapchain();
        }
    }
}
