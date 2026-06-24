import 'package:flutter/material.dart';

/// Named runtime palette variants the user can pick from in Settings.
enum WrongclThemeVariant {
  /// The original warm wrongcl palette shipped before runtime palette
  /// switching landed.
  wrongcl,

  /// https://www.nordtheme.com — arctic, bluish palette.
  nord,

  /// https://rosepinetheme.com — Dawn (light) paired with Rosé Pine (dark).
  rosePineDawn,

  /// https://catppuccin.com — Latte (light) paired with Mocha (dark).
  catppuccin,
}

extension WrongclThemeVariantId on WrongclThemeVariant {
  /// Stable identifier used in persisted app settings JSON.
  String get id {
    switch (this) {
      case WrongclThemeVariant.wrongcl:
        return 'wrongcl';
      case WrongclThemeVariant.nord:
        return 'nord';
      case WrongclThemeVariant.rosePineDawn:
        return 'rose-pine-dawn';
      case WrongclThemeVariant.catppuccin:
        return 'catppuccin';
    }
  }

  /// Human-readable label rendered in the Basic settings dropdown.
  String get label {
    switch (this) {
      case WrongclThemeVariant.wrongcl:
        return 'Wrongcl (default)';
      case WrongclThemeVariant.nord:
        return 'Nord';
      case WrongclThemeVariant.rosePineDawn:
        return 'Rosé Pine Dawn';
      case WrongclThemeVariant.catppuccin:
        return 'Catppuccin';
    }
  }

  static WrongclThemeVariant fromId(String? id) {
    switch (id) {
      case 'nord':
        return WrongclThemeVariant.nord;
      case 'rose-pine-dawn':
        return WrongclThemeVariant.rosePineDawn;
      case 'catppuccin':
        return WrongclThemeVariant.catppuccin;
      default:
        return WrongclThemeVariant.wrongcl;
    }
  }
}

/// Centralized color tokens for the wrongcl Flutter UI.
///
/// All raw hex values live in the named palette constants on this class.
/// Widgets read tokens through [Theme.of(context).extension<WrongclColors>()],
/// so a runtime palette swap is a single edit here, not a sweep across views.
@immutable
class WrongclColors extends ThemeExtension<WrongclColors> {
  const WrongclColors({
    required this.surface,
    required this.border,
    required this.text,
    required this.accent,
    required this.status,
    required this.topBar,
    required this.chart,
  });

  final WrongclSurfaceColors surface;
  final WrongclBorderColors border;
  final WrongclTextColors text;
  final WrongclAccentColors accent;
  final WrongclStatusColors status;
  final WrongclTopBarColors topBar;
  final WrongclChartColors chart;

  /// Returns the palette matching [variant] for the requested [brightness].
  ///
  /// Each variant exposes a paired light/dark palette so that the Theme mode
  /// picker (system/light/dark) and the Theme palette picker remain
  /// independent settings.
  static WrongclColors forVariant(
    WrongclThemeVariant variant,
    Brightness brightness,
  ) {
    final dark = brightness == Brightness.dark;
    switch (variant) {
      case WrongclThemeVariant.wrongcl:
        return dark ? dark_ : light;
      case WrongclThemeVariant.nord:
        return dark ? nordDark : nordLight;
      case WrongclThemeVariant.rosePineDawn:
        return dark ? rosePine : rosePineDawn;
      case WrongclThemeVariant.catppuccin:
        return dark ? catppuccinMocha : catppuccinLatte;
    }
  }

  // ---------------------------------------------------------------------------
  // Wrongcl default (existing warm palette)
  // ---------------------------------------------------------------------------

