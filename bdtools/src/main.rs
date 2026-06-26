use eframe::egui;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 450.0])
            .with_title("BD Tools"),
        ..Default::default()
    };
    eframe::run_native(
        "BD Tools",
        options,
        Box::new(|_cc| Box::new(DeployApp::default())),
    )
}

struct DeployApp {
    commit_message: String,
    log: Arc<Mutex<Vec<String>>>,
    deploying: Arc<Mutex<bool>>,
    server_ip: String,
    server_user: String,
    repo_path: String,
    env_path: String,
}

impl DeployApp {
    fn new() -> Self {
        Self {
            commit_message: String::new(),
            log: Arc::new(Mutex::new(Vec::new())),
            deploying: Arc::new(Mutex::new(false)),
            server_ip: "135.181.157.201".to_string(),
            server_user: "toscanono".to_string(),
            repo_path: "/Users/bottesini/Desktop/ebbweb".to_string(),
            env_path: "/Users/bottesini/Desktop/ebbweb/.env".to_string(),
        }
    }

    fn read_server_pass(&self) -> String {
        std::fs::read_to_string(&self.env_path)
            .unwrap_or_default()
            .lines()
            .find(|l| l.starts_with("SERVER_PASS="))
            .and_then(|l| l.split('=').nth(1))
            .unwrap_or_default()
            .to_string()
    }

    fn deploy(&self) {
        let log = Arc::clone(&self.log);
        let deploying = Arc::clone(&self.deploying);
        let commit_message = self.commit_message.clone();
        let repo_path = self.repo_path.clone();
        let server_ip = self.server_ip.clone();
        let server_user = self.server_user.clone();
        let password = self.read_server_pass();

        thread::spawn(move || {
            *deploying.lock().unwrap() = true;

            log.lock().unwrap().push("🚀 Starting deploy...".to_string());

            // git add .
            let output = Command::new("git")
                .args(["add", "."])
                .current_dir(&repo_path)
                .output();

            match output {
                Ok(o) if o.status.success() => {
                    log.lock().unwrap().push("✅ git add .".to_string());
                }
                _ => {
                    log.lock().unwrap().push("❌ git add failed".to_string());
                    *deploying.lock().unwrap() = false;
                    return;
                }
            }

            // git commit
            let msg = if commit_message.is_empty() {
                "deploy".to_string()
            } else {
                commit_message.clone()
            };

            let output = Command::new("git")
                .args(["commit", "-m", &msg])
                .current_dir(&repo_path)
                .output();

            match output {
                Ok(o) => {
                    if o.status.success() {
                        log.lock().unwrap().push(format!("✅ git commit: {}", msg));
                    } else {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        if stderr.contains("nothing to commit") {
                            log.lock().unwrap().push("ℹ️ Nothing to commit, continuing...".to_string());
                        } else {
                            log.lock().unwrap().push("❌ git commit failed".to_string());
                            *deploying.lock().unwrap() = false;
                            return;
                        }
                    }
                }
                _ => {
                    log.lock().unwrap().push("❌ git commit failed".to_string());
                    *deploying.lock().unwrap() = false;
                    return;
                }
            }

            // git push
            let output = Command::new("git")
                .args(["push"])
                .current_dir(&repo_path)
                .output();

            match output {
                Ok(o) if o.status.success() => {
                    log.lock().unwrap().push("✅ git push".to_string());
                }
                _ => {
                    log.lock().unwrap().push("❌ git push failed".to_string());
                    *deploying.lock().unwrap() = false;
                    return;
                }
            }

            // SSH deploy with sshpass
            log.lock().unwrap().push("🔗 Connecting to server...".to_string());

            if password.is_empty() {
                log.lock().unwrap().push("❌ SERVER_PASS not found in .env".to_string());
                *deploying.lock().unwrap() = false;
                return;
            }

            let output = Command::new("sshpass")
                .args([
                    "-p", &password,
                    "ssh",
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "ServerAliveInterval=60",
                    &format!("{}@{}", server_user, server_ip),
                    "~/deploy.sh"
                ])
                .output();

            match output {
                Ok(o) if o.status.success() => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    log.lock().unwrap().push("✅ Server deployed!".to_string());
                    for line in stdout.lines() {
                        log.lock().unwrap().push(format!("  {}", line));
                    }
                    log.lock().unwrap().push("🎉 Done!".to_string());
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    log.lock().unwrap().push(format!("❌ Deploy failed: {}", stderr));
                }
                Err(e) => {
                    log.lock().unwrap().push(format!("❌ sshpass error: {} — is sshpass installed? brew install hudochenkov/sshpass/sshpass", e));
                }
            }

            *deploying.lock().unwrap() = false;
        });
    }
}

impl eframe::App for DeployApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🛠 BD Tools");
            ui.separator();

            egui::CollapsingHeader::new("⚙️ Settings")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Server IP:  ");
                        ui.text_edit_singleline(&mut self.server_ip);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Server User:");
                        ui.text_edit_singleline(&mut self.server_user);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Repo Path:  ");
                        ui.text_edit_singleline(&mut self.repo_path);
                    });
                    ui.horizontal(|ui| {
                        ui.label(".env Path:  ");
                        ui.text_edit_singleline(&mut self.env_path);
                    });
                    ui.label("SERVER_PASS is read from your .env file automatically.");
                });

            ui.add_space(10.0);

            ui.label("Commit message:");
            ui.text_edit_singleline(&mut self.commit_message);

            ui.add_space(10.0);

            let deploying = *self.deploying.lock().unwrap();

            ui.horizontal(|ui| {
                if deploying {
                    ui.spinner();
                    ui.label("Deploying...");
                } else {
                    if ui.button("🚀 Push & Deploy").clicked() {
                        self.log.lock().unwrap().clear();
                        self.deploy();
                    }
                    if ui.button("🗑 Clear Log").clicked() {
                        self.log.lock().unwrap().clear();
                    }
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.label("Log:");

            egui::ScrollArea::vertical()
                .max_height(220.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let log = self.log.lock().unwrap();
                    for line in log.iter() {
                        ui.label(line);
                    }
                });
        });
    }
}

impl Default for DeployApp {
    fn default() -> Self {
        Self::new()
    }
}
