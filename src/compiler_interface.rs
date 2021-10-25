use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use egui::{mutex::Mutex, Align, CtxRef, Direction, FontDefinitions, FontFamily, Layout};
use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};

use crate::{
    code_editor::code_editor_ui,
    compiler::{init_compiler_thread, CompiledDSPPayload, CompiledUIPayload, DEFAULT_CODE},
    correlation_match::display::DisplayBuffer,
    graphs::graphs_ui,
    highligher::MemoizedSyntaxHighlighter,
    sarus_egui_lib::DebuggerOutput,
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
    pub current_file: String,
    pub file_saved: bool,
    pub highlighter: MemoizedSyntaxHighlighter,
    pub code_buf_in: Arc<Mutex<triple_buffer::Input<String>>>,
    pub errors_buf_out: Arc<Mutex<triple_buffer::Output<String>>>,
    pub trigger_compile: Arc<AtomicBool>,
    pub debug_out: Arc<Mutex<DebuggerOutput>>,
    pub waveforms: Vec<WaveformDisplay>,
}

pub fn setup_fonts(ctx: &CtxRef) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "FiraCode".to_owned(),
        std::borrow::Cow::Borrowed(include_bytes!("../resources/FiraCode-Regular.ttf")),
    );

    fonts
        .fonts_for_family
        .get_mut(&FontFamily::Monospace)
        .unwrap()[0] = "FiraCode".to_owned();

    for (_text_style, (_family, size)) in fonts.family_and_size.iter_mut() {
        *size = 25.0;
    }
    ctx.set_fonts(fonts);
}

pub fn init_compiler_editor_thread(
    code_editor_is_open: Arc<AtomicBool>,
    trigger_compile: Arc<AtomicBool>,
    ui_payload_in: Input<Option<CompiledUIPayload>>,
    dsp_payload_in: Input<Option<CompiledDSPPayload>>,
    debug_out: DebuggerOutput,
) {
    let code_buffer = TripleBuffer::new(DEFAULT_CODE.to_owned());
    let (code_buf_in, code_buf_out) = code_buffer.split();

    let errors_buffer = TripleBuffer::new(String::new());
    let (errors_buf_in, errors_buf_out) = errors_buffer.split();

    let code_buf_in = Arc::new(Mutex::new(code_buf_in));
    let errors_buf_out = Arc::new(Mutex::new(errors_buf_out));

    let debug_out = Arc::new(Mutex::new(debug_out));

    init_compiler_thread(
        code_buf_out,
        errors_buf_in,
        trigger_compile.clone(),
        ui_payload_in,
        dsp_payload_in,
    );

    init_code_editor_thread(
        code_editor_is_open,
        code_buf_in,
        errors_buf_out,
        trigger_compile,
        debug_out,
    );
}

fn init_code_editor_thread(
    code_editor_is_open: Arc<AtomicBool>,
    code_buf_in: Arc<Mutex<Input<String>>>,
    errors_buf_out: Arc<Mutex<Output<String>>>,
    trigger_compile: Arc<AtomicBool>,
    debug_out: Arc<Mutex<DebuggerOutput>>,
) {
    thread::spawn(move || {
        loop {
            if code_editor_is_open.load(Ordering::Relaxed) {
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
                            current_file: String::new(),
                            highlighter: MemoizedSyntaxHighlighter::default(),
                            code_buf_in: code_buf_in.clone(),
                            errors_buf_out: errors_buf_out.clone(),
                            trigger_compile: trigger_compile.clone(),
                            debug_out: debug_out.clone(),
                            file_saved: true,
                            waveforms,
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
                                    ui.with_layout(layout, |ui| {
                                        if ui.button("COMPILE").clicked() {
                                            state.errors = String::from("");
                                            state.trigger_compile.store(true, Ordering::Relaxed);
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
                code_editor_is_open.store(false, Ordering::Relaxed)
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
}
