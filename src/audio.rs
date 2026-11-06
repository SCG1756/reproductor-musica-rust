use rodio::{Decoder, DeviceSinkBuilder, Player, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub enum ComandoAudio {
    Reproducir(PathBuf),
    Pausar,
    Reanudar,
    Detener,
    Volumen(f32),
    Adelantar(f32),
    Velocidad(f32),
}

pub struct MotorAudio {
    pub tx: Sender<ComandoAudio>,
    pub pausado: Arc<Mutex<bool>>,
    pub tiempo_actual: Arc<Mutex<f32>>,
    pub duracion_total: Arc<Mutex<f32>>,
    pub cancion_terminada: Arc<Mutex<bool>>,
}

impl MotorAudio {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let pausado = Arc::new(Mutex::new(true));
        let pausado_clone = pausado.clone();

        let tiempo_actual = Arc::new(Mutex::new(0.0));
        let tiempo_clone = tiempo_actual.clone();

        let duracion_total = Arc::new(Mutex::new(0.0));
        let duracion_clone = duracion_total.clone();

        let cancion_terminada = Arc::new(Mutex::new(false));
        let terminada_clone = cancion_terminada.clone();

        thread::spawn(move || {
            let stream_handle = DeviceSinkBuilder::open_default_sink().unwrap();
            let player = Player::connect_new(stream_handle.mixer());
            player.set_volume(1.0);

            let pausado_reloj = pausado_clone.clone();
            let tiempo_reloj = tiempo_clone.clone();
            let terminada_reloj = terminada_clone.clone();
            
            let player_compartido = Arc::new(Mutex::new(player)); 
            let player_reloj = player_compartido.clone();

            // Reloj interno
            thread::spawn(move || loop {
                thread::sleep(Duration::from_millis(200));
                let p = player_reloj.lock().unwrap();
                if p.empty() && !*pausado_reloj.lock().unwrap() {
                    *terminada_reloj.lock().unwrap() = true;
                    *pausado_reloj.lock().unwrap() = true; 
                } 
                else if !p.empty() && !*pausado_reloj.lock().unwrap() {
                    *tiempo_reloj.lock().unwrap() += 0.2;
                }
                drop(p); 
            });

            let mut ruta_actual: Option<PathBuf> = None;
            let mut velocidad_actual: f32 = 1.0;

            while let Ok(comando) = rx.recv() {
                let player = player_compartido.lock().unwrap();
                match comando {
                    ComandoAudio::Reproducir(ruta) => {
                        player.stop();
                        if let Ok(file) = File::open(&ruta) {
                            if let Ok(source) = Decoder::try_from(BufReader::new(file)) {
                                *duracion_clone.lock().unwrap() = source.total_duration().unwrap_or_default().as_secs_f32();
                                *tiempo_clone.lock().unwrap() = 0.0;
                                *terminada_clone.lock().unwrap() = false; 
                                
                                // Aplicamos la velocidad actual
                                let source_con_vel = source.speed(velocidad_actual);
                                player.append(source_con_vel);
                                player.play();
                                *pausado_clone.lock().unwrap() = false;
                                
                                ruta_actual = Some(ruta.clone());
                            }
                        }
                    }
                    ComandoAudio::Pausar => { player.pause(); *pausado_clone.lock().unwrap() = true; }
                    ComandoAudio::Reanudar => { 
                        if !*terminada_clone.lock().unwrap() {
                            player.play(); 
                            *pausado_clone.lock().unwrap() = false; 
                        }
                    }
                    ComandoAudio::Detener => {
                        player.stop();
                        *pausado_clone.lock().unwrap() = true;
                        *tiempo_clone.lock().unwrap() = 0.0;
                        *duracion_clone.lock().unwrap() = 0.0;
                        *terminada_clone.lock().unwrap() = false;
                        ruta_actual = None;
                    }
                    ComandoAudio::Volumen(v) => player.set_volume(v),
                    ComandoAudio::Adelantar(segundos) => {
                        let pos = Duration::from_secs_f32(segundos);
                        let _ = player.try_seek(pos);
                        *tiempo_clone.lock().unwrap() = segundos;
                    }
                    ComandoAudio::Velocidad(nueva_velocidad) => {
                        velocidad_actual = nueva_velocidad;
                        // Solo reiniciamos si hay una canción cargada
                        if let Some(ruta) = &ruta_actual {
                            let tiempo_guardado = *tiempo_clone.lock().unwrap();
                            
                            // Guardamos si estaba pausado antes de reiniciar
                            let estaba_pausado = *pausado_clone.lock().unwrap();

                            player.stop();
                            if let Ok(file) = File::open(ruta) {
                                if let Ok(source) = Decoder::try_from(BufReader::new(file)) {
                                    let source_con_vel = source.speed(nueva_velocidad);
                                    player.append(source_con_vel);
                                    
                                    // Volvemos al segundo exacto
                                    let _ = player.try_seek(Duration::from_secs_f32(tiempo_guardado));
                                    
                                    // Si no estaba pausado, le damos play
                                    if !estaba_pausado {
                                        player.play();
                                    } else {
                                        player.pause();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Self { tx, pausado, tiempo_actual, duracion_total, cancion_terminada }
    }
}