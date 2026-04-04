use ratatui::style::Color;

// ── UI chrome colors (TokyoNight Storm inspired) ─────────────────────────────
pub const BG_COLOR:        Color = Color::Rgb(36,  40,  59);   // deep dark blue-grey
pub const FG_COLOR:        Color = Color::Rgb(169, 177, 214);  // soft white-grey
pub const ACCENT_COLOR:    Color = Color::Rgb(122, 162, 247);  // calm blue
pub const WARNING_COLOR:   Color = Color::Rgb(224, 175, 104);  // amber
pub const ERROR_COLOR:     Color = Color::Rgb(247, 118, 142);  // muted red
pub const SUCCESS_COLOR:   Color = Color::Rgb(158, 206, 106);  // green
pub const FOLDER_COLOR:    Color = Color::Rgb(122, 162, 247);  // blue
pub const BORDER_COLOR:    Color = Color::Rgb(65,  72,  104);  // dark blue-grey
pub const HIGHLIGHT_BG:    Color = Color::Rgb(47,  51,  73);   // subtly lighter than BG
pub const SELECTION_BG:    Color = Color::Rgb(54,  58,  79);   // selection highlight
pub const SELECTION_FG:    Color = Color::Rgb(192, 202, 245);  // selection text

// ── Diff panel — line type backgrounds ───────────────────────────────────────
pub const DIFF_ADD_BG:   Color = Color::Rgb(40,  54,  45);   
pub const DIFF_DEL_BG:   Color = Color::Rgb(68,  42,  45);   
pub const DIFF_HUNK_BG:  Color = Color::Rgb(30,  32,  48);   

// ── Diff panel — line type foregrounds ───────────────────────────────────────
pub const DIFF_ADD_FG:   Color = Color::Rgb(158, 206, 106);  
pub const DIFF_ADD_SYM:  Color = Color::Rgb(185, 243, 109);  
pub const DIFF_DEL_FG:   Color = Color::Rgb(247, 118, 142);  
pub const DIFF_DEL_SYM:  Color = Color::Rgb(255, 117, 127);  
pub const DIFF_HUNK_FG:  Color = Color::Rgb(187, 154, 247);  
pub const DIFF_FILE_FG:  Color = Color::Rgb(42,  195, 222);  
pub const DIFF_CTX_FG:   Color = Color::Rgb(169, 177, 214);  
pub const DIFF_GUTTER_FG:Color = Color::Rgb(59,  66,  97);   
pub const DIFF_META_FG:  Color = Color::Rgb(86,  95,  137);  

// ── Syntax token colors ──────────────────────────────────────────────────────
pub const SYN_KEYWORD:   Color = Color::Rgb(187, 154, 247);  
pub const SYN_TYPE:      Color = Color::Rgb(42,  195, 222);  
pub const SYN_FUNCTION:  Color = Color::Rgb(122, 162, 247);  
pub const SYN_STRING:    Color = Color::Rgb(158, 206, 106);  
pub const SYN_COMMENT:   Color = Color::Rgb(86,  95,  137);  
pub const SYN_NUMBER:    Color = Color::Rgb(255, 158, 100);  
pub const SYN_MACRO:     Color = Color::Rgb(224, 175, 104);  
pub const SYN_ATTRIBUTE: Color = Color::Rgb(255, 158, 100);  
pub const SYN_OPERATOR:  Color = Color::Rgb(137, 221, 255);  
