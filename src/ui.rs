use eframe::egui;
use rand::prelude::*;
use std::path::PathBuf;
use walkdir::WalkDir;
use crate::audio::{ComandoAudio, MotorAudio};

#[derive(PartialEq, Clone, Copy)]
pub enum ModoRepetir {
    Ninguno,
    RepetirUna,
    RepetirTodas,
}

pub struct MiReproductor {
    pub motor_audio: MotorAudio,
    pub cancion_actual: String,
    pub volumen: f32,
    pub canciones: Vec<PathBuf>,
    pub indice_seleccionado: Option<usize>,
    pub modo_aleatorio: bool,
    pub repetir: ModoRepetir, 
    pub silenciado: bool,
    pub velocidad: f32,
}

impl MiReproductor {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark(); 
        visuals.window_corner_radius = 15.0.into();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20); 
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_unmultiplied(40, 60, 90, 200); 
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_unmultiplied(60, 100, 150, 250);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 150, 220); 
        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
        visuals.override_text_color = Some(egui::Color32::WHITE);
        cc.egui_ctx.set_visuals(visuals);

        let mut canciones = Vec::new();
        for entry in WalkDir::new("musica").into_iter().filter_map(|e| e.ok()) {
            let ruta = entry.path();
            if ruta.is_file() {
                if let Some(ext) = ruta.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if ext == "mp3" || ext == "wav" || ext == "flac" {
                        canciones.push(ruta.to_path_buf());
                    }
                }
            }
        }

        Self {
            motor_audio: MotorAudio::new(),
            cancion_actual: "Ninguna canción seleccionada".to_string(),
            volumen: 1.0,
            canciones,
            indice_seleccionado: None,
            modo_aleatorio: false,
            repetir: ModoRepetir::Ninguno, 
            silenciado: false,
            velocidad: 1.0,
        }
    }

    fn obtener_siguiente_indice(&self) -> Option<usize> {
        if self.canciones.is_empty() { return None; }
        
        if self.modo_aleatorio {
            let mut rng = rand::rng();
            let mut nuevo = rng.random_range(0..self.canciones.len());
            if let Some(actual) = self.indice_seleccionado {
                if nuevo == actual && self.canciones.len() > 1 {
                    nuevo = (nuevo + 1) % self.canciones.len();
                }
            }
            Some(nuevo)
        } else {
            if let Some(actual) = self.indice_seleccionado {
                Some((actual + 1) % self.canciones.len())
            } else {
                Some(0)
            }
        }
    }
}

fn pintar_interfaz_aero(ui: &mut egui::Ui, tiempo: f64, pausado: bool, hay_musica: bool) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(4, 8, 16)); 
    
    let centro = rect.center();
    for i in (0..30).rev() {
        let radio = i as f32 * 25.0;
        let alpha = (30 - i) as u8; 
        painter.circle_filled(centro, radio, egui::Color32::from_rgba_unmultiplied(15, 40, 80, alpha));
    }

    if hay_musica {
        let num_lineas = 60;
        let ancho_total = rect.width() * 0.8; 
        let inicio_x = rect.left() + (rect.width() - ancho_total) / 2.0;
        let paso = ancho_total / num_lineas as f32;
        let y_base = rect.center().y + 140.0;
        let mut rng = rand::rng();
        let animar = !pausado; 

        for i in 0..num_lineas {
            let normalizado = (i as f32 / num_lineas as f32 - 0.5).abs() * 2.0;
            let factor_campana = 1.0 - normalizado.powi(2);
            let altura = if animar {
                let ruido = rng.random_range(10.0..50.0);
                let onda = (tiempo * 5.0 + i as f64 * 0.2).sin().abs() as f32 * 30.0;
                (ruido + onda) * factor_campana
            } else { 2.0 };
            let x = inicio_x + i as f32 * paso;
            let y_fin = y_base - altura;
            painter.line_segment([egui::pos2(x, y_base), egui::pos2(x, y_fin)], egui::Stroke::new(3.0, egui::Color32::from_rgb(100, 200, 255)));
            if altura > 10.0 { painter.circle_filled(egui::pos2(x, y_fin), 2.0, egui::Color32::WHITE); }
        }
    }
}

fn formatear_tiempo(segundos: f32) -> String {
    let mins = (segundos / 60.0) as u32;
    let segs = (segundos % 60.0) as u32;
    format!("{:02}:{:02}", mins, segs)
}

