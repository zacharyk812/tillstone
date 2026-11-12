// Generates lib/src/tokens.g.dart from the repository's tokens/tokens.json.
//
//   dart run tool/generate_tokens.dart          # write the file
//   dart run tool/generate_tokens.dart --check  # fail if it's out of date (CI)

import 'dart:convert';
import 'dart:io';

void main(List<String> args) {
  final source = File('../../tokens/tokens.json');
  final target = File('lib/src/tokens.g.dart');
  final tokens = jsonDecode(source.readAsStringSync()) as Map<String, dynamic>;
  final out = StringBuffer()
    ..writeln('// GENERATED from tokens/tokens.json. Do not edit by hand.')
    ..writeln('// Run `dart run tool/generate_tokens.dart` in packages/design_system.')
    ..writeln()
    ..writeln("import 'dart:ui' show Color, FontWeight;")
    ..writeln();

  final brand = tokens['brand'] as Map<String, dynamic>;
  out
    ..writeln('/// The product as people see it. Forks rename it in tokens.json.')
    ..writeln('abstract final class PosBrandInfo {')
    ..writeln('  static const name = ${_dartString(brand['name'] as String)};')
    ..writeln('}')
    ..writeln();

  final colors = tokens['color'] as Map<String, dynamic>;
  final light = colors['light'] as Map<String, dynamic>;
  out
    ..writeln('/// Colors for one theme.')
    ..writeln('class PosColors {')
    ..writeln('  const PosColors({');
  for (final name in light.keys) {
    out.writeln('    required this.$name,');
  }
  out.writeln('  });');
  for (final name in light.keys) {
    out.writeln('  final Color $name;');
  }
  for (final theme in colors.keys) {
    final values = colors[theme] as Map<String, dynamic>;
    out
      ..writeln()
      ..writeln('  static const $theme = PosColors(');
    for (final name in light.keys) {
      final hex = (values[name] as String).substring(1);
      out.writeln('    $name: Color(0xFF$hex),');
    }
    out.writeln('  );');
  }
  out
    ..writeln('}')
    ..writeln();

  void numbers(String className, String key, String doc) {
    final values = tokens[key] as Map<String, dynamic>;
    out
      ..writeln('/// $doc')
      ..writeln('abstract final class $className {');
    values.forEach((name, value) {
      out.writeln('  static const double $name = ${(value as num).toDouble()};');
    });
    out
      ..writeln('}')
      ..writeln();
  }

  numbers('PosSpacing', 'spacing', 'Spacing on a 4px grid.');
  numbers('PosRadius', 'radius', 'Corner radii.');
  numbers('PosSize', 'size', 'Minimum sizes, e.g. tap targets.');

  final motion = tokens['motion'] as Map<String, dynamic>;
  out
    ..writeln('/// Animation durations.')
    ..writeln('abstract final class PosMotion {');
  motion.forEach((name, value) {
    out.writeln('  static const ${name.replaceAll('Ms', '')} = Duration(milliseconds: $value);');
  });
  out
    ..writeln('}')
    ..writeln();

  final type = tokens['type'] as Map<String, dynamic>;
  out
    ..writeln('/// One step of the type scale.')
    ..writeln('class PosTypeStyle {')
    ..writeln('  const PosTypeStyle(this.size, this.weight, this.height);')
    ..writeln('  final double size;')
    ..writeln('  final FontWeight weight;')
    ..writeln('  final double height;')
    ..writeln('}')
    ..writeln()
    ..writeln('/// The type scale.')
    ..writeln('abstract final class PosType {');
  type.forEach((name, value) {
    final v = value as Map<String, dynamic>;
    out.writeln(
      '  static const $name = PosTypeStyle(${(v['size'] as num).toDouble()}, '
      'FontWeight.w${v['weight']}, ${(v['height'] as num).toDouble()});',
    );
  });
  out.writeln('}');

  final generated = out.toString();
  if (args.contains('--check')) {
    if (!target.existsSync() || target.readAsStringSync() != generated) {
      stderr.writeln('tokens.g.dart is out of date. Run: dart run tool/generate_tokens.dart');
      exit(1);
    }
    return;
  }
  target.writeAsStringSync(generated);
  stdout.writeln('Wrote ${target.path}');
}

/// A single-quoted Dart string literal.
String _dartString(String s) =>
    "'${s.replaceAll('\\', '\\\\').replaceAll("'", "\\'").replaceAll('\$', '\\\$')}'";
