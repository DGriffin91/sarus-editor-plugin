#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(min_specialization)]

use baseplug::{Model, Plugin, PluginContext, ProcessContext, UIFloatParam, WindowOpenResult};
use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use raw_window_handle::HasRawWindowHandle;
use ringbuf::RingBuffer;
use sarus_egui_lib::{DebuggerInput, DebuggerOutput};
use serde::{Deserialize, Serialize};

use egui::{Align, CtxRef, Direction, Layout};
use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};
use triple_buffer::{Output, TripleBuffer};
use units::ConsumerRingBuf;

use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use compiler::{AudioData, CompiledDSPPayload, CompiledUIPayload};
use compiler_interface::setup_fonts;

pub mod code_editor;
pub mod compiler;
pub mod compiler_interface;
pub mod correlation_match;
mod float_id;
pub mod graphs;
pub mod heap_data;
pub mod logging;
mod preset_manager;
pub mod units;

use logging::init_logging;

use std::sync::Arc;

pub mod highligher;
pub mod sarus_egui_lib;

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SarusPluginModel {
        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 1", unit = "Generic",
            gradient = "Linear")]
        pub param1: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 2", unit = "Generic",
            gradient = "Linear")]
        pub param2: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 3", unit = "Generic",
            gradient = "Linear")]
        pub param3: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 4", unit = "Generic",
            gradient = "Linear")]
        pub param4: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 5", unit = "Generic",
            gradient = "Linear")]
        pub param5: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 6", unit = "Generic",
            gradient = "Linear")]
        pub param6: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 7", unit = "Generic",
            gradient = "Linear")]
        pub param7: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 8", unit = "Generic",
            gradient = "Linear")]
        pub param8: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 9", unit = "Generic",
            gradient = "Linear")]
        pub param9: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 10", unit = "Generic",
            gradient = "Linear")]
        pub param10: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 11", unit = "Generic",
            gradient = "Linear")]
        pub param11: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 12", unit = "Generic",
            gradient = "Linear")]
        pub param12: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 13", unit = "Generic",
            gradient = "Linear")]
        pub param13: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 14", unit = "Generic",
            gradient = "Linear")]
        pub param14: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 15", unit = "Generic",
            gradient = "Linear")]
        pub param15: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Parameter 16", unit = "Generic",
            gradient = "Linear")]
        pub param16: f32,

        #[model(min = -90.0, max = 6.0)]
        #[parameter(name = "Master Gain", unit = "Decibels",
            gradient = "Power(0.15)")]
        pub gain_master: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "ID1", unit = "Generic", smoothing = false,
            gradient = "Linear")]
        pub id1: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "ID2", unit = "Generic", smoothing = false,
            gradient = "Linear")]
        pub id2: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "ID3", unit = "Generic", smoothing = false,
            gradient = "Linear")]
        pub id3: f32,
    }
}

impl Default for SarusPluginModel {
    fn default() -> Self {
        Self {
            // "gain" is converted from dB to coefficient in the parameter handling code,
            // so in the model here it's a coefficient.
            // -0dB == 1.0
            param1: 0.0,
            param2: 0.0,
            param3: 0.0,
            param4: 0.0,
            param5: 0.0,
            param6: 0.0,
            param7: 0.0,
            param8: 0.0,
            param9: 0.0,
            param10: 0.0,
            param11: 0.0,
            param12: 0.0,
            param13: 0.0,
            param14: 0.0,
            param15: 0.0,
            param16: 0.0,
            gain_master: 1.0,
            id1: 0.0,
            id2: 0.0,
            id3: 0.0,
        }
    }
}

//extern "C"  {
//    fn sarus_ui(ui: &mut Ui, data:&mut [f32; 4]);
//}

pub struct SarusPluginShared {
    code_editor_is_open: Arc<AtomicBool>,
    trigger_compile: Arc<AtomicBool>,
    ui_payload_out: Arc<Mutex<Output<Option<CompiledUIPayload>>>>,
    dsp_payload_out: Arc<RefCell<Output<Option<CompiledDSPPayload>>>>,
    debug_in: Arc<RefCell<DebuggerInput>>,
}

unsafe impl Send for SarusPluginShared {}
unsafe impl Sync for SarusPluginShared {}

