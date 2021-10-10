use std::ffi::CStr;

use crate::units::ConsumerRingBuf;
use egui::Ui;
use sarus::decl;
use sarus::frontend::Arg;
use sarus::frontend::Declaration;
use sarus::frontend::Function;

use sarus::jit::JITBuilder;

use sarus::validator::struct_t;
use sarus::validator::{address_t, bool_t, f64_t, i64_t};

extern "C" fn label(ui: &mut Ui, s: *const i8) {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    ui.label(s);
}

extern "C" fn button(ui: &mut Ui, s: *const i8) -> bool {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    ui.button(s).clicked()
}

extern "C" fn slider(ui: &mut Ui, s: *const i8, x: f64, range_btm: f64, range_top: f64) -> f64 {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    let mut slider_f32 = x as f32;
    ui.add(egui::Slider::new(&mut slider_f32, range_btm as f32..=range_top as f32).text(s));
    slider_f32 as f64
}

extern "C" fn to_range(x: f64, bottom: f64, top: f64) -> f64 {
    x * (top - bottom) + bottom
}

extern "C" fn from_range(x: f64, bottom: f64, top: f64) -> f64 {
    (x - bottom) / (top - bottom)
}

extern "C" fn to_normalized(x: f64, bottom: f64, top: f64, exponent: f64) -> f64 {
    from_range(x, bottom, top).powf(1.0 / exponent)
}

extern "C" fn from_normalized(x: f64, bottom: f64, top: f64, exponent: f64) -> f64 {
    to_range(x.powf(exponent), bottom, top)
}

extern "C" fn db_to_lin(x: f64) -> f64 {
    (10.0f64).powf(x * 0.05)
}

extern "C" fn lin_to_db(x: f64) -> f64 {
    x.max(0.0).log(10.0) * 20.0
}

extern "C" fn slider_normalized(
    ui: &mut Ui,
    s: *const i8,
    x: f64,
    range_btm: f64,
    range_top: f64,
    exponent: f64,
) -> f64 {
    let s = unsafe { CStr::from_ptr(s).to_str().unwrap() };
    let text = format!(
        "{} {:.2}",
        s,
        from_normalized(x, range_btm, range_top, exponent)
    );
    let mut slider_f32 = x as f32;
    ui.add(
        egui::Slider::new(&mut slider_f32, 0f32..=1f32)
            .text(text)
            .show_value(false),
    );
    slider_f32 as f64
}

pub struct DebuggerInput {
    pub producers: Vec<ringbuf::Producer<f64>>,
}

pub struct DebuggerOutput {
    pub consumers: Vec<ConsumerRingBuf<f64>>,
}

extern "C" fn show(debugger: &mut DebuggerInput, i: i64, v: f64) -> bool {
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
    decl!(prog, jb, "Ui.label",label,(struct_t("Ui"),address_t()),());
    decl!(prog, jb, "Ui.button",button,(struct_t("Ui"),address_t()),(bool_t()));
    decl!(prog, jb, "Ui.slider",slider,(struct_t("Ui"),address_t(),f64_t(),f64_t(),f64_t()),(f64_t()));
    decl!(prog, jb, "Ui.slider_normalized",slider_normalized,(struct_t("Ui"),address_t(),f64_t(),f64_t(),f64_t(),f64_t()),(f64_t()));
    
    decl!(prog, jb, "f64.from_range",      from_range,       (f64_t(),f64_t(),f64_t()),        (f64_t()));
    decl!(prog, jb, "f64.to_range",        to_range,         (f64_t(),f64_t(),f64_t()),        (f64_t()));
    decl!(prog, jb, "f64.from_normalized", from_normalized,  (f64_t(),f64_t(),f64_t(),f64_t()), (f64_t()));
    decl!(prog, jb, "f64.to_normalized",   to_normalized,    (f64_t(),f64_t(),f64_t(),f64_t()), (f64_t()));
    decl!(prog, jb, "f64.db_to_lin",       db_to_lin,        (f64_t()),                      (f64_t()));
    decl!(prog, jb, "f64.lin_to_db",       lin_to_db,        (f64_t()),                      (f64_t()));

    decl!(prog, jb, "Debugger.show",show,(struct_t("Debugger"),i64_t(),f64_t()),(bool_t()));
}
