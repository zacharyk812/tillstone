/// Test helpers: load the bundled font so golden images show real text
/// instead of placeholder blocks.
library;

import 'dart:io';

import 'package:flutter/services.dart';

import 'src/theme.dart';

/// Loads Inter from [fontsDir] (the design system's `fonts/` folder, relative
/// to the package being tested) under the family name the theme uses.
Future<void> loadPosFonts(String fontsDir) async {
  final loader = FontLoader('packages/${PosTheme.fontPackage}/${PosTheme.fontFamily}');
  for (final weight in ['Regular', 'Medium', 'SemiBold', 'Bold']) {
    final bytes = File('$fontsDir/Inter-$weight.ttf').readAsBytesSync();
    loader.addFont(Future.value(ByteData.view(Uint8List.fromList(bytes).buffer)));
  }
  await loader.load();
  // Icons too, so they don't render as boxes.
  final icons = FontLoader('MaterialIcons');
  final iconFile = _materialIcons();
  if (iconFile != null) {
    icons.addFont(Future.value(ByteData.view(Uint8List.fromList(iconFile.readAsBytesSync()).buffer)));
    await icons.load();
  }
}

File? _materialIcons() {
  final flutterRoot = Platform.environment['FLUTTER_ROOT'];
  if (flutterRoot == null) return null;
  final f = File('$flutterRoot/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf');
  return f.existsSync() ? f : null;
}