  static const WrongclColors light = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFFF2EFE8),
      surface: Color(0xFFF7F6F2),
      surfaceRaised: Color(0xFFFBFAF7),
      surfaceMuted: Color(0xFFF4F1EA),
      surfaceWarm: Color(0xFFF8F6F1),
      surfaceHighlight: Color(0xFFEFE6D5),
      surfaceTinted: Color(0xFFEDE7DC),
      surfaceAccent: Color(0xFFF7F2E5),
      surfaceSelected: Color(0xFFF2EEE6),
      onAccent: Color(0xFFE6F1EF),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFFD7D2C8),
      regular: Color(0xFFDCD5CA),
      muted: Color(0xFFD8D1C5),
      strong: Color(0xFFB8B1A4),
      accent: Color(0xFF7A6F5C),
      contrast: Color(0xFF1F2933),
    ),
    text: WrongclTextColors(
      primary: Color(0xFF1F2933),
      secondary: Color(0xFF8B8579),
      tertiary: Color(0xFF6F6A5F),
      muted: Color(0xFF6F6558),
      inverse: Color(0xFFE6F1EF),
      neutral: Color(0xFF616161),
      danger: Color(0xFFB7401E),
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF2F4858),
      runtime: Color(0xFF111111),
      runtimeOn: Color(0xFFE6F1EF),
      seed: Color(0xFF006D77),
      soft: Color(0xFFB8AE9D),
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFF0B8A6E),
      warning: Color(0xFF9A6700),
      danger: Color(0xFFB00020),
      info: Color(0xFF2F4858),
      neutral: Color(0xFF616161),
      success: Color(0xFF4E7A3C),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFFEDE7DC),
      foreground: Color(0xFF1F2933),
      foregroundMuted: Color(0xFF6F6A5F),
      activeCell: Color(0xFFFBFAF7),
      activeBorder: Color(0xFF7A6F5C),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFFE5E2DA),
      gridSubtle: Color(0xFFD8D1C5),
      upload: Color(0xFF2F4858),
      download: Color(0xFF0B8A6E),
    ),
  );

  // Trailing underscore avoids shadowing the dart:core `dark` getter when the
  // file is imported alongside material/widget toolkits that ship `dark`-style
  // helpers; the public name reachable via [forVariant] is unaffected.
  static const WrongclColors dark_ = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFF101417),
      surface: Color(0xFF171C1F),
      surfaceRaised: Color(0xFF1B2125),
      surfaceMuted: Color(0xFF1F262B),
      surfaceWarm: Color(0xFF1B2125),
      surfaceHighlight: Color(0xFF2A3239),
      surfaceTinted: Color(0xFF1F262B),
      surfaceAccent: Color(0xFF253038),
      surfaceSelected: Color(0xFF253038),
      onAccent: Color(0xFF101417),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFF2F3940),
      regular: Color(0xFF3A444C),
      muted: Color(0xFF2F3940),
      strong: Color(0xFF4B5963),
      accent: Color(0xFF6E8390),
      contrast: Color(0xFFE6F1EF),
    ),
    text: WrongclTextColors(
      primary: Color(0xFFE6F1EF),
      secondary: Color(0xFFA9B2B8),
      tertiary: Color(0xFF8B949C),
      muted: Color(0xFF7B848C),
      inverse: Color(0xFF101417),
      neutral: Color(0xFFB4BCC2),
      danger: Color(0xFFE08068),
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF4A8C8A),
      runtime: Color(0xFFE6F1EF),
      runtimeOn: Color(0xFF101417),
      seed: Color(0xFF4A8C8A),
      soft: Color(0xFF4B5963),
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFF4FB39B),
      warning: Color(0xFFE0B056),
      danger: Color(0xFFE0664F),
      info: Color(0xFF6FA8C0),
      neutral: Color(0xFFB4BCC2),
      success: Color(0xFF7DB36A),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFF1A2128),
      foreground: Color(0xFFE6F1EF),
      foregroundMuted: Color(0xFFA9B2B8),
      activeCell: Color(0xFF253038),
      activeBorder: Color(0xFF6E8390),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFF253038),
      gridSubtle: Color(0xFF2F3940),
      upload: Color(0xFF6FA8C0),
      download: Color(0xFF4FB39B),
    ),
  );

  /// Backwards-compatible alias kept so external callers that referenced
  /// `WrongclColors.dark` before the variant rename keep compiling.
  static const WrongclColors dark = dark_;

  // ---------------------------------------------------------------------------
  // Nord — https://www.nordtheme.com (Snow Storm light / Polar Night dark)
  // ---------------------------------------------------------------------------

  static const WrongclColors nordLight = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFFECEFF4), // nord6
      surface: Color(0xFFE5E9F0), // nord5
      surfaceRaised: Color(0xFFECEFF4),
      surfaceMuted: Color(0xFFD8DEE9), // nord4
      surfaceWarm: Color(0xFFE5E9F0),
      surfaceHighlight: Color(0xFFD8DEE9),
      surfaceTinted: Color(0xFFD8DEE9),
      surfaceAccent: Color(0xFFD8DEE9),
      surfaceSelected: Color(0xFFD8DEE9),
      onAccent: Color(0xFFECEFF4),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFFD8DEE9),
      regular: Color(0xFFD8DEE9),
      muted: Color(0xFFD8DEE9),
      strong: Color(0xFF4C566A), // nord3
      accent: Color(0xFF5E81AC), // nord10
      contrast: Color(0xFF2E3440), // nord0
    ),
    text: WrongclTextColors(
      primary: Color(0xFF2E3440), // nord0
      secondary: Color(0xFF4C566A), // nord3
      tertiary: Color(0xFF434C5E), // nord2
      muted: Color(0xFF4C566A),
      inverse: Color(0xFFECEFF4),
      neutral: Color(0xFF434C5E),
      danger: Color(0xFFBF616A), // nord11
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF5E81AC),
      runtime: Color(0xFF5E81AC),
      runtimeOn: Color(0xFFECEFF4),
      seed: Color(0xFF5E81AC),
      soft: Color(0xFF81A1C1), // nord9
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFFA3BE8C), // nord14
      warning: Color(0xFFD08770), // nord12
      danger: Color(0xFFBF616A),
      info: Color(0xFF5E81AC),
      neutral: Color(0xFF4C566A),
      success: Color(0xFFA3BE8C),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFFE5E9F0),
      foreground: Color(0xFF2E3440),
      foregroundMuted: Color(0xFF4C566A),
      activeCell: Color(0xFFECEFF4),
      activeBorder: Color(0xFF5E81AC),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFFD8DEE9),
      gridSubtle: Color(0xFFE5E9F0),
      upload: Color(0xFF5E81AC),
      download: Color(0xFFA3BE8C),
    ),
  );

  static const WrongclColors nordDark = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFF2E3440), // nord0
      surface: Color(0xFF3B4252), // nord1
      surfaceRaised: Color(0xFF434C5E), // nord2
      surfaceMuted: Color(0xFF3B4252),
      surfaceWarm: Color(0xFF434C5E),
      surfaceHighlight: Color(0xFF4C566A), // nord3
      surfaceTinted: Color(0xFF3B4252),
      surfaceAccent: Color(0xFF4C566A),
      surfaceSelected: Color(0xFF4C566A),
      onAccent: Color(0xFFECEFF4),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFF3B4252),
      regular: Color(0xFF434C5E),
      muted: Color(0xFF3B4252),
      strong: Color(0xFF4C566A),
      accent: Color(0xFF88C0D0), // nord8
      contrast: Color(0xFFECEFF4),
    ),
    text: WrongclTextColors(
      primary: Color(0xFFECEFF4),
      secondary: Color(0xFFD8DEE9),
      tertiary: Color(0xFFE5E9F0),
      muted: Color(0xFFD8DEE9),
      inverse: Color(0xFF2E3440),
      neutral: Color(0xFFD8DEE9),
      danger: Color(0xFFBF616A),
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF88C0D0),
      runtime: Color(0xFF88C0D0),
      runtimeOn: Color(0xFF2E3440),
      seed: Color(0xFF5E81AC),
      soft: Color(0xFF81A1C1),
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFFA3BE8C),
      warning: Color(0xFFEBCB8B), // nord13
      danger: Color(0xFFBF616A),
      info: Color(0xFF88C0D0),
      neutral: Color(0xFFD8DEE9),
      success: Color(0xFFA3BE8C),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFF3B4252),
      foreground: Color(0xFFECEFF4),
      foregroundMuted: Color(0xFFD8DEE9),
      activeCell: Color(0xFF434C5E),
      activeBorder: Color(0xFF88C0D0),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFF434C5E),
      gridSubtle: Color(0xFF3B4252),
      upload: Color(0xFF81A1C1),
      download: Color(0xFFA3BE8C),
    ),
  );

  // ---------------------------------------------------------------------------
  // Rosé Pine Dawn (light) paired with Rosé Pine (dark)
  // https://rosepinetheme.com
  // ---------------------------------------------------------------------------

  static const WrongclColors rosePineDawn = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFFFAF4ED), // base
      surface: Color(0xFFFFFAF3), // surface
      surfaceRaised: Color(0xFFFFFAF3),
      surfaceMuted: Color(0xFFF2E9E1), // overlay
      surfaceWarm: Color(0xFFF4EDE8), // highlight low
      surfaceHighlight: Color(0xFFDFDAD9), // highlight med
      surfaceTinted: Color(0xFFF2E9E1),
      surfaceAccent: Color(0xFFDFDAD9),
      surfaceSelected: Color(0xFFDFDAD9),
      onAccent: Color(0xFFFFFAF3),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFFF4EDE8),
      regular: Color(0xFFCECACD), // highlight high
      muted: Color(0xFFDFDAD9),
      strong: Color(0xFF797593), // subtle
      accent: Color(0xFF286983), // pine
      contrast: Color(0xFF575279), // text
    ),
    text: WrongclTextColors(
      primary: Color(0xFF575279),
      secondary: Color(0xFF797593),
      tertiary: Color(0xFF9893A5), // muted
      muted: Color(0xFF9893A5),
      inverse: Color(0xFFFFFAF3),
      neutral: Color(0xFF797593),
      danger: Color(0xFFB4637A), // love
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF286983),
      runtime: Color(0xFF286983),
      runtimeOn: Color(0xFFFFFAF3),
      seed: Color(0xFF286983),
      soft: Color(0xFF56949F), // foam
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFF286983),
      warning: Color(0xFFEA9D34), // gold
      danger: Color(0xFFB4637A),
      info: Color(0xFF56949F),
      neutral: Color(0xFF797593),
      success: Color(0xFF286983),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFFF2E9E1),
      foreground: Color(0xFF575279),
      foregroundMuted: Color(0xFF797593),
      activeCell: Color(0xFFFFFAF3),
      activeBorder: Color(0xFF286983),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFFDFDAD9),
      gridSubtle: Color(0xFFF4EDE8),
      upload: Color(0xFF286983),
      download: Color(0xFF56949F),
    ),
  );

  static const WrongclColors rosePine = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFF191724), // base
      surface: Color(0xFF1F1D2E), // surface
      surfaceRaised: Color(0xFF26233A), // overlay
      surfaceMuted: Color(0xFF1F1D2E),
      surfaceWarm: Color(0xFF21202E), // highlight low
      surfaceHighlight: Color(0xFF403D52), // highlight med
      surfaceTinted: Color(0xFF21202E),
      surfaceAccent: Color(0xFF403D52),
      surfaceSelected: Color(0xFF403D52),
      onAccent: Color(0xFF191724),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFF21202E),
      regular: Color(0xFF403D52),
      muted: Color(0xFF21202E),
      strong: Color(0xFF524F67), // highlight high
      accent: Color(0xFF9CCFD8), // foam
      contrast: Color(0xFFE0DEF4), // text
    ),
    text: WrongclTextColors(
      primary: Color(0xFFE0DEF4),
      secondary: Color(0xFF908CAA), // subtle
      tertiary: Color(0xFF6E6A86), // muted
      muted: Color(0xFF6E6A86),
      inverse: Color(0xFF191724),
      neutral: Color(0xFF908CAA),
      danger: Color(0xFFEB6F92), // love
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF9CCFD8),
      runtime: Color(0xFF9CCFD8),
      runtimeOn: Color(0xFF191724),
      seed: Color(0xFF31748F), // pine
      soft: Color(0xFFC4A7E7), // iris
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFF9CCFD8),
      warning: Color(0xFFF6C177), // gold
      danger: Color(0xFFEB6F92),
      info: Color(0xFF9CCFD8),
      neutral: Color(0xFF908CAA),
      success: Color(0xFF9CCFD8),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFF1F1D2E),
      foreground: Color(0xFFE0DEF4),
      foregroundMuted: Color(0xFF908CAA),
      activeCell: Color(0xFF26233A),
      activeBorder: Color(0xFF9CCFD8),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFF403D52),
      gridSubtle: Color(0xFF21202E),
      upload: Color(0xFF9CCFD8),
      download: Color(0xFFC4A7E7),
    ),
  );

  // ---------------------------------------------------------------------------
  // Catppuccin Latte (light) paired with Catppuccin Mocha (dark)
  // https://catppuccin.com
  // ---------------------------------------------------------------------------

  static const WrongclColors catppuccinLatte = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFFEFF1F5), // base
      surface: Color(0xFFE6E9EF), // mantle
      surfaceRaised: Color(0xFFEFF1F5),
      surfaceMuted: Color(0xFFCCD0DA), // surface0
      surfaceWarm: Color(0xFFE6E9EF),
      surfaceHighlight: Color(0xFFCCD0DA),
      surfaceTinted: Color(0xFFCCD0DA),
      surfaceAccent: Color(0xFFBCC0CC), // surface1
      surfaceSelected: Color(0xFFCCD0DA),
      onAccent: Color(0xFFEFF1F5),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFFDCE0E8), // crust
      regular: Color(0xFFCCD0DA),
      muted: Color(0xFFCCD0DA),
      strong: Color(0xFF8C8FA1), // overlay1
      accent: Color(0xFF1E66F5), // blue
      contrast: Color(0xFF4C4F69), // text
    ),
    text: WrongclTextColors(
      primary: Color(0xFF4C4F69),
      secondary: Color(0xFF5C5F77), // subtext1
      tertiary: Color(0xFF6C6F85), // subtext0
      muted: Color(0xFF7C7F93), // overlay2
      inverse: Color(0xFFEFF1F5),
      neutral: Color(0xFF5C5F77),
      danger: Color(0xFFD20F39), // red
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF1E66F5),
      runtime: Color(0xFF1E66F5),
      runtimeOn: Color(0xFFEFF1F5),
      seed: Color(0xFF1E66F5),
      soft: Color(0xFF7287FD), // lavender
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFF40A02B), // green
      warning: Color(0xFFDF8E1D), // yellow
      danger: Color(0xFFD20F39),
      info: Color(0xFF209FB5), // sapphire
      neutral: Color(0xFF8C8FA1),
      success: Color(0xFF40A02B),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFFE6E9EF),
      foreground: Color(0xFF4C4F69),
      foregroundMuted: Color(0xFF5C5F77),
      activeCell: Color(0xFFEFF1F5),
      activeBorder: Color(0xFF1E66F5),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFFCCD0DA),
      gridSubtle: Color(0xFFDCE0E8),
      upload: Color(0xFF1E66F5),
      download: Color(0xFF40A02B),
    ),
  );

  static const WrongclColors catppuccinMocha = WrongclColors(
    surface: WrongclSurfaceColors(
      scaffold: Color(0xFF1E1E2E), // base
      surface: Color(0xFF181825), // mantle
      surfaceRaised: Color(0xFF313244), // surface0
      surfaceMuted: Color(0xFF181825),
      surfaceWarm: Color(0xFF313244),
      surfaceHighlight: Color(0xFF45475A), // surface1
      surfaceTinted: Color(0xFF313244),
      surfaceAccent: Color(0xFF45475A),
      surfaceSelected: Color(0xFF45475A),
      onAccent: Color(0xFF1E1E2E),
    ),
    border: WrongclBorderColors(
      subtle: Color(0xFF181825),
      regular: Color(0xFF45475A),
      muted: Color(0xFF313244),
      strong: Color(0xFF585B70), // surface2
      accent: Color(0xFF89B4FA), // blue
      contrast: Color(0xFFCDD6F4), // text
    ),
    text: WrongclTextColors(
      primary: Color(0xFFCDD6F4),
      secondary: Color(0xFFBAC2DE), // subtext1
      tertiary: Color(0xFFA6ADC8), // subtext0
      muted: Color(0xFF9399B2), // overlay2
      inverse: Color(0xFF1E1E2E),
      neutral: Color(0xFFBAC2DE),
      danger: Color(0xFFF38BA8), // red
    ),
    accent: WrongclAccentColors(
      primary: Color(0xFF89B4FA),
      runtime: Color(0xFF89B4FA),
      runtimeOn: Color(0xFF1E1E2E),
      seed: Color(0xFF89B4FA),
      soft: Color(0xFFB4BEFE), // lavender
    ),
    status: WrongclStatusColors(
      healthy: Color(0xFFA6E3A1), // green
      warning: Color(0xFFF9E2AF), // yellow
      danger: Color(0xFFF38BA8),
      info: Color(0xFF74C7EC), // sapphire
      neutral: Color(0xFF7F849C), // overlay1
      success: Color(0xFFA6E3A1),
    ),
    topBar: WrongclTopBarColors(
      background: Color(0xFF181825),
      foreground: Color(0xFFCDD6F4),
      foregroundMuted: Color(0xFFBAC2DE),
      activeCell: Color(0xFF313244),
      activeBorder: Color(0xFF89B4FA),
    ),
    chart: WrongclChartColors(
      grid: Color(0xFF45475A),
      gridSubtle: Color(0xFF181825),
      upload: Color(0xFF89B4FA),
      download: Color(0xFFA6E3A1),
    ),
  );

  @override
  WrongclColors copyWith({
    WrongclSurfaceColors? surface,
    WrongclBorderColors? border,
    WrongclTextColors? text,
    WrongclAccentColors? accent,
    WrongclStatusColors? status,
    WrongclTopBarColors? topBar,
    WrongclChartColors? chart,
  }) {
    return WrongclColors(
      surface: surface ?? this.surface,
      border: border ?? this.border,
      text: text ?? this.text,
      accent: accent ?? this.accent,
      status: status ?? this.status,
      topBar: topBar ?? this.topBar,
      chart: chart ?? this.chart,
    );
  }

  @override
  WrongclColors lerp(ThemeExtension<WrongclColors>? other, double t) {
    if (other is! WrongclColors) {
      return this;
    }
    return WrongclColors(
      surface: surface.lerp(other.surface, t),
      border: border.lerp(other.border, t),
      text: text.lerp(other.text, t),
      accent: accent.lerp(other.accent, t),
      status: status.lerp(other.status, t),
      topBar: topBar.lerp(other.topBar, t),
      chart: chart.lerp(other.chart, t),
    );
  }
}

