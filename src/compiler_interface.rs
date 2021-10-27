use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    thread,
    time::Duration,
};

use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use egui::{Align, CtxRef, Direction, FontDefinitions, Layout};
use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};

use crate::{
    code_editor::code_editor_ui,
    compiler::{init_compiler_thread, CompiledDSPPayload, CompiledUIPayload, DEFAULT_CODE},
    correlation_match::display::DisplayBuffer,
    graphs::graphs_ui,
    sarus_egui_lib::DebuggerOutput,
    SarusSharedState,
};

use triple_buffer::{Input, Output, TripleBuffer};

pub struct WaveformDisplay {
    pub buffer: DisplayBuffer,
    pub display_decay: f32,
    pub memory_decay: f32,
    pub enable_waveform: bool,
    pub enable_smoothing: bool,
    pub offset: usize,
}

pub struct CompilerEditorState {
    pub code: String,
    pub line_numbers: String,
    pub errors: String,
    pub file_saved: bool,
    pub errors_buf_out: Arc<Mutex<triple_buffer::Output<String>>>,
    pub shared_ctx: Arc<SarusSharedState>,
    pub debug_out: Arc<Mutex<DebuggerOutput>>,
    pub waveforms: Vec<WaveformDisplay>,
    pub last_project_float_id: u64,
    pub new_file_name: Option<String>,
    pub compile_on_save: bool,
    pub file_name: String,
}

pub fn setup_fonts(ctx: &CtxRef) {
    let mut fonts = FontDefinitions::default();

    for (_text_style, (_family, size)) in fonts.family_and_size.iter_mut() {
        *size = 25.0;
    }
    ctx.set_fonts(fonts);
}

pub fn init_compiler_editor_thread(
    ui_payload_in: Input<Option<CompiledUIPayload>>,
    dsp_payload_in: Input<Option<CompiledDSPPayload>>,
    debug_out: DebuggerOutput,
    shared_ctx: Arc<SarusSharedState>,
) {
    let errors_buffer = TripleBuffer::new(String::new());
    let (errors_buf_in, errors_buf_out) = errors_buffer.split();

    let errors_buf_out = Arc::new(Mutex::new(errors_buf_out));

    let debug_out = Arc::new(Mutex::new(debug_out));

    init_compiler_thread(
        errors_buf_in,
        ui_payload_in,
        dsp_payload_in,
        shared_ctx.clone(),
    );

    init_code_editor_thread(errors_buf_out, debug_out, shared_ctx);
}

fn init_code_editor_thread(
    errors_buf_out: Arc<Mutex<Output<String>>>,
    debug_out: Arc<Mutex<DebuggerOutput>>,
    shared_ctx: Arc<SarusSharedState>,
) {
    thread::spawn(move || {
        loop {
            if shared_ctx.code_editor_is_open.load(Ordering::Relaxed) {
                {
                    let settings = Settings {
                        window: WindowOpenOptions {
                            title: String::from("egui-baseplug-examples gain"),
                            size: Size::new(1800.0, 1600.0),
                            scale: WindowScalePolicy::SystemScaleFactor,
                        },
                        render_settings: RenderSettings::default(),
                    };

                    let mut waveforms = Vec::new();
                    for _ in 0..4 {
                        waveforms.push(WaveformDisplay {
                            buffer: DisplayBuffer::new(1024, 768),
                            display_decay: 0.6,
                            memory_decay: 0.8,
                            enable_waveform: true,
                            enable_smoothing: false,
                            offset: 0,
                        });
                    }

                    EguiWindow::open_blocking(
                        settings,
                        CompilerEditorState {
                            code: DEFAULT_CODE.to_owned(),
                            errors: String::new(),
                            line_numbers: String::new(),
                            errors_buf_out: errors_buf_out.clone(),
                            shared_ctx: shared_ctx.clone(),
                            debug_out: debug_out.clone(),
                            file_saved: true,
                            waveforms,
                            last_project_float_id: 0,
                            new_file_name: None,
                            compile_on_save: true,
                            file_name: "".to_string(),
                        },
                        // Called once before the first frame. Allows you to do setup code and to
                        // call `ctx.set_fonts()`. Optional.
                        |ctx: &CtxRef,
                         _queue: &mut Queue,
                         _editor_state: &mut CompilerEditorState| {
                            setup_fonts(ctx);
                            let mut style: egui::Style = (*ctx.style()).clone();
                            style.spacing.interact_size = egui::vec2(40.0, 40.0);
                            style.spacing.slider_width = 250.0;
                            ctx.set_style(style);
                        },
                        // Called before each frame. Here you should update the state of your
                        // application and build the UI.
                        |ctx: &CtxRef, _queue: &mut Queue, state: &mut CompilerEditorState| {
                            egui::SidePanel::left("Debug")
                                .default_width(500.0)
                                .show(ctx, |ui| {
                                    let layout = Layout::from_main_dir_and_cross_align(
                                        Direction::TopDown,
                                        Align::Center,
                                    )
                                    .with_cross_justify(true);
                                    ui.checkbox(&mut state.compile_on_save, "Compile On Save");
                                    ui.with_layout(layout, |ui| {
                                        if ui.button("COMPILE").clicked() {
                                            state.errors = String::from("");
                                            state
                                                .shared_ctx
                                                .trigger_compile
                                                .store(true, Ordering::Relaxed);
                                        }
                                    });
                                    graphs_ui(ui, state)
                                });
                            egui::CentralPanel::default().show(ctx, |ui| {
                                code_editor_ui(ui, state);
                            });

                            ctx.request_repaint();
                        },
                    );
                }
                shared_ctx
                    .code_editor_is_open
                    .store(false, Ordering::Relaxed)
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
}
