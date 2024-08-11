use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
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
    let options = eframe::NativeOptions::default();
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Enter user information");

            ui.label("Domain:");
            ui.text_edit_singleline(&mut self.domain);

            ui.label("Name: ");
            ui.text_edit_singleline(&mut self.user_info.name);

            ui.label("Block Key:");
            ui.text_edit_singleline(&mut self.user_info.operations[0].blockKey);

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

            ui.label("Path:");
            ui.text_edit_singleline(
                &mut self.user_info.operations[0].data[0].path,
            );

            ui.label("Value (as JSON):");
            egui::ScrollArea::vertical()
                .id_source("value_input_scroll_area")
                .max_height(275.0) // Limit the height of the input area
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.value_input)
                            .desired_rows(18)
                            .frame(true),
                    );
                });

            ui.checkbox(&mut self.user_info.published, "Publish?");

            if ui.button("Create JSON").clicked() {
                let filename = format!("{}.json", self.user_info.name);
                self.filename = filename.clone();
                if self.user_info.name.trim().is_empty() {
                    self.json_output = "Error: Name is required.".to_string();
                    return;
                }
                if !self.user_info.operations[0].data[0].path.starts_with('/') {
                    self.json_output =
                        "Error: Path must start with '/'.".to_string();
                    return;
                }

                println!("Raw input: {}", self.value_input);

                match serde_json::from_str(&self.value_input) {
                    Ok(parsed_value) => {
                        println!("Parsed value: {:?}", parsed_value);
                        self.user_info.operations[0].data[0].value =
                            parsed_value;
                        if self.user_info.operations[0].blockKey.contains('@') {
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
                                        "JSON file saved.".to_string()
                                }
                                Err(e) => {
                                    self.json_output =
                                        format!("Failed to save file: {}", e)
                                }
                            }
                        } else {
                            self.json_output =
                                "Error: Block Key must contain '@'".to_string();
                        }
                    }
                    Err(e) => {
                        println! {"Parsing error: {}:", e};
                        self.json_output =
                            "Error: Invalid JSON in value field".to_string();
                    }
                }
            }

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

            ui.label("JSON Output:");
            egui::ScrollArea::vertical()
                .id_source("json_output_scroll_area")
                .max_height(75.0) // Limit the height of the input area
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.json_output)
                            .desired_rows(5)
                            .frame(true),
                    );
                });
        });
    }
}

impl MyApp {
    fn execute_command(&mut self) -> Result<(), std::io::Error> {
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
            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Command execution failed",
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