extension WrongclThemeAccess on BuildContext {
  WrongclColors get wrongclColors {
    final ext = Theme.of(this).extension<WrongclColors>();
    return ext ?? WrongclColors.light;
  }
}

@immutable
class WrongclSurfaceColors {
  const WrongclSurfaceColors({
    required this.scaffold,
    required this.surface,
    required this.surfaceRaised,
    required this.surfaceMuted,
    required this.surfaceWarm,
    required this.surfaceHighlight,
    required this.surfaceTinted,
    required this.surfaceAccent,
    required this.surfaceSelected,
    required this.onAccent,
  });

  final Color scaffold;
  final Color surface;
  final Color surfaceRaised;
  final Color surfaceMuted;
  final Color surfaceWarm;
  final Color surfaceHighlight;
  final Color surfaceTinted;
  final Color surfaceAccent;
  final Color surfaceSelected;
  final Color onAccent;

  WrongclSurfaceColors lerp(WrongclSurfaceColors other, double t) {
    return WrongclSurfaceColors(
      scaffold: Color.lerp(scaffold, other.scaffold, t)!,
      surface: Color.lerp(surface, other.surface, t)!,
      surfaceRaised: Color.lerp(surfaceRaised, other.surfaceRaised, t)!,
      surfaceMuted: Color.lerp(surfaceMuted, other.surfaceMuted, t)!,
      surfaceWarm: Color.lerp(surfaceWarm, other.surfaceWarm, t)!,
      surfaceHighlight: Color.lerp(surfaceHighlight, other.surfaceHighlight, t)!,
      surfaceTinted: Color.lerp(surfaceTinted, other.surfaceTinted, t)!,
      surfaceAccent: Color.lerp(surfaceAccent, other.surfaceAccent, t)!,
      surfaceSelected: Color.lerp(surfaceSelected, other.surfaceSelected, t)!,
      onAccent: Color.lerp(onAccent, other.onAccent, t)!,
    );
  }
}

