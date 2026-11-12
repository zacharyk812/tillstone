/// Test helpers: load the bundled font so golden images show real text
/// instead of placeholder blocks.
library;

import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

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

/// Compares golden images but tolerates the faint anti-aliasing differences
/// that font rendering produces on different machines.
///
/// A pixel only counts as changed if one of its color channels moves by more
/// than [channelThreshold] (out of 255). Anti-aliasing noise shifts edge
/// pixels slightly; a real change (different text, color or layout) moves
/// at least a few pixels a lot (changing one "i" to an "l" moves about 4).
/// The test fails if more than [maxChangedPixels] pixels changed strongly.
class TolerantGoldenComparator extends LocalFileComparator {
  TolerantGoldenComparator(super.testFile, {this.maxChangedPixels = 0, this.channelThreshold = 96});

  final int maxChangedPixels;
  final int channelThreshold;

  @override
  Future<bool> compare(Uint8List imageBytes, Uri golden) async {
    final goldenBytes = await getGoldenBytes(golden);
    final result = await GoldenFileComparator.compareLists(imageBytes, goldenBytes);
    if (result.passed) {
      result.dispose();
      return true;
    }
    final strong = await _strongDifference(imageBytes, Uint8List.fromList(goldenBytes));
    if (strong != null && strong <= maxChangedPixels) {
      result.dispose();
      return true;
    }
    final error = await generateFailureOutput(result, golden, basedir);
    result.dispose();
    final detail = strong == null ? 'image sizes differ' : '$strong pixels changed strongly';
    throw FlutterError('$error\n$detail (allowed $maxChangedPixels)');
  }

  Future<int?> _strongDifference(Uint8List a, Uint8List b) async {
    final ia = await _decode(a);
    final ib = await _decode(b);
    try {
      if (ia.width != ib.width || ia.height != ib.height) return null;
      final pa = (await ia.toByteData(format: ui.ImageByteFormat.rawRgba))!.buffer.asUint8List();
      final pb = (await ib.toByteData(format: ui.ImageByteFormat.rawRgba))!.buffer.asUint8List();
      var changed = 0;
      for (var i = 0; i < pa.length; i += 4) {
        for (var c = 0; c < 4; c++) {
          if ((pa[i + c] - pb[i + c]).abs() > channelThreshold) {
            changed++;
            break;
          }
        }
      }
      return changed;
    } finally {
      ia.dispose();
      ib.dispose();
    }
  }

  static Future<ui.Image> _decode(Uint8List bytes) async {
    final codec = await ui.instantiateImageCodec(bytes);
    final frame = await codec.getNextFrame();
    codec.dispose();
    return frame.image;
  }
}

/// Installs [TolerantGoldenComparator] for the test file being run. Call it
/// from a package's `test/flutter_test_config.dart`.
void useTolerantGoldens() {
  final current = goldenFileComparator;
  if (current is LocalFileComparator) {
    goldenFileComparator = TolerantGoldenComparator(current.basedir.resolve('placeholder_test.dart'));
  }
}
