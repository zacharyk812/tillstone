// GENERATED from tokens/tokens.json. Do not edit by hand.
// Run `dart run tool/generate_tokens.dart` in packages/design_system.

import 'dart:ui' show Color, FontWeight;

/// The product as people see it. Forks rename it in tokens.json.
abstract final class PosBrandInfo {
  static const name = 'Tillstone';
}

/// Colors for one theme.
class PosColors {
  const PosColors({
    required this.background,
    required this.surface,
    required this.surfaceMuted,
    required this.text,
    required this.textMuted,
    required this.border,
    required this.primary,
    required this.onPrimary,
    required this.danger,
    required this.onDanger,
    required this.warning,
    required this.success,
  });
  final Color background;
  final Color surface;
  final Color surfaceMuted;
  final Color text;
  final Color textMuted;
  final Color border;
  final Color primary;
  final Color onPrimary;
  final Color danger;
  final Color onDanger;
  final Color warning;
  final Color success;

  static const light = PosColors(
    background: Color(0xFFFAFAF9),
    surface: Color(0xFFFFFFFF),
    surfaceMuted: Color(0xFFF1F0EE),
    text: Color(0xFF1C1B1A),
    textMuted: Color(0xFF5E5B57),
    border: Color(0xFFDAD7D2),
    primary: Color(0xFF1F5F4A),
    onPrimary: Color(0xFFFFFFFF),
    danger: Color(0xFFB3261E),
    onDanger: Color(0xFFFFFFFF),
    warning: Color(0xFF8A5A00),
    success: Color(0xFF1E6B3A),
  );

  static const dark = PosColors(
    background: Color(0xFF121211),
    surface: Color(0xFF1C1B1A),
    surfaceMuted: Color(0xFF262523),
    text: Color(0xFFF2F1EF),
    textMuted: Color(0xFFB1ADA7),
    border: Color(0xFF3A3835),
    primary: Color(0xFF7FC8AC),
    onPrimary: Color(0xFF0B2A20),
    danger: Color(0xFFF2B8B5),
    onDanger: Color(0xFF601410),
    warning: Color(0xFFF0C36A),
    success: Color(0xFF8FD5A6),
  );
}

/// Spacing on a 4px grid.
abstract final class PosSpacing {
  static const double xxs = 2.0;
  static const double xs = 4.0;
  static const double sm = 8.0;
  static const double md = 12.0;
  static const double lg = 16.0;
  static const double xl = 24.0;
  static const double xxl = 32.0;
}

/// Corner radii.
abstract final class PosRadius {
  static const double sm = 6.0;
  static const double md = 10.0;
  static const double lg = 16.0;
}

/// Minimum sizes, e.g. tap targets.
abstract final class PosSize {
  static const double minTapTarget = 48.0;
  static const double primaryTapTarget = 64.0;
}

/// Animation durations.
abstract final class PosMotion {
  static const fast = Duration(milliseconds: 100);
  static const normal = Duration(milliseconds: 180);
}

/// One step of the type scale.
class PosTypeStyle {
  const PosTypeStyle(this.size, this.weight, this.height);
  final double size;
  final FontWeight weight;
  final double height;
}

/// The type scale.
abstract final class PosType {
  static const display = PosTypeStyle(32.0, FontWeight.w700, 1.2);
  static const title = PosTypeStyle(22.0, FontWeight.w600, 1.25);
  static const body = PosTypeStyle(16.0, FontWeight.w400, 1.4);
  static const label = PosTypeStyle(16.0, FontWeight.w600, 1.2);
  static const caption = PosTypeStyle(13.0, FontWeight.w400, 1.3);
}