@immutable
class WrongclBorderColors {
  const WrongclBorderColors({
    required this.subtle,
    required this.regular,
    required this.muted,
    required this.strong,
    required this.accent,
    required this.contrast,
  });

  final Color subtle;
  final Color regular;
  final Color muted;
  final Color strong;
  final Color accent;
  final Color contrast;

  WrongclBorderColors lerp(WrongclBorderColors other, double t) {
    return WrongclBorderColors(
      subtle: Color.lerp(subtle, other.subtle, t)!,
      regular: Color.lerp(regular, other.regular, t)!,
      muted: Color.lerp(muted, other.muted, t)!,
      strong: Color.lerp(strong, other.strong, t)!,
      accent: Color.lerp(accent, other.accent, t)!,
      contrast: Color.lerp(contrast, other.contrast, t)!,
    );
  }
}

@immutable
class WrongclTextColors {
  const WrongclTextColors({
    required this.primary,
    required this.secondary,
    required this.tertiary,
    required this.muted,
    required this.inverse,
    required this.neutral,
    required this.danger,
  });

  final Color primary;
  final Color secondary;
  final Color tertiary;
  final Color muted;
  final Color inverse;
  final Color neutral;
  final Color danger;

  WrongclTextColors lerp(WrongclTextColors other, double t) {
    return WrongclTextColors(
      primary: Color.lerp(primary, other.primary, t)!,
      secondary: Color.lerp(secondary, other.secondary, t)!,
      tertiary: Color.lerp(tertiary, other.tertiary, t)!,
      muted: Color.lerp(muted, other.muted, t)!,
      inverse: Color.lerp(inverse, other.inverse, t)!,
      neutral: Color.lerp(neutral, other.neutral, t)!,
      danger: Color.lerp(danger, other.danger, t)!,
    );
  }
}