impl PluginContext<SarusPlugin> for SarusPluginShared {
    fn new() -> Self {
        init_logging("SarusEditorPlugin.log");

        let code_editor_is_open = Arc::new(AtomicBool::new(false));
        let trigger_compile = Arc::new(AtomicBool::new(false));

        let ui_payload_buffer: TripleBuffer<Option<CompiledUIPayload>> = TripleBuffer::new(None);
        let (ui_payload_in, ui_payload_out) = ui_payload_buffer.split();

        let dsp_payload_buffer: TripleBuffer<Option<CompiledDSPPayload>> = TripleBuffer::new(None);
        let (dsp_payload_in, dsp_payload_out) = dsp_payload_buffer.split();

        let ui_payload_out = Arc::new(Mutex::new(ui_payload_out));
        let dsp_payload_out = Arc::new(RefCell::new(dsp_payload_out));

        let mut producers = Vec::new();
        let mut consumers = Vec::new();
        for _ in 0..4 {
            let (prod, cons) = RingBuffer::<f32>::new(1024).split();
            producers.push(prod);
            consumers.push(ConsumerRingBuf::new(cons, 1024));
        }

        //TODO share float is as atomic f32s between process, editor, and compiler
        //do Projects::load() here

        compiler_interface::init_compiler_editor_thread(
            code_editor_is_open.clone(),
            trigger_compile.clone(),
            ui_payload_in,
            dsp_payload_in,
            DebuggerOutput { consumers },
        );

        Self {
            code_editor_is_open,
            trigger_compile,
            ui_payload_out,
            dsp_payload_out,
            debug_in: Arc::new(RefCell::new(DebuggerInput { producers })),
        }
    }
}

pub struct SarusPlugin {
    sample_rate: f32,
}

impl Plugin for SarusPlugin {
    const NAME: &'static str = "Sarus Editor Plugin";
    const PRODUCT: &'static str = "Sarus Editor Plugin";
    const VENDOR: &'static str = "DGriffin";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = SarusPluginModel;
    type PluginContext = SarusPluginShared;

    #[inline]
    fn new(sample_rate: f32, _model: &SarusPluginModel, _shared: &SarusPluginShared) -> Self {
        Self { sample_rate }
    }

    #[inline]
    fn process(
        &mut self,
        model: &SarusPluginModelProcess,
        ctx: &mut ProcessContext<Self>,
        shared: &SarusPluginShared,
    ) {
        let mut dsp_payload_borrow = shared.dsp_payload_out.borrow_mut();
        let dsp_payload = dsp_payload_borrow.read();

        let mut debug_in_borrow = shared.debug_in.borrow_mut();

        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;
        if let Some(dsp_payload) = dsp_payload {
            let mut sarus_params = SarusDSPModelParams::from_dsp_model(model);
            let mut audio_data = AudioData {
                in_left: input[0].as_ptr(),
                in_right: input[1].as_ptr(),
                out_left: output[0].as_mut_ptr(),
                out_right: output[1].as_mut_ptr(),
                len: ctx.nframes as i64,
                sample_rate: self.sample_rate,
            };
            (dsp_payload.process_func)(
                &mut sarus_params,
                &mut audio_data,
                dsp_payload.process_data.get_ptr(),
                &mut debug_in_borrow,
            );

            for i in 0..ctx.nframes {
                output[0][i] = output[0][i] * model.gain_master[i];
                output[1][i] = output[1][i] * model.gain_master[i];
            }
        } else {
            for i in 0..ctx.nframes {
                output[0][i] = input[0][i] * model.gain_master[i];
                output[1][i] = input[1][i] * model.gain_master[i];
            }
        }
    }
}

pub fn param_slider(
    ui: &mut egui::Ui,
    label: &str,
    value_text: &mut String,
    param: &mut UIFloatParam<SarusPluginModel, SarusPluginModelSmooth>,
) {
    ui.label(label);

    // Use the normalized value of the param so we can take advantage of baseplug's value curves.
    //
    // You could opt to use your own custom widget if you wish, as long as it can operate with
    // a normalized range from [0.0, 1.0].
    let mut normal = param.normalized();
    if ui
        .add(
            egui::Slider::new(&mut normal, 0.0..=1.0)
                .show_value(false)
                .text(&value_text),
        )
        .changed()
    {
        param.set_from_normalized(normal);
        format_value(value_text, param);
    };
}

