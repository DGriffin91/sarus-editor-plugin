use std::{
    mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use egui::{mutex::Mutex, Align, CtxRef, Direction, FontDefinitions, FontFamily, Layout, Ui};
use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};
use sarus::{default_std_jit_from_code_with_importer, jit::JIT};

use crate::{
    code_editor::code_editor_ui,
    correlation_match::display::DisplayBuffer,
    graphs::graphs_ui,
    heap_data::Heap,
    highligher::MemoizedSyntaxHighlighter,
    sarus_egui_lib::{append_egui, DebuggerInput, DebuggerOutput},
    SarusDSPModelParams, SarusUIModelParams,
};

use triple_buffer::{Input, TripleBuffer};

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

const DEFAULT_CODE: &str = include_str!("../resources/example.sarus");

pub fn compile(code: &str) -> anyhow::Result<JIT> {
    let jit = default_std_jit_from_code_with_importer(&code, |ast, jit_builder| {
        append_egui(ast, jit_builder);
    })?;
    Ok(jit)
}

#[repr(C)]
pub struct AudioData {
    pub in_left: *const f32,
    pub in_right: *const f32,
    pub out_left: *const f32,
    pub out_right: *const f32,
    pub len: i64,
    pub sample_rate: f32,
}

#[derive(Clone)]
pub struct CompiledUIPayload {
    pub editor_func: extern "C" fn(&mut Ui, &mut SarusUIModelParams, *mut u8),
    pub editor_data: Heap,
}

#[derive(Clone)]
pub struct CompiledDSPPayload {
    pub process_func:
        extern "C" fn(&mut SarusDSPModelParams, &mut AudioData, *mut u8, &mut DebuggerInput),
    pub process_data: Heap,
}

pub fn init_compiler_thread(
    mut code_buf_out: triple_buffer::Output<String>,
    mut errors_buf_in: triple_buffer::Input<String>,
    trigger_compile: Arc<AtomicBool>,
    mut ui_payload_in: Input<Option<CompiledUIPayload>>,
    mut dsp_payload_in: Input<Option<CompiledDSPPayload>>,
) {
    thread::spawn(move || {
        //let mut sarus_ui_func: Option<extern "C" fn(&mut Ui, &mut SarusModelParams, *mut u8)> = None;
        //let mut sarus_ui_data: Option<Heap> = None;
        let mut code: String;
        loop {
            code = code_buf_out.read().to_string();
            if trigger_compile.load(Ordering::Relaxed) {
                trigger_compile.store(false, Ordering::Relaxed);

                match start_compile(code) {
                    Ok((ui_payload, dsp_payload)) => {
                        //sarus_ui_func = Some(func);
                        //sarus_ui_data = Some(data);
                        ::log::info!("Compile Successful");
                        errors_buf_in.write(String::from("Compile Successful"));
                        ui_payload_in.write(Some(ui_payload));
                        dsp_payload_in.write(Some(dsp_payload));
                    }
                    Err(e) => {
                        ::log::error!("Compile error {}", e.to_string());
                        errors_buf_in.write(e.to_string())
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    });
}

fn start_compile(code: String) -> anyhow::Result<(CompiledUIPayload, CompiledDSPPayload)> {
    let mut jit = compile(&code.replace("\r\n", "\n"))?;
    let func_ptr = jit.get_func("editor")?;
    let editor_func = unsafe {
        mem::transmute::<_, extern "C" fn(&mut Ui, &mut SarusUIModelParams, *mut u8)>(func_ptr)
    };
    let func_ptr = jit.get_func("process")?;
    let process_func = unsafe {
        mem::transmute::<
            _,
            extern "C" fn(&mut SarusDSPModelParams, &mut AudioData, *mut u8, &mut DebuggerInput),
        >(func_ptr)
    };
    let ui_payload = CompiledUIPayload {
        editor_func,
        editor_data: get_state(&mut jit, "EditorState::size", "init_editor_state")?,
    };
    let dsp_payload = CompiledDSPPayload {
        process_func,
        process_data: get_state(&mut jit, "ProcessState::size", "init_process_state")?,
    };
    Ok((ui_payload, dsp_payload))
}

fn get_state(jit: &mut JIT, size_name: &str, state_name: &str) -> anyhow::Result<Heap> {
    let (data_ptr, _size) = jit.get_data(size_name)?;
    let size: &i64 = unsafe { mem::transmute(data_ptr) };
    let data = Heap::new(*size as usize)?;
    let func_ptr = jit.get_func(state_name)?;
    let init = unsafe { mem::transmute::<_, extern "C" fn(*mut u8)>(func_ptr) };
    init(data.get_ptr());
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_plugin_just_compile() -> anyhow::Result<()> {
        let mut jit = compile(&DEFAULT_CODE)?;

        let _func_ptr = jit.get_func("process")?;

        Ok(())
    }
}
