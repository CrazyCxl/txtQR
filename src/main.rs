#![windows_subsystem = "windows"]

use eframe::egui::{self, Color32, ColorImage};
use eframe::{run_native, NativeOptions};
use qrcode::{types::Color as QrColor, EcLevel, QrCode};
use rqrr::PreparedImage;
use arboard::Clipboard;
use image::DynamicImage;

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

    fn recognize_qr_from_clipboard(&mut self) {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.get_image() {
                    Ok(image_data) => {
                        // 将剪贴板图片数据转换为DynamicImage
                        let width = image_data.width as u32;
                        let height = image_data.height as u32;
                        let rgba_data = image_data.bytes.into_owned();

                        // 创建灰度图像用于二维码识别
                        let img = match image::RgbaImage::from_raw(width, height, rgba_data) {
                            Some(rgba_img) => DynamicImage::ImageRgba8(rgba_img).to_luma8(),
                            None => {
                                self.last_error = "无法处理剪贴板图片数据".to_string();
                                return;
                            }
                        };

                        // 使用rqrr识别二维码
                        let mut img = PreparedImage::prepare(img);
                        let grids = img.detect_grids();

                        if grids.is_empty() {
                            self.last_error = "剪贴板中未检测到二维码".to_string();
                            return;
                        }

                        // 尝试解码第一个检测到的二维码
                        for grid in grids {
                            match grid.decode() {
                                Ok((_metadata, content)) => {
                                    // content已经是String类型，直接使用
                                    self.input_text = content;
                                    self.pages.clear();
                                    self.current_page = 0;
                                    self.last_error.clear();
                                    return;
                                }
                                Err(e) => {
                                    self.last_error = format!("二维码解码失败: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.last_error = format!("无法从剪贴板获取图片: {}", e);
                    }
                }
            }
            Err(e) => {
                self.last_error = format!("无法访问剪贴板: {}", e);
            }
        }
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

        // 使用UTF-8编码生成二维码，避免中文乱码
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                // 设置全局间距
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                ui.spacing_mut().button_padding = egui::vec2(12.0, 6.0);

            // // 标题区域
            // ui.vertical_centered(|ui| {
            //     ui.add_space(10.0);
            //     ui.heading(egui::RichText::new("📱 TxtQR").size(28.0).color(egui::Color32::from_rgb(63, 81, 181)));
            //     ui.label(egui::RichText::new("文本转二维码桌面应用").size(14.0).color(egui::Color32::from_rgb(117, 117, 117)));
            //     ui.add_space(5.0);
            // });

            ui.separator();

            // 输入区域
            ui.add_space(10.0);
            ui.label(egui::RichText::new("📝 输入文本数据").size(16.0).color(egui::Color32::from_rgb(33, 33, 33)));
            egui::Frame::none()
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
                .rounding(egui::Rounding::same(4.0))
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                        ui.add_sized(
                            [ui.available_width(), 160.0],
                            egui::TextEdit::multiline(&mut self.input_text)
                                .hint_text("在此输入要转换为二维码的文本内容...")
                        );
                    });
                });

            ui.add_space(15.0);

            // 配置区域
            ui.label(egui::RichText::new("⚙️ 配置参数").size(16.0).color(egui::Color32::from_rgb(33, 33, 33)));
            ui.add_space(8.0);

            egui::Grid::new("config_grid")
                .num_columns(3)
                .spacing([20.0, 10.0])
                .show(ui, |ui| {
                    // 第一行：字符上限
                    ui.label(egui::RichText::new("字符上限").size(13.0));
                    ui.add(egui::Slider::new(&mut self.chunk_size_chars, 50..=2000)
                        .text("字符"));
                    ui.label(egui::RichText::new(format!("{}", self.chunk_size_chars))
                        .size(13.0)
                        .color(egui::Color32::from_rgb(63, 81, 181)));
                    ui.end_row();

                    // 第二行：显示尺寸
                    ui.label(egui::RichText::new("显示尺寸").size(13.0));
                    ui.add(egui::Slider::new(&mut self.image_size_px, 128..=1000)
                        .text("像素"));
                    ui.label(egui::RichText::new(format!("{} px", self.image_size_px))
                        .size(13.0)
                        .color(egui::Color32::from_rgb(63, 81, 181)));
                    ui.end_row();
                });

            ui.add_space(15.0);

            // 操作按钮区域
            ui.label(egui::RichText::new("🎯 操作").size(16.0).color(egui::Color32::from_rgb(33, 33, 33)));
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                let button_height = 32.0;

                if ui.add_sized([120.0, button_height],
                    egui::Button::new(egui::RichText::new("🚀 生成二维码").size(13.0))
                        .fill(egui::Color32::from_rgb(76, 175, 80))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(56, 142, 60)))
                ).clicked() {
                    self.rebuild_pages();
                }

                if ui.add_sized([100.0, button_height],
                    egui::Button::new(egui::RichText::new("🗑 清空").size(13.0))
                        .fill(egui::Color32::from_rgb(244, 67, 54))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(211, 47, 47)))
                ).clicked() {
                    self.input_text.clear();
                    self.pages.clear();
                    self.current_page = 0;
                    self.last_error.clear();
                }

                if ui.add_sized([140.0, button_height],
                    egui::Button::new(egui::RichText::new("📋 识别剪贴板").size(13.0))
                        .fill(egui::Color32::from_rgb(255, 152, 0))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(245, 127, 23)))
                ).clicked() {
                    self.recognize_qr_from_clipboard();
                }
            });

            // 分页控制区域
            if !self.pages.is_empty() {
                ui.add_space(20.0);
                ui.label(egui::RichText::new("📄 分页浏览").size(16.0).color(egui::Color32::from_rgb(33, 33, 33)));
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_space(10.0);

                    if ui.add_enabled(self.current_page > 0,
                        egui::Button::new("⬅️ 上一页").frame(false)
                    ).clicked() {
                        if self.current_page > 0 {
                            self.current_page -= 1;
                        }
                    }

                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.label(egui::RichText::new(format!("📄 {}/{} 页", self.current_page + 1, self.pages.len()))
                            .size(14.0)
                            .color(egui::Color32::from_rgb(63, 81, 181)));
                    });

                    if ui.add_enabled(self.current_page + 1 < self.pages.len(),
                        egui::Button::new("下一页 ➡️").frame(false)
                    ).clicked() {
                        if self.current_page + 1 < self.pages.len() {
                            self.current_page += 1;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("📊 {} 字符", self.page_text().chars().count()))
                            .size(12.0)
                            .color(egui::Color32::from_rgb(117, 117, 117)));
                    });
                });

                ui.add_space(15.0);

                // 二维码显示区域
                ui.label(egui::RichText::new("🔲 二维码预览").size(16.0).color(egui::Color32::from_rgb(33, 33, 33)));
                ui.add_space(8.0);

                egui::Frame::none()
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 200)))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(15.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            self.render_qr_image(ui);
                        });
                    });
            }

            // 错误信息显示
            if !self.last_error.is_empty() {
                ui.add_space(15.0);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(255, 235, 238))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(244, 67, 54)))
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚠️").size(16.0).color(egui::Color32::from_rgb(244, 67, 54)));
                            ui.label(egui::RichText::new(&self.last_error)
                                .size(13.0)
                                .color(egui::Color32::from_rgb(183, 28, 28)));
                        });
                    });
            }

            // 底部提示信息
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(8.0);

            egui::Frame::none()
                .fill(egui::Color32::from_rgb(248, 248, 248))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("💡 提示：").strong());
                        ui.label("单个 QR 码的最大容量受条码版本和纠错级别影响，如果输入太长无法生成，请适当减小字符上限或增大显示尺寸。");
                    });
                });

            ui.add_space(10.0);
            });
        });
    }
}

fn apply_chinese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let chinese_font_candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\simhei.ttf",
        r"/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        r"/usr/share/fonts/truetype/arphic/uming.ttc",
    ];

    for path in chinese_font_candidates.iter() {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert("chinese".to_owned(), egui::FontData::from_owned(bytes));
            let prop = fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap();
            if !prop.contains(&"chinese".to_owned()) {
                prop.insert(0, "chinese".to_owned());
            }
            let mono = fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap();
            if !mono.contains(&"chinese".to_owned()) {
                mono.push("chinese".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }

    // 没找到系统中文字体则保持默认
    ctx.set_fonts(fonts);
}

fn main() {
    let options = NativeOptions::default();
    let _ = run_native(
        "TxtQR 桌面二维码生成器",
        options,
        Box::new(|cc| {
            apply_chinese_font(&cc.egui_ctx);
            Box::new(TxtQrApp::default())
        }),
    );
}