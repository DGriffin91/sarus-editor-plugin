use egui::{
    plot::{Line, Plot, Value, Values},
    Ui,
};

use crate::compiler_interface::CompilerEditorState;

fn decay_time_to_factor(time: f32) -> f32 {
    // arbitrary constant that gives a useful range
    1. - (-1. / 6. / time).exp()
}

pub fn graphs_ui(ui: &mut Ui, state: &mut CompilerEditorState) {
    egui::ScrollArea::vertical()
        .enable_scrolling(true)
        .id_source("log")
        .show(ui, |ui| {
            let mut debug_out = state.debug_out.lock();
            for i in 0..debug_out.consumers.len() {
                debug_out.consumers[i].consume();
                let waveform = &mut state.waveforms[i];

                ui.label(format!(
                    "dbg.show({}, {:.6})",
                    i, debug_out.consumers[i].data[0]
                ));

                ui.checkbox(&mut waveform.enable_waveform, "Waveform");

                ui.checkbox(&mut waveform.enable_smoothing, "Smoothing");

                if waveform.enable_waveform {
                    if waveform.enable_smoothing {
                        ui.add(
                            egui::Slider::new(&mut waveform.memory_decay, 0.1..=2.0)
                                .text("Memory Decay"),
                        );

                        ui.add(
                            egui::Slider::new(&mut waveform.display_decay, 0.1..=2.0)
                                .text("Display Decay"),
                        );

                        for (n, buf) in debug_out.consumers[i]
                            .iter()
                            .zip(waveform.buffer.get_buffer_mut())
                        {
                            *buf = *n
                        }

                        waveform.buffer.update_match(
                            true,
                            decay_time_to_factor(waveform.memory_decay),
                            decay_time_to_factor(waveform.display_decay),
                        );

                        waveform
                            .buffer
                            .update_display(decay_time_to_factor(waveform.display_decay));

                        let data = waveform.buffer.get_display();
                        let line = Line::new(Values::from_values_iter(
                            data.iter()
                                .enumerate()
                                .map(|(i, v)| Value::new(i as f32, *v)),
                        ));
                        ui.add(
                            Plot::new(format!("debug{}", i))
                                .line(line)
                                .view_aspect(1.0)
                                .allow_drag(false)
                                .allow_zoom(false)
                                .show_x(false)
                                .show_axes([false, true]),
                        );
                    } else {
                        let data = debug_out.consumers[i].iter();
                        let line = Line::new(Values::from_values_iter(
                            data.enumerate().map(|(i, v)| Value::new(i as f32, *v)),
                        ));
                        ui.add(
                            Plot::new(format!("debug{}", i))
                                .line(line)
                                .view_aspect(1.0)
                                .allow_drag(false)
                                .allow_zoom(false)
                                .show_x(false)
                                .show_axes([false, true]),
                        );
                    };

                    ui.separator();
                }
            }
        });
}
