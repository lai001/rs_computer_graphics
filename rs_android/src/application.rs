use crate::enviroment::Enviroment;
use crate::error::Result;
use crate::motion_event;
use rs_artifact::artifact::{ArtifactFileHeader, ArtifactReader};
use rs_artifact::java_input_stream::JavaInputStream;
use rs_artifact::{
    file_header::{FileHeader, ARTIFACT_FILE_MAGIC_NUMBERS},
    EEndianType,
};

pub struct Application {
    native_window: crate::native_window::NativeWindow,
    raw_input: egui::RawInput,
    scale_factor: f32,
    enviroment: Option<Enviroment>,
    engine: rs_engine::engine::Engine,
    gui_context: egui::Context,
}

impl Application {
    pub fn from_native_window(
        native_window: crate::native_window::NativeWindow,
        artifact_input_stream: JavaInputStream,
    ) -> Result<Application> {
        let scale_factor = 1.0f32;

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::default(),
                egui::vec2(
                    native_window.get_width() as f32,
                    native_window.get_height() as f32,
                ) / scale_factor as f32,
            )),
            ..Default::default()
        };
        let artifact_reader = ArtifactReader::new(artifact_input_stream, Some(EEndianType::Little))
            .map_err(|err| crate::error::Error::Artifact(err))?;

        let width = native_window.get_width();
        let height = native_window.get_height();
        let gui_context = egui::Context::default();
        let engine = rs_engine::engine::Engine::new(
            &native_window,
            width,
            height,
            scale_factor,
            Some(artifact_reader),
            std::collections::HashMap::new(),
        )
        .map_err(|err| crate::error::Error::Engine(err))?;
        Ok(Application {
            native_window,
            raw_input,
            scale_factor,
            enviroment: None,
            engine,
            gui_context,
        })
    }

    pub fn redraw(&mut self) {
        let context = &self.gui_context;
        context.begin_frame(self.raw_input.clone());

        egui::Window::new("Pannel")
            .default_pos((200.0, 200.0))
            .show(&context, |ui| {
                let response = ui.button("Button");
                if response.clicked() {}
                if ui.button("Button2").clicked() {}
                ui.label(format!("Time: {:.2}", 0.0f32));
            });

        let full_output = context.end_frame();
        let gui_render_output = rs_render::egui_render::EGUIRenderOutput {
            textures_delta: full_output.textures_delta,
            clipped_primitives: context
                .tessellate(full_output.shapes, full_output.pixels_per_point),
        };
        self.engine.redraw(gui_render_output);
    }

    pub fn get_status_bar_height(&self) -> i32 {
        let status_bar_height = {
            if let Some(ref enviroment) = self.enviroment {
                enviroment.status_bar_height
            } else {
                0
            }
        };
        status_bar_height
    }

    pub fn set_new_window(
        &mut self,
        native_window: &crate::native_window::NativeWindow,
    ) -> Result<()> {
        let surface_width = native_window.get_width();
        let surface_height = native_window.get_height();
        self.engine
            .set_new_window(native_window, surface_width, surface_height)
            .map_err(|err| crate::error::Error::Engine(err))?;
        Ok(())
    }
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_fromSurface(
    mut env: jni::JNIEnv,
    _: jni::objects::JClass,
    surface: jni::sys::jobject,
    artifact_input_stream: jni::objects::JObject,
) -> *mut Application {
    debug_assert_ne!(surface, std::ptr::null_mut());
    let logger = rs_engine::logger::Logger::new(rs_engine::logger::LoggerConfiguration {
        is_write_to_file: false,
    });
    let result: crate::error::Result<*mut Application> = (|| {
        let native_window = crate::native_window::NativeWindow::new(&mut env, surface)
            .ok_or(crate::error::Error::NativeWindowNull)?;
        let mut artifact_input_stream = JavaInputStream::new(env, artifact_input_stream)
            .map_err(|_| crate::error::Error::JavaInputStreamNull)?;
        FileHeader::check_identification(&mut artifact_input_stream, ARTIFACT_FILE_MAGIC_NUMBERS)
            .map_err(|err| crate::error::Error::CheckIdentificationFail(err))?;

        let header: ArtifactFileHeader =
            FileHeader::get_header2(&mut artifact_input_stream, Some(EEndianType::Little))
                .map_err(|err| crate::error::Error::Artifact(err))?;
        let application = Application::from_native_window(native_window, artifact_input_stream)?;
        Ok(Box::into_raw(Box::new(application)))
    })();
    match result {
        Ok(application) => application,
        Err(err) => {
            log::warn!("{}", err);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_setNewSurface(
    mut env: jni::JNIEnv,
    _: jni::objects::JClass,
    application: *mut Application,
    surface: jni::sys::jobject,
) -> jni::sys::jboolean {
    debug_assert_ne!(application, std::ptr::null_mut());
    debug_assert_ne!(surface, std::ptr::null_mut());
    let native_window = crate::native_window::NativeWindow::new(&mut env, surface);
    if let Some(native_window) = native_window {
        let mut application: Box<Application> = unsafe { Box::from_raw(application) };
        application.set_new_window(&native_window);
        Box::into_raw(Box::new(application));
        jni::sys::JNI_TRUE
    } else {
        jni::sys::JNI_FALSE
    }
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_drop(_: jni::JNIEnv, _: jni::objects::JClass, application: *mut Application) {
    debug_assert_ne!(application, std::ptr::null_mut());
    let _: Box<Application> = unsafe { Box::from_raw(application) };
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_redraw(_: jni::JNIEnv, _: jni::objects::JClass, application: *mut Application) {
    debug_assert_ne!(application, std::ptr::null_mut());
    let mut application: Box<Application> = unsafe { Box::from_raw(application) };
    application.redraw();
    application.raw_input.events.clear();
    Box::into_raw(Box::new(application));
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_onTouchEvent(
    mut env: jni::JNIEnv,
    _: jni::objects::JClass,
    application: *mut Application,
    event: jni::objects::JClass,
) -> jni::sys::jboolean {
    debug_assert_ne!(application, std::ptr::null_mut());

    let mut motion_event = motion_event::MotionEvent::new(env, event);
    let mut application: Box<Application> = unsafe { Box::from_raw(application) };
    let status_bar_height = application.get_status_bar_height();

    let raw_input = &mut application.raw_input;

    let phase: egui::TouchPhase = {
        if motion_event.get_action() == 3 {
            egui::TouchPhase::Cancel
        } else if motion_event.get_action() == 0 {
            egui::TouchPhase::Start
        } else if motion_event.get_action() == 2 {
            egui::TouchPhase::Move
        } else if motion_event.get_action() == 1 {
            egui::TouchPhase::End
        } else {
            egui::TouchPhase::End
        }
    };
    let pointer_pos = egui::pos2(
        (motion_event.get_x() as f32) / application.scale_factor,
        (motion_event.get_y() as f32 - status_bar_height as f32) / application.scale_factor,
    );
    match phase {
        egui::TouchPhase::Start => {
            raw_input.events.push(egui::Event::PointerButton {
                pos: pointer_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            });
        }
        egui::TouchPhase::Move => {
            raw_input
                .events
                .push(egui::Event::PointerMoved(pointer_pos));
        }
        egui::TouchPhase::End => {
            raw_input.events.push(egui::Event::PointerButton {
                pos: pointer_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            });
            raw_input.events.push(egui::Event::PointerGone);
        }
        egui::TouchPhase::Cancel => {}
    }

    Box::into_raw(Box::new(application));
    return jni::sys::JNI_TRUE;
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_surfaceChanged(
    _: jni::JNIEnv,
    _: jni::objects::JClass,
    application: *mut Application,
    _: jni::sys::jint,
    w: jni::sys::jint,
    h: jni::sys::jint,
) {
    debug_assert_ne!(application, std::ptr::null_mut());

    // let format = ndk_sys::AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM.0;
    let mut application: Box<Application> = unsafe { Box::from_raw(application) };
    application.raw_input.screen_rect = Some(egui::Rect::from_min_size(
        egui::Pos2::default(),
        egui::vec2(w as f32, h as f32) / application.scale_factor as f32,
    ));

    application.native_window.set_buffers_geometry(
        w as u32,
        h as u32,
        application.native_window.get_format(),
    );
    Box::into_raw(Box::new(application));
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_surfaceDestroyed(
    _: jni::JNIEnv,
    _: jni::objects::JClass,
    application: *mut Application,
    surface: jni::sys::jobject,
) {
}

#[no_mangle]
#[jni_fn::jni_fn("com.lai001.rs_android.Application")]
pub fn Application_setEnvironment(
    mut env: jni::JNIEnv,
    _: jni::objects::JClass,
    application: *mut Application,
    mut android_enviroment: jni::objects::JClass,
) {
    debug_assert_ne!(application, std::ptr::null_mut());

    let mut application: Box<Application> = unsafe { Box::from_raw(application) };
    application.enviroment = Some(Enviroment::new(&mut env, &mut android_enviroment));
    Box::into_raw(Box::new(application));
}
