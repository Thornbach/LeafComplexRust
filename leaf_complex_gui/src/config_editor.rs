// Configuration Editor Dialog
use eframe::egui;
use leaf_complex_rust_lib::{Config, config::ReferencePointChoice};

pub struct ConfigEditor {
    config: Config,
    modified: bool,
}

impl ConfigEditor {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            modified: false,
        }
    }
    
    pub fn get_config(&self) -> Config {
        self.config.clone()
    }
    
    /// Show the configuration editor window
    /// Returns true if configuration was updated
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> bool {
        let mut config_updated = false;
        let initial_modified = self.modified;
        // egui borrows `open` for the window lifetime; keep a local flag and write back after rendering.
        let mut is_open = *open;
        
        let response = egui::Window::new("⚙️ Configuration Editor")
            .open(&mut is_open)
            .resizable(true)
            .default_width(600.0)
            .default_height(500.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Analysis Parameters");
                    ui.separator();
                    
                    // Image Processing Section
                    ui.collapsing("📐 Image Processing", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Resize Images:");
                            if let Some(ref mut dims) = self.config.resize_dimensions {
                                if ui.add(egui::DragValue::new(&mut dims[0]).range(128..=2048)).changed() {
                                    self.modified = true;
                                }
                                ui.label("×");
                                if ui.add(egui::DragValue::new(&mut dims[1]).range(128..=2048)).changed() {
                                    self.modified = true;
                                }
                            } else {
                                ui.label("Original size");
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Opening Kernel Size:");
                            if ui.add(egui::DragValue::new(&mut self.config.opening_kernel_size)
                                .range(1..=50)).changed() {
                                self.modified = true;
                            }
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Adaptive Opening Section
                    ui.collapsing("🎯 Adaptive Opening (EC)", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Max Density Threshold (%):");
                            if ui.add(egui::DragValue::new(&mut self.config.adaptive_opening_max_density)
                                .range(0.0..=100.0)
                                .speed(1.0)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Max Opening Percentage (%):");
                            if ui.add(egui::DragValue::new(&mut self.config.adaptive_opening_max_percentage)
                                .range(0.0..=50.0)
                                .speed(0.5)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Min Opening Percentage (%):");
                            if ui.add(egui::DragValue::new(&mut self.config.adaptive_opening_min_percentage)
                                .range(0.0..=10.0)
                                .speed(0.1)).changed() {
                                self.modified = true;
                            }
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Reference Point Section
                    ui.collapsing("📍 Reference Point", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Reference Point Choice:");
                            let mut is_com = self.config.reference_point_choice == ReferencePointChoice::Com;
                            if ui.radio_value(&mut is_com, true, "COM (Center of Mass)").changed() {
                                self.config.reference_point_choice = ReferencePointChoice::Com;
                                self.modified = true;
                            }
                            if ui.radio_value(&mut is_com, false, "EP (Emerge Point)").changed() {
                                self.config.reference_point_choice = ReferencePointChoice::Ep;
                                self.modified = true;
                            }
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Petiole Filtering Section
                    ui.collapsing("🌿 Petiole Filtering (EC)", |ui| {
                        if ui.checkbox(&mut self.config.enable_petiole_filter_ec, "Enable Petiole Filter").changed() {
                            self.modified = true;
                        }
                        
                        if ui.checkbox(&mut self.config.enable_petiole_filter_ec_complexity, 
                            "Enable in Complexity Calculation").changed() {
                            self.modified = true;
                        }
                        
                        if ui.checkbox(&mut self.config.petiole_remove_completely, 
                            "Remove Completely (vs. Set to Zero)").changed() {
                            self.modified = true;
                        }
                    });
                    
                    ui.add_space(10.0);
                    
                    // Pink Threshold Filtering Section
                    ui.collapsing("💗 Pink Threshold Filter", |ui| {
                        if ui.checkbox(&mut self.config.enable_pink_threshold_filter, 
                            "Enable Pink Threshold Filter").changed() {
                            self.modified = true;
                        }
                        
                        ui.horizontal(|ui| {
                            ui.label("Threshold Value:");
                            if ui.add(egui::DragValue::new(&mut self.config.pink_threshold_value)
                                .range(0.0..=10.0)
                                .speed(0.1)).changed() {
                                self.modified = true;
                            }
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Thornfiddle (MC) Section
                    ui.collapsing("⚡ Thornfiddle (MC)", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Max Opening % (circular):");
                            if ui.add(egui::DragValue::new(&mut self.config.thornfiddle_max_opening_percentage)
                                .range(0.0..=50.0)
                                .speed(0.5)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Min Opening % (elongated):");
                            if ui.add(egui::DragValue::new(&mut self.config.thornfiddle_min_opening_percentage)
                                .range(0.0..=50.0)
                                .speed(0.5)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Pixel Threshold:");
                            if ui.add(egui::DragValue::new(&mut self.config.thornfiddle_pixel_threshold)
                                .range(1..=20)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Smoothing Strength:");
                            if ui.add(egui::DragValue::new(&mut self.config.thornfiddle_smoothing_strength)
                                .range(0.5..=5.0)
                                .speed(0.1)).changed() {
                                self.modified = true;
                            }
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Harmonic Enhancement Section
                    ui.collapsing("🎵 Harmonic Enhancement", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Max Harmonics:");
                            if ui.add(egui::DragValue::new(&mut self.config.harmonic_max_harmonics)
                                .range(1..=24)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Strength Multiplier:");
                            if ui.add(egui::DragValue::new(&mut self.config.harmonic_strength_multiplier)
                                .range(0.5..=5.0)
                                .speed(0.1)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Min Chain Length:");
                            if ui.add(egui::DragValue::new(&mut self.config.harmonic_min_chain_length)
                                .range(5..=50)).changed() {
                                self.modified = true;
                            }
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Approximate Entropy Section
                    ui.collapsing("📊 Approximate Entropy (EC)", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Pattern Length (m):");
                            if ui.add(egui::DragValue::new(&mut self.config.approximate_entropy_m)
                                .range(1..=5)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Tolerance (r):");
                            if ui.add(egui::DragValue::new(&mut self.config.approximate_entropy_r)
                                .range(0.05..=0.5)
                                .speed(0.01)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Scaling Factor:");
                            if ui.add(egui::DragValue::new(&mut self.config.ec_scaling_factor)
                                .range(1.0..=10.0)
                                .speed(0.1)).changed() {
                                self.modified = true;
                            }
                        });
                    });
                    
                    ui.add_space(10.0);
                    
                    // Spectral Entropy Section
                    ui.collapsing("🌊 Spectral Entropy (MC)", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Sigmoid Steepness (k):");
                            if ui.add(egui::DragValue::new(&mut self.config.spectral_entropy_sigmoid_k)
                                .range(5.0..=50.0)
                                .speed(1.0)).changed() {
                                self.modified = true;
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Sigmoid Center (c):");
                            if ui.add(egui::DragValue::new(&mut self.config.spectral_entropy_sigmoid_c)
                                .range(0.01..=0.1)
                                .speed(0.001)).changed() {
                                self.modified = true;
                            }
                        });
                    });
                });
                
                ui.separator();
                
                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("💾 Save Config").clicked() {
                        if let Err(e) = self.config.save_to_file("config.toml") {
                            eprintln!("Failed to save config: {}", e);
                        } else {
                            println!("Configuration saved successfully");
                        }
                    }
                    
                    if ui.button("🔄 Reset to Defaults").clicked() {
                        self.config = Config::default();
                        self.modified = true;
                    }
                    
                    if self.modified {
                        ui.colored_label(egui::Color32::YELLOW, "⚠ Modified");
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Apply & Close").clicked() {
                            config_updated = true;
                            *open = false;
                        }
                        
                        if ui.button("Cancel").clicked() {
                            *open = false;
                        }
                    });
                });
            });
        
        // propagate open state back to caller
        *open = is_open;
        
        if response.is_some() {
            // `self.modified` may have been toggled during UI interactions
            config_updated |= self.modified && !initial_modified;
        }
        
        // Return true if config was applied
        config_updated && (self.modified || initial_modified)
    }
}
