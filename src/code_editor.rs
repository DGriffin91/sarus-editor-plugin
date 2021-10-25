use egui::{Key, Ui};

use std::fs;
use std::path::Path;

use crate::compiler_interface::CompilerEditorState;

pub fn code_editor_ui(ui: &mut Ui, state: &mut CompilerEditorState) {
    let mut errors = state.errors_buf_out.lock().read().clone();
    let current_file = state.current_file.clone();
    ui.horizontal(|ui| {
        let temp_path = Path::new(&current_file);
        if !temp_path.exists() {
            ui.label("File not found");
        } else if let Err(e) = is_sarus_file(temp_path) {
            ui.label(e);
        } else {
            if ui.button("Reload File").clicked() {
                match fs::read_to_string(temp_path) {
                    Ok(contents) => {
                        state.code = contents;
                        state.file_saved = true;
                    }
                    Err(e) => state.errors = format!("Load File Error {}", e.to_string()),
                }
            } else if ui.button("Save File").clicked()
                || (ui.input().key_down(Key::S) && ui.input().modifiers.ctrl)
            {
                match fs::write(temp_path, state.code.clone()) {
                    Ok(_) => {
                        state.errors = format!("File Saved");
                        state.file_saved = true;
                    }
                    Err(e) => state.errors = format!("Load Save Error {}", e.to_string()),
                }
            }
        }
        if !state.file_saved {
            ui.label("*");
        }
        ui.add(
            egui::TextEdit::singleline(&mut state.current_file)
                .desired_width(f32::INFINITY)
                .text_style(egui::TextStyle::Monospace),
        );
    });
    let mut code = state.code.clone();
    let mut line_numbers = state.line_numbers.clone();
    /*if ui.button("Open File").clicked() {
        //let path = FileDialog::new()
        //    .set_location("~/Desktop") //&dirs::document_dir().unwrap_or(Path::new("~/").to_path_buf())
        //    .add_filter("Sarus File", &["sarus"])
        //    .show_open_single_file()
        //    .unwrap();

        //let path = match path {
        //    Some(path) => path,
        //    None => (),
        //};

        //if yes {
        //if let Some(path) = path.to_str() {
        //    state.current_file = path.to_string();
        //}
        //}
        let file = rfd::FileDialog::new()
            .add_filter("Sarus Files", &["sarus"])
            .set_directory("/")
            .pick_file();
        if let Some(path) = file {
            if let Some(path) = path.to_str() {
                state.current_file = path.to_string();
            }
        }
    }*/
    egui::ScrollArea::vertical()
        .enable_scrolling(true)
        .id_source("log")
        .show(ui, |ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(20, 20, 20);
            ui.add(
                egui::TextEdit::multiline(&mut errors)
                    .desired_width(f32::INFINITY)
                    .text_style(egui::TextStyle::Monospace), // for cursor height
            );
        });
    egui::ScrollArea::vertical()
        .enable_scrolling(true)
        .always_show_scroll(true)
        .id_source("code_editor")
        .show(ui, |ui| {
            ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(39, 40, 34);
            let mut theme = crate::syntax_highlighting::CodeTheme::from_memory(ui.ctx());
            ui.collapsing("Theme", |ui| {
                ui.group(|ui| {
                    theme.ui(ui);
                    theme.store_in_memory(ui.ctx());
                });
            });
            let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                let mut layout_job =
                    crate::syntax_highlighting::highlight(ui.ctx(), &theme, string, "rs".into());
                layout_job.wrap_width = f32::INFINITY;
                ui.fonts().layout_job(layout_job)
            };
            ui.horizontal_top(|ui| {
                ui.add_enabled(
                    false,
                    egui::TextEdit::multiline(&mut line_numbers)
                        .desired_width(60.0)
                        .lock_focus(true)
                        .text_style(egui::TextStyle::Monospace)
                        .frame(false),
                );
                egui::ScrollArea::horizontal()
                    .enable_scrolling(true)
                    .always_show_scroll(true)
                    .id_source("code_editor_hor")
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut code)
                                .desired_width(f32::INFINITY)
                                .code_editor()
                                .layouter(&mut layouter)
                                .frame(false), // for cursor height
                        );
                    })
            })
        });

    if state.code != code {
        state
            .code_buf_in
            .lock()
            .write(code.replace("\t", "    ").clone());
        state.code = code;
        state.file_saved = false;
        setup_line_numbers(state)
    } else if state.line_numbers.len() == 0 {
        setup_line_numbers(state)
    }
}

fn setup_line_numbers(state: &mut CompilerEditorState) {
    if state.code.matches("\n").count() != state.line_numbers.matches("\n").count() {
        state.line_numbers = (0..state.code.matches("\n").count())
            .enumerate()
            .map(|(i, _)| format!("{: >4}\n", i))
            .collect::<String>();
    }
}

fn is_sarus_file(path: &Path) -> Result<(), String> {
    if let Some(ext) = path.extension() {
        if let Some(ext) = ext.to_str() {
            if ext == "sarus" {
                return Ok(());
            } else {
                return Err(format!("incorrect file type {}", ext));
            }
        }
    }
    return Err(format!("incorrect file type"));
}