@immutable
class WrongclAccentColors {
  const WrongclAccentColors({
    required this.primary,
    required this.runtime,
    required this.runtimeOn,
    required this.seed,
    required this.soft,
  });

  final Color primary;
  final Color runtime;
  final Color runtimeOn;
  final Color seed;
  final Color soft;

  WrongclAccentColors lerp(WrongclAccentColors other, double t) {
    return WrongclAccentColors(
      primary: Color.lerp(primary, other.primary, t)!,
      runtime: Color.lerp(runtime, other.runtime, t)!,
      runtimeOn: Color.lerp(runtimeOn, other.runtimeOn, t)!,
      seed: Color.lerp(seed, other.seed, t)!,
      soft: Color.lerp(soft, other.soft, t)!,
    );
  }
}

@immutable
class WrongclStatusColors {
  const WrongclStatusColors({
    required this.healthy,
    required this.warning,
    required this.danger,
    required this.info,
    required this.neutral,
    required this.success,
  });

  final Color healthy;
  final Color warning;
  final Color danger;
  final Color info;
  final Color neutral;
  final Color success;

  WrongclStatusColors lerp(WrongclStatusColors other, double t) {
    return WrongclStatusColors(
      healthy: Color.lerp(healthy, other.healthy, t)!,
      warning: Color.lerp(warning, other.warning, t)!,
      danger: Color.lerp(danger, other.danger, t)!,
      info: Color.lerp(info, other.info, t)!,
      neutral: Color.lerp(neutral, other.neutral, t)!,
      success: Color.lerp(success, other.success, t)!,
    );
  }
}

