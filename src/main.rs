use eframe::egui;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use std::f32::INFINITY;
use std::fs::File;
use std::io;
use std::io::Write;
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    op: String,
    path: String,
    value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct Operation {
    blockKey: String,
    op: String,
    data: Vec<Data>,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserInfo {
    name: String,
    sourceSpecialization: Vec<String>,
    operations: Vec<Operation>,
    published: bool,
}

impl Default for UserInfo {
    fn default() -> Self {
        UserInfo {
            name: "".to_owned(),
            sourceSpecialization: vec![],
            operations: vec![Operation {
                blockKey: "".to_owned(),
                op: "patch".to_string(),
                data: vec![Data {
                    op: "replace".to_string(),
                    path: "".to_string(),
                    value: serde_json::Value::Null,
                }],
            }],
            published: true,
        }
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "JSON Creator",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    );
}

struct MyApp {
    user_info: UserInfo,
    json_output: String,
    value_input: String,
    domain: String,
    filename: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            user_info: UserInfo::default(),
            json_output: "".to_owned(),
            value_input: "".to_owned(),
            domain: "".to_owned(),
            filename: "user_info.json".to_owned(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Name: ");
                    ui.text_edit_singleline(&mut self.user_info.name);
                });

                ui.horizontal(|ui| {
                    ui.label("Block Key:");
                    ui.text_edit_singleline(
                        &mut self.user_info.operations[0].blockKey,
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Operation:");
                    egui::ComboBox::from_label("Select operation")
                        .selected_text(&self.user_info.operations[0].data[0].op)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.user_info.operations[0].data[0].op,
                                "add".to_string(),
                                "add",
                            );
                            ui.selectable_value(
                                &mut self.user_info.operations[0].data[0].op,
                                "patch".to_string(),
                                "patch",
                            );
                            ui.selectable_value(
                                &mut self.user_info.operations[0].data[0].op,
                                "remove".to_string(),
                                "remove",
                            );
                            ui.selectable_value(
                                &mut self.user_info.operations[0].data[0].op,
                                "replace".to_string(),
                                "replace",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.text_edit_singleline(
                        &mut self.user_info.operations[0].data[0].path,
                    );
                });

                ui.label("Value (as JSON):");
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() / 1.66) as f32)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.value_input)
                                .desired_width(ui.available_width())
                                .desired_rows(
                                    (ui.available_height() / 20.0) as usize,
                                ),
                        );
                    });

                ui.checkbox(&mut self.user_info.published, "Publish?");

                ui.horizontal(|ui| {
                    if ui.button("Create Change-Set").clicked() {
                        // Perform validation first
                        if self.user_info.name.trim().is_empty() {
                            self.json_output =
                                "Error: Name is required.".to_string();
                            return;
                        }
                        if !self.user_info.operations[0].data[0]
                            .path
                            .starts_with('/')
                        {
                            self.json_output =
                                "Error: Path must start with '/'.".to_string();
                            return;
                        }
                        if !self.user_info.operations[0].blockKey.contains('@')
                        {
                            self.json_output =
                                "Error: Block Key must contain '@'."
                                    .to_string();
                            return;
                        }

                        // Validate the JSON input
                        match serde_json::from_str(&self.value_input) {
                            Ok(parsed_value) => {
                                self.user_info.operations[0].data[0].value =
                                    parsed_value;
                            }
                            Err(e) => {
                                self.json_output = format!(
                                    "Error: Invalid JSON in value field: {}",
                                    e
                                );
                                return;
                            }
                        }

                        // Open the file dialog if all validation passes
                        if let Some(path) = FileDialog::new().save_file() {
                            let filename = path.display().to_string();
                            self.filename = filename.clone();
                            println!("Saving to: {}", filename);

                            self.json_output =
                                match to_string_pretty(&self.user_info) {
                                    Ok(json) => json,
                                    Err(err) => {
                                        format!("Error creating JSON: {}", err)
                                    }
                                };
                            match self
                                .save_to_file(&filename, &self.json_output)
                            {
                                Ok(_) => {
                                    self.json_output =
                                        "JSON file saved.".to_string();
                                }
                                Err(e) => {
                                    self.json_output =
                                        format!("Failed to save file: {}", e);
                                }
                            }
                        }
                    }
                });
            });

            ui.horizontal(|ui| {
                if ui.button("Post Change-Set").clicked() {
                    match self.execute_command() {
                        Ok(_) => {
                            ui.ctx().request_repaint();
                        }
                        Err(e) => {
                            self.json_output =
                                format!("Failed to execute command: {}", e);
                            ui.ctx().request_repaint();
                        }
                    }
                }

                ui.label("Domain:");
                ui.text_edit_singleline(&mut self.domain);
            });

            ui.label("JSON Output:");
            egui::ScrollArea::vertical()
                .id_source("json_output_scroll_area")
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.json_output)
                            .frame(true)
                            .desired_width(ui.available_width())
                            .desired_rows(
                                (ui.available_height() / 50.0) as usize,
                            ),
                    );
                });
        });
    }
}

impl MyApp {
    fn execute_command(&mut self) -> Result<(), std::io::Error> {
        if self.domain.trim().is_empty() {
            self.json_output = "Error: Domain field is empty.".to_string();
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Domain field is empty",
            ));
        }
        if let Some(path) = FileDialog::new().pick_file() {
            let filename = path.display().to_string();

            let output = Command::new("cmd")
                .args(&[
                    "/C",
                    "ao-config",
                    "change-set",
                    "post",
                    &self.filename,
                    "-d",
                    &self.domain,
                ])
                .output()?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let guid = stdout.trim();
                println!("GUID: {}", guid);
                self.json_output = format!("{}", guid);
                Ok(())
            } else {
                eprintln!("Command failed to execute.");
                eprintln!(
                    "stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Command execution failed",
                ))
            }
        } else {
            self.json_output = "No file selected.".to_string();
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "File selection cancelled",
            ))
        }
    }

    fn save_to_file(&self, filename: &str, data: &str) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        file.write_all(data.as_bytes())?;
        println!("Data saved to {}", filename);
        Ok(())
    }
}
