use std::{
    mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use egui::Ui;
use sarus::{default_std_jit_from_code_with_importer, jit::JIT};

use crate::{
    heap_data::Heap,
    sarus_egui_lib::{append_egui, DebuggerInput},
    SarusDSPModelParams, SarusUIModelParams,
};

use triple_buffer::Input;

pub const DEFAULT_CODE: &str = include_str!("../resources/example.sarus");
pub const START_CODE: &str = include_str!("../resources/start.sarus");

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
            if trigger_compile.load(Ordering::Relaxed) {
                code = code_buf_out.read().to_string();
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
            std::thread::sleep(Duration::from_millis(50));
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

        let mut jit = compile(&START_CODE)?;
        let _func_ptr = jit.get_func("process")?;
        Ok(())
    }
}