@immutable
class WrongclTopBarColors {
  const WrongclTopBarColors({
    required this.background,
    required this.foreground,
    required this.foregroundMuted,
    required this.activeCell,
    required this.activeBorder,
  });

  final Color background;
  final Color foreground;
  final Color foregroundMuted;
  final Color activeCell;
  final Color activeBorder;

  WrongclTopBarColors lerp(WrongclTopBarColors other, double t) {
    return WrongclTopBarColors(
      background: Color.lerp(background, other.background, t)!,
      foreground: Color.lerp(foreground, other.foreground, t)!,
      foregroundMuted: Color.lerp(foregroundMuted, other.foregroundMuted, t)!,
      activeCell: Color.lerp(activeCell, other.activeCell, t)!,
      activeBorder: Color.lerp(activeBorder, other.activeBorder, t)!,
    );
  }
}

@immutable
class WrongclChartColors {
  const WrongclChartColors({
    required this.grid,
    required this.gridSubtle,
    required this.upload,
    required this.download,
  });

  final Color grid;
  final Color gridSubtle;
  final Color upload;
  final Color download;

  WrongclChartColors lerp(WrongclChartColors other, double t) {
    return WrongclChartColors(
      grid: Color.lerp(grid, other.grid, t)!,
      gridSubtle: Color.lerp(gridSubtle, other.gridSubtle, t)!,
      upload: Color.lerp(upload, other.upload, t)!,
      download: Color.lerp(download, other.download, t)!,
    );
  }
}
