// Golden images of every M1 screen design, at a 10" tablet size in both
// themes. Rendered on macOS in CI (fonts render slightly differently per OS).
//
//   flutter test --update-goldens test/screens_test.dart

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:tillstone/screens/gallery.dart';
import 'package:tillstone_design_system/testing.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

void main() {
  setUpAll(() => loadPosFonts('../packages/design_system/fonts'));

  final screens = ScreenGallery.screens.where((s) => s.$1 != 'M0 proof').toList();

  for (final (name, theme) in [('light', PosTheme.light()), ('dark', PosTheme.dark())]) {
    for (final (title, _, _, build) in screens) {
      final file = title.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]+'), '_').replaceAll(RegExp(r'^_|_$'), '');
      testWidgets('golden: $title ($name)', (tester) async {
        tester.view.physicalSize = const Size(1280, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.reset);
        await tester.pumpWidget(MaterialApp(debugShowCheckedModeBanner: false, theme: theme, home: build()));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull, reason: 'no layout overflow');
        await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/${file}_$name.png'));
      });
    }
  }

  testWidgets('gallery lists every screen', (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(MaterialApp(theme: PosTheme.light(), home: const ScreenGallery()));
    for (final s in ScreenGallery.screens) {
      expect(find.text(s.$1), findsOneWidget);
    }
  });
}
