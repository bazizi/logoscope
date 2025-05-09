use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum AppTheme {
    /// The built-in light variant.
    Light,
    /// The built-in dark variant.
    Dark,
    /// The built-in Dracula variant.
    Dracula,
    /// The built-in Nord variant.
    Nord,
    /// The built-in Solarized Light variant.
    SolarizedLight,
    /// The built-in Solarized Dark variant.
    SolarizedDark,
    /// The built-in Gruvbox Light variant.
    GruvboxLight,
    /// The built-in Gruvbox Dark variant.
    GruvboxDark,
    /// The built-in Catppuccin Latte variant.
    CatppuccinLatte,
    /// The built-in Catppuccin Frappé variant.
    CatppuccinFrappe,
    /// The built-in Catppuccin Macchiato variant.
    CatppuccinMacchiato,
    /// The built-in Catppuccin Mocha variant.
    CatppuccinMocha,
    /// The built-in Tokyo Night variant.
    TokyoNight,
    /// The built-in Tokyo Night Storm variant.
    TokyoNightStorm,
    /// The built-in Tokyo Night Light variant.
    TokyoNightLight,
    /// The built-in Kanagawa Wave variant.
    KanagawaWave,
    /// The built-in Kanagawa Dragon variant.
    KanagawaDragon,
    /// The built-in Kanagawa Lotus variant.
    KanagawaLotus,
    /// The built-in Moonfly variant.
    Moonfly,
    /// The built-in Nightfly variant.
    Nightfly,
    /// The built-in Oxocarbon variant.
    Oxocarbon,
    /// The built-in Ferra variant:
    Ferra,
}

impl From<usize> for AppTheme {
    fn from(value: usize) -> Self {
        AppTheme::ALL[value.clamp(0, AppTheme::ALL.len() - 1)].clone()
    }
}

impl From<AppTheme> for iced::Theme {
    fn from(theme: AppTheme) -> Self {
        match theme {
            AppTheme::Light => iced::Theme::Light,
            AppTheme::Dark => iced::Theme::Dark,
            AppTheme::Dracula => iced::Theme::Dracula,
            AppTheme::Nord => iced::Theme::Nord,
            AppTheme::SolarizedLight => iced::Theme::SolarizedLight,
            AppTheme::SolarizedDark => iced::Theme::SolarizedDark,
            AppTheme::GruvboxLight => iced::Theme::GruvboxLight,
            AppTheme::GruvboxDark => iced::Theme::GruvboxDark,
            AppTheme::CatppuccinLatte => iced::Theme::CatppuccinLatte,
            AppTheme::CatppuccinFrappe => iced::Theme::CatppuccinFrappe,
            AppTheme::CatppuccinMacchiato => iced::Theme::CatppuccinMacchiato,
            AppTheme::CatppuccinMocha => iced::Theme::CatppuccinMocha,
            AppTheme::TokyoNight => iced::Theme::TokyoNight,
            AppTheme::TokyoNightStorm => iced::Theme::TokyoNightStorm,
            AppTheme::TokyoNightLight => iced::Theme::TokyoNightLight,
            AppTheme::KanagawaWave => iced::Theme::KanagawaWave,
            AppTheme::KanagawaDragon => iced::Theme::KanagawaDragon,
            AppTheme::KanagawaLotus => iced::Theme::KanagawaLotus,
            AppTheme::Moonfly => iced::Theme::Moonfly,
            AppTheme::Nightfly => iced::Theme::Nightfly,
            AppTheme::Oxocarbon => iced::Theme::Oxocarbon,
            AppTheme::Ferra => iced::Theme::Ferra,
        }
    }
}

impl std::fmt::Display for AppTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Light => write!(f, "Light"),
            Self::Dark => write!(f, "Dark"),
            Self::Dracula => write!(f, "Dracula"),
            Self::Nord => write!(f, "Nord"),
            Self::SolarizedLight => write!(f, "Solarized Light"),
            Self::SolarizedDark => write!(f, "Solarized Dark"),
            Self::GruvboxLight => write!(f, "Gruvbox Light"),
            Self::GruvboxDark => write!(f, "Gruvbox Dark"),
            Self::CatppuccinLatte => write!(f, "Catppuccin Latte"),
            Self::CatppuccinFrappe => write!(f, "Catppuccin Frappé"),
            Self::CatppuccinMacchiato => write!(f, "Catppuccin Macchiato"),
            Self::CatppuccinMocha => write!(f, "Catppuccin Mocha"),
            Self::TokyoNight => write!(f, "Tokyo Night"),
            Self::TokyoNightStorm => write!(f, "Tokyo Night Storm"),
            Self::TokyoNightLight => write!(f, "Tokyo Night Light"),
            Self::KanagawaWave => write!(f, "Kanagawa Wave"),
            Self::KanagawaDragon => write!(f, "Kanagawa Dragon"),
            Self::KanagawaLotus => write!(f, "Kanagawa Lotus"),
            Self::Moonfly => write!(f, "Moonfly"),
            Self::Nightfly => write!(f, "Nightfly"),
            Self::Oxocarbon => write!(f, "Oxocarbon"),
            Self::Ferra => write!(f, "Ferra"),
        }
    }
}

impl AppTheme {
    /// A list with all the defined themes.
    pub const ALL: &'static [Self] = &[
        Self::Light,
        Self::Dark,
        Self::Dracula,
        Self::Nord,
        Self::SolarizedLight,
        Self::SolarizedDark,
        Self::GruvboxLight,
        Self::GruvboxDark,
        Self::CatppuccinLatte,
        Self::CatppuccinFrappe,
        Self::CatppuccinMacchiato,
        Self::CatppuccinMocha,
        Self::TokyoNight,
        Self::TokyoNightStorm,
        Self::TokyoNightLight,
        Self::KanagawaWave,
        Self::KanagawaDragon,
        Self::KanagawaLotus,
        Self::Moonfly,
        Self::Nightfly,
        Self::Oxocarbon,
        Self::Ferra,
    ];
}
