use eframe::egui::{self, Color32, ColorImage, RichText};
use eframe::{run_native, NativeOptions};
use qrcode::{types::Color as QrColor, EcLevel, QrCode};

struct TxtQrApp {
    input_text: String,
    chunk_size_chars: usize,
    image_size_px: usize,
    current_page: usize,
    pages: Vec<String>,
    last_error: String,
}

impl Default for TxtQrApp {
    fn default() -> Self {
        Self {
            input_text: "欢迎使用TxtQR，输入任意文本然后点击生成。支持自动分页（当文本太长，单个二维码无法容纳时）".to_string(),
            chunk_size_chars: 500,
            image_size_px: 400,
            current_page: 0,
            pages: Vec::new(),
            last_error: String::new(),
        }
    }
}

impl TxtQrApp {
    fn rebuild_pages(&mut self) {
        self.last_error.clear();
        self.pages.clear();

        let text = self.input_text.trim();
        if text.is_empty() {
            return;
        }

        let chunk_size = self.chunk_size_chars.max(1);
        let mut current = String::new();
        let mut count = 0;

        for c in text.chars() {
            current.push(c);
            count += 1;
            if count >= chunk_size {
                self.pages.push(current);
                current = String::new();
                count = 0;
            }
        }
        if !current.is_empty() {
            self.pages.push(current);
        }

        if self.pages.is_empty() {
            self.pages.push(String::new());
        }

        self.current_page = 0;
    }

    fn page_text(&self) -> &str {
        if self.pages.is_empty() {
            ""
        } else {
            &self.pages[self.current_page.min(self.pages.len().saturating_sub(1))]
        }
    }

    fn render_qr_image(&mut self, ui: &mut egui::Ui) {
        let text = self.page_text();
        if text.is_empty() {
            ui.label("当前页没有内容，输入文本后点击生成。");
            return;
        }

        match QrCode::with_error_correction_level(text.as_bytes(), EcLevel::M) {
            Ok(code) => {
                let module_count = code.width();
                let pixels_per_module = (self.image_size_px.max(64) as u32 + module_count as u32 - 1) / module_count as u32;
                let img_pixels = module_count as u32 * pixels_per_module;

                let mut img = ColorImage::new([img_pixels as usize, img_pixels as usize], Color32::WHITE);

                for y in 0..module_count {
                    for x in 0..module_count {
                        let dark = code[(x, y)] == QrColor::Dark;
                        let color = if dark { Color32::BLACK } else { Color32::WHITE };
                        let start_x = x as u32 * pixels_per_module;
                        let start_y = y as u32 * pixels_per_module;
                        for py in 0..pixels_per_module {
                            for px in 0..pixels_per_module {
                                let ix = (start_x + px) as usize;
                                let iy = (start_y + py) as usize;
                                if ix < img.size[0] && iy < img.size[1] {
                                    img[(ix, iy)] = color;
                                }
                            }
                        }
                    }
                }

                let texture = ui.ctx().load_texture("qr_texture", img, egui::TextureOptions::default());
                ui.add(egui::Image::new((texture.id(), texture.size_vec2())));

                ui.label(format!("总页面: {}  当前: {}", self.pages.len(), self.current_page + 1));
            }
            Err(e) => {
                self.last_error = format!("二维码生成失败：{} (当前内容长度={}，建议减小 chunk 大小)", e, text.len());
                ui.colored_label(egui::Color32::RED, &self.last_error);
            }
        }
    }
}

impl eframe::App for TxtQrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("TxtQR - 文本转二维码 (分页支持)");
            ui.separator();

            ui.label("1. 输入文本数据：");
            ui.add(egui::TextEdit::multiline(&mut self.input_text).desired_rows(8));

            ui.horizontal(|ui| {
                ui.label("2. 每个二维码字符上限（chunk，建议 100-1200）：");
                ui.add(egui::Slider::new(&mut self.chunk_size_chars, 50..=2000));
                ui.label(format!("{}", self.chunk_size_chars));
            });

            ui.horizontal(|ui| {
                ui.label("3. 显示尺寸（像素）：");
                ui.add(egui::Slider::new(&mut self.image_size_px, 128..=1000));
                ui.label(format!("{} px", self.image_size_px));
            });

            ui.horizontal(|ui| {
                if ui.button("生成二维码并分页").clicked() {
                    self.rebuild_pages();
                }
                if ui.button("清空文本").clicked() {
                    self.input_text.clear();
                    self.pages.clear();
                    self.current_page = 0;
                    self.last_error.clear();
                }
            });

            if !self.pages.is_empty() {
                ui.horizontal(|ui| {
                    if ui.button("上一页").clicked() {
                        if self.current_page > 0 {
                            self.current_page -= 1;
                        }
                    }
                    if ui.button("下一页").clicked() {
                        if self.current_page + 1 < self.pages.len() {
                            self.current_page += 1;
                        }
                    }
                    ui.label(RichText::new(format!("当前页: {}/{}", self.current_page + 1, self.pages.len())).strong());
                });

                ui.separator();
                ui.label(format!("当前页面字符数: {}", self.page_text().chars().count()));
                ui.separator();

                self.render_qr_image(ui);
            }

            if !self.last_error.is_empty() {
                ui.colored_label(egui::Color32::RED, &self.last_error);
            }

            ui.separator();
            ui.label("提示：单个 QR 码的最大容量受条码版本和纠错级别影响，如果输入太长无法生成，请适当减小 chunk 大小或增长二维码尺寸。");
        });
    }
}

fn main() {
    let options = NativeOptions::default();
    let _ = run_native(
        "TxtQR 桌面二维码生成器",
        options,
        Box::new(|_cc| Box::new(TxtQrApp::default())),
    );
}
