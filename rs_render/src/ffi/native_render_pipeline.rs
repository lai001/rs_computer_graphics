use wgpu::RenderPipeline;

#[repr(C)]
#[derive(Debug)]
pub struct NativeWGPURenderPipelineFunctions {
    pub native_render_pipeline_delete: *mut std::ffi::c_void,
}

impl NativeWGPURenderPipelineFunctions {
    pub fn new() -> NativeWGPURenderPipelineFunctions {
        NativeWGPURenderPipelineFunctions {
            native_render_pipeline_delete: nativeRenderPipelineDelete as *mut std::ffi::c_void,
        }
    }
}

#[no_mangle]
pub extern "C" fn nativeRenderPipelineDelete(native_object: *mut RenderPipeline) {
    if !native_object.is_null() {
        log::trace!(
            "nativeRenderPipelineDelete(native_object: {:?})",
            native_object
        );
        unsafe {
            let _ = Box::from_raw(native_object);
        };
    }
}
