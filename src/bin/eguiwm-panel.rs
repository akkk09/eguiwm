use std::{io::Write, os::unix::net::UnixStream, process::Command, time::Duration};

use eframe::egui::{self, Color32, Frame, RichText, Stroke, Vec2};

const PANEL_HEIGHT: f32 = 42.0;

struct Panel {
    control_socket: String,
    status: String,
    active_workspace: usize,
}

impl Panel {
    fn send(&mut self, command: &str) {
        match UnixStream::connect(&self.control_socket) {
            Ok(mut stream) => {
                if writeln!(stream, "{command}").is_ok() {
                    self.status.clear();
                } else {
                    self.status = "Control request failed".into();
                }
            }
            Err(error) => self.status = format!("Compositor IPC: {error}"),
        }
    }

    fn launch(&mut self, command: &str) {
        match Command::new(command)
            .env("WAYLAND_DISPLAY", std::env::var("WAYLAND_DISPLAY").unwrap_or_default())
            .env("GDK_BACKEND", "wayland")
            .env("QT_QPA_PLATFORM", "wayland")
            .env("MOZ_ENABLE_WAYLAND", "1")
            .env_remove("DISPLAY")
            .spawn()
        {
            Ok(_) => self.status.clear(),
            Err(error) => self.status = format!("Could not launch {command}: {error}"),
        }
    }

    fn system_summary() -> String {
        let load = std::fs::read_to_string("/proc/loadavg")
            .ok()
            .and_then(|s| s.split_whitespace().next().map(str::to_owned))
            .unwrap_or_else(|| "n/a".into());
        let memory = std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                let total = s.lines().find(|l| l.starts_with("MemTotal:"))?.split_whitespace().nth(1)?.parse::<u64>().ok()?;
                let available = s.lines().find(|l| l.starts_with("MemAvailable:"))?.split_whitespace().nth(1)?.parse::<u64>().ok()?;
                Some(format!("RAM {} / {} GiB", (total - available) / 1_048_576, total / 1_048_576))
            })
            .unwrap_or_else(|| "RAM n/a".into());
        format!("Load {load}  ·  {memory}")
    }
}

impl eframe::App for Panel {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_secs(2));
        egui::CentralPanel::default()
            .frame(Frame {
                fill: Color32::from_rgb(25, 27, 32),
                stroke: Stroke::new(1.0, Color32::from_rgb(48, 52, 60)),
                inner_margin: egui::Margin::symmetric(10, 4),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.set_min_height(PANEL_HEIGHT - 8.0);
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("eguiwm").strong().color(Color32::from_rgb(135, 190, 255)));
                    ui.separator();

                    for workspace in 0..4 {
                        let label = if workspace == self.active_workspace {
                            format!("● {}", workspace + 1)
                        } else {
                            format!("○ {}", workspace + 1)
                        };
                        if ui.add_sized(Vec2::new(42.0, 28.0), egui::Button::new(label)).clicked() {
                            self.active_workspace = workspace;
                            self.send(&format!("workspace:{workspace}"));
                        }
                    }

                    ui.separator();
                    for (label, command) in [
                        ("Terminal", "xfce4-terminal"),
                        ("Browser", "firefox"),
                        ("Files", "thunar"),
                    ] {
                        if ui.button(label).clicked() {
                            self.launch(command);
                        }
                    }

                    ui.separator();
                    ui.label(RichText::new(Self::system_summary()).color(Color32::from_rgb(190, 195, 205)));
                    if !self.status.is_empty() {
                        ui.label(RichText::new(&self.status).color(Color32::LIGHT_RED));
                    }
                });
            });
    }
}

fn main() -> eframe::Result<()> {
    let control_socket = std::env::var("EGUIWM_CONTROL_SOCKET")
        .expect("eguiwm must start this panel with EGUIWM_CONTROL_SOCKET set");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("eguiwm-panel")
            .with_app_id("eguiwm-panel")
            .with_inner_size([1280.0, PANEL_HEIGHT])
            .with_min_inner_size([640.0, PANEL_HEIGHT])
            .with_max_inner_size([4096.0, PANEL_HEIGHT])
            .with_decorations(false)
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "eguiwm-panel",
        options,
        Box::new(move |_cc| Ok(Box::new(Panel {
            control_socket,
            status: String::new(),
            active_workspace: 0,
        }))),
    )
}