pub fn format_value(
    value_text: &mut String,
    param: &UIFloatParam<SarusPluginModel, SarusPluginModelSmooth>,
) {
    *value_text = format!("{:.1} {}", param.unit_value(), param.unit_label());
}

impl baseplug::PluginUI for SarusPlugin {
    type Handle = ();

    fn ui_size() -> (i16, i16) {
        (700, 700)
    }

    fn ui_open(
        parent: &impl HasRawWindowHandle,
        shared_ctx: &SarusPluginShared,
        model: <Self::Model as Model<Self>>::UI,
    ) -> WindowOpenResult<Self::Handle> {
        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("egui-baseplug-examples gain"),
                size: Size::new(Self::ui_size().0 as f64, Self::ui_size().1 as f64),
                scale: WindowScalePolicy::SystemScaleFactor,
            },
            render_settings: RenderSettings::default(),
        };

        EguiWindow::open_parented(
            parent,
            settings,
            PluginEditorState {
                state: EditorModelState::new(model),
                code_editor_is_open: shared_ctx.code_editor_is_open.clone(),
                trigger_compile: shared_ctx.trigger_compile.clone(),
                ui_payload_out: shared_ctx.ui_payload_out.clone(),
            },
            // Called once before the first frame. Allows you to do setup code and to
            // call `ctx.set_fonts()`. Optional.
            |ctx: &CtxRef, _queue: &mut Queue, _editor_state: &mut PluginEditorState| {
                setup_fonts(ctx);
                let mut style: egui::Style = (*ctx.style()).clone();
                style.spacing.interact_size = egui::vec2(40.0, 40.0);
                style.spacing.slider_width = 300.0;
                ctx.set_style(style);
            },
            // Called before each frame. Here you should update the state of your
            // application and build the UI.
            |ctx: &CtxRef, _queue: &mut Queue, editor_state: &mut PluginEditorState| {
                // Must be called on the top of each frame in order to sync values from the rt thread.

                egui::CentralPanel::default().show(ctx, |ui| {
                    let layout =
                        Layout::from_main_dir_and_cross_align(Direction::TopDown, Align::LEFT)
                            .with_cross_justify(true);
                    ui.with_layout(layout, |ui| {
                        if ui.button("Open Editor").clicked() {
                            editor_state
                                .code_editor_is_open
                                .store(true, Ordering::Relaxed);
                        }

                        if ui.button("Compile").clicked() {
                            editor_state.trigger_compile.store(true, Ordering::Relaxed);
                        }
                        ui.separator();

                        let state = &mut editor_state.state;

                        // Sync text values if there was automation.

                        format_value(&mut state.gain_master_value, &state.model.gain_master);
                        if let Some(compiled_payload) =
                            editor_state.ui_payload_out.lock().unwrap().read()
                        {
                            let mut sarus_params = SarusUIModelParams::from_ui_model(&state.model);
                            (compiled_payload.editor_func)(
                                ui,
                                &mut sarus_params,
                                compiled_payload.editor_data.get_ptr(),
                            );
                            sarus_params.to_model(&mut state.model);
                        }
                        ui.separator();
                        param_slider(
                            ui,
                            "Gain Master",
                            &mut state.gain_master_value,
                            &mut state.model.gain_master,
                        );
                    });
                });

                ctx.request_repaint();
            },
        );

        Ok(())
    }

    fn ui_close(mut _handle: Self::Handle, _ctx: &SarusPluginShared) {
        // TODO: Close window once baseview gets the ability to do this.
    }

    fn ui_key_down(_plug_ctx: &Self::PluginContext, _ev: keyboard_types::KeyboardEvent) -> bool {
        true
    }

    fn ui_key_up(_plug_ctx: &Self::PluginContext, _ev: keyboard_types::KeyboardEvent) -> bool {
        true
    }

    fn ui_param_notify(
        _handle: &Self::Handle,
        _param: &'static baseplug::Param<
            Self,
            <Self::Model as Model<Self>>::Smooth,
            <Self as baseplug::PluginUI>::Handle,
        >,
        _val: f32,
    ) {
    }
}

pub struct PluginEditorState {
    state: EditorModelState,
    code_editor_is_open: Arc<AtomicBool>,
    trigger_compile: Arc<AtomicBool>,
    ui_payload_out: Arc<Mutex<Output<Option<CompiledUIPayload>>>>,
}

