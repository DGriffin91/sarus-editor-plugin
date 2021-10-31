use std::{path::Path, process::Command, sync::atomic::Ordering};

use egui::{Key, Ui};
use log::info;

use crate::compiler_interface::CompilerEditorState;

pub fn code_editor_ui(ui: &mut Ui, state: &mut CompilerEditorState) {
    let mut errors = state.errors_buf_out.lock().unwrap().read().clone();
    if state.new_file_name.is_none() {
        ui.horizontal(|ui| {
            if ui.button("Show file").clicked() {
                if let Ok(projects) = state.shared_ctx.projects.lock() {
                    open_file(&projects.project_paths.projects_dir);
                }
            }
            if ui.button("Reload File").clicked() {
                if let Ok(ref mut projects) = state.shared_ctx.projects.lock() {
                    if let Err(e) = projects.reload() {
                        state.errors = format!("Load File Error {}", e.to_string())
                    }
                }
            } else if ui.button("Save File").clicked()
                || (ui.input().key_down(Key::S) && ui.input().modifiers.ctrl)
            {
                if let Ok(ref mut projects) = state.shared_ctx.projects.lock() {
                    let id = state.shared_ctx.project_float_id.get_u64();
                    match projects.set_code_by_id(id, state.code.replace("\t", "    ").to_string())
                    {
                        Ok(_) => {
                            state.errors = format!("Code Saved");
                        }
                        Err(e) => state.errors = format!("Error {}", e.to_string()),
                    }
                    match projects.save_code_by_id(id) {
                        Ok(_) => {
                            state.errors = format!("File Saved");
                            state.file_saved = true;
                            if state.compile_on_save {
                                state
                                    .shared_ctx
                                    .trigger_compile
                                    .store(true, Ordering::Relaxed)
                            }
                        }
                        Err(e) => state.errors = format!("Load Save Error {}", e.to_string()),
                    }
                }
            } else if ui.button("New File").clicked() {
                state.new_file_name = Some("".to_string());
            }

            if !state.file_saved {
                ui.label("*");
            }
            ui.label("File Name");
            ui.label(&state.file_name);
            ui.label("\tFile ID");
            ui.label(&state.shared_ctx.project_float_id.to_string());
        });
    }
    ui.horizontal(|ui| {
        if state.new_file_name.is_some() {
            if ui.button("Cancel").clicked() {
                state.new_file_name = None;
            }
            if ui.button("Create File").clicked() {
                if let Ok(ref mut projects) = state.shared_ctx.projects.lock() {
                    let file_name = state.new_file_name.as_ref().unwrap();
                    match projects.new_project(file_name) {
                        Ok(id) => {
                            projects.reload().unwrap(); //TODO don't reload everything, and don't just unwrap
                            state.shared_ctx.project_float_id.update_from_u64(id);
                            info!("new project, file name: {:?} id: {}", file_name, id);
                        }
                        Err(e) => state.errors = format!("New File Error {}", e.to_string()),
                    }
                }
                state.new_file_name = None;
            }
        }
        if let Some(new_file_name) = &mut state.new_file_name {
            ui.label("File Name");
            ui.add(
                egui::TextEdit::singleline(new_file_name)
                    .desired_width(f32::INFINITY)
                    .text_style(egui::TextStyle::Monospace),
            );
        }
    });
    let new_project_float_id = state.shared_ctx.project_float_id.get_u64();
    if state.last_project_float_id != new_project_float_id {
        state.last_project_float_id = new_project_float_id;
        if let Ok(ref mut projects) = state.shared_ctx.projects.lock() {
            if let Some(code) = projects.get_code_from_id(new_project_float_id) {
                state.code = code.to_string();
                state.line_numbers = "".to_string();
                state.file_name = projects
                    .get_name_from_id(new_project_float_id)
                    .unwrap()
                    .to_string();
            }
        }
    }
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
        state.code = code;
        state.file_saved = false;
        setup_line_numbers(state)
    } else if state.line_numbers.len() == 0 {
        setup_line_numbers(state)
    }
}

fn setup_line_numbers(state: &mut CompilerEditorState) {
    if state.code.matches("\n").count() != state.line_numbers.matches("\n").count() {
        state.line_numbers = (1..state.code.matches("\n").count() + 1)
            .enumerate()
            .map(|(i, _)| format!("{: >4}\n", i))
            .collect::<String>();
    }
}

fn open_file(path: &Path) {
    if cfg!(target_os = "windows") {
        Command::new("explorer")
            .arg(path.to_str().unwrap())
            .spawn()
            .unwrap();
    } else if cfg!(target_os = "macos") {
        Command::new("open")
            .arg(path.to_str().unwrap())
            .spawn()
            .unwrap();
    } else if cfg!(target_os = "linux") {
        Command::new("xdg-open")
            .arg(path.to_str().unwrap())
            .spawn()
            .unwrap();
    }
}
