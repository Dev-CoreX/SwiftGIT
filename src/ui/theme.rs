use ratatui::style::Color;

// ── UI chrome colors ──────────────────────────────────────────────────────────
pub const BG_COLOR:        Color = Color::Rgb(36,  40,  59);   // app background
pub const FG_COLOR:        Color = Color::Rgb(192, 202, 245);  // default foreground
pub const ACCENT_COLOR:    Color = Color::Rgb(122, 162, 247);  // active borders, titles
pub const WARNING_COLOR:   Color = Color::Rgb(224, 175, 104);  // warnings
pub const ERROR_COLOR:     Color = Color::Rgb(247, 120, 107);  // errors
pub const SUCCESS_COLOR:   Color = Color::Rgb(158, 206, 106);  // success messages
pub const FOLDER_COLOR:    Color = Color::Rgb(130, 170, 255);  // folder icons
pub const _SELECTED_COLOR: Color = Color::Rgb(255, 158, 100);  // selected item
pub const BORDER_COLOR:    Color = Color::Rgb(86,  95,  137);  // inactive borders
pub const HIGHLIGHT_BG:    Color = Color::Rgb(41,  46,  66);   // hover highlight

// ── Diff panel — line type backgrounds ───────────────────────────────────────
pub const DIFF_ADD_BG:   Color = Color::Rgb(26,  46,  26);   // added line bg
pub const DIFF_DEL_BG:   Color = Color::Rgb(46,  26,  26);   // deleted line bg
pub const DIFF_HUNK_BG:  Color = Color::Rgb(30,  32,  48);   // hunk header bg

// ── Diff panel — line type foregrounds ───────────────────────────────────────
pub const DIFF_ADD_FG:   Color = Color::Rgb(158, 206, 106);  // added line text
pub const DIFF_ADD_SYM:  Color = Color::Rgb(115, 218, 90);   // '+' glyph
pub const DIFF_DEL_FG:   Color = Color::Rgb(247, 118, 142);  // deleted line text
pub const DIFF_DEL_SYM:  Color = Color::Rgb(247, 118, 142);  // '-' glyph
pub const DIFF_HUNK_FG:  Color = Color::Rgb(42,  195, 222);  // @@ hunk header
pub const DIFF_FILE_FG:  Color = Color::Rgb(187, 154, 247);  // diff/--- /+++ header
pub const DIFF_CTX_FG:   Color = Color::Rgb(169, 177, 214);  // context lines
pub const DIFF_GUTTER_FG:Color = Color::Rgb(59,  66,  97);   // line number gutter
pub const DIFF_META_FG:  Color = Color::Rgb(86,  95,  137);  // "no newline" etc

// ── Diff panel — syntax token colors ─────────────────────────────────────────
pub const SYN_KEYWORD:   Color = Color::Rgb(187, 154, 247);  // pub fn use let match
pub const SYN_TYPE:      Color = Color::Rgb(42,  195, 222);  // Result Option String
pub const SYN_FUNCTION:  Color = Color::Rgb(122, 162, 247);  // open() commit()
pub const SYN_STRING:    Color = Color::Rgb(158, 206, 106);  // "string literals"
pub const SYN_COMMENT:   Color = Color::Rgb(86,  95,  137);  // // comments  //! docs
pub const SYN_NUMBER:    Color = Color::Rgb(255, 158, 100);  // 42  0o700  consts
pub const SYN_MACRO:     Color = Color::Rgb(224, 175, 104);  // bail!  matches!
pub const SYN_ATTRIBUTE: Color = Color::Rgb(255, 158, 100);  // #[derive(…)]
pub const SYN_OPERATOR:  Color = Color::Rgb(137, 221, 255);  // => | & != ==
