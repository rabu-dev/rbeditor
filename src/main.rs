use eframe::egui;
use std::fs;
use std::path::PathBuf;
use tree_sitter::{Parser, Language};
use tree_sitter_highlight::{Highlighter, HighlightConfiguration, HighlightEvent};

extern "C" {
    fn tree_sitter_rust() -> Language;
}

struct TextEditor {
    content: String,
    file_path: Option<PathBuf>,
    current_dir: Option<PathBuf>,
    dir_contents: Vec<PathBuf>,
    new_file_name: String,
    show_settings: bool,
    font_size: f32,
    background_color: egui::Color32,
    text_color: egui::Color32,
    font_family: egui::FontFamily,
    line_spacing: f32,
    parser: Parser,
    highlighter: Highlighter,
    highlight_config: HighlightConfiguration,
}

impl TextEditor {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut parser = Parser::new();
        parser.set_language(unsafe { tree_sitter_rust() }).expect("Error loading Rust grammar");

        let highlighter = Highlighter::new();

        let highlight_config = HighlightConfiguration::new(
            unsafe { tree_sitter_rust() },
            tree_sitter_rust::HIGHLIGHT_QUERY,
            "",
            "",
        ).expect("Error creating highlight configuration");

        Self {
            content: String::new(),
            file_path: None,
            current_dir: None,
            dir_contents: Vec::new(),
            new_file_name: String::new(),
            show_settings: false,
            font_size: 14.0,
            background_color: egui::Color32::from_rgb(255, 255, 255),
            text_color: egui::Color32::from_rgb(0, 0, 0),
            font_family: egui::FontFamily::Monospace,
            line_spacing: 1.5,
            parser,
            highlighter,
            highlight_config,
        }
    }

    fn save(&mut self) {
        if let Some(path) = &self.file_path {
            if let Err(e) = fs::write(path, &self.content) {
                eprintln!("Unable to save file: {}", e);
            }
        }
    }

    fn load(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.content = content;
                self.file_path = Some(path);
            }
            Err(e) => eprintln!("Unable to read file: {}", e),
        }
    }

    fn open_directory(&mut self, path: PathBuf) {
        self.current_dir = Some(path);
        self.update_dir_contents();
    }

    fn update_dir_contents(&mut self) {
        self.dir_contents.clear();
        if let Some(dir) = &self.current_dir {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        self.dir_contents.push(entry.path());
                    }
                }
            }
        }
    }

    fn create_new_file(&mut self) {
        if let Some(dir) = &self.current_dir {
            let new_file_path = dir.join(&self.new_file_name.trim());
            if !new_file_path.exists() {
                if let Ok(_) = fs::File::create(&new_file_path) {
                    self.dir_contents.push(new_file_path);
                    self.new_file_name.clear();
                }
            }
        }
    }

    fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
    }

    fn show_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Settings", |ui| {
            ui.label("Font size:");
            ui.add(egui::Slider::new(&mut self.font_size, 10.0..=100.0).show_value(true));

            ui.separator();
            ui.label("Background color:");
            ui.color_edit_button_srgba(&mut self.background_color);

            ui.separator();
            ui.label("Text color:");
            ui.color_edit_button_srgba(&mut self.text_color);

            ui.separator();
            ui.label("Font style:");
            if ui.selectable_label(self.font_family == egui::FontFamily::Monospace, "Monospace").clicked() {
                self.font_family = egui::FontFamily::Monospace;
            }
            if ui.selectable_label(self.font_family == egui::FontFamily::Proportional, "Proportional").clicked() {
                self.font_family = egui::FontFamily::Proportional;
            }

            ui.separator();
            ui.label("Line spacing:");
            ui.add(egui::Slider::new(&mut self.line_spacing, 1.0..=5.0).show_value(true));
        });
    }

    fn parse_and_highlight(&mut self) -> Vec<(egui::Color32, String)> {
        let mut highlights = Vec::new();
        let mut last_index = 0;
    
        let tree = self.parser.parse(&self.content, None).expect("Error parsing content");
        let root_node = tree.root_node();
        self.highlighter.highlight(
            &self.highlight_config,
            self.content.as_bytes(),
            None,
            |capture: &str| {
                // Process the capture string
                highlights.push((self.text_color, capture.to_string()));
                None
            },
        ).expect("Error highlighting");
    
        highlights.push((self.text_color, self.content[last_index..].to_string()));
    
        highlights
    }
}

impl eframe::App for TextEditor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            if let Some(dir) = &self.current_dir {
                ui.heading("Current Directory:");
                ui.label(dir.to_string_lossy());
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("New file:");
                    if ui.text_edit_singleline(&mut self.new_file_name).lost_focus() && ui.input(|input_state| input_state.key_pressed(egui::Key::Enter)) {
                        self.create_new_file();
                    }
                    if ui.button("Create").clicked() {
                        self.create_new_file();
                    }
                });

                ui.separator();

                let mut file_to_load = None;
                let mut dir_to_open = None;

                for path in &self.dir_contents {
                    if ui.button(path.file_name().unwrap().to_string_lossy()).clicked() {
                        if path.is_file() {
                            file_to_load = Some(path.clone());
                        } else if path.is_dir() {
                            dir_to_open = Some(path.clone());
                        }
                    }
                }

                if let Some(path) = file_to_load {
                    self.load(path);
                }
                if let Some(path) = dir_to_open {
                    self.open_directory(path);
                }
            } else {
                if ui.button("Open Directory").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.open_directory(path);
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open File").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.load(path);
                    }
                }
                if ui.button("Save").clicked() {
                    if self.file_path.is_none() {
                        if let Some(path) = rfd::FileDialog::new().save_file() {
                            self.file_path = Some(path);
                        }
                    }
                    if self.file_path.is_some() {
                        self.save();
                    }
                }
                if ui.button("Settings").clicked() {
                    self.toggle_settings();
                }
            });

            ui.separator();

            let rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(rect, 0.0, self.background_color);

            let highlighted_text = self.parse_and_highlight();
            for (color, text) in &highlighted_text {
                ui.label(egui::RichText::new(text).color(*color).font(egui::FontId::new(self.font_size, self.font_family.clone())));
            }

            let response = ui.add(
                egui::TextEdit::multiline(&mut self.content)
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .font(egui::FontId::new(self.font_size, self.font_family.clone()))
            );
            if response.changed() {
                println!("El texto ha cambiado");
                // Aquí puedes agregar cualquier lógica adicional que necesites cuando el texto cambie
                self.save();
            }

            if self.show_settings {
                self.show_settings_panel(ui);
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "RBeditor",
        native_options,
        Box::new(|cc| Box::new(TextEditor::new(cc))),
    )
}
