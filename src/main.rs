// Esconde o console na versão release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};
use windows_sys::Win32::Graphics::Gdi::*;

const BASE_W: u32 = 1920;
const BASE_H: u32 = 1080;
const DEFAULT_H_PCT: u32 = 98;
const DEFAULT_V_PCT: u32 = 100;
const CONFIRM_SECONDS: u64 = 10;

fn current_mode() -> Option<DEVMODEW> {
    unsafe {
        let mut dm: DEVMODEW = std::mem::zeroed();
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        if EnumDisplaySettingsW(null(), ENUM_CURRENT_SETTINGS, &mut dm) != 0 {
            Some(dm)
        } else {
            None
        }
    }
}

fn disp_error(code: i32) -> String {
    match code {
        1 => "É necessário reiniciar o computador.".into(),
        -1 => "Falha ao mudar a resolução.".into(),
        -2 => "Resolução não suportada pelo driver/monitor. Crie uma resolução \
               personalizada no painel da NVIDIA/AMD (ou use o CRU) e tente de novo."
            .into(),
        -3 => "Não foi possível gravar a configuração.".into(),
        -4 => "Flags inválidas.".into(),
        -5 => "Parâmetro inválido.".into(),
        _ => format!("Erro desconhecido ({code})."),
    }
}

fn set_resolution(w: u32, h: u32) -> Result<(), String> {
    let mut dm = current_mode().ok_or("Não foi possível ler a resolução atual.")?;
    dm.dmPelsWidth = w;
    dm.dmPelsHeight = h;
    dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL | DM_DISPLAYFREQUENCY;

    unsafe {
        // Primeiro apenas testa se o modo é válido
        let test = ChangeDisplaySettingsExW(null(), &dm, null_mut(), CDS_TEST, null());
        if test != DISP_CHANGE_SUCCESSFUL {
            return Err(disp_error(test));
        }
        // Aplica de forma dinâmica (não grava no registro)
        let r = ChangeDisplaySettingsExW(null(), &dm, null_mut(), 0, null());
        if r != DISP_CHANGE_SUCCESSFUL {
            return Err(disp_error(r));
        }
    }
    Ok(())
}

fn restore_resolution() -> Result<(), String> {
    unsafe {
        // devmode nulo = volta às configurações padrão salvas
        let r = ChangeDisplaySettingsExW(null(), null(), null_mut(), 0, null());
        if r == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(disp_error(r))
        }
    }
}

fn scaled(base: u32, pct: u32) -> u32 {
    ((base as f64) * (pct as f64) / 100.0).round() as u32
}

struct App {
    h_pct: u32,
    v_pct: u32,
    base_w: u32,
    base_h: u32,
    status: String,
    revert_at: Option<Instant>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            h_pct: DEFAULT_H_PCT,
            v_pct: DEFAULT_V_PCT,
            base_w: BASE_W,
            base_h: BASE_H,
            status: "Pronto.".into(),
            revert_at: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Reversão automática se o usuário não confirmar
        if let Some(t) = self.revert_at {
            if Instant::now() >= t {
                self.status = match restore_resolution() {
                    Ok(_) => "Sem confirmação: resolução restaurada.".into(),
                    Err(e) => e,
                };
                self.revert_at = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        }

        let target_w = scaled(self.base_w, self.h_pct);
        let target_h = scaled(self.base_h, self.v_pct);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("RAGE Stretch");
            ui.label("Escala horizontal e vertical independentes");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Resolução base:");
                ui.add(egui::DragValue::new(&mut self.base_w).range(640..=7680));
                ui.label("x");
                ui.add(egui::DragValue::new(&mut self.base_h).range(480..=4320));
            });

            ui.add_space(8.0);
            ui.add(
                egui::Slider::new(&mut self.h_pct, 50..=100)
                    .suffix("%")
                    .text("Horizontal"),
            );
            ui.add(
                egui::Slider::new(&mut self.v_pct, 50..=100)
                    .suffix("%")
                    .text("Vertical"),
            );

            ui.add_space(8.0);
            ui.label(format!("Resultado: {} x {}", target_w, target_h));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Aplicar").clicked() {
                    match set_resolution(target_w, target_h) {
                        Ok(_) => {
                            self.status = format!("Aplicado: {}x{}", target_w, target_h);
                            self.revert_at =
                                Some(Instant::now() + Duration::from_secs(CONFIRM_SECONDS));
                        }
                        Err(e) => self.status = e,
                    }
                }
                if ui.button("Restaurar").clicked() {
                    self.revert_at = None;
                    self.status = match restore_resolution() {
                        Ok(_) => "Resolução original restaurada.".into(),
                        Err(e) => e,
                    };
                }
                if ui.button("Padrão (98% / 100%)").clicked() {
                    self.h_pct = DEFAULT_H_PCT;
                    self.v_pct = DEFAULT_V_PCT;
                    self.base_w = BASE_W;
                    self.base_h = BASE_H;
                }
            });

            if let Some(t) = self.revert_at {
                let left = t.saturating_duration_since(Instant::now()).as_secs() + 1;
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("Manter esta resolução? Revertendo em {left}s"),
                    );
                    if ui.button("Manter").clicked() {
                        self.revert_at = None;
                        self.status = "Resolução mantida.".into();
                    }
                });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.label(&self.status);
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 320.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "RAGE Stretch",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
