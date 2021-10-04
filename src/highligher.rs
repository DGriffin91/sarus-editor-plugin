use egui::text::LayoutJob;

// ----------------------------------------------------------------------------

#[derive(Default)]
pub struct MemoizedSyntaxHighlighter {
    is_dark_mode: bool,
    code: String,
    language: String,
    output: LayoutJob,
    highligher: Highligher,
}

impl MemoizedSyntaxHighlighter {
    pub fn highlight(&mut self, is_dark_mode: bool, code: &str, language: &str) -> LayoutJob {
        if (
            self.is_dark_mode,
            self.code.as_str(),
            self.language.as_str(),
        ) != (is_dark_mode, code, language)
        {
            self.is_dark_mode = is_dark_mode;
            self.code = code.to_owned();
            self.language = language.to_owned();
            self.output = self
                .highligher
                .highlight(is_dark_mode, code, language)
                .unwrap_or_else(|| {
                    LayoutJob::simple(
                        code.into(),
                        egui::TextStyle::Monospace,
                        if is_dark_mode {
                            egui::Color32::LIGHT_GRAY
                        } else {
                            egui::Color32::DARK_GRAY
                        },
                        f32::INFINITY,
                    )
                });
        }
        self.output.clone()
    }
}

// ----------------------------------------------------------------------------

#[cfg(not(feature = "syntect"))]
#[derive(Default)]
pub struct Highligher {}

#[cfg(not(feature = "syntect"))]
impl Highligher {
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn highlight(&self, is_dark_mode: bool, mut text: &str, _language: &str) -> Option<LayoutJob> {
        // Extremely simple syntax highlighter for when we compile without syntect

        use egui::text::TextFormat;
        use egui::Color32;
        let monospace = egui::TextStyle::Monospace;

        let comment_format = TextFormat::simple(monospace, Color32::from_rgb(116, 113, 94));
        let quoted_string_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::from_rgb(230, 218, 116)
            } else {
                Color32::BROWN
            },
        );
        let keyword_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::from_rgb(226, 40, 85)
            } else {
                Color32::DARK_RED
            },
        );
        let literal_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::from_rgb(255, 255, 255)
            } else {
                Color32::DARK_GREEN
            },
        );
        let whitespace_format = TextFormat::simple(monospace, Color32::WHITE);
        let punctuation_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::LIGHT_GRAY
            } else {
                Color32::DARK_GRAY
            },
        );
        let numbers_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::from_rgb(174, 129, 255)
            } else {
                Color32::DARK_GRAY
            },
        );
        let parentheses_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::from_rgb(220, 220, 60)
            } else {
                Color32::DARK_GRAY
            },
        );
        let function_format = TextFormat::simple(
            monospace,
            if is_dark_mode {
                Color32::from_rgb(167, 226, 46)
            } else {
                Color32::DARK_GRAY
            },
        );

        let mut job = LayoutJob::default();

        while !text.is_empty() {
            if text.starts_with("//") {
                let end = text.find('\n').unwrap_or_else(|| text.len());
                job.append(&text[..end], 0.0, comment_format);
                text = &text[end..];
            } else if text.starts_with('"') {
                let end = text[1..]
                    .find('"')
                    .map(|i| i + 2)
                    .or_else(|| text.find('\n'))
                    .unwrap_or_else(|| text.len());
                job.append(&text[..end], 0.0, quoted_string_format);
                text = &text[end..];
            } else if text.starts_with(|c: char| {
                ['=', '.', '|', '-', '*', '/', '+', '&', '>', ':'].contains(&c)
            }) {
                let word = &text[..1];
                job.append(word, 0.0, keyword_format);
                text = &text[1..];
            } else if text.starts_with(|c: char| ['(', ')', '{', '}', '[', ']'].contains(&c)) {
                let word = &text[..1];
                job.append(word, 0.0, parentheses_format);
                text = &text[1..];
            } else if text.starts_with(|c: char| c.is_ascii_alphabetic()) {
                let end = text[1..]
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .map(|i| i + 1)
                    .unwrap_or_else(|| text.len());
                let word = &text[..end];
                if is_keyword(word) {
                    job.append(word, 0.0, keyword_format);
                } else if &text[end..end + 1] == "(" {
                    job.append(word, 0.0, function_format);
                } else {
                    job.append(word, 0.0, literal_format);
                };
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_numeric()) {
                let end = text[1..]
                    .find(|c: char| !c.is_numeric())
                    .map(|i| i + 1)
                    .unwrap_or_else(|| text.len());
                let word = &text[..end];
                job.append(word, 0.0, numbers_format);
                text = &text[end..];
            } else if text.starts_with(|c: char| c.is_ascii_whitespace()) {
                let end = text[1..]
                    .find(|c: char| !c.is_ascii_whitespace())
                    .map(|i| i + 1)
                    .unwrap_or_else(|| text.len());
                job.append(&text[..end], 0.0, whitespace_format);
                text = &text[end..];
            } else {
                let mut it = text.char_indices();
                it.next();
                let end = it.next().map_or(text.len(), |(idx, _chr)| idx);
                job.append(&text[..end], 0.0, punctuation_format);
                text = &text[end..];
            }
        }

        Some(job)
    }
}

#[cfg(not(feature = "syntect"))]
fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "f64"
            | "i64"
            | "bool"
    )
}