impl eframe::App for MiReproductor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint();
        let tiempo_actual_ui = ui.input(|i| i.time);
        let mut cancion_a_reproducir: Option<PathBuf> = None;

        let terminada = {
            let mut flag = self.motor_audio.cancion_terminada.lock().unwrap();
            if *flag { *flag = false; true } else { false }
        };

        if terminada {
            match self.repetir {
                ModoRepetir::RepetirUna => {
                    if let Some(idx) = self.indice_seleccionado {
                        cancion_a_reproducir = Some(self.canciones[idx].clone());
                    }
                }
                ModoRepetir::RepetirTodas => {
                    if let Some(actual) = self.indice_seleccionado {
                        let siguiente = (actual + 1) % self.canciones.len();
                        self.indice_seleccionado = Some(siguiente);
                        cancion_a_reproducir = Some(self.canciones[siguiente].clone());
                    }
                }
                ModoRepetir::Ninguno => {
                    self.cancion_actual = "Ninguna canción seleccionada".to_string();
                    self.indice_seleccionado = None;
                    let _ = self.motor_audio.tx.send(ComandoAudio::Detener);
                }
            }
        }

        egui::Panel::left("playlist_panel")
            .min_size(220.0)
            .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(10, 20, 35, 180)).inner_margin(10.0))
            .show(ui, |ui| {
                ui.heading(egui::RichText::new("🎵 Playlist").size(20.0).color(egui::Color32::from_rgb(150, 200, 255)));
                ui.separator();

                if self.canciones.is_empty() {
                    ui.label("No hay canciones.");
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, cancion) in self.canciones.iter().enumerate() {
                            let nombre = cancion.file_name().unwrap_or_default().to_string_lossy();
                            let es_actual = Some(i) == self.indice_seleccionado;
                            ui.add_space(4.0); 
                            let boton = egui::Button::selectable(es_actual, egui::RichText::new(nombre).size(14.0));
                            if ui.add_sized([200.0, 28.0], boton).clicked() {
                                self.indice_seleccionado = Some(i);
                                cancion_a_reproducir = Some(cancion.clone());
                            }
                        }
                    });
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let pausado = *self.motor_audio.pausado.lock().unwrap();
                let hay_musica = self.indice_seleccionado.is_some();
                
                pintar_interfaz_aero(ui, tiempo_actual_ui, pausado, hay_musica);

                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    
                    // Recuadro del titulo
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgba_unmultiplied(10, 25, 45, 150))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(100, 150, 255, 100)))
                        .corner_radius(8.0)
                        .inner_margin(20.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Sonando ahora:").size(14.0).color(egui::Color32::from_rgb(150, 200, 255)));
                            ui.label(egui::RichText::new(&self.cancion_actual).strong().size(22.0).color(egui::Color32::WHITE));
                        });
                    
                    ui.add_space(20.0);

                    // Barra de progreso
                    let tiempo = *self.motor_audio.tiempo_actual.lock().unwrap();
                    let duracion = *self.motor_audio.duracion_total.lock().unwrap();
                    
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 200.0);
                        ui.label(egui::RichText::new(formatear_tiempo(tiempo)).size(12.0).color(egui::Color32::GRAY));
                        
                        let mut valor_barra = if duracion > 0.0 { tiempo / duracion } else { 0.0 };
                        if ui.add_sized([300.0, 15.0], egui::Slider::new(&mut valor_barra, 0.0..=1.0).show_value(false)).changed() {
                            let _ = self.motor_audio.tx.send(ComandoAudio::Adelantar(valor_barra * duracion));
                        }
                        
                        ui.label(egui::RichText::new(formatear_tiempo(duracion)).size(12.0).color(egui::Color32::GRAY));
                    });

                    ui.add_space(20.0);

                    // --- FILA 1: BOTONES DE REPRODUCCIÓN CENTRADOS ---
                    // Usamos un contenedor horizontal con distribucion a la derecha
                    // para asegurarnos que todo el bloque quede en el centro.
                    ui.horizontal(|ui| {
                        // Empujamos el contenido al centro calculando el espacio vacio
                        let ancho_total_botones = 320.0; // Ancho aprox de todos los botones juntos
                        ui.add_space((ui.available_width() - ancho_total_botones) / 2.0);

                        // Bloque de botones
                        if ui.add(egui::Button::new("🔀").selected(self.modo_aleatorio).min_size(egui::vec2(40.0, 40.0))).clicked() {
                            self.modo_aleatorio = !self.modo_aleatorio;
                        }
                        ui.add_space(8.0);
                        
                        if ui.add(egui::Button::new("⏮").min_size(egui::vec2(40.0, 40.0))).clicked() {
                            if let Some(indice) = self.indice_seleccionado {
                                if !self.canciones.is_empty() {
                                    let nuevo_indice = if indice == 0 { self.canciones.len() - 1 } else { indice - 1 };
                                    self.indice_seleccionado = Some(nuevo_indice);
                                    cancion_a_reproducir = Some(self.canciones[nuevo_indice].clone());
                                }
                            }
                        }
                        ui.add_space(8.0);

                        let texto_play = if pausado { "▶" } else { "⏸" };
                        if ui.add_enabled(hay_musica, egui::Button::new(egui::RichText::new(texto_play).size(24.0)).min_size(egui::vec2(50.0, 50.0))).clicked() {
                            let cmd = if pausado { ComandoAudio::Reanudar } else { ComandoAudio::Pausar };
                            let _ = self.motor_audio.tx.send(cmd);
                        }
                        ui.add_space(8.0);
                        
                        if ui.add(egui::Button::new("⏭").min_size(egui::vec2(40.0, 40.0))).clicked() {
                            if let Some(siguiente) = self.obtener_siguiente_indice() {
                                self.indice_seleccionado = Some(siguiente);
                                cancion_a_reproducir = Some(self.canciones[siguiente].clone());
                            }
                        }
                        ui.add_space(8.0);

                        if ui.add(egui::Button::new("⏹").min_size(egui::vec2(40.0, 40.0))).clicked() {
                            let _ = self.motor_audio.tx.send(ComandoAudio::Detener);
                            self.cancion_actual = "Ninguna canción seleccionada".to_string();
                            self.indice_seleccionado = None;
                        }
                        ui.add_space(8.0);

                        let (icono_repetir, tooltip) = match self.repetir {
                            ModoRepetir::Ninguno => ("🔁", "No repetir"),
                            ModoRepetir::RepetirUna => ("🔂", "Repetir esta canción"),
                            ModoRepetir::RepetirTodas => ("🔁", "Repetir toda la lista"),
                        };

                        let seleccionado = self.repetir != ModoRepetir::Ninguno;
                        
                        if ui.add(egui::Button::new(icono_repetir).selected(seleccionado).min_size(egui::vec2(40.0, 40.0))).on_hover_text(tooltip).clicked() {
                            self.repetir = match self.repetir {
                                ModoRepetir::Ninguno => ModoRepetir::RepetirUna,
                                ModoRepetir::RepetirUna => ModoRepetir::RepetirTodas,
                                ModoRepetir::RepetirTodas => ModoRepetir::Ninguno,
                            };
                        }
                    });

                    ui.add_space(20.0);

                    // --- FILA 2: VOLUMEN Y VELOCIDAD CENTRADOS ---
                    ui.horizontal(|ui| {
                        // Calculamos el ancho de la fila 2 (Mute 40 + Vol 120 + Vel 100) = 260px aprox
                        ui.add_space((ui.available_width() - 280.0) / 2.0);

                        // Mute
                        let texto_mute = if self.silenciado { "🔇" } else { "🔊" };
                        if ui.add(egui::Button::new(texto_mute).min_size(egui::vec2(40.0, 30.0))).clicked() {
                            self.silenciado = !self.silenciado;
                            let vol = if self.silenciado { 0.0 } else { self.volumen };
                            let _ = self.motor_audio.tx.send(ComandoAudio::Volumen(vol));
                        }

                        ui.add_space(5.0);

                        // Slider Volumen
                        if ui.add_sized([120.0, 20.0], egui::Slider::new(&mut self.volumen, 0.0..=1.5).show_value(false)).changed() {
                            if !self.silenciado {
                                let _ = self.motor_audio.tx.send(ComandoAudio::Volumen(self.volumen));
                            }
                        }

                        ui.add_space(20.0);

                        // Velocidad
                        ui.label("Vel:");
                        egui::ComboBox::from_id_salt("velocidad_combo")
                            .width(60.0)
                            .selected_text(format!("{}x", self.velocidad))
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut self.velocidad, 0.5, "0.5x").changed() {
                                    let _ = self.motor_audio.tx.send(ComandoAudio::Velocidad(self.velocidad));
                                }
                                if ui.selectable_value(&mut self.velocidad, 1.0, "1.0x").changed() {
                                    let _ = self.motor_audio.tx.send(ComandoAudio::Velocidad(self.velocidad));
                                }
                                if ui.selectable_value(&mut self.velocidad, 1.5, "1.5x").changed() {
                                    let _ = self.motor_audio.tx.send(ComandoAudio::Velocidad(self.velocidad));
                                }
                                if ui.selectable_value(&mut self.velocidad, 2.0, "2.0x").changed() {
                                    let _ = self.motor_audio.tx.send(ComandoAudio::Velocidad(self.velocidad));
                                }
                            });
                    });
                });
            });

        if let Some(ruta) = cancion_a_reproducir {
            if let Some(nombre) = ruta.file_name() {
                self.cancion_actual = nombre.to_string_lossy().to_string();
            }
            let _ = self.motor_audio.tx.send(ComandoAudio::Reproducir(ruta));
        }
    }
}