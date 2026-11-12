import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:tillstone_design_system/testing.dart';
import 'package:tillstone_design_system/tillstone_design_system.dart';

Widget _frame(ThemeData theme, Widget child) => MaterialApp(
  debugShowCheckedModeBanner: false,
  theme: theme,
  home: Scaffold(
    body: Padding(padding: const EdgeInsets.all(PosSpacing.lg), child: child),
  ),
);

Widget _gallery() => Column(
  crossAxisAlignment: CrossAxisAlignment.start,
  children: [
    PosButton(label: 'Pay', large: true, onPressed: () {}),
    const SizedBox(height: PosSpacing.sm),
    PosButton(label: 'Add note', kind: PosButtonKind.secondary, onPressed: () {}),
    const SizedBox(height: PosSpacing.sm),
    PosButton(label: 'Void', kind: PosButtonKind.danger, onPressed: () {}),
    const SizedBox(height: PosSpacing.sm),
    const PosButton(label: 'Disabled', onPressed: null),
  ],
);

void main() {
  setUpAll(() => loadPosFonts('fonts'));

  testWidgets('meets minimum tap targets', (tester) async {
    await tester.pumpWidget(
      _frame(
        PosTheme.light(),
        Column(
          children: [
            PosButton(label: 'Small', onPressed: () {}),
            PosButton(label: 'Pay', large: true, onPressed: () {}),
          ],
        ),
      ),
    );
    expect(tester.getSize(find.byType(PosButton).first).height, greaterThanOrEqualTo(48));
    expect(tester.getSize(find.byType(PosButton).last).height, greaterThanOrEqualTo(64));
  });

  testWidgets('calls onPressed and exposes a button to accessibility', (tester) async {
    var taps = 0;
    await tester.pumpWidget(_frame(PosTheme.light(), PosButton(label: 'Pay', onPressed: () => taps++)));
    await tester.tap(find.byType(PosButton));
    expect(taps, 1);
    expect(find.bySemanticsLabel('Pay'), findsOneWidget);
  });

  for (final (name, theme) in [('light', PosTheme.light()), ('dark', PosTheme.dark())]) {
    testWidgets('golden: buttons ($name)', (tester) async {
      tester.view.physicalSize = const Size(400, 360);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(_frame(theme, _gallery()));
      await expectLater(find.byType(MaterialApp), matchesGoldenFile('goldens/buttons_$name.png'));
    });
  }
}
