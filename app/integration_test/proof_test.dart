// M0 proof, run on each real platform: the Rust core loads, money math runs
// in Rust, and two in-app peers exchange a signed message over iroh while an
// outsider is refused.
//
//   flutter test integration_test -d macos   (or windows, linux, an Android device)

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart' show Key, Text;
import 'package:integration_test/integration_test.dart';
import 'package:tillstone/main.dart';
import 'package:tillstone/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('Rust core and peer-to-peer work on this platform', (tester) async {
    await tester.pumpWidget(const TillstoneApp(home: ProofScreen()));
    expect(find.textContaining('3.34 USD + 3.33 USD + 3.33 USD'), findsOneWidget);

    await tester.tap(find.text('Run peer-to-peer self-test'));
    final result = find.textContaining('Outsider refused: yes');
    for (var i = 0; i < 100 && result.evaluate().isEmpty; i++) {
      await tester.pump(const Duration(milliseconds: 100));
    }
    expect(
      result,
      findsOneWidget,
      reason:
          'self-test output: '
          '${tester.widget<Text>(find.byKey(const Key('peer-result'))).data}',
    );
  });
}
