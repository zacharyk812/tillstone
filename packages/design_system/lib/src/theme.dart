import 'package:material_ui/material_ui.dart';

import 'tokens.g.dart';

/// Builds the app theme from tokens, with every platform-dependent behavior
/// pinned so Android, Windows, Linux and macOS look and act the same.
abstract final class PosTheme {
  static ThemeData light() => _build(PosColors.light, Brightness.light);
  static ThemeData dark() => _build(PosColors.dark, Brightness.dark);

  static ThemeData _build(PosColors c, Brightness brightness) {
    TextStyle style(PosTypeStyle t, Color color) => TextStyle(
          fontSize: t.size,
          fontWeight: t.weight,
          height: t.height,
          color: color,
        );

    return ThemeData(
      brightness: brightness,
      // One platform for everyone: no per-OS scroll physics, fonts or menus.
      platform: TargetPlatform.android,
      useMaterial3: true,
      visualDensity: VisualDensity.standard,
      materialTapTargetSize: MaterialTapTargetSize.padded,
      scaffoldBackgroundColor: c.background,
      colorScheme: ColorScheme(
        brightness: brightness,
        primary: c.primary,
        onPrimary: c.onPrimary,
        secondary: c.primary,
        onSecondary: c.onPrimary,
        error: c.danger,
        onError: c.onDanger,
        surface: c.surface,
        onSurface: c.text,
      ),
      pageTransitionsTheme: PageTransitionsTheme(
        builders: <TargetPlatform, PageTransitionsBuilder>{
          for (final p in TargetPlatform.values) p: const FadeForwardsPageTransitionsBuilder(),
        },
      ),
      textTheme: TextTheme(
        headlineMedium: style(PosType.display, c.text),
        titleLarge: style(PosType.title, c.text),
        bodyMedium: style(PosType.body, c.text),
        bodyLarge: style(PosType.body, c.text),
        labelLarge: style(PosType.label, c.text),
        bodySmall: style(PosType.caption, c.textMuted),
      ),
      extensions: [PosThemeColors(c)],
    );
  }
}

/// Gives components direct access to the token colors.
class PosThemeColors extends ThemeExtension<PosThemeColors> {
  const PosThemeColors(this.colors);
  final PosColors colors;

  static PosColors of(BuildContext context) =>
      Theme.of(context).extension<PosThemeColors>()!.colors;

  @override
  PosThemeColors copyWith({PosColors? colors}) => PosThemeColors(colors ?? this.colors);

  @override
  PosThemeColors lerp(PosThemeColors? other, double t) => t < 0.5 ? this : (other ?? this);
}