pub struct EditorModelState {
    pub model: SarusPluginModelUI<SarusPlugin>,
    pub gain_master_value: String,
}

impl EditorModelState {
    pub fn new(model: SarusPluginModelUI<SarusPlugin>) -> EditorModelState {
        EditorModelState {
            model,
            gain_master_value: String::new(),
        }
    }
}

//TODO try to get sarus to be able to take the whole model directly
//(this may be an issue without repr(C))
#[repr(C)]
pub struct SarusUIModelParams {
    pub param1: f32,
    pub param2: f32,
    pub param3: f32,
    pub param4: f32,
    pub param5: f32,
    pub param6: f32,
    pub param7: f32,
    pub param8: f32,
    pub param9: f32,
    pub param10: f32,
    pub param11: f32,
    pub param12: f32,
    pub param13: f32,
    pub param14: f32,
    pub param15: f32,
    pub param16: f32,
}

impl SarusUIModelParams {
    fn from_ui_model(model: &SarusPluginModelUI<SarusPlugin>) -> Self {
        SarusUIModelParams {
            param1: model.param1.normalized(),
            param2: model.param2.normalized(),
            param3: model.param3.normalized(),
            param4: model.param4.normalized(),
            param5: model.param5.normalized(),
            param6: model.param6.normalized(),
            param7: model.param7.normalized(),
            param8: model.param8.normalized(),
            param9: model.param9.normalized(),
            param10: model.param10.normalized(),
            param11: model.param11.normalized(),
            param12: model.param12.normalized(),
            param13: model.param13.normalized(),
            param14: model.param14.normalized(),
            param15: model.param15.normalized(),
            param16: model.param16.normalized(),
        }
    }
    fn to_model(&self, model: &mut SarusPluginModelUI<SarusPlugin>) {
        model.param1.set_from_normalized(self.param1);
        model.param2.set_from_normalized(self.param2);
        model.param3.set_from_normalized(self.param3);
        model.param4.set_from_normalized(self.param4);
        model.param5.set_from_normalized(self.param5);
        model.param6.set_from_normalized(self.param6);
        model.param7.set_from_normalized(self.param7);
        model.param8.set_from_normalized(self.param8);
        model.param9.set_from_normalized(self.param9);
        model.param10.set_from_normalized(self.param10);
        model.param11.set_from_normalized(self.param11);
        model.param12.set_from_normalized(self.param12);
        model.param13.set_from_normalized(self.param13);
        model.param14.set_from_normalized(self.param14);
        model.param15.set_from_normalized(self.param15);
        model.param16.set_from_normalized(self.param16);
    }
}

#[repr(C)]
pub struct SarusDSPModelParams {
    pub param1: *const f32,
    pub param2: *const f32,
    pub param3: *const f32,
    pub param4: *const f32,
    pub param5: *const f32,
    pub param6: *const f32,
    pub param7: *const f32,
    pub param8: *const f32,
    pub param9: *const f32,
    pub param10: *const f32,
    pub param11: *const f32,
    pub param12: *const f32,
    pub param13: *const f32,
    pub param14: *const f32,
    pub param15: *const f32,
    pub param16: *const f32,
}

impl SarusDSPModelParams {
    fn from_dsp_model(model: &SarusPluginModelProcess) -> Self {
        //scary that no lifetime is needed
        SarusDSPModelParams {
            param1: model.param1.values.as_ptr(),
            param2: model.param2.values.as_ptr(),
            param3: model.param3.values.as_ptr(),
            param4: model.param4.values.as_ptr(),
            param5: model.param5.values.as_ptr(),
            param6: model.param6.values.as_ptr(),
            param7: model.param7.values.as_ptr(),
            param8: model.param8.values.as_ptr(),
            param9: model.param9.values.as_ptr(),
            param10: model.param10.values.as_ptr(),
            param11: model.param11.values.as_ptr(),
            param12: model.param12.values.as_ptr(),
            param13: model.param13.values.as_ptr(),
            param14: model.param14.values.as_ptr(),
            param15: model.param15.values.as_ptr(),
            param16: model.param16.values.as_ptr(),
        }
    }
}

#[cfg(not(test))]
baseplug::vst2!(SarusPlugin, b"SaRu");
