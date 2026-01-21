use ratatui::style::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub highlight: Color,
    pub border: Color,
}

pub const DEFAULT_THEME: Theme = Theme {
    name: "Default",
    bg: Color::Rgb(20, 20, 25),      // Dark slate/blue
    fg: Color::Rgb(220, 220, 240),   // Soft white
    accent: Color::Rgb(100, 200, 255), // Cyan-ish
    highlight: Color::Rgb(230, 30, 30), // YouTube Red
    border: Color::Rgb(80, 80, 120),   // Muted blue-purple
};

pub const DRACULA_THEME: Theme = Theme {
    name: "Dracula",
    bg: Color::Rgb(40, 42, 54),
    fg: Color::Rgb(248, 248, 242),
    accent: Color::Rgb(189, 147, 249), // Purple
    highlight: Color::Rgb(255, 121, 198), // Pink
    border: Color::Rgb(98, 114, 164), // Comment Purple
};

pub const MATRIX_THEME: Theme = Theme {
    name: "Matrix",
    bg: Color::Black,
    fg: Color::Rgb(0, 255, 70),
    accent: Color::Rgb(0, 180, 50),
    highlight: Color::White,
    border: Color::Rgb(0, 100, 0),
};

pub const CYBERPUNK_THEME: Theme = Theme {
    name: "Cyberpunk",
    bg: Color::Rgb(10, 10, 16),
    fg: Color::Rgb(0, 240, 255), // Cyan
    accent: Color::Rgb(255, 0, 85), // Red/Pink
    highlight: Color::Rgb(252, 238, 10), // Yellow
    border: Color::Rgb(113, 28, 145), // Purple
};

pub const CATPPUCCIN_THEME: Theme = Theme {
    name: "Catppuccin",
    bg: Color::Rgb(30, 30, 46),      // Mocha Base
    fg: Color::Rgb(205, 214, 244),   // Mocha Text
    accent: Color::Rgb(137, 180, 250), // Mocha Blue
    highlight: Color::Rgb(245, 194, 231), // Mocha Pink
    border: Color::Rgb(88, 91, 112),   // Mocha Surface1
};

pub const AVAILABLE_THEMES: &[Theme] = &[
    DEFAULT_THEME,
    DRACULA_THEME,
    MATRIX_THEME,
    CYBERPUNK_THEME,
    CATPPUCCIN_THEME,
];