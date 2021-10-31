use std::ffi::CStr;

use crate::units::ConsumerRingBuf;
use egui::plot::Line;
use egui::plot::Plot;
use egui::plot::Value;
use egui::plot::Values;
use egui::Ui;
use sarus::decl;
use sarus::frontend::Arg;
use sarus::frontend::Declaration;
use sarus::frontend::Function;

use sarus::jit::JITBuilder;

use sarus::validator::struct_t;
use sarus::validator::{address_t, bool_t, f32_t, i64_t};

#[repr(C)]
pub struct SliceF32 {
    pub arr: *const f32,
    pub len: i64,
}

#[repr(C)]
pub struct SliceI64 {
    pub arr: *const i64,
    pub len: i64,
}

#[repr(C)]
pub struct SliceBool {
    pub arr: *const bool,
    pub len: i64,
}

extern "C" fn plot(ui: &mut Ui, buf: SliceF32) {
    let buf_slice = unsafe { std::slice::from_raw_parts(buf.arr, buf.len as usize) };
    let line = Line::new(Values::from_values_iter(
        buf_slice
            .iter()
            .enumerate()
            .map(|(i, v)| Value::new(i as f32, *v)),
    ));
    ui.add(
        Plot::new("plot")
            .line(line)
            .view_aspect(1.0)
            .allow_drag(false)
            .allow_zoom(false)
            .show_x(false)
            .show_axes([false, true]),
    );
}

extern "C" fn label(ui: &mut Ui, s: *const i8) {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    ui.label(s);
}

extern "C" fn button(ui: &mut Ui, s: *const i8) -> bool {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    ui.button(s).clicked()
}

extern "C" fn slider(ui: &mut Ui, s: *const i8, x: f32, range_btm: f32, range_top: f32) -> f32 {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    let mut slider_f32 = x;
    ui.add(egui::Slider::new(&mut slider_f32, range_btm..=range_top).text(s));
    slider_f32
}

extern "C" fn to_range(x: f32, bottom: f32, top: f32) -> f32 {
    x * (top - bottom) + bottom
}

extern "C" fn from_range(x: f32, bottom: f32, top: f32) -> f32 {
    (x - bottom) / (top - bottom)
}

extern "C" fn to_normalized(x: f32, bottom: f32, top: f32, exponent: f32) -> f32 {
    from_range(x, bottom, top).powf(1.0 / exponent)
}

extern "C" fn from_normalized(x: f32, bottom: f32, top: f32, exponent: f32) -> f32 {
    to_range(x.powf(exponent), bottom, top)
}

extern "C" fn db_to_lin(x: f32) -> f32 {
    (10.0f32).powf(x * 0.05)
}

extern "C" fn lin_to_db(x: f32) -> f32 {
    x.max(0.0).log(10.0) * 20.0
}

extern "C" fn slider_normalized(
    ui: &mut Ui,
    s: *const i8,
    x: f32,
    range_btm: f32,
    range_top: f32,
    exponent: f32,
) -> f32 {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    let text = format!(
        "{} {:.2}",
        s,
        from_normalized(x, range_btm, range_top, exponent)
    );
    let mut slider_f32 = x;
    ui.add(
        egui::Slider::new(&mut slider_f32, 0f32..=1f32)
            .text(text)
            .show_value(false),
    );
    slider_f32
}

pub struct DebuggerInput {
    pub producers: Vec<ringbuf::Producer<f32>>,
}

pub struct DebuggerOutput {
    pub consumers: Vec<ConsumerRingBuf<f32>>,
}

extern "C" fn show(debugger: &mut DebuggerInput, i: i64, v: f32) -> bool {
    if i >= 0 {
        let i = i as usize;
        if i < debugger.producers.len() && !debugger.producers[i].is_full() {
            if let Ok(_) = debugger.producers[i as usize].push(v) {
                return true;
            }
        }
    }
    return false;
}

#[rustfmt::skip]
pub fn append_egui(
    prog: &mut Vec<Declaration>,
    jit_builder: &mut JITBuilder,
) {
    let jb = jit_builder;
    decl!(prog, jb, "Ui.plot",plot,(struct_t("Ui"), struct_t("Slice::f32")),());
    decl!(prog, jb, "Ui.label",label,(struct_t("Ui"),address_t()),());
    decl!(prog, jb, "Ui.button",button,(struct_t("Ui"),address_t()),(bool_t()));
    decl!(prog, jb, "Ui.slider",slider,(struct_t("Ui"),address_t(),f32_t(),f32_t(),f32_t()),(f32_t()));
    decl!(prog, jb, "Ui.slider_normalized",slider_normalized,(struct_t("Ui"),address_t(),f32_t(),f32_t(),f32_t(),f32_t()),(f32_t()));
    
    decl!(prog, jb, "f32.from_range",      from_range,       (f32_t(),f32_t(),f32_t()),         (f32_t()));
    decl!(prog, jb, "f32.to_range",        to_range,         (f32_t(),f32_t(),f32_t()),         (f32_t()));
    decl!(prog, jb, "f32.from_normalized", from_normalized,  (f32_t(),f32_t(),f32_t(),f32_t()), (f32_t()));
    decl!(prog, jb, "f32.to_normalized",   to_normalized,    (f32_t(),f32_t(),f32_t(),f32_t()), (f32_t()));
    decl!(prog, jb, "f32.db_to_lin",       db_to_lin,        (f32_t()),                         (f32_t()));
    decl!(prog, jb, "f32.lin_to_db",       lin_to_db,        (f32_t()),                         (f32_t()));

    decl!(prog, jb, "Debugger.show",show,(struct_t("Debugger"),i64_t(),f32_t()),(bool_t()));
    prog.push(Declaration::StructMacro("Slice".to_string(), Box::new(f32_t())));
    prog.push(Declaration::StructMacro("Slice".to_string(), Box::new(i64_t())));
    prog.push(Declaration::StructMacro("Slice".to_string(), Box::new(bool_t())));
}